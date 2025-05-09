use crate::idb::{
    DB_NAME, SESSIONS_STORE, STATE_STORE, clear_store, object_delete, object_get, transaction_put,
};
/// Storage impls to persis OAuth sessions if you are not using the memory stores
/// https://github.com/bluesky-social/statusphere-example-app/blob/main/src/auth/storage.ts
use atrium_api::types::string::Did;
use atrium_common::store::Store;
use atrium_oauth::store::session::SessionStore;
use atrium_oauth::store::state::StateStore;
use indexed_db_futures::database::Database;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::hash::Hash;

#[derive(Debug)]
pub enum AuthStoreError {
    InvalidSession,
    NoSessionFound,
    DatabaseError(String),
}

impl Display for AuthStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSession => write!(f, "Invalid session"),
            Self::NoSessionFound => write!(f, "No session found"),
            Self::DatabaseError(err) => write!(f, "Database error: {}", err),
        }
    }
}

impl StdError for AuthStoreError {}

///Persistent session store in sqlite
impl SessionStore for IndexDBSessionStore {}

pub struct IndexDBSessionStore {
    // db: Database,
}

impl IndexDBSessionStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl<K, V> Store<K, V> for IndexDBSessionStore
where
    K: Debug + Eq + Hash + Send + Sync + 'static + From<Did> + AsRef<str>,
    V: Debug + Clone + Send + Sync + 'static + Serialize + DeserializeOwned,
{
    type Error = AuthStoreError;
    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        let did = key.as_ref().to_string();
        let db = Database::open(DB_NAME).await.unwrap();
        match object_get::<V>(db.clone(), SESSIONS_STORE, &*did).await {
            Ok(Some(session)) => Ok(Some(session)),
            Ok(None) => Err(AuthStoreError::NoSessionFound),
            Err(e) => {
                log::error!("Database error: {}", e.to_string());
                Err(AuthStoreError::DatabaseError(e.to_string()))
            }
        }
    }

    async fn set(&self, key: K, value: V) -> Result<(), Self::Error> {
        let did = key.as_ref().to_string();
        let db = Database::open(DB_NAME).await.unwrap();
        transaction_put(db.clone(), &value, SESSIONS_STORE, Some(did))
            .await
            .map_err(|e| AuthStoreError::DatabaseError(e.to_string()))
    }

    async fn del(&self, _key: &K) -> Result<(), Self::Error> {
        let key = _key.as_ref().to_string();
        let db = Database::open(DB_NAME).await.unwrap();
        object_delete(db.clone(), SESSIONS_STORE, &*key)
            .await
            .map_err(|e| AuthStoreError::DatabaseError(e.to_string()))
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        let db = Database::open(DB_NAME).await.unwrap();
        clear_store(db.clone(), SESSIONS_STORE)
            .await
            .map_err(|e| AuthStoreError::DatabaseError(e.to_string()))
    }
}

impl StateStore for IndexDBStateStore {}

pub struct IndexDBStateStore {}

impl IndexDBStateStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl<K, V> Store<K, V> for IndexDBStateStore
where
    K: Debug + Eq + Hash + Send + Sync + 'static + From<Did> + AsRef<str>,
    V: Debug + Clone + Send + Sync + 'static + Serialize + DeserializeOwned,
{
    type Error = AuthStoreError;
    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        let key = key.as_ref().to_string();
        let db = Database::open(DB_NAME).await.unwrap();
        match object_get::<V>(db.clone(), STATE_STORE, &*key).await {
            Ok(Some(session)) => Ok(Some(session)),
            Ok(None) => Err(AuthStoreError::NoSessionFound),
            Err(e) => {
                log::error!("Database error: {}", e.to_string());
                Err(AuthStoreError::DatabaseError(e.to_string()))
            }
        }
    }

    async fn set(&self, key: K, value: V) -> Result<(), Self::Error> {
        let did = key.as_ref().to_string();
        let db = Database::open(DB_NAME).await.unwrap();
        match transaction_put(db.clone(), &value, STATE_STORE, Some(did))
            .await
            .map_err(|e| AuthStoreError::DatabaseError(e.to_string()))
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn del(&self, _key: &K) -> Result<(), Self::Error> {
        let key = _key.as_ref().to_string();
        let db = Database::open(DB_NAME).await.unwrap();
        object_delete(db.clone(), STATE_STORE, &*key)
            .await
            .map_err(|e| AuthStoreError::DatabaseError(e.to_string()))
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        let db = Database::open(DB_NAME).await.unwrap();
        clear_store(db.clone(), STATE_STORE)
            .await
            .map_err(|e| AuthStoreError::DatabaseError(e.to_string()))
    }
}
