use crate::{
	sql::{language::Language, Algorithm},
	syn::v2::token::{DistanceKind, Keyword, TokenKind},
};
use phf::phf_map;
use unicase::UniCase;

/// A map for mapping keyword strings to a tokenkind,
pub(crate) static KEYWORDS: phf::Map<UniCase<&'static str>, Option<TokenKind>> = phf_map! {
	// Keywords
	UniCase::ascii("AFTER") => Some(TokenKind::Keyword(Keyword::After)),
	UniCase::ascii("ALL") => Some(TokenKind::Keyword(Keyword::All)),
	UniCase::ascii("ANALYZE") => Some(TokenKind::Keyword(Keyword::Analyze)),
	UniCase::ascii("ANALYZER") => Some(TokenKind::Keyword(Keyword::Analyzer)),
	UniCase::ascii("AS") => Some(TokenKind::Keyword(Keyword::As)),
	UniCase::ascii("ASCENDING") => Some(TokenKind::Keyword(Keyword::Ascending)),
	UniCase::ascii("ASC") => Some(TokenKind::Keyword(Keyword::Ascending)),
	UniCase::ascii("ASCII") => Some(TokenKind::Keyword(Keyword::Ascii)),
	UniCase::ascii("ASSERT") => Some(TokenKind::Keyword(Keyword::Assert)),
	UniCase::ascii("AT") => Some(TokenKind::Keyword(Keyword::At)),
	UniCase::ascii("BEFORE") => Some(TokenKind::Keyword(Keyword::Before)),
	UniCase::ascii("BEGIN") => Some(TokenKind::Keyword(Keyword::Begin)),
	UniCase::ascii("BLANK") => Some(TokenKind::Keyword(Keyword::Blank)),
	UniCase::ascii("BM25") => Some(TokenKind::Keyword(Keyword::Bm25)),
	UniCase::ascii("BREAK") => Some(TokenKind::Keyword(Keyword::Break)),
	UniCase::ascii("BY") => Some(TokenKind::Keyword(Keyword::By)),
	UniCase::ascii("CAMEL") => Some(TokenKind::Keyword(Keyword::Camel)),
	UniCase::ascii("CANCEL") => Some(TokenKind::Keyword(Keyword::Cancel)),
	UniCase::ascii("CHANGEFEED") => Some(TokenKind::Keyword(Keyword::ChangeFeed)),
	UniCase::ascii("CHANGES") => Some(TokenKind::Keyword(Keyword::Changes)),
	UniCase::ascii("CAPACITY") => Some(TokenKind::Keyword(Keyword::Capacity)),
	UniCase::ascii("CLASS") => Some(TokenKind::Keyword(Keyword::Class)),
	UniCase::ascii("COMMENT") => Some(TokenKind::Keyword(Keyword::Comment)),
	UniCase::ascii("COMMIT") => Some(TokenKind::Keyword(Keyword::Commit)),
	UniCase::ascii("CONTENT") => Some(TokenKind::Keyword(Keyword::Content)),
	UniCase::ascii("CONTINUE") => Some(TokenKind::Keyword(Keyword::Continue)),
	UniCase::ascii("CREATE") => Some(TokenKind::Keyword(Keyword::Create)),
	UniCase::ascii("DATABASE") => Some(TokenKind::Keyword(Keyword::Database)),
	UniCase::ascii("DB") => Some(TokenKind::Keyword(Keyword::Database)),
	UniCase::ascii("DEFAULT") => Some(TokenKind::Keyword(Keyword::Default)),
	UniCase::ascii("DEFINE") => Some(TokenKind::Keyword(Keyword::Define)),
	UniCase::ascii("DELETE") => Some(TokenKind::Keyword(Keyword::Delete)),
	UniCase::ascii("DESCENDING") => Some(TokenKind::Keyword(Keyword::Descending)),
	UniCase::ascii("DESC") => Some(TokenKind::Keyword(Keyword::Descending)),
	UniCase::ascii("DIFF") => Some(TokenKind::Keyword(Keyword::Diff)),
	UniCase::ascii("DIMENSION") => Some(TokenKind::Keyword(Keyword::Dimension)),
	UniCase::ascii("DISTANCE") => Some(TokenKind::Keyword(Keyword::Distance)),
	UniCase::ascii("DIST") => Some(TokenKind::Keyword(Keyword::Distance)),
	UniCase::ascii("DOC_IDS_CACHE") => Some(TokenKind::Keyword(Keyword::DocIdsCache)),
	UniCase::ascii("DOC_IDS_ORDER") => Some(TokenKind::Keyword(Keyword::DocIdsOrder)),
	UniCase::ascii("DOC_LENGTHS_CACHE") => Some(TokenKind::Keyword(Keyword::DocLengthsCache)),
	UniCase::ascii("DOC_LENGTHS_ORDER") => Some(TokenKind::Keyword(Keyword::DocLengthsOrder)),
	UniCase::ascii("DROP") => Some(TokenKind::Keyword(Keyword::Drop)),
	UniCase::ascii("DUPLICATE") => Some(TokenKind::Keyword(Keyword::Duplicate)),
	UniCase::ascii("EDGENGRAM") => Some(TokenKind::Keyword(Keyword::Edgengram)),
	UniCase::ascii("EVENT") => Some(TokenKind::Keyword(Keyword::Event)),
	UniCase::ascii("ELSE") => Some(TokenKind::Keyword(Keyword::Else)),
	UniCase::ascii("END") => Some(TokenKind::Keyword(Keyword::End)),
	UniCase::ascii("EXISTS") => Some(TokenKind::Keyword(Keyword::Exists)),
	UniCase::ascii("EXPLAIN") => Some(TokenKind::Keyword(Keyword::Explain)),
	UniCase::ascii("false") => Some(TokenKind::Keyword(Keyword::False)),
	UniCase::ascii("FETCH") => Some(TokenKind::Keyword(Keyword::Fetch)),
	UniCase::ascii("FIELD") => Some(TokenKind::Keyword(Keyword::Field)),
	UniCase::ascii("FIELDS") => Some(TokenKind::Keyword(Keyword::Fields)),
	UniCase::ascii("COLUMNS") => Some(TokenKind::Keyword(Keyword::Fields)),
	UniCase::ascii("FILTERS") => Some(TokenKind::Keyword(Keyword::Filters)),
	UniCase::ascii("FLEXIBLE") => Some(TokenKind::Keyword(Keyword::Flexible)),
	UniCase::ascii("FLEXI") => Some(TokenKind::Keyword(Keyword::Flexible)),
	UniCase::ascii("FLEX") => Some(TokenKind::Keyword(Keyword::Flexible)),
	UniCase::ascii("FOR") => Some(TokenKind::Keyword(Keyword::For)),
	UniCase::ascii("FROM") => Some(TokenKind::Keyword(Keyword::From)),
	UniCase::ascii("FULL") => Some(TokenKind::Keyword(Keyword::Full)),
	UniCase::ascii("FUNCTION") => Some(TokenKind::Keyword(Keyword::Function)),
	UniCase::ascii("GROUP") => Some(TokenKind::Keyword(Keyword::Group)),
	UniCase::ascii("HIGHLIGHTS") => Some(TokenKind::Keyword(Keyword::Highlights)),
	UniCase::ascii("IGNORE") => Some(TokenKind::Keyword(Keyword::Ignore)),
	UniCase::ascii("INDEX") => Some(TokenKind::Keyword(Keyword::Index)),
	UniCase::ascii("INFO") => Some(TokenKind::Keyword(Keyword::Info)),
	UniCase::ascii("INSERT") => Some(TokenKind::Keyword(Keyword::Insert)),
	UniCase::ascii("INTO") => Some(TokenKind::Keyword(Keyword::Into)),
	UniCase::ascii("IF") => Some(TokenKind::Keyword(Keyword::If)),
	UniCase::ascii("IS") => Some(TokenKind::Keyword(Keyword::Is)),
	UniCase::ascii("KEY") => Some(TokenKind::Keyword(Keyword::Key)),
	UniCase::ascii("KILL") => Some(TokenKind::Keyword(Keyword::Kill)),
	UniCase::ascii("KNN") => Some(TokenKind::Keyword(Keyword::Knn)),
	UniCase::ascii("LET") => Some(TokenKind::Keyword(Keyword::Let)),
	UniCase::ascii("LIMIT") => Some(TokenKind::Keyword(Keyword::Limit)),
	UniCase::ascii("LIVE") => Some(TokenKind::Keyword(Keyword::Live)),
	UniCase::ascii("LOWERCASE") => Some(TokenKind::Keyword(Keyword::Lowercase)),
	UniCase::ascii("MERGE") => Some(TokenKind::Keyword(Keyword::Merge)),
	UniCase::ascii("MODEL") => Some(TokenKind::Keyword(Keyword::Model)),
	UniCase::ascii("MTREE") => Some(TokenKind::Keyword(Keyword::MTree)),
	UniCase::ascii("MTREE_CACHE") => Some(TokenKind::Keyword(Keyword::MTreeCache)),
	UniCase::ascii("NAMESPACE") => Some(TokenKind::Keyword(Keyword::Namespace)),
	UniCase::ascii("NS") => Some(TokenKind::Keyword(Keyword::Namespace)),
	UniCase::ascii("NGRAM") => Some(TokenKind::Keyword(Keyword::Ngram)),
	UniCase::ascii("NO") => Some(TokenKind::Keyword(Keyword::No)),
	UniCase::ascii("NOINDEX") => Some(TokenKind::Keyword(Keyword::NoIndex)),
	UniCase::ascii("NONE") => Some(TokenKind::Keyword(Keyword::None)),
	UniCase::ascii("NULL") => Some(TokenKind::Keyword(Keyword::Null)),
	UniCase::ascii("NUMERIC") => Some(TokenKind::Keyword(Keyword::Numeric)),
	UniCase::ascii("OMIT") => Some(TokenKind::Keyword(Keyword::Omit)),
	UniCase::ascii("ON") => Some(TokenKind::Keyword(Keyword::On)),
	UniCase::ascii("ONLY") => Some(TokenKind::Keyword(Keyword::Only)),
	UniCase::ascii("OPTION") => Some(TokenKind::Keyword(Keyword::Option)),
	UniCase::ascii("ORDER") => Some(TokenKind::Keyword(Keyword::Order)),
	UniCase::ascii("PARALLEL") => Some(TokenKind::Keyword(Keyword::Parallel)),
	UniCase::ascii("PARAM") => Some(TokenKind::Keyword(Keyword::Param)),
	UniCase::ascii("PASSHASH") => Some(TokenKind::Keyword(Keyword::Passhash)),
	UniCase::ascii("PASSWORD") => Some(TokenKind::Keyword(Keyword::Password)),
	UniCase::ascii("PATCH") => Some(TokenKind::Keyword(Keyword::Patch)),
	UniCase::ascii("PERMISSIONS") => Some(TokenKind::Keyword(Keyword::Permissions)),
	UniCase::ascii("POSTINGS_CACHE") => Some(TokenKind::Keyword(Keyword::PostingsCache)),
	UniCase::ascii("POSTINGS_ORDER") => Some(TokenKind::Keyword(Keyword::PostingsOrder)),
	UniCase::ascii("PUNCT") => Some(TokenKind::Keyword(Keyword::Punct)),
	UniCase::ascii("READONLY") => Some(TokenKind::Keyword(Keyword::Readonly)),
	UniCase::ascii("RELATE") => Some(TokenKind::Keyword(Keyword::Relate)),
	UniCase::ascii("REMOVE") => Some(TokenKind::Keyword(Keyword::Remove)),
	UniCase::ascii("REPLACE") => Some(TokenKind::Keyword(Keyword::Replace)),
	UniCase::ascii("RETURN") => Some(TokenKind::Keyword(Keyword::Return)),
	UniCase::ascii("ROLES") => Some(TokenKind::Keyword(Keyword::Roles)),
	UniCase::ascii("ROOT") => Some(TokenKind::Keyword(Keyword::Root)),
	UniCase::ascii("KV") => Some(TokenKind::Keyword(Keyword::Root)),
	UniCase::ascii("SCHEMAFULL") => Some(TokenKind::Keyword(Keyword::Schemafull)),
	UniCase::ascii("SCHEMAFUL") => Some(TokenKind::Keyword(Keyword::Schemafull)),
	UniCase::ascii("SCHEMALESS") => Some(TokenKind::Keyword(Keyword::Schemaless)),
	UniCase::ascii("SCOPE") => Some(TokenKind::Keyword(Keyword::Scope)),
	UniCase::ascii("SC") => Some(TokenKind::Keyword(Keyword::Scope)),
	UniCase::ascii("SEARCH") => Some(TokenKind::Keyword(Keyword::Search)),
	UniCase::ascii("SELECT") => Some(TokenKind::Keyword(Keyword::Select)),
	UniCase::ascii("SESSION") => Some(TokenKind::Keyword(Keyword::Session)),
	UniCase::ascii("SET") => Some(TokenKind::Keyword(Keyword::Set)),
	UniCase::ascii("SHOW") => Some(TokenKind::Keyword(Keyword::Show)),
	UniCase::ascii("SIGNIN") => Some(TokenKind::Keyword(Keyword::Signin)),
	UniCase::ascii("SIGNUP") => Some(TokenKind::Keyword(Keyword::Signup)),
	UniCase::ascii("SINCE") => Some(TokenKind::Keyword(Keyword::Since)),
	UniCase::ascii("SLEEP") => Some(TokenKind::Keyword(Keyword::Sleep)),
	UniCase::ascii("SNOWBALL") => Some(TokenKind::Keyword(Keyword::Snowball)),
	UniCase::ascii("SPLIT") => Some(TokenKind::Keyword(Keyword::Split)),
	UniCase::ascii("START") => Some(TokenKind::Keyword(Keyword::Start)),
	UniCase::ascii("TABLE") => Some(TokenKind::Keyword(Keyword::Table)),
	UniCase::ascii("TB") => Some(TokenKind::Keyword(Keyword::Table)),
	UniCase::ascii("TERMS_CACHE") => Some(TokenKind::Keyword(Keyword::TermsCache)),
	UniCase::ascii("TERMS_ORDER") => Some(TokenKind::Keyword(Keyword::TermsOrder)),
	UniCase::ascii("THEN") => Some(TokenKind::Keyword(Keyword::Then)),
	UniCase::ascii("THROW") => Some(TokenKind::Keyword(Keyword::Throw)),
	UniCase::ascii("TIMEOUT") => Some(TokenKind::Keyword(Keyword::Timeout)),
	UniCase::ascii("TOKENIZERS") => Some(TokenKind::Keyword(Keyword::Tokenizers)),
	UniCase::ascii("TOKEN") => Some(TokenKind::Keyword(Keyword::Token)),
	UniCase::ascii("TRANSACTION") => Some(TokenKind::Keyword(Keyword::Transaction)),
	UniCase::ascii("true") => Some(TokenKind::Keyword(Keyword::True)),
	UniCase::ascii("TYPE") => Some(TokenKind::Keyword(Keyword::Type)),
	UniCase::ascii("UNIQUE") => Some(TokenKind::Keyword(Keyword::Unique)),
	UniCase::ascii("UNSET") => Some(TokenKind::Keyword(Keyword::Unset)),
	UniCase::ascii("UPDATE") => Some(TokenKind::Keyword(Keyword::Update)),
	UniCase::ascii("UPPERCASE") => Some(TokenKind::Keyword(Keyword::Uppercase)),
	UniCase::ascii("USE") => Some(TokenKind::Keyword(Keyword::Use)),
	UniCase::ascii("USER") => Some(TokenKind::Keyword(Keyword::User)),
	UniCase::ascii("VALUE") => Some(TokenKind::Keyword(Keyword::Value)),
	UniCase::ascii("VALUES") => Some(TokenKind::Keyword(Keyword::Values)),
	UniCase::ascii("VERSION") => Some(TokenKind::Keyword(Keyword::Version)),
	UniCase::ascii("VS") => Some(TokenKind::Keyword(Keyword::Vs)),
	UniCase::ascii("WHEN") => Some(TokenKind::Keyword(Keyword::When)),
	UniCase::ascii("WHERE") => Some(TokenKind::Keyword(Keyword::Where)),
	UniCase::ascii("WITH") => Some(TokenKind::Keyword(Keyword::With)),
	UniCase::ascii("ALLINSIDE") => Some(TokenKind::Keyword(Keyword::AllInside)),
	UniCase::ascii("ANDKW") => Some(TokenKind::Keyword(Keyword::AndKw)),
	UniCase::ascii("ANYINSIDE") => Some(TokenKind::Keyword(Keyword::AnyInside)),
	UniCase::ascii("INSIDE") => Some(TokenKind::Keyword(Keyword::Inside)),
	UniCase::ascii("INTERSECTS") => Some(TokenKind::Keyword(Keyword::Intersects)),
	UniCase::ascii("NONEINSIDE") => Some(TokenKind::Keyword(Keyword::NoneInside)),
	UniCase::ascii("NOTINSIDE") => Some(TokenKind::Keyword(Keyword::NotInside)),
	UniCase::ascii("OR") => Some(TokenKind::Keyword(Keyword::OrKw)),
	UniCase::ascii("OUTSIDE") => Some(TokenKind::Keyword(Keyword::Outside)),
	UniCase::ascii("NOT") => Some(TokenKind::Keyword(Keyword::Not)),
	UniCase::ascii("AND") => Some(TokenKind::Keyword(Keyword::And)),
	UniCase::ascii("COLLATE") => Some(TokenKind::Keyword(Keyword::Collate)),
	UniCase::ascii("CONTAINSALL") => Some(TokenKind::Keyword(Keyword::ContainsAll)),
	UniCase::ascii("CONTAINSANY") => Some(TokenKind::Keyword(Keyword::ContainsAny)),
	UniCase::ascii("CONTAINSNONE") => Some(TokenKind::Keyword(Keyword::ContainsNone)),
	UniCase::ascii("CONTAINSNOT") => Some(TokenKind::Keyword(Keyword::ContainsNot)),
	UniCase::ascii("CONTAINS") => Some(TokenKind::Keyword(Keyword::Contains)),
	UniCase::ascii("IN") => Some(TokenKind::Keyword(Keyword::In)),

	UniCase::ascii("ANY") => Some(TokenKind::Keyword(Keyword::Any)),
	UniCase::ascii("ARRAY") => Some(TokenKind::Keyword(Keyword::Array)),
	UniCase::ascii("GEOMETRY") => Some(TokenKind::Keyword(Keyword::Geometry)),
	UniCase::ascii("RECORD") => Some(TokenKind::Keyword(Keyword::Record)),
	UniCase::ascii("FUTURE") => Some(TokenKind::Keyword(Keyword::Future)),
	UniCase::ascii("BOOL") => Some(TokenKind::Keyword(Keyword::Bool)),
	UniCase::ascii("BYTES") => Some(TokenKind::Keyword(Keyword::Bytes)),
	UniCase::ascii("DATETIME") => Some(TokenKind::Keyword(Keyword::Datetime)),
	UniCase::ascii("DECIMAL") => Some(TokenKind::Keyword(Keyword::Decimal)),
	UniCase::ascii("DURATION") => Some(TokenKind::Keyword(Keyword::Duration)),
	UniCase::ascii("FLOAT") => Some(TokenKind::Keyword(Keyword::Float)),
	UniCase::ascii("fn") => Some(TokenKind::Keyword(Keyword::Fn)),
	UniCase::ascii("ml") => Some(TokenKind::Keyword(Keyword::ML)),
	UniCase::ascii("INT") => Some(TokenKind::Keyword(Keyword::Int)),
	UniCase::ascii("NUMBER") => Some(TokenKind::Keyword(Keyword::Number)),
	UniCase::ascii("OBJECT") => Some(TokenKind::Keyword(Keyword::Object)),
	UniCase::ascii("STRING") => Some(TokenKind::Keyword(Keyword::String)),
	UniCase::ascii("UUID") => Some(TokenKind::Keyword(Keyword::Uuid)),
	UniCase::ascii("ULID") => Some(TokenKind::Keyword(Keyword::Ulid)),
	UniCase::ascii("RAND") => Some(TokenKind::Keyword(Keyword::Rand)),
	UniCase::ascii("FEATURE") => Some(TokenKind::Keyword(Keyword::Feature)),
	UniCase::ascii("LINE") => Some(TokenKind::Keyword(Keyword::Line)),
	UniCase::ascii("POINT") => Some(TokenKind::Keyword(Keyword::Point)),
	UniCase::ascii("POLYGON") => Some(TokenKind::Keyword(Keyword::Polygon)),
	UniCase::ascii("MULTIPOINT") => Some(TokenKind::Keyword(Keyword::MultiPoint)),
	UniCase::ascii("MULTILINE") => Some(TokenKind::Keyword(Keyword::MultiLine)),
	UniCase::ascii("MULTIPOLYGON") => Some(TokenKind::Keyword(Keyword::MultiPolygon)),
	UniCase::ascii("COLLECTION") => Some(TokenKind::Keyword(Keyword::Collection)),

	// Languages
	UniCase::ascii("ARABIC") => Some(TokenKind::Language(Language::Arabic)),
	UniCase::ascii("ARA") => Some(TokenKind::Language(Language::Arabic)),
	UniCase::ascii("AR") => Some(TokenKind::Language(Language::Arabic)),
	UniCase::ascii("DANISH") => Some(TokenKind::Language(Language::Danish)),
	UniCase::ascii("DAN") => Some(TokenKind::Language(Language::Danish)),
	UniCase::ascii("DA") => Some(TokenKind::Language(Language::Danish)),
	UniCase::ascii("DUTCH") => Some(TokenKind::Language(Language::Dutch)),
	UniCase::ascii("NLD") => Some(TokenKind::Language(Language::Dutch)),
	UniCase::ascii("NL") => Some(TokenKind::Language(Language::Dutch)),
	UniCase::ascii("ENGLISH") => Some(TokenKind::Language(Language::English)),
	UniCase::ascii("ENG") => Some(TokenKind::Language(Language::English)),
	UniCase::ascii("EN") => Some(TokenKind::Language(Language::English)),
	UniCase::ascii("FRENCH") => Some(TokenKind::Language(Language::French)),
	UniCase::ascii("FRA") => Some(TokenKind::Language(Language::French)),
	UniCase::ascii("FR") => Some(TokenKind::Language(Language::French)),
	UniCase::ascii("GERMAN") => Some(TokenKind::Language(Language::German)),
	UniCase::ascii("DEU") => Some(TokenKind::Language(Language::German)),
	UniCase::ascii("DE") => Some(TokenKind::Language(Language::German)),
	UniCase::ascii("GREEK") => Some(TokenKind::Language(Language::Greek)),
	UniCase::ascii("ELL") => Some(TokenKind::Language(Language::Greek)),
	UniCase::ascii("EL") => Some(TokenKind::Language(Language::Greek)),
	UniCase::ascii("HUNGARIAN") => Some(TokenKind::Language(Language::Hungarian)),
	UniCase::ascii("HUN") => Some(TokenKind::Language(Language::Hungarian)),
	UniCase::ascii("HU") => Some(TokenKind::Language(Language::Hungarian)),
	UniCase::ascii("ITALIAN") => Some(TokenKind::Language(Language::Italian)),
	UniCase::ascii("ITA") => Some(TokenKind::Language(Language::Italian)),
	UniCase::ascii("IT") => Some(TokenKind::Language(Language::Italian)),
	UniCase::ascii("NORWEGIAN") => Some(TokenKind::Language(Language::Norwegian)),
	UniCase::ascii("NOR") => Some(TokenKind::Language(Language::Norwegian)),
	UniCase::ascii("PORTUGUESE") => Some(TokenKind::Language(Language::Portuguese)),
	UniCase::ascii("POR") => Some(TokenKind::Language(Language::Portuguese)),
	UniCase::ascii("PT") => Some(TokenKind::Language(Language::Portuguese)),
	UniCase::ascii("ROMANIAN") => Some(TokenKind::Language(Language::Romanian)),
	UniCase::ascii("RON") => Some(TokenKind::Language(Language::Romanian)),
	UniCase::ascii("RO") => Some(TokenKind::Language(Language::Romanian)),
	UniCase::ascii("RUSSIAN") => Some(TokenKind::Language(Language::Russian)),
	UniCase::ascii("RUS") => Some(TokenKind::Language(Language::Russian)),
	UniCase::ascii("RU") => Some(TokenKind::Language(Language::Russian)),
	UniCase::ascii("SPANISH") => Some(TokenKind::Language(Language::Spanish)),
	UniCase::ascii("SPA") => Some(TokenKind::Language(Language::Spanish)),
	UniCase::ascii("ES") => Some(TokenKind::Language(Language::Spanish)),
	UniCase::ascii("SWEDISH") => Some(TokenKind::Language(Language::Swedish)),
	UniCase::ascii("SWE") => Some(TokenKind::Language(Language::Swedish)),
	UniCase::ascii("SV") => Some(TokenKind::Language(Language::Swedish)),
	UniCase::ascii("TAMIL") => Some(TokenKind::Language(Language::Tamil)),
	UniCase::ascii("TAM") => Some(TokenKind::Language(Language::Tamil)),
	UniCase::ascii("TA") => Some(TokenKind::Language(Language::Tamil)),
	UniCase::ascii("TURKISH") => Some(TokenKind::Language(Language::Turkish)),
	UniCase::ascii("TUR") => Some(TokenKind::Language(Language::Turkish)),
	UniCase::ascii("TR") => Some(TokenKind::Language(Language::Turkish)),

	// Algorithms
	UniCase::ascii("EDDSA") => Some(TokenKind::Algorithm(Algorithm::EdDSA)),
	UniCase::ascii("ES256") => Some(TokenKind::Algorithm(Algorithm::Es256)),
	UniCase::ascii("ES384") => Some(TokenKind::Algorithm(Algorithm::Es384)),
	UniCase::ascii("ES512") => Some(TokenKind::Algorithm(Algorithm::Es512)),
	UniCase::ascii("HS256") => Some(TokenKind::Algorithm(Algorithm::Hs256)),
	UniCase::ascii("HS384") => Some(TokenKind::Algorithm(Algorithm::Hs384)),
	UniCase::ascii("HS512") => Some(TokenKind::Algorithm(Algorithm::Hs512)),
	UniCase::ascii("PS256") => Some(TokenKind::Algorithm(Algorithm::Ps256)),
	UniCase::ascii("PS384") => Some(TokenKind::Algorithm(Algorithm::Ps384)),
	UniCase::ascii("PS512") => Some(TokenKind::Algorithm(Algorithm::Ps512)),
	UniCase::ascii("RS256") => Some(TokenKind::Algorithm(Algorithm::Rs256)),
	UniCase::ascii("RS384") => Some(TokenKind::Algorithm(Algorithm::Rs384)),
	UniCase::ascii("RS512") => Some(TokenKind::Algorithm(Algorithm::Rs512)),
	UniCase::ascii("JWKS") => jwks_token_kind(), // Necessary because `phf_map!` doesn't support `cfg` attributes

	// Distance
	UniCase::ascii("EUCLIDEAN") => Some(TokenKind::Distance(DistanceKind::Euclidean)),
	UniCase::ascii("MANHATTAN") => Some(TokenKind::Distance(DistanceKind::Manhattan)),
	UniCase::ascii("HAMMING") => Some(TokenKind::Distance(DistanceKind::Hamming)),
	UniCase::ascii("MINKOWSKI") => Some(TokenKind::Distance(DistanceKind::Minkowski)),
};

const fn jwks_token_kind() -> Option<TokenKind> {
	#[cfg(feature = "jwks")]
	let token = Some(TokenKind::Algorithm(Algorithm::Jwks));
	#[cfg(not(feature = "jwks"))]
	let token = None;
	token
}
