use crate::err::Error;
use crate::idx::docids::DocId;
use crate::idx::trees::graph::UndirectedGraph;
use crate::idx::trees::knn::{
	DoublePriorityQueue, Ids64, KnnResult, KnnResultBuilder, PriorityNode,
};
use crate::idx::trees::vector::{SharedVector, Vector};
use crate::kvs::Key;
use crate::sql::index::{Distance, HnswParams, VectorType};
use crate::sql::{Array, Thing, Value};
use radix_trie::Trie;
use rand::prelude::SmallRng;
use rand::{Rng, SeedableRng};
use roaring::RoaringTreemap;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct HnswIndex {
	dim: usize,
	vector_type: VectorType,
	hnsw: Hnsw,
	docs: HnswDocs,
	vec_docs: HashMap<SharedVector, (Ids64, ElementId)>,
}

impl HnswIndex {
	pub fn new(p: &HnswParams) -> Self {
		Self {
			dim: p.dimension as usize,
			vector_type: p.vector_type,
			hnsw: Hnsw::new(p),
			docs: HnswDocs::default(),
			vec_docs: HashMap::default(),
		}
	}

	pub fn index_document(&mut self, rid: &Thing, content: &Vec<Value>) -> Result<(), Error> {
		// Resolve the doc_id
		let doc_id = self.docs.resolve(rid);
		// Index the values
		for value in content {
			// Extract the vector
			let vector = Vector::try_from_value(self.vector_type, self.dim, value)?;
			vector.check_dimension(self.dim)?;
			self.insert(vector.into(), doc_id);
		}
		Ok(())
	}

	fn insert(&mut self, o: SharedVector, d: DocId) {
		match self.vec_docs.entry(o) {
			Entry::Occupied(mut e) => {
				let (docs, element_id) = e.get_mut();
				if let Some(new_docs) = docs.insert(d) {
					let element_id = *element_id;
					e.insert((new_docs, element_id));
				}
			}
			Entry::Vacant(e) => {
				let o = e.key().clone();
				let element_id = self.hnsw.insert(o);
				e.insert((Ids64::One(d), element_id));
			}
		}
	}

	fn remove(&mut self, o: SharedVector, d: DocId) {
		if let Entry::Occupied(mut e) = self.vec_docs.entry(o) {
			let (docs, e_id) = e.get_mut();
			if let Some(new_docs) = docs.remove(d) {
				let e_id = *e_id;
				if new_docs.is_empty() {
					e.remove();
					self.hnsw.remove(e_id);
				} else {
					e.insert((new_docs, e_id));
				}
			}
		}
	}

	pub(crate) fn remove_document(
		&mut self,
		rid: &Thing,
		content: &Vec<Value>,
	) -> Result<(), Error> {
		if let Some(doc_id) = self.docs.remove(rid) {
			for v in content {
				// Extract the vector
				let vector = Vector::try_from_value(self.vector_type, self.dim, v)?;
				vector.check_dimension(self.dim)?;
				// Remove the vector
				self.remove(vector.into(), doc_id);
			}
		}
		Ok(())
	}

	pub fn knn_search(
		&self,
		a: &Array,
		n: usize,
		ef: usize,
	) -> Result<VecDeque<(Thing, f64)>, Error> {
		// Extract the vector
		let vector = Vector::try_from_array(self.vector_type, a)?;
		vector.check_dimension(self.dim)?;
		// Do the search
		let res = self.search(&vector.into(), n, ef);
		Ok(self.result(res))
	}

	fn result(&self, res: KnnResult) -> VecDeque<(Thing, f64)> {
		res.docs
			.into_iter()
			.filter_map(|(doc_id, dist)| self.docs.get(doc_id).map(|t| (t.clone(), dist)))
			.collect()
	}

	fn search(&self, o: &SharedVector, n: usize, ef: usize) -> KnnResult {
		let neighbors = self.hnsw.knn_search(o, n, ef);

		let mut builder = KnnResultBuilder::new(n);
		for (e_dist, e_id) in neighbors {
			if builder.check_add(e_dist) {
				let v = &self.hnsw.elements[&e_id];
				if let Some((docs, _)) = self.vec_docs.get(v) {
					builder.add(e_dist, docs);
				}
			}
		}

		builder.build(
			#[cfg(debug_assertions)]
			HashMap::new(),
		)
	}
}

#[derive(Default)]
struct HnswDocs {
	doc_ids: Trie<Key, DocId>,
	ids_doc: Vec<Option<Thing>>,
	available: RoaringTreemap,
}

impl HnswDocs {
	fn resolve(&mut self, rid: &Thing) -> DocId {
		let doc_key: Key = rid.into();
		if let Some(doc_id) = self.doc_ids.get(&doc_key) {
			*doc_id
		} else {
			let doc_id = self.next_doc_id();
			self.ids_doc.push(Some(rid.clone()));
			self.doc_ids.insert(doc_key, doc_id);
			doc_id
		}
	}

	fn next_doc_id(&mut self) -> DocId {
		if let Some(doc_id) = self.available.iter().next() {
			self.available.remove(doc_id);
			doc_id
		} else {
			self.ids_doc.len() as DocId
		}
	}

	fn get(&self, doc_id: DocId) -> Option<Thing> {
		if let Some(t) = self.ids_doc.get(doc_id as usize) {
			t.clone()
		} else {
			None
		}
	}

	fn remove(&mut self, rid: &Thing) -> Option<DocId> {
		let doc_key: Key = rid.into();
		if let Some(doc_id) = self.doc_ids.remove(&doc_key) {
			let n = doc_id as usize;
			if n < self.ids_doc.len() {
				self.ids_doc[n] = None;
			}
			self.available.insert(doc_id);
			Some(doc_id)
		} else {
			None
		}
	}
}

