use super::verify::{verify_creds_legacy, verify_db_creds, verify_ns_creds, verify_root_creds};
use super::{Actor, Level};
use crate::cnf::{INSECURE_FORWARD_SCOPE_ERRORS, SERVER_NAME};
use crate::dbs::Session;
use crate::err::Error;
use crate::iam::token::{Claims, HEADER};
use crate::iam::Auth;
use crate::kvs::{Datastore, LockType::*, TransactionType::*};
use crate::sql::AccessType;
use crate::sql::Object;
use crate::sql::Value;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;
use uuid::Uuid;

pub async fn signin(
	kvs: &Datastore,
	session: &mut Session,
	vars: Object,
) -> Result<Option<String>, Error> {
	// Parse the specified variables
	let ns = vars.get("NS").or_else(|| vars.get("ns"));
	let db = vars.get("DB").or_else(|| vars.get("db"));
	let ac = vars.get("AC").or_else(|| vars.get("ac"));

	// Check if the parameters exist
	match (ns, db, ac) {
		// DB signin with access method
		(Some(ns), Some(db), Some(ac)) => {
			// Process the provided values
			let ns = ns.to_raw_string();
			let db = db.to_raw_string();
			let ac = ac.to_raw_string();
			// Attempt to signin using specified access method
			super::signin::db(kvs, session, ns, db, ac, vars).await
		}
		// DB signin with user credentials
		(Some(ns), Some(db), None) => {
			// Get the provided user and pass
			let user = vars.get("user");
			let pass = vars.get("pass");
			// Validate the user and pass
			match (user, pass) {
				// There is a username and password
				(Some(user), Some(pass)) => {
					// Process the provided values
					let ns = ns.to_raw_string();
					let db = db.to_raw_string();
					let user = user.to_raw_string();
					let pass = pass.to_raw_string();
					// Attempt to signin to database
					super::signin::db_user(kvs, session, ns, db, user, pass).await
				}
				_ => Err(Error::MissingUserOrPass),
			}
		}
		// NS signin with user credentials
		(Some(ns), None, None) => {
			// Get the provided user and pass
			let user = vars.get("user");
			let pass = vars.get("pass");
			// Validate the user and pass
			match (user, pass) {
				// There is a username and password
				(Some(user), Some(pass)) => {
					// Process the provided values
					let ns = ns.to_raw_string();
					let user = user.to_raw_string();
					let pass = pass.to_raw_string();
					// Attempt to signin to namespace
					super::signin::ns_user(kvs, session, ns, user, pass).await
				}
				_ => Err(Error::MissingUserOrPass),
			}
		}
		// ROOT signin with user credentials
		(None, None, None) => {
			// Get the provided user and pass
			let user = vars.get("user");
			let pass = vars.get("pass");
			// Validate the user and pass
			match (user, pass) {
				// There is a username and password
				(Some(user), Some(pass)) => {
					// Process the provided values
					let user = user.to_raw_string();
					let pass = pass.to_raw_string();
					// Attempt to signin to root
					super::signin::root_user(kvs, session, user, pass).await
				}
				_ => Err(Error::MissingUserOrPass),
			}
		}
		_ => Err(Error::NoSigninTarget),
	}
}

