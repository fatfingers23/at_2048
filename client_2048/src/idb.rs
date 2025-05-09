use atrium_api::types::string::RecordKey;
use indexed_db_futures::database::Database;
use indexed_db_futures::error::OpenDbError;
use indexed_db_futures::prelude::*;
use indexed_db_futures::transaction::TransactionMode;
use indexed_db_futures::{KeyPath, SerialiseToJs};
use serde::{Deserialize, Serialize};

//Object Store names
/// db name
pub const DB_NAME: &str = "2048";
/// Store for game history(blue.2048.game), keys are record keys
pub const GAME_STORE: &str = "games";
/// Store for current game, 1 record for the store uses self as the key
pub const CURRENT_GAME_STORE: &str = "current_game";
/// Store for the user stats(blue.2048.player.stats) self as the key
pub const STATS_STORE: &str = "stats";
/// Store for the user profile(blue.2048.player.profile) self as the key
pub const PROFILE_STORE: &str = "profile";
/// Store for did:keys like blue.2048.key.game or blue.2048.key.player.stats
pub const KEY_STORE: &str = "did:keys";
/// did resolver store
pub const DID_RESOLVER_STORE: &str = "did:resolver";
/// atrium StateStore
pub const STATE_STORE: &str = "states";
/// atrium SessionStore
pub const SESSIONS_STORE: &str = "sessions";

/// Static keys for one record stores
pub const SELF_KEY: &str = "self";

pub async fn create_database() -> Result<Database, OpenDbError> {
    let db = Database::open(DB_NAME)
        .with_version(1u8)
        .with_on_blocked(|event| {
            log::debug!("DB upgrade blocked: {:?}", event);
            Ok(())
        })
        .with_on_upgrade_needed_fut(|event, db| async move {
            match (event.old_version(), event.new_version()) {
                (0.0, Some(1.0)) => {
                    let record_key_path = KeyPath::from("rkey");
                    let game_store = db
                        .create_object_store(GAME_STORE)
                        .with_key_path(record_key_path.clone())
                        .build()?;
                    game_store
                        .create_index("index_hash", KeyPath::from("index_hash"))
                        .build()?;
                    db.create_object_store(CURRENT_GAME_STORE).build()?;
                    db.create_object_store(STATS_STORE).build()?;
                    db.create_object_store(PROFILE_STORE).build()?;
                    db.create_object_store(KEY_STORE).build()?;
                    db.create_object_store(DID_RESOLVER_STORE).build()?;
                    db.create_object_store(STATE_STORE).build()?;
                    db.create_object_store(SESSIONS_STORE).build()?;
                }
                _ => {}
            }

            Ok(())
        })
        .await?;
    Ok(db)
}

/// A think wrapper around a at proto record with the record key for storage
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecordStorageWrapper<T> {
    pub rkey: RecordKey,
    pub record: T,
    pub index_hash: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StorageError {
    Error(String),
    OpenDbError(String),
}

impl StorageError {
    pub fn to_string(&self) -> String {
        match self {
            StorageError::Error(err) => err.to_string(),
            StorageError::OpenDbError(err) => err.to_string(),
        }
    }
}

pub async fn transaction_put<T>(
    db: Database,
    item: T,
    store: &str,
    key: Option<String>,
) -> Result<(), StorageError>
where
    T: SerialiseToJs,
{
    let transaction = match db
        .transaction(store)
        .with_mode(TransactionMode::Readwrite)
        .build()
    {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    let store = match transaction.object_store(store) {
        Ok(store) => store,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    match key {
        None => match store.put(item).serde() {
            Ok(action) => match action.await {
                Ok(_) => {}
                Err(err) => {
                    return Err(StorageError::Error(err.to_string()));
                }
            },
            Err(err) => return Err(StorageError::Error(err.to_string())),
        },
        Some(key) => match store.put(item).with_key(key).serde() {
            Ok(action) => match action.await {
                Ok(_) => {}
                Err(err) => return Err(StorageError::Error(err.to_string())),
            },
            Err(err) => return Err(StorageError::Error(err.to_string())),
        },
    };

    match transaction.commit().await {
        Ok(_) => Ok(()),
        Err(err) => Err(StorageError::Error(err.to_string())),
    }
}

pub async fn object_get<T>(db: Database, store: &str, key: &str) -> Result<Option<T>, StorageError>
where
    T: for<'de> Deserialize<'de>,
{
    let transaction = match db
        .transaction(store)
        .with_mode(TransactionMode::Readonly)
        .build()
    {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    let store = match transaction.object_store(store) {
        Ok(store) => store,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };
    match store.get(key).serde() {
        Ok(action) => match action.await {
            Ok(result) => Ok(result),
            Err(err) => Err(StorageError::Error(err.to_string())),
        },
        Err(err) => Err(StorageError::Error(err.to_string())),
    }
}

pub async fn object_get_index<T>(
    db: Database,
    store: &str,
    index_key: &str,
) -> Result<Option<T>, StorageError>
where
    T: for<'de> Deserialize<'de>,
{
    let transaction = match db
        .transaction(store)
        .with_mode(TransactionMode::Readonly)
        .build()
    {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    let store = match transaction.object_store(store) {
        Ok(store) => store,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    let index = store
        .index("index_hash")
        .map_err(|err| StorageError::Error(err.to_string()))?;

    let Some(mut cursor) = index
        .open_cursor()
        .with_query(index_key)
        .serde()
        .map_err(|e| StorageError::Error(e.to_string()))?
        .await
        .map_err(|e| StorageError::Error(e.to_string()))?
    else {
        log::debug!("Cursor empty");
        return Ok(None);
    };
    cursor
        .next_record_ser()
        .await
        .map_err(|err| StorageError::Error(err.to_string()))
}

pub async fn object_delete(db: Database, store: &str, key: &str) -> Result<(), StorageError> {
    let transaction = match db
        .transaction(store)
        .with_mode(TransactionMode::Readwrite)
        .build()
    {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    let store = match transaction.object_store(store) {
        Ok(store) => store,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };
    match store.delete(key).await {
        Ok(_) => match transaction.commit().await {
            Ok(_) => Ok(()),
            Err(_) => Err(StorageError::Error(
                "Failed to commit transaction".to_string(),
            )),
        },
        Err(err) => Err(StorageError::Error(err.to_string())),
    }
}

pub async fn clear_store(db: Database, store: &str) -> Result<(), StorageError> {
    let transaction = match db
        .transaction(store)
        .with_mode(TransactionMode::Readwrite)
        .build()
    {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };

    let store = match transaction.object_store(store) {
        Ok(store) => store,
        Err(err) => {
            return Err(StorageError::Error(err.to_string()));
        }
    };
    match store.clear() {
        Ok(_) => match transaction.commit().await {
            Ok(_) => Ok(()),
            Err(_) => Err(StorageError::Error(
                "Failed to commit transaction".to_string(),
            )),
        },
        Err(err) => Err(StorageError::Error(err.to_string())),
    }
}