struct Hnsw {
	m: usize,
	m0: usize,
	efc: usize,
	ml: f64,
	dist: Distance,
	layers: Vec<UndirectedGraph>,
	enter_point: Option<ElementId>,
	elements: HashMap<ElementId, SharedVector>,
	next_element_id: ElementId,
	rng: SmallRng,
	neighbors: SelectNeighbors,
}

pub(super) type ElementId = u64;

impl Hnsw {
	fn new(p: &HnswParams) -> Self {
		Self {
			m: p.m as usize,
			m0: p.m0 as usize,
			efc: p.ef_construction as usize,
			ml: p.ml.to_float(),
			dist: p.distance.clone(),
			enter_point: None,
			layers: Vec::default(),
			elements: HashMap::default(),
			next_element_id: 0,
			rng: SmallRng::from_entropy(),
			neighbors: p.into(),
		}
	}

	fn insert(&mut self, q_pt: SharedVector) -> ElementId {
		let q_level = self.get_random_level();
		self.insert_level(q_pt, q_level)
	}

	fn insert_level(&mut self, q_pt: SharedVector, q_level: usize) -> ElementId {
		let q_id = self.next_element_id;
		let layers = self.layers.len();

		// Be sure we have existing layers
		for l in layers..=q_level {
			let m = if l == 0 {
				self.m0
			} else {
				self.m
			};
			#[cfg(debug_assertions)]
			debug!("Create Layer {l} - m_max: {m}");
			self.layers.push(m.into());
		}

		self.elements.insert(q_id, q_pt.clone());

		if let Some(ep_id) = self.enter_point {
			self.insert_element(q_id, &q_pt, q_level, ep_id, layers - 1);
		} else {
			self.insert_first_element(q_id, q_level);
		}

		self.next_element_id += 1;
		q_id
	}

	fn remove(&mut self, e_id: ElementId) -> bool {
		#[cfg(debug_assertions)]
		debug!("Remove {e_id}");

		let mut removed = false;

		let e_pt = self.elements.get(&e_id).cloned();
		if let Some(e_pt) = e_pt {
			let layers = self.layers.len();
			let mut new_enter_point = None;

			// Are we deleting the current enter point?
			if Some(e_id) == self.enter_point {
				let layer = &self.layers[layers - 1];
				let ep = PriorityNode::new(0.0, e_id);
				new_enter_point = self.search_layer_single_ignore_ep(&e_pt, ep, layer);
			}

			self.elements.remove(&e_id);

			let mut m_max = self.m;

			for lc in (0..layers).rev() {
				if lc == 0 {
					m_max = self.m0;
				}
				if let Some(f_ids) = self.layers[lc].remove_node(&e_id) {
					for q_id in f_ids {
						if let Some(q_pt) = self.elements.get(&q_id) {
							let q_pn = PriorityNode::new(0.0, q_id);
							let layer = &self.layers[lc];
							let c = self.search_layer_multi_ignore_ep(q_pt, q_pn, self.efc, layer);
							let neighbors =
								self.neighbors.select(self, layer, q_id, q_pt, c, m_max);
							assert!(
								!neighbors.contains(&q_id),
								"!neighbors.contains(&q_id) = layer: {lc} - q_id: {q_id} - f_ids: {neighbors:?}"
							);
							self.layers[lc].set_node(q_id, neighbors);
						}
					}
					removed = true;
				}
			}

			if removed && Some(e_id) == self.enter_point {
				self.enter_point = new_enter_point.map(|(_, e_id)| e_id);
			}
		}
		removed
	}

	fn get_random_level(&mut self) -> usize {
		let unif: f64 = self.rng.gen(); // generate a uniform random number between 0 and 1
		(-unif.ln() * self.ml).floor() as usize // calculate the layer
	}

	fn insert_first_element(&mut self, id: ElementId, level: usize) {
		#[cfg(debug_assertions)]
		debug!("insert_first_element - id: {id} - level: {level}");
		for lc in 0..=level {
			self.layers[lc].add_empty_node(id);
		}
		self.enter_point = Some(id);
		#[cfg(debug_assertions)]
		debug!("E - EP: {id}");
	}

	fn insert_element(
		&mut self,
		q_id: ElementId,
		q_pt: &SharedVector,
		q_level: usize,
		ep_id: ElementId,
		top_layer_level: usize,
	) {
		#[cfg(debug_assertions)]
		debug!("insert_element q_pt: {q_pt:?} - q_id: {q_id} - level: {q_level} -  ep_id: {ep_id:?} - top-layer: {top_layer_level}");
		let mut ep = self.get_pn(q_pt, ep_id);
		for lc in ((q_level + 1)..=top_layer_level).rev() {
			ep = self
				.search_layer_single(q_pt, ep, 1, &self.layers[lc])
				.first()
				.unwrap_or_else(|| unreachable!())
				.into()
		}

		let mut m_max = self.m;
		let mut eps = DoublePriorityQueue::from(ep);
		for lc in (0..=top_layer_level.min(q_level)).rev() {
			if lc == 0 {
				m_max = self.m0;
			}

			let w;
			let neighbors;
			{
				let layer = &self.layers[lc];
				w = self.search_layer_multi(q_pt, eps, self.efc, layer);
				eps = w.clone();
				neighbors = self.neighbors.select(self, layer, q_id, q_pt, w, m_max);
			}

			let neighbors = self.layers[lc]
				.add_node(q_id, neighbors)
				.unwrap_or_else(|| unreachable!("add node: {}", q_id));

			for n_id in neighbors {
				let e_conn = self.layers[lc]
					.get_edges(&n_id)
					.unwrap_or_else(|| unreachable!("Element: {}", n_id));
				if e_conn.len() > m_max {
					let n_pt = &self.elements[&n_id];
					let n_c = self.build_priority_list(n_id, e_conn);
					let conn_neighbors =
						self.neighbors.select(self, &self.layers[lc], n_id, n_pt, n_c, m_max);
					self.layers[lc].set_node(n_id, conn_neighbors);
				}
			}
		}

		for lc in (top_layer_level + 1)..=q_level {
			if !self.layers[lc].add_empty_node(q_id) {
				unreachable!("Already there {}", q_id);
			}
		}

		if q_level > top_layer_level {
			self.enter_point = Some(q_id);
			#[cfg(debug_assertions)]
			debug!("E - ep_id: {q_id}");
		}
	}