pub async fn db(
	kvs: &Datastore,
	session: &mut Session,
	ns: String,
	db: String,
	ac: String,
	vars: Object,
) -> Result<Option<String>, Error> {
	// Create a new readonly transaction
	let mut tx = kvs.transaction(Read, Optimistic).await?;
	// Fetch the specified access method from storage
	let access = tx.get_db_access(&ns, &db, &ac).await;
	// Ensure that the transaction is cancelled
	tx.cancel().await?;
	// Check the provided access method exists
	match access {
		Ok(av) => {
			// Check the access method type
			// All access method types are supported except for JWT
			// The JWT access method is the one that is internal to SurrealDB
			// The equivalent of signing in with JWT is to authenticate it
			match av.kind {
				AccessType::Record(at) => {
					// Check if the record access method supports issuing tokens
					let iss = match at.jwt.issue {
						Some(iss) => iss,
						_ => return Err(Error::AccessMethodMismatch),
					};
					match at.signin {
						// This record access allows signin
						Some(val) => {
							// Setup the query params
							let vars = Some(vars.0);
							// Setup the system session for finding the signin record
							let mut sess = Session::editor().with_ns(&ns).with_db(&db);
							sess.ip.clone_from(&session.ip);
							sess.or.clone_from(&session.or);
							// Compute the value with the params
							match kvs.evaluate(val, &sess, vars).await {
								// The signin value succeeded
								Ok(val) => {
									match val.record() {
										// There is a record returned
										Some(rid) => {
											// Create the authentication key
											let key = EncodingKey::from_secret(iss.key.as_ref());
											// Create the authentication claim
											let exp =
												Some(
													match at.duration {
														Some(v) => {
															// The defined session duration must be valid
															match Duration::from_std(v.0) {
														// The resulting session expiration must be valid
														Ok(d) => match Utc::now().checked_add_signed(d) {
															Some(exp) => exp,
															None => {
																return Err(Error::InvalidSessionExpiration)
															}
														},
														Err(_) => {
															return Err(Error::InvalidSessionDuration)
														}
													}
														}
														_ => Utc::now() + Duration::hours(1),
													}
													.timestamp(),
												);
											let val = Claims {
												iss: Some(SERVER_NAME.to_owned()),
												iat: Some(Utc::now().timestamp()),
												nbf: Some(Utc::now().timestamp()),
												exp,
												jti: Some(Uuid::new_v4().to_string()),
												ns: Some(ns.to_owned()),
												db: Some(db.to_owned()),
												ac: Some(ac.to_owned()),
												id: Some(rid.to_raw()),
												..Claims::default()
											};
											// Log the authenticated access method info
											trace!("Signing in with access method `{}`", ac);
											// Create the authentication token
											let enc =
												encode(&Header::new(iss.alg.into()), &val, &key);
											// Set the authentication on the session
											session.tk = Some(val.into());
											session.ns = Some(ns.to_owned());
											session.db = Some(db.to_owned());
											session.ac = Some(ac.to_owned());
											session.sd = Some(Value::from(rid.to_owned()));
											session.exp = exp;
											session.au = Arc::new(Auth::new(Actor::new(
												rid.to_string(),
												Default::default(),
												Level::Record(ns, db, rid.to_string()),
											)));
											// Check the authentication token
											match enc {
												// The auth token was created successfully
												Ok(tk) => Ok(Some(tk)),
												_ => Err(Error::TokenMakingFailed),
											}
										}
										_ => Err(Error::NoRecordFound),
									}
								}
								Err(e) => match e {
									Error::Thrown(_) => Err(e),
									e if *INSECURE_FORWARD_SCOPE_ERRORS => Err(e),
									_ => Err(Error::AccessRecordSigninQueryFailed),
								},
							}
						}
						_ => Err(Error::AccessRecordNoSignin),
					}
				}
				_ => Err(Error::AccessMethodMismatch),
			}
		}
		_ => Err(Error::AccessNotFound),
	}
}

pub async fn db_user(
	kvs: &Datastore,
	session: &mut Session,
	ns: String,
	db: String,
	user: String,
	pass: String,
) -> Result<Option<String>, Error> {
	let verify_creds = if kvs.is_auth_level_enabled() {
		verify_db_creds(kvs, &ns, &db, &user, &pass).await
	} else {
		// TODO(gguillemas): Remove this condition once the legacy authentication is deprecated in v2.0.0
		match verify_creds_legacy(kvs, Some(&ns), Some(&db), &user, &pass).await {
			Ok((_, u)) => Ok(u),
			Err(e) => Err(e),
		}
	};
	match verify_creds {
		Ok(u) => {
			// Create the authentication key
			let key = EncodingKey::from_secret(u.code.as_ref());
			// Create the authentication claim
			let exp = Some((Utc::now() + Duration::hours(1)).timestamp());
			let val = Claims {
				iss: Some(SERVER_NAME.to_owned()),
				iat: Some(Utc::now().timestamp()),
				nbf: Some(Utc::now().timestamp()),
				exp,
				jti: Some(Uuid::new_v4().to_string()),
				ns: Some(ns.to_owned()),
				db: Some(db.to_owned()),
				id: Some(user),
				..Claims::default()
			};
			// Log the authenticated database info
			trace!("Signing in to database `{}`", db);
			// Create the authentication token
			let enc = encode(&HEADER, &val, &key);
			// Set the authentication on the session
			session.tk = Some(val.into());
			session.ns = Some(ns.to_owned());
			session.db = Some(db.to_owned());
			// TODO(gguillemas): Enforce expiration once session lifetime can be customized.
			session.exp = None;
			session.au = Arc::new((&u, Level::Database(ns.to_owned(), db.to_owned())).into());
			// Check the authentication token
			match enc {
				// The auth token was created successfully
				Ok(tk) => Ok(Some(tk)),
				_ => Err(Error::TokenMakingFailed),
			}
		}
		_ => Err(Error::InvalidAuth),
	}
}

pub async fn ns_user(
	kvs: &Datastore,
	session: &mut Session,
	ns: String,
	user: String,
	pass: String,
) -> Result<Option<String>, Error> {
	let verify_creds = if kvs.is_auth_level_enabled() {
		verify_ns_creds(kvs, &ns, &user, &pass).await
	} else {
		// TODO(gguillemas): Remove this condition once the legacy authentication is deprecated in v2.0.0
		match verify_creds_legacy(kvs, Some(&ns), None, &user, &pass).await {
			Ok((_, u)) => Ok(u),
			Err(e) => Err(e),
		}
	};
	match verify_creds {
		Ok(u) => {
			// Create the authentication key
			let key = EncodingKey::from_secret(u.code.as_ref());
			// Create the authentication claim
			let exp = Some((Utc::now() + Duration::hours(1)).timestamp());
			let val = Claims {
				iss: Some(SERVER_NAME.to_owned()),
				iat: Some(Utc::now().timestamp()),
				nbf: Some(Utc::now().timestamp()),
				exp,
				jti: Some(Uuid::new_v4().to_string()),
				ns: Some(ns.to_owned()),
				id: Some(user),
				..Claims::default()
			};
			// Log the authenticated namespace info
			trace!("Signing in to namespace `{}`", ns);
			// Create the authentication token
			let enc = encode(&HEADER, &val, &key);
			// Set the authentication on the session
			session.tk = Some(val.into());
			session.ns = Some(ns.to_owned());
			// TODO(gguillemas): Enforce expiration once session lifetime can be customized.
			session.exp = None;
			session.au = Arc::new((&u, Level::Namespace(ns.to_owned())).into());
			// Check the authentication token
			match enc {
				// The auth token was created successfully
				Ok(tk) => Ok(Some(tk)),
				_ => Err(Error::TokenMakingFailed),
			}
		}
		// The password did not verify
		_ => Err(Error::InvalidAuth),
	}
}