	fn build_priority_list(
		&self,
		e_id: ElementId,
		neighbors: &HashSet<ElementId>,
	) -> DoublePriorityQueue {
		let e_pt = &self.elements[&e_id];
		let mut w = DoublePriorityQueue::with_capacity(neighbors.len());
		for n_id in neighbors {
			if let Some(n_pt) = self.elements.get(n_id) {
				let dist = self.dist.calculate(e_pt, n_pt);
				w.push(dist, *n_id);
			}
		}
		w
	}

	fn get_pn(&self, q: &SharedVector, e_id: ElementId) -> (f64, u64) {
		let e_pt = &self.elements[&e_id];
		let dist = self.dist.calculate(e_pt, q);
		(dist, e_id)
	}

	fn search_layer_single(
		&self,
		q: &SharedVector,
		ep: (f64, u64),
		ef: usize,
		l: &UndirectedGraph,
	) -> DoublePriorityQueue {
		let visited = HashSet::from([ep.1]);
		let candidates = DoublePriorityQueue::from(ep);
		let w = candidates.clone();
		self.search_layer(q, candidates, visited, w, ef, l)
	}

	/// query element q
	/// enter points ep
	/// number of nearest to q
	/// elements to return ef
	/// layer number lc
	/// Output: ef closest neighbors to q
	fn search_layer_multi(
		&self,
		q: &SharedVector,
		candidates: DoublePriorityQueue,
		ef: usize,
		l: &UndirectedGraph,
	) -> DoublePriorityQueue {
		let mut visited: HashSet<ElementId> = HashSet::with_capacity(candidates.len());
		let mut w = DoublePriorityQueue::with_capacity(candidates.len());
		for (&e_id, dist) in candidates.iter() {
			visited.insert(e_id);
			w.push((*dist).into(), e_id);
		}
		self.search_layer(q, candidates, visited, w, ef, l)
	}

	fn search_layer_single_ignore_ep(
		&self,
		q: &SharedVector,
		ep: PriorityNode,
		l: &UndirectedGraph,
	) -> Option<(f64, u64)> {
		let visited = HashSet::from([ep.doc()]);
		let candidates = DoublePriorityQueue::from(ep);
		let w = candidates.clone();
		let q = self.search_layer(q, candidates, visited, w, 1, l);
		q.first()
	}

	fn search_layer_multi_ignore_ep(
		&self,
		q: &SharedVector,
		ep: PriorityNode,
		ef: usize,
		l: &UndirectedGraph,
	) -> DoublePriorityQueue {
		let candidates = DoublePriorityQueue::from(ep);
		let visited: HashSet<ElementId> = candidates.iter().map(|(&e_id, _)| e_id).collect();
		let w = DoublePriorityQueue::with_capacity(ef);
		self.search_layer(q, candidates, visited, w, ef, l)
	}

	fn search_layer(
		&self,
		q: &SharedVector,
		mut candidates: DoublePriorityQueue,
		mut visited: HashSet<ElementId>,
		mut w: DoublePriorityQueue,
		ef: usize,
		l: &UndirectedGraph,
	) -> DoublePriorityQueue {
		let mut f_dist = w.last().map(|(e_dist, _)| e_dist).unwrap_or_else(|| unreachable!());
		while let Some((dist, doc)) = candidates.pop_first() {
			if dist > f_dist {
				break;
			}
			if let Some(neighbourhood) = l.get_edges(&doc) {
				for &e_id in neighbourhood {
					if visited.insert(e_id) {
						if let Some(e_pt) = self.elements.get(&e_id) {
							let e_dist = self.dist.calculate(e_pt, q);
							if e_dist < f_dist || w.len() < ef {
								candidates.push(e_dist, e_id);
								w.push(e_dist, e_id);
								if w.len() > ef {
									w.pop_last();
								}
								f_dist = w
									.last()
									.map(|(e_dist, _)| e_dist)
									.unwrap_or_else(|| unreachable!());
							}
						}
					}
				}
			}
		}
		w
	}

	fn knn_search(&self, q: &SharedVector, k: usize, efs: usize) -> Vec<(f64, u64)> {
		#[cfg(debug_assertions)]
		let expected_w_len = self.elements.len().min(k);
		if let Some(ep_id) = self.enter_point {
			let mut ep = self.get_pn(q, ep_id);
			let l = self.layers.len();
			for lc in (1..l).rev() {
				ep = self
					.search_layer_single(q, ep, 1, &self.layers[lc])
					.first()
					.unwrap_or_else(|| unreachable!());
			}
			{
				let w = self.search_layer_single(q, ep, efs, &self.layers[0]);
				#[cfg(debug_assertions)]
				if w.len() < expected_w_len {
					debug!(
						"0 search_layer - ep: {ep:?} - ef_search: {efs} - k: {k} - w.len: {} < {expected_w_len}",
						w.len()
					);
				}
				w.into_iter().take(k).map(|(e_id, e_dist)| (e_dist.into(), e_id)).collect()
			}
		} else {
			vec![]
		}
	}
}

#[derive(Debug)]
enum SelectNeighbors {
	Simple,
	Heuristic,
	HeuristicExt,
	HeuristicKeep,
	HeuristicExtKeep,
}

impl From<&HnswParams> for SelectNeighbors {
	fn from(p: &HnswParams) -> Self {
		if p.heuristic {
			if p.keep_pruned_connections {
				if p.extend_candidates {
					Self::HeuristicExtKeep
				} else {
					Self::HeuristicKeep
				}
			} else if p.extend_candidates {
				Self::HeuristicExt
			} else {
				Self::Heuristic
			}
		} else {
			Self::Simple
		}
	}
}

impl SelectNeighbors {
	fn select(
		&self,
		h: &Hnsw,
		lc: &UndirectedGraph,
		q_id: ElementId,
		q_pt: &SharedVector,
		c: DoublePriorityQueue,
		m_max: usize,
	) -> HashSet<ElementId> {
		match self {
			Self::Simple => Self::simple(c, m_max),
			Self::Heuristic => Self::heuristic(c, m_max),
			Self::HeuristicExt => Self::heuristic_ext(h, lc, q_id, q_pt, c, m_max),
			Self::HeuristicKeep => Self::heuristic_keep(c, m_max),
			Self::HeuristicExtKeep => Self::heuristic_ext_keep(h, lc, q_id, q_pt, c, m_max),
		}
	}

	fn simple(w: DoublePriorityQueue, m_max: usize) -> HashSet<ElementId> {
		w.into_iter().take(m_max).map(|(e_id, _)| e_id).collect()
	}

	fn heuristic(mut c: DoublePriorityQueue, m_max: usize) -> HashSet<ElementId> {
		let mut r = HashSet::with_capacity(m_max.min(c.len()));
		let mut closest_neighbors_distance = f64::MAX;
		while let Some((e_dist, e_id)) = c.pop_first() {
			if e_dist < closest_neighbors_distance {
				r.insert(e_id);
				closest_neighbors_distance = e_dist;
				if r.len() >= m_max {
					break;
				}
			}
		}
		r
	}

	fn heuristic_keep(mut c: DoublePriorityQueue, m_max: usize) -> HashSet<ElementId> {
		let mut r = HashSet::with_capacity(m_max.min(c.len()));
		let mut closest_neighbors_distance = f64::INFINITY;
		let mut wd = Vec::new();
		while let Some((e_dist, e_id)) = c.pop_first() {
			if e_dist < closest_neighbors_distance {
				r.insert(e_id);
				closest_neighbors_distance = e_dist;
				if r.len() >= m_max {
					break;
				}
			} else {
				wd.push(e_id);
			}
		}
		let d = (m_max - r.len()).min(wd.len());
		if d > 0 {
			wd.drain(0..d).for_each(|e_id| {
				r.insert(e_id);
			});
		}
		r
	}

	fn extand(
		h: &Hnsw,
		lc: &UndirectedGraph,
		q_id: ElementId,
		q_pt: &SharedVector,
		c: &mut DoublePriorityQueue,
		m_max: usize,
	) {
		let mut ex: HashSet<ElementId> = c.iter().map(|(&e_id, _)| e_id).collect();
		let mut ext = Vec::with_capacity(m_max.min(c.len()));
		for (e_id, _) in c.iter() {
			for &e_adj in
				lc.get_edges(e_id).unwrap_or_else(|| unreachable!("Missing element {}", e_id))
			{
				if e_adj != q_id && ex.insert(e_adj) {
					if let Some(pt) = h.elements.get(&e_adj) {
						ext.push((h.dist.calculate(q_pt, pt), e_adj));
					}
				}
			}
		}
		for (e_dist, e_id) in ext {
			c.push(e_dist, e_id);
		}
	}

	fn heuristic_ext(
		h: &Hnsw,
		lc: &UndirectedGraph,
		q_id: ElementId,
		q_pt: &SharedVector,
		mut c: DoublePriorityQueue,
		m_max: usize,
	) -> HashSet<ElementId> {
		Self::extand(h, lc, q_id, q_pt, &mut c, m_max);
		Self::heuristic(c, m_max)
	}

	fn heuristic_ext_keep(
		h: &Hnsw,
		lc: &UndirectedGraph,
		q_id: ElementId,
		q_pt: &SharedVector,
		mut c: DoublePriorityQueue,
		m_max: usize,
	) -> HashSet<ElementId> {
		Self::extand(h, lc, q_id, q_pt, &mut c, m_max);
		Self::heuristic_keep(c, m_max)
	}
}

#[cfg(test)]
mod tests {
	use crate::err::Error;
	use crate::idx::docids::DocId;
	use crate::idx::trees::hnsw::{Hnsw, HnswIndex};
	use crate::idx::trees::knn::tests::{new_vectors_from_file, TestCollection};
	use crate::idx::trees::knn::{Ids64, KnnResult, KnnResultBuilder};
	use crate::idx::trees::vector::{SharedVector, Vector};
	use crate::sql::index::{Distance, HnswParams, VectorType};
	use roaring::RoaringTreemap;
	use serial_test::serial;
	use std::collections::hash_map::Entry;
	use std::collections::{HashMap, HashSet};