pub async fn root_user(
	kvs: &Datastore,
	session: &mut Session,
	user: String,
	pass: String,
) -> Result<Option<String>, Error> {
	let verify_creds = if kvs.is_auth_level_enabled() {
		verify_root_creds(kvs, &user, &pass).await
	} else {
		// TODO(gguillemas): Remove this condition once the legacy authentication is deprecated in v2.0.0
		match verify_creds_legacy(kvs, None, None, &user, &pass).await {
			Ok((_, u)) => Ok(u),
			Err(e) => Err(e),
		}
	};
	match verify_creds {
		Ok(u) => {
			// Create the authentication key
			let key = EncodingKey::from_secret(u.code.as_ref());
			// Create the authentication claim
			let exp = Some((Utc::now() + Duration::hours(1)).timestamp());
			let val = Claims {
				iss: Some(SERVER_NAME.to_owned()),
				iat: Some(Utc::now().timestamp()),
				nbf: Some(Utc::now().timestamp()),
				exp,
				jti: Some(Uuid::new_v4().to_string()),
				id: Some(user),
				..Claims::default()
			};
			// Log the authenticated root info
			trace!("Signing in as root");
			// Create the authentication token
			let enc = encode(&HEADER, &val, &key);
			// Set the authentication on the session
			session.tk = Some(val.into());
			// TODO(gguillemas): Enforce expiration once session lifetime can be customized.
			session.exp = None;
			session.au = Arc::new((&u, Level::Root).into());
			// Check the authentication token
			match enc {
				// The auth token was created successfully
				Ok(tk) => Ok(Some(tk)),
				_ => Err(Error::TokenMakingFailed),
			}
		}
		// The password did not verify
		_ => Err(Error::InvalidAuth),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::iam::Role;
	use std::collections::HashMap;

	#[tokio::test]
	async fn test_signin_record() {
		// Test with correct credentials
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test").with_db("test");
			ds.execute(
				r#"
				DEFINE ACCESS user ON DATABASE TYPE RECORD DURATION 1h
					SIGNIN (
						SELECT * FROM user WHERE name = $user AND crypto::argon2::compare(pass, $pass)
					)
					SIGNUP (
						CREATE user CONTENT {
							name: $user,
							pass: crypto::argon2::generate($pass)
						}
					);

				CREATE user:test CONTENT {
					name: 'user',
					pass: crypto::argon2::generate('pass')
				}
				"#,
				&sess,
				None,
			)
			.await
			.unwrap();

			// Signin with the user
			let mut sess = Session {
				ns: Some("test".to_string()),
				db: Some("test".to_string()),
				..Default::default()
			};
			let mut vars: HashMap<&str, Value> = HashMap::new();
			vars.insert("user", "user".into());
			vars.insert("pass", "pass".into());
			let res = db(
				&ds,
				&mut sess,
				"test".to_string(),
				"test".to_string(),
				"user".to_string(),
				vars.into(),
			)
			.await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.ns, Some("test".to_string()));
			assert_eq!(sess.db, Some("test".to_string()));
			assert_eq!(sess.au.id(), "user:test");
			assert!(sess.au.is_record());
			assert_eq!(sess.au.level().ns(), Some("test"));
			assert_eq!(sess.au.level().db(), Some("test"));
			assert_eq!(sess.au.level().id(), Some("user:test"));
			// Record users should not have roles
			assert!(!sess.au.has_role(&Role::Viewer), "Auth user expected to not have Viewer role");
			assert!(!sess.au.has_role(&Role::Editor), "Auth user expected to not have Editor role");
			assert!(!sess.au.has_role(&Role::Owner), "Auth user expected to not have Owner role");
			// Expiration should always be set for tokens issued by SurrealDB
			let exp = sess.exp.unwrap();
			// Expiration should match the current time plus session duration with some margin
			let min_exp = (Utc::now() + Duration::hours(1) - Duration::seconds(10)).timestamp();
			let max_exp = (Utc::now() + Duration::hours(1) + Duration::seconds(10)).timestamp();
			assert!(
				exp > min_exp && exp < max_exp,
				"Session expiration is expected to follow access method duration"
			);
		}

		// Test with incorrect credentials
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test").with_db("test");
			ds.execute(
				r#"
				DEFINE ACCESS user ON DATABASE TYPE RECORD DURATION 1h
					SIGNIN (
						SELECT * FROM user WHERE name = $user AND crypto::argon2::compare(pass, $pass)
					)
					SIGNUP (
						CREATE user CONTENT {
							name: $user,
							pass: crypto::argon2::generate($pass)
						}
					);

				CREATE user:test CONTENT {
					name: 'user',
					pass: crypto::argon2::generate('pass')
				}
				"#,
				&sess,
				None,
			)
			.await
			.unwrap();

			// Signin with the user
			let mut sess = Session {
				ns: Some("test".to_string()),
				db: Some("test".to_string()),
				..Default::default()
			};
			let mut vars: HashMap<&str, Value> = HashMap::new();
			vars.insert("user", "user".into());
			vars.insert("pass", "incorrect".into());
			let res = db(
				&ds,
				&mut sess,
				"test".to_string(),
				"test".to_string(),
				"user".to_string(),
				vars.into(),
			)
			.await;

			assert!(res.is_err(), "Unexpected successful signin: {:?}", res);
		}
	}

	#[tokio::test]
	async fn test_signin_db_user() {
		//
		// Test without roles defined
		//
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test").with_db("test");
			ds.execute("DEFINE USER user ON DB PASSWORD 'pass'", &sess, None).await.unwrap();

			// Signin with the user
			let mut sess = Session {
				ns: Some("test".to_string()),
				db: Some("test".to_string()),
				..Default::default()
			};
			let res = db_user(
				&ds,
				&mut sess,
				"test".to_string(),
				"test".to_string(),
				"user".to_string(),
				"pass".to_string(),
			)
			.await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.ns, Some("test".to_string()));
			assert_eq!(sess.db, Some("test".to_string()));
			assert_eq!(sess.au.id(), "user");
			assert!(sess.au.is_db());
			assert_eq!(sess.au.level().ns(), Some("test"));
			assert_eq!(sess.au.level().db(), Some("test"));
			assert!(sess.au.has_role(&Role::Viewer), "Auth user expected to have Viewer role");
			assert!(!sess.au.has_role(&Role::Editor), "Auth user expected to not have Editor role");
			assert!(!sess.au.has_role(&Role::Owner), "Auth user expected to not have Owner role");
			assert_eq!(sess.exp, None, "Default system user expiration is expected to be None");
		}

		//
		// Test with roles defined
		//
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test").with_db("test");
			ds.execute("DEFINE USER user ON DB PASSWORD 'pass' ROLES EDITOR, OWNER", &sess, None)
				.await
				.unwrap();

			// Signin with the user
			let mut sess = Session {
				ns: Some("test".to_string()),
				db: Some("test".to_string()),
				..Default::default()
			};
			let res = db_user(
				&ds,
				&mut sess,
				"test".to_string(),
				"test".to_string(),
				"user".to_string(),
				"pass".to_string(),
			)
			.await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.ns, Some("test".to_string()));
			assert_eq!(sess.db, Some("test".to_string()));
			assert_eq!(sess.au.id(), "user");
			assert!(sess.au.is_db());
			assert_eq!(sess.au.level().ns(), Some("test"));
			assert_eq!(sess.au.level().db(), Some("test"));
			assert!(!sess.au.has_role(&Role::Viewer), "Auth user expected to not have Viewer role");
			assert!(sess.au.has_role(&Role::Editor), "Auth user expected to have Editor role");
			assert!(sess.au.has_role(&Role::Owner), "Auth user expected to have Owner role");
			assert_eq!(sess.exp, None, "Default system user expiration is expected to be None");
		}

		// Test invalid password
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test").with_db("test");
			ds.execute("DEFINE USER user ON DB PASSWORD 'pass'", &sess, None).await.unwrap();

			let mut sess = Session {
				..Default::default()
			};
			let res = db_user(
				&ds,
				&mut sess,
				"test".to_string(),
				"test".to_string(),
				"user".to_string(),
				"invalid".to_string(),
			)
			.await;

			assert!(res.is_err(), "Unexpected successful signin: {:?}", res);
		}
	}

	#[tokio::test]
	async fn test_signin_ns_user() {
		//
		// Test without roles defined
		//
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test");
			ds.execute("DEFINE USER user ON NS PASSWORD 'pass'", &sess, None).await.unwrap();

			// Signin with the user
			let mut sess = Session {
				ns: Some("test".to_string()),
				..Default::default()
			};
			let res =
				ns_user(&ds, &mut sess, "test".to_string(), "user".to_string(), "pass".to_string())
					.await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.ns, Some("test".to_string()));
			assert_eq!(sess.au.id(), "user");
			assert!(sess.au.is_ns());
			assert_eq!(sess.au.level().ns(), Some("test"));
			assert!(sess.au.has_role(&Role::Viewer), "Auth user expected to have Viewer role");
			assert!(!sess.au.has_role(&Role::Editor), "Auth user expected to not have Editor role");
			assert!(!sess.au.has_role(&Role::Owner), "Auth user expected to not have Owner role");
			assert_eq!(sess.exp, None, "Default system user expiration is expected to be None");
		}

		//
		// Test with roles defined
		//
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test");
			ds.execute("DEFINE USER user ON NS PASSWORD 'pass' ROLES EDITOR, OWNER", &sess, None)
				.await
				.unwrap();

			// Signin with the user
			let mut sess = Session {
				ns: Some("test".to_string()),
				..Default::default()
			};
			let res =
				ns_user(&ds, &mut sess, "test".to_string(), "user".to_string(), "pass".to_string())
					.await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.ns, Some("test".to_string()));
			assert_eq!(sess.au.id(), "user");
			assert!(sess.au.is_ns());
			assert_eq!(sess.au.level().ns(), Some("test"));
			assert!(!sess.au.has_role(&Role::Viewer), "Auth user expected to not have Viewer role");
			assert!(sess.au.has_role(&Role::Editor), "Auth user expected to have Editor role");
			assert!(sess.au.has_role(&Role::Owner), "Auth user expected to have Owner role");
			assert_eq!(sess.exp, None, "Default system user expiration is expected to be None");
		}

		// Test invalid password
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test");
			ds.execute("DEFINE USER user ON NS PASSWORD 'pass'", &sess, None).await.unwrap();

			let mut sess = Session {
				..Default::default()
			};
			let res = ns_user(
				&ds,
				&mut sess,
				"test".to_string(),
				"user".to_string(),
				"invalid".to_string(),
			)
			.await;

			assert!(res.is_err(), "Unexpected successful signin: {:?}", res);
		}
	}

	#[tokio::test]
	async fn test_signin_root_user() {
		//
		// Test without roles defined
		//
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner();
			ds.execute("DEFINE USER user ON ROOT PASSWORD 'pass'", &sess, None).await.unwrap();

			// Signin with the user
			let mut sess = Session {
				..Default::default()
			};
			let res = root_user(&ds, &mut sess, "user".to_string(), "pass".to_string()).await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.au.id(), "user");
			assert!(sess.au.is_root());
			assert!(sess.au.has_role(&Role::Viewer), "Auth user expected to have Viewer role");
			assert!(!sess.au.has_role(&Role::Editor), "Auth user expected to not have Editor role");
			assert!(!sess.au.has_role(&Role::Owner), "Auth user expected to not have Owner role");
			assert_eq!(sess.exp, None, "Default system user expiration is expected to be None");
		}

		//
		// Test with roles defined
		//
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner();
			ds.execute("DEFINE USER user ON ROOT PASSWORD 'pass' ROLES EDITOR, OWNER", &sess, None)
				.await
				.unwrap();

			// Signin with the user
			let mut sess = Session {
				..Default::default()
			};
			let res = root_user(&ds, &mut sess, "user".to_string(), "pass".to_string()).await;

			assert!(res.is_ok(), "Failed to signin with credentials: {:?}", res);
			assert_eq!(sess.au.id(), "user");
			assert!(sess.au.is_root());
			assert!(!sess.au.has_role(&Role::Viewer), "Auth user expected to not have Viewer role");
			assert!(sess.au.has_role(&Role::Editor), "Auth user expected to have Editor role");
			assert!(sess.au.has_role(&Role::Owner), "Auth user expected to have Owner role");
			assert_eq!(sess.exp, None, "Default system user expiration is expected to be None");
		}

		// Test invalid password
		{
			let ds = Datastore::new("memory").await.unwrap();
			let sess = Session::owner().with_ns("test");
			ds.execute("DEFINE USER user ON ROOT PASSWORD 'pass'", &sess, None).await.unwrap();

			let mut sess = Session {
				..Default::default()
			};
			let res = root_user(&ds, &mut sess, "user".to_string(), "invalid".to_string()).await;

			assert!(res.is_err(), "Unexpected successful signin: {:?}", res);
		}
	}
}