	fn insert_collection_hnsw(
		h: &mut Hnsw,
		collection: &TestCollection<SharedVector>,
	) -> HashSet<SharedVector> {
		let mut set = HashSet::new();
		for (_, obj) in collection.as_ref() {
			let obj: SharedVector = obj.clone().into();
			h.insert(obj.clone());
			set.insert(obj);
			check_hnsw_properties(h, set.len());
			h.debug_print_check();
		}
		set
	}
	fn find_collection_hnsw(h: &mut Hnsw, collection: &TestCollection<SharedVector>) {
		let max_knn = 20.min(collection.as_ref().len());
		for (_, obj) in collection.as_ref() {
			let obj = obj.clone().into();
			for knn in 1..max_knn {
				let res = h.knn_search(&obj, knn, 80);
				if collection.is_unique() {
					let mut found = false;
					for (_, e_id) in &res {
						if h.elements[&e_id].eq(&obj) {
							found = true;
							break;
						}
					}
					assert!(
						found,
						"Search: {:?} - Knn: {} - Vector not found - Got: {:?} - Dist: {} - Coll: {}",
						obj,
						knn,
						res,
						h.dist,
						collection.as_ref().len(),
					);
				}
				let expected_len = collection.as_ref().len().min(knn);
				if expected_len != res.len() {
					info!("expected_len != res.len()")
				}
				assert_eq!(
					expected_len,
					res.len(),
					"Wrong knn count - Expected: {} - Got: {} - Collection: {} - Dist: {} - Res: {:?}",
					expected_len,
					res.len(),
					collection.as_ref().len(),
					h.dist,
					res,
				)
			}
		}
	}

	fn test_hnsw_collection(p: &HnswParams, collection: &TestCollection<SharedVector>) {
		let mut h = Hnsw::new(p);
		insert_collection_hnsw(&mut h, collection);
		find_collection_hnsw(&mut h, &collection);
	}

	fn new_params(
		dimension: usize,
		vector_type: VectorType,
		distance: Distance,
		m: usize,
		efc: usize,
		heuristic: bool,
		extend_candidates: bool,
		keep_pruned_connections: bool,
	) -> HnswParams {
		let m = m as u16;
		let m0 = m * 2;
		HnswParams {
			dimension: dimension as u16,
			distance,
			vector_type,
			m,
			m0,
			ef_construction: efc as u16,
			ml: (1.0 / (m as f64).ln()).into(),
			heuristic,
			extend_candidates,
			keep_pruned_connections,
		}
	}

	fn test_hnsw(
		distance: Distance,
		vt: VectorType,
		collection_size: usize,
		dimension: usize,
		m: usize,
		extend_candidates: bool,
		keep_pruned_connections: bool,
	) {
		info!("test_hnsw - dist: {distance} - type: {vt} - coll size: {collection_size} - dim: {dimension} - m: {m} - ext: {extend_candidates} - keep: {keep_pruned_connections}");
		let collection = TestCollection::new(true, collection_size, vt, dimension, &distance);
		let params = new_params(
			dimension,
			vt,
			distance,
			m,
			500,
			true,
			extend_candidates,
			keep_pruned_connections,
		);
		test_hnsw_collection(&params, &collection);
	}

	#[test_log::test]
	#[serial]
	fn test_hnsw_xs() {
		for d in [
			Distance::Chebyshev,
			Distance::Cosine,
			Distance::Euclidean,
			Distance::Hamming,
			Distance::Jaccard,
			Distance::Manhattan,
			Distance::Minkowski(2.into()),
			Distance::Pearson,
		] {
			for vt in [
				VectorType::F64,
				VectorType::F32,
				VectorType::I64,
				VectorType::I32,
				VectorType::I16,
			] {
				for extend in [false, true] {
					for keep in [false, true] {
						test_hnsw(d.clone(), vt, 30, 3, 12, extend, keep);
					}
				}
			}
		}
	}

	#[test_log::test]
	#[serial]
	fn test_hnsw_small_euclidean_check() {
		test_hnsw(Distance::Euclidean, VectorType::F64, 100, 2, 24, true, true)
	}

	#[test_log::test]
	#[serial]
	fn test_hnsw_small() {
		for d in [
			Distance::Chebyshev,
			Distance::Cosine,
			Distance::Euclidean,
			Distance::Hamming,
			Distance::Jaccard,
			Distance::Manhattan,
			Distance::Minkowski(2.into()),
			Distance::Pearson,
		] {
			for vt in [
				VectorType::F64,
				VectorType::F32,
				VectorType::I64,
				VectorType::I32,
				VectorType::I16,
			] {
				for extend in [false, true] {
					for keep in [false, true] {
						test_hnsw(d.clone(), vt, 200, 5, 12, extend, keep);
					}
				}
			}
		}
	}

	#[test_log::test]
	#[serial]
	fn test_hnsw_large_euclidean() {
		test_hnsw(Distance::Euclidean, VectorType::F64, 200, 5, 12, false, false)
	}

	fn insert_collection_hnsw_index(
		h: &mut HnswIndex,
		collection: &TestCollection<SharedVector>,
	) -> HashMap<SharedVector, HashSet<DocId>> {
		let mut map: HashMap<SharedVector, HashSet<DocId>> = HashMap::new();
		for (doc_id, obj) in collection.as_ref() {
			let obj: SharedVector = obj.clone().into();
			h.insert(obj.clone(), *doc_id);
			match map.entry(obj) {
				Entry::Occupied(mut e) => {
					e.get_mut().insert(*doc_id);
				}
				Entry::Vacant(e) => {
					e.insert(HashSet::from([*doc_id]));
				}
			}
			check_hnsw_properties(&h.hnsw, map.len());
		}
		map
	}

	fn find_collection_hnsw_index(h: &mut HnswIndex, collection: &TestCollection<SharedVector>) {
		let max_knn = 20.min(collection.as_ref().len());
		for (doc_id, obj) in collection.as_ref() {
			for knn in 1..max_knn {
				let obj: SharedVector = obj.clone().into();
				let res = h.search(&obj, knn, 500);
				if knn == 1 && res.docs.len() == 1 && res.docs[0].1 > 0.0 {
					let docs: Vec<DocId> = res.docs.iter().map(|(d, _)| *d).collect();
					if collection.is_unique() {
						assert!(
							docs.contains(doc_id),
							"Search: {:?} - Knn: {} - Wrong Doc - Expected: {} - Got: {:?}",
							obj,
							knn,
							doc_id,
							res.docs
						);
					}
				}
				let expected_len = collection.as_ref().len().min(knn);
				assert_eq!(
					expected_len,
					res.docs.len(),
					"Wrong knn count - Expected: {} - Got: {} - - Docs: {:?} - Collection: {}",
					expected_len,
					res.docs.len(),
					res.docs,
					collection.as_ref().len(),
				)
			}
		}
	}

	fn delete_hnsw_index_collection(
		h: &mut HnswIndex,
		collection: &TestCollection<SharedVector>,
		mut map: HashMap<SharedVector, HashSet<DocId>>,
	) {
		for (doc_id, obj) in collection.as_ref() {
			let obj: SharedVector = obj.clone().into();
			h.remove(obj.clone(), *doc_id);
			if let Entry::Occupied(mut e) = map.entry(obj.clone()) {
				let set = e.get_mut();
				set.remove(doc_id);
				if set.is_empty() {
					e.remove();
				}
			}
			check_hnsw_properties(&h.hnsw, map.len());
		}
	}

	fn test_hnsw_index(
		distance: Distance,
		vt: VectorType,
		collection_size: usize,
		dimension: usize,
		unique: bool,
		m: usize,
		heuristic: bool,
		extend_candidates: bool,
		keep_pruned_connections: bool,
	) {
		info!("test_hnsw_index - dist: {distance} - type: {vt} - coll size: {collection_size} - dim: {dimension} - unique: {unique} - m: {m} - ext: {extend_candidates} - keep: {keep_pruned_connections}");
		let collection = TestCollection::new(unique, collection_size, vt, dimension, &distance);
		let p = new_params(
			dimension,
			vt,
			distance,
			m,
			500,
			heuristic,
			extend_candidates,
			keep_pruned_connections,
		);
		let mut h = HnswIndex::new(&p);
		let map = insert_collection_hnsw_index(&mut h, &collection);
		find_collection_hnsw_index(&mut h, &collection);
		delete_hnsw_index_collection(&mut h, &collection, map);
	}

	#[test_log::test]
	#[serial]
	fn test_hnsw_index_xs() {
		for d in [
			Distance::Chebyshev,
			Distance::Cosine,
			Distance::Euclidean,
			Distance::Hamming,
			Distance::Jaccard,
			Distance::Manhattan,
			Distance::Minkowski(2.into()),
			Distance::Pearson,
		] {
			for vt in [
				VectorType::F64,
				VectorType::F32,
				VectorType::I64,
				VectorType::I32,
				VectorType::I16,
			] {
				for unique in [false, true] {
					test_hnsw_index(d.clone(), vt, 30, 2, unique, 12, true, true, true);
				}
			}
		}
	}

	#[test_log::test]
	#[serial]
	fn test_building() {
		let p = new_params(2, VectorType::I16, Distance::Euclidean, 2, 500, true, true, true);
		let mut hnsw = Hnsw::new(&p);
		assert_eq!(hnsw.elements.len(), 0);
		assert_eq!(hnsw.enter_point, None);
		assert_eq!(hnsw.layers.len(), 0);

		let a_vec = new_i16_vec(1, 1);
		let a0 = hnsw.insert_level(a_vec.clone(), 0);
		assert_eq!(hnsw.elements.len(), 1);
		assert_eq!(hnsw.enter_point, Some(a0));
		assert_eq!(hnsw.layers.len(), 1);
		hnsw.layers[0].check(vec![(a0, vec![])]);

		let b1 = hnsw.insert_level(new_i16_vec(2, 2), 0);
		assert_eq!(hnsw.elements.len(), 2);
		assert_eq!(hnsw.enter_point, Some(a0));
		assert_eq!(hnsw.layers.len(), 1);
		hnsw.layers[0].check(vec![(a0, vec![b1]), (b1, vec![a0])]);

		let c2 = hnsw.insert_level(new_i16_vec(3, 3), 0);
		assert_eq!(hnsw.elements.len(), 3);
		assert_eq!(hnsw.enter_point, Some(a0));
		assert_eq!(hnsw.layers.len(), 1);
		hnsw.layers[0].check(vec![(a0, vec![b1, c2]), (b1, vec![a0, c2]), (c2, vec![b1, a0])]);

		let d3 = hnsw.insert_level(new_i16_vec(4, 4), 1);
		assert_eq!(hnsw.elements.len(), 4);
		assert_eq!(hnsw.enter_point, Some(d3));
		assert_eq!(hnsw.layers.len(), 2);
		hnsw.layers[1].check(vec![(d3, vec![])]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3]),
			(b1, vec![a0, c2, d3]),
			(c2, vec![b1, a0, d3]),
			(d3, vec![c2, b1, a0]),
		]);

		let e4 = hnsw.insert_level(new_i16_vec(5, 5), 2);
		assert_eq!(hnsw.elements.len(), 5);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![])]);
		hnsw.layers[1].check(vec![(d3, vec![e4]), (e4, vec![d3])]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, a0]),
			(e4, vec![d3, c2, b1, a0]),
		]);

		let f5 = hnsw.insert_level(new_i16_vec(6, 6), 2);
		assert_eq!(hnsw.elements.len(), 6);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![f5]), (f5, vec![e4])]);
		hnsw.layers[1].check(vec![(d3, vec![e4, f5]), (e4, vec![d3, f5]), (f5, vec![e4, d3])]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, f5]),
			(e4, vec![d3, f5, c2, b1]),
			(f5, vec![e4, d3, c2, b1]),
		]);

		let g6 = hnsw.insert_level(new_i16_vec(7, 7), 1);
		assert_eq!(hnsw.elements.len(), 7);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![f5]), (f5, vec![e4])]);
		hnsw.layers[1].check(vec![
			(d3, vec![e4, f5]),
			(e4, vec![d3, f5]),
			(f5, vec![e4, g6]),
			(g6, vec![f5, e4]),
		]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, f5]),
			(e4, vec![d3, f5, c2, g6]),
			(f5, vec![e4, g6, d3, c2]),
			(g6, vec![f5, e4, d3, c2]),
		]);

		let h7 = hnsw.insert_level(new_i16_vec(8, 8), 0);
		assert_eq!(hnsw.elements.len(), 8);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![f5]), (f5, vec![e4])]);
		hnsw.layers[1].check(vec![
			(d3, vec![e4, f5]),
			(e4, vec![d3, f5]),
			(f5, vec![e4, g6]),
			(g6, vec![f5, e4]),
		]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, f5]),
			(e4, vec![d3, f5, c2, g6]),
			(f5, vec![e4, g6, d3, h7]),
			(g6, vec![f5, h7, e4, d3]),
			(h7, vec![g6, f5, e4, d3]),
		]);

		let i8 = hnsw.insert_level(new_i16_vec(9, 9), 0);
		assert_eq!(hnsw.elements.len(), 9);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![f5]), (f5, vec![e4])]);
		hnsw.layers[1].check(vec![
			(d3, vec![e4, f5]),
			(e4, vec![d3, f5]),
			(f5, vec![e4, g6]),
			(g6, vec![f5, e4]),
		]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, f5]),
			(e4, vec![d3, f5, c2, g6]),
			(f5, vec![e4, g6, d3, h7]),
			(g6, vec![f5, h7, e4, i8]),
			(h7, vec![g6, i8, f5, e4]),
			(i8, vec![h7, g6, f5, e4]),
		]);

		let j9 = hnsw.insert_level(new_i16_vec(10, 10), 0);
		assert_eq!(hnsw.elements.len(), 10);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![f5]), (f5, vec![e4])]);
		hnsw.layers[1].check(vec![
			(d3, vec![e4, f5]),
			(e4, vec![d3, f5]),
			(f5, vec![e4, g6]),
			(g6, vec![f5, e4]),
		]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, f5]),
			(e4, vec![d3, f5, c2, g6]),
			(f5, vec![e4, g6, d3, h7]),
			(g6, vec![f5, h7, e4, i8]),
			(h7, vec![g6, i8, f5, j9]),
			(i8, vec![h7, j9, g6, f5]),
			(j9, vec![i8, h7, g6, f5]),
		]);

		let h10 = hnsw.insert_level(new_i16_vec(11, 11), 1);
		assert_eq!(hnsw.elements.len(), 11);
		assert_eq!(hnsw.enter_point, Some(e4));
		assert_eq!(hnsw.layers.len(), 3);
		hnsw.layers[2].check(vec![(e4, vec![f5]), (f5, vec![e4])]);
		hnsw.layers[1].check(vec![
			(d3, vec![e4, f5]),
			(e4, vec![d3, f5]),
			(f5, vec![e4, g6]),
			(g6, vec![f5, e4]),
			(h10, vec![g6, f5]),
		]);
		hnsw.layers[0].check(vec![
			(a0, vec![b1, c2, d3, e4]),
			(b1, vec![a0, c2, d3, e4]),
			(c2, vec![b1, d3, a0, e4]),
			(d3, vec![c2, e4, b1, f5]),
			(e4, vec![d3, f5, c2, g6]),
			(f5, vec![e4, g6, d3, h7]),
			(g6, vec![f5, h7, e4, i8]),
			(h7, vec![g6, i8, f5, j9]),
			(i8, vec![h7, j9, g6, h10]),
			(j9, vec![i8, h10, h7, g6]),
			(h10, vec![j9, i8, h7, g6]),
		]);
	}

	#[test_log::test]
	#[serial]
	fn test_invalid_size() {
		let collection = TestCollection::Unique(vec![
			(0, new_i16_vec(-2, -3)),
			(1, new_i16_vec(-2, 1)),
			(2, new_i16_vec(-4, 3)),
			(3, new_i16_vec(-3, 1)),
			(4, new_i16_vec(-1, 1)),
			(5, new_i16_vec(-2, 3)),
			(6, new_i16_vec(3, 0)),
			(7, new_i16_vec(-1, -2)),
			(8, new_i16_vec(-2, 2)),
			(9, new_i16_vec(-4, -2)),
			(10, new_i16_vec(0, 3)),
		]);
		let p = new_params(2, VectorType::I16, Distance::Euclidean, 3, 500, true, true, true);
		let mut h = Hnsw::new(&p);
		insert_collection_hnsw(&mut h, &collection);
		let pt = new_i16_vec(-2, -3);
		let knn = 10;
		let efs = 501;
		let hnsw_res = h.knn_search(&pt, knn, efs);
		assert_eq!(hnsw_res.len(), knn);
		// let brute_force_res = collection.knn(&pt, Distance::Euclidean, knn);
		// let recall = brute_force_res.recall(&hnsw_res);
		// assert_eq!(1.0, recall);
	}

	#[test_log::test]
	#[serial]
	fn test_recall() -> Result<(), Error> {
		let (dim, vt, m) = (20, VectorType::F32, 24);
		info!("Build data collection");
		let collection: TestCollection<SharedVector> =
			TestCollection::NonUnique(new_vectors_from_file(
				VectorType::F32,
				"../tests/data/hnsw-random-9000-20-euclidean.gz",
			)?);
		let p = new_params(dim, vt, Distance::Euclidean, m, 500, false, false, false);
		let mut h = HnswIndex::new(&p);
		info!("Insert collection");
		for (doc_id, obj) in collection.as_ref() {
			h.insert(obj.clone(), *doc_id);
		}

		info!("Build query collection");
		let queries = TestCollection::NonUnique(new_vectors_from_file(
			VectorType::F32,
			"../tests/data/hnsw-random-5000-20-euclidean.gz",
		)?);

		info!("Check recall");
		for (efs, expected_recall) in [(10, 0.82), (80, 0.87)] {
			let mut total_recall = 0.0;
			for (_, pt) in queries.as_ref() {
				let knn = 10;
				let hnsw_res = h.search(pt, knn, efs);
				assert_eq!(
					hnsw_res.docs.len(),
					knn,
					"Different size - knn: {knn} - efs: {efs} - doc: {:?}",
					collection.as_ref().len()
				);
				let brute_force_res = collection.knn(pt, Distance::Euclidean, knn);
				let rec = brute_force_res.recall(&hnsw_res);
				// assert_eq!(brute_force_res.docs, hnsw_res.docs);
				total_recall += rec;
			}
			let recall = total_recall / queries.as_ref().len() as f64;
			info!("EFS: {efs} - Recall: {recall}");
			assert!(
				recall >= expected_recall,
				"Recall: {} - Expected: {}",
				recall,
				expected_recall
			);
		}
		Ok(())
	}

	fn check_hnsw_properties(h: &Hnsw, expected_count: usize) {
		// let mut deleted_foreign_elements = 0;
		// let mut foreign_elements = 0;
		let mut layer_size = h.elements.len();
		assert_eq!(layer_size, expected_count);
		for (lc, l) in h.layers.iter().enumerate() {
			assert!(l.len() <= layer_size, "{} - {}", l.len(), layer_size);
			layer_size = l.len();
			let m_layer = if lc == 0 {
				h.m0
			} else {
				h.m
			};
			for (e_id, f_ids) in l.nodes() {
				assert!(f_ids.len() <= m_layer, "Foreign list len");
				assert!(
					!f_ids.contains(e_id),
					"!f_ids.contains(e_id) = layer: {lc} - el: {e_id} - f_ids: {f_ids:?}"
				);
				assert!(
					h.elements.contains_key(e_id),
					"h.elements.contains_key(e_id) - layer: {lc} - el: {e_id} - f_ids: {f_ids:?}"
				);

				// for f_id in f_ids {
				// 	if !h.elements.contains_key(f_id) {
				// 		deleted_foreign_elements += 1;
				// 	}
				// }
				// foreign_elements += f_ids.len();
			}
		}
		// if deleted_foreign_elements > 0 && deleted_foreign_elements > 0 {
		// 	let miss_rate = deleted_foreign_elements as f64 / foreign_elements as f64;
		// 	assert!(miss_rate < 0.5, "Miss rate: {miss_rate}");
		// }
	}

	impl TestCollection<SharedVector> {
		fn knn(&self, pt: &SharedVector, dist: Distance, n: usize) -> KnnResult {
			let mut b = KnnResultBuilder::new(n);
			for (doc_id, doc_pt) in self.as_ref() {
				let d = dist.calculate(doc_pt, pt);
				if b.check_add(d) {
					b.add(d, &Ids64::One(*doc_id));
				}
			}
			b.build(
				#[cfg(debug_assertions)]
				HashMap::new(),
			)
		}
	}

	impl KnnResult {
		fn recall(&self, res: &KnnResult) -> f64 {
			let mut bits = RoaringTreemap::new();
			for &(doc_id, _) in &self.docs {
				bits.insert(doc_id);
			}
			let mut found = 0;
			for &(doc_id, _) in &res.docs {
				if bits.contains(doc_id) {
					found += 1;
				}
			}
			found as f64 / bits.len() as f64
		}
	}

	fn new_i16_vec(x: isize, y: isize) -> SharedVector {
		let mut vec = Vector::new(VectorType::I16, 2);
		vec.add(&x.into());
		vec.add(&y.into());
		vec.into()
	}

	impl Hnsw {
		fn debug_print_check(&self) {
			debug!("EP: {:?}", self.enter_point);
			for (i, l) in self.layers.iter().enumerate() {
				debug!("LAYER {i} - len: {}", l.len());
				let m_max = if i == 0 {
					self.m0
				} else {
					self.m
				};
				for f in l.nodes().values() {
					assert!(f.len() <= m_max);
				}
			}
		}
	}
}
