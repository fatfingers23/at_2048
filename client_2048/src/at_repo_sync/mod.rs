use crate::atrium_stores::IndexDBSessionStore;
use crate::idb::{
    DB_NAME, GAME_STORE, PROFILE_STORE, RecordStorageWrapper, SELF_KEY, STATS_STORE, StorageError,
    object_get, transaction_put,
};
use crate::resolver::ApiDNSTxtResolver;
use atrium_api::agent::Agent;
use atrium_api::types::string::{AtIdentifier, Datetime, Did, RecordKey};
use atrium_api::types::{Collection, LimitedNonZeroU8};
use atrium_identity::did::CommonDidResolver;
use atrium_identity::handle::AtprotoHandleResolver;
use atrium_oauth::{DefaultHttpClient, OAuthSession};
use atrium_xrpc::Error::Authentication;
use indexed_db_futures::database::Database;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use types_2048::blue;
use types_2048::blue::_2048;
use types_2048::blue::_2048::player;
use types_2048::record::KnownRecord;

use xxhash_rust::const_xxh3::xxh3_64 as const_xxh3;

type AgentType = Agent<
    OAuthSession<
        DefaultHttpClient,
        CommonDidResolver<DefaultHttpClient>,
        AtprotoHandleResolver<ApiDNSTxtResolver, DefaultHttpClient>,
        IndexDBSessionStore,
    >,
>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AtRepoSyncError {
    LocalIsNewer,
    RemoteIsNewer,
    AtRepoCallError(String),
    LocalRepoError(String),
    Error(String),
    AuthErrorNeedToReLogin,
}

pub trait AtRepoSyncTrait {
    fn get_sync_status(&self) -> Result<blue::_2048::defs::SyncStatus, AtRepoSyncError>;
    fn get_index_db_store_name(&self) -> &str;
    fn get_repo_collection(&self) -> &str;

    fn set_sync_status(
        &self,
        sync_status: blue::_2048::defs::SyncStatus,
    ) -> Result<(), StorageError>;
}

impl std::fmt::Display for AtRepoSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtRepoSyncError::LocalIsNewer => write!(f, "Local is newer"),
            AtRepoSyncError::RemoteIsNewer => write!(f, "Remote is newer"),
            AtRepoSyncError::AtRepoCallError(err) => write!(f, "AtRepoCallError: {}", err),
            AtRepoSyncError::Error(err) => write!(f, "ThereWasAnError: {}", err),
            AtRepoSyncError::LocalRepoError(err) => {
                write!(f, "LocalRepoError: {}", err)
            }
            AtRepoSyncError::AuthErrorNeedToReLogin => {
                write!(f, "There was an error with the auth, need to relogin")
            }
        }
    }
}

pub struct AtRepoSync
// where
//     Repo: AtRepoSyncTrait,
{
    //May have to swap back to using the oauth client and create a new session for each request cause of clone
    client: Option<AgentType>,
    users_did: Option<Did>,
    // store: Repo,
}

impl AtRepoSync
// where
// Repo: AtRepoSyncTrait,
{
    /// A new repo that is logged in and can sync remotely to the AT repo as well as locally
    pub fn new_logged_in_repo(client: AgentType, did: Did) -> Self {
        Self {
            client: Some(client),
            users_did: Some(did),
        }
    }

    /// A new local only repo. Not logged in or cannot connect to the AT repo
    pub fn new_local_repo() -> Self {
        Self {
            client: None,
            users_did: None,
        }
    }

    /// Means that the repo can sync remotely to the AT repo and the user is logged in
    pub fn can_remote_sync(&self) -> bool {
        self.client.is_some() && self.users_did.is_some()
    }

    fn _set_synced_status<Record>(&self, record: Record, synced: bool) -> Option<KnownRecord>
    where
        Record: Serialize + Into<KnownRecord>,
    {
        match record.into() {
            KnownRecord::Blue2048Game(record) => {
                // record.sync_status.synced_with_at_repo = synced;
                // record.sync_status.updated_at = Datetime::now();
                // let string_data = serde_json::to_string(&record).unwrap();
                // let hash = const_xxh3(string_data.as_bytes());
                // record.sync_status.hash = format!("{:x}", hash);
                Some(KnownRecord::Blue2048Game(record))
            }
            KnownRecord::Blue2048PlayerProfile(mut record) => {
                record.sync_status.synced_with_at_repo = synced;
                record.sync_status.updated_at = Datetime::now();
                let string_data = serde_json::to_string(&record).unwrap();
                let hash = const_xxh3(string_data.as_bytes());
                record.sync_status.hash = format!("{:x}", hash);
                Some(KnownRecord::Blue2048PlayerProfile(record))
            }
            KnownRecord::Blue2048PlayerStats(mut record) => {
                record.sync_status.synced_with_at_repo = synced;
                record.sync_status.updated_at = Datetime::now();
                let string_data = serde_json::to_string(&record).unwrap();
                let hash = const_xxh3(string_data.as_bytes());
                record.sync_status.hash = format!("{:x}", hash);
                Some(KnownRecord::Blue2048PlayerStats(record))
            }
            //Not every record is a write or needs a hash set
            _ => None,
        }
    }

    pub async fn get_remote_record<Record: std::convert::From<atrium_api::types::Unknown>>(
        &self,
        collection: &str,
        key: RecordKey,
    ) -> Result<Record, AtRepoSyncError> {
        match &self.client {
            None => Err(AtRepoSyncError::Error("No client".to_string())),
            Some(client) => {
                match client
                    .api
                    .com
                    .atproto
                    .repo
                    .get_record(
                        atrium_api::com::atproto::repo::get_record::ParametersData {
                            cid: None,
                            collection: collection.parse().unwrap(),
                            repo: AtIdentifier::Did(self.users_did.clone().unwrap()),
                            rkey: key,
                        }
                        .into(),
                    )
                    .await
                {
                    Ok(result) => Ok(result.value.clone().into()),
                    Err(err) => Err(AtRepoSyncError::AtRepoCallError(err.to_string())),
                }
            }
        }
    }

    pub async fn create_a_new_player_profile(
        &self,
    ) -> Result<player::profile::RecordData, AtRepoSyncError> {
        let mut new_user_profile = player::profile::RecordData {
            created_at: Datetime::now(),
            solo_play: false,
            sync_status: _2048::defs::SyncStatusData {
                created_at: Datetime::now(),
                hash: "".to_string(),
                synced_with_at_repo: true,
                updated_at: Datetime::now(),
            }
            .into(),
        };
        let string_data = serde_json::to_string(&new_user_profile).unwrap();
        let hash = const_xxh3(string_data.as_bytes());
        new_user_profile.sync_status.hash = format!("{:x}", hash);

        let new_user_profile_record: KnownRecord = new_user_profile.clone().into();
        let mut synced_with_at_repo = false;
        match &self.client {
            None => {}
            Some(client) => {
                let create_request = client
                    .api
                    .com
                    .atproto
                    .repo
                    .create_record(
                        atrium_api::com::atproto::repo::create_record::InputData {
                            collection: blue::_2048::player::Profile::NSID.parse().unwrap(),
                            record: new_user_profile_record.into(),
                            //TODO unwrap is not best, but ideally if we have a client we should have a did
                            repo: AtIdentifier::Did(self.users_did.clone().unwrap()),
                            rkey: Some(SELF_KEY.parse().unwrap()),
                            swap_commit: None,
                            validate: None,
                        }
                        .into(),
                    )
                    .await;
                match create_request {
                    Ok(_) => {
                        synced_with_at_repo = true;
                    }
                    Err(err) => {
                        //Just going to log errors "quietly" as I figure out how to handle them
                        log::error!("{:?}", err);
                    }
                }
            }
        }

        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        new_user_profile.sync_status.synced_with_at_repo = synced_with_at_repo;
        //Since it did not sync with the at repo we need to update the hash
        if !synced_with_at_repo {
            let string_data = serde_json::to_string(&new_user_profile).unwrap();
            let hash = const_xxh3(string_data.as_bytes());
            new_user_profile.sync_status.hash = format!("{:x}", hash);
        }
        match transaction_put(
            db,
            new_user_profile.clone(),
            PROFILE_STORE,
            Some(SELF_KEY.to_string()),
        )
        .await
        {
            Ok(_) => Ok(new_user_profile),
            Err(err) => Err(AtRepoSyncError::Error(err.to_string())),
        }
    }

    pub async fn get_local_player_profile(
        &self,
    ) -> Result<Option<player::profile::RecordData>, AtRepoSyncError> {
        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        match object_get::<player::profile::RecordData>(db, PROFILE_STORE, SELF_KEY).await {
            Ok(profile) => Ok(profile),
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        }
    }

    pub async fn sync_profiles(&self) -> Result<(), AtRepoSyncError> {
        let local_profile = self.get_local_player_profile().await?;
        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        match self
            .get_remote_record::<blue::_2048::player::profile::RecordData>(
                blue::_2048::player::Profile::NSID,
                SELF_KEY.parse().unwrap(),
            )
            .await
        {
            // There is a remote profile
            Ok(remote_profile) => match local_profile {
                //There is a local profile and a remote one
                Some(local_profile) => {
                    if local_profile.sync_status.hash != remote_profile.sync_status.hash {
                        //Just taking remote over all rn
                        transaction_put(
                            db,
                            remote_profile.clone(),
                            PROFILE_STORE,
                            Some(SELF_KEY.to_string()),
                        )
                        .await
                        .map_err(|err| AtRepoSyncError::LocalRepoError(err.to_string()))
                    } else {
                        Ok(())
                    }
                }
                //There was no local profile saving a remoteone
                None => transaction_put(
                    db,
                    remote_profile.clone(),
                    PROFILE_STORE,
                    Some(SELF_KEY.to_string()),
                )
                .await
                .map_err(|err| AtRepoSyncError::LocalRepoError(err.to_string())),
            },
            //There is no remote profile or was unable to get it
            Err(_err) => {
                //Not right but assuming I just create a new one
                self.create_a_new_player_profile().await?;
                Ok(())
            }
        }
    }

    //A copy and paste of the same method for profile. I don't have it in me to do a proper trait/generic method
    pub async fn create_a_new_player_stats(
        &self,
    ) -> Result<player::stats::RecordData, AtRepoSyncError> {
        let mut new_player_stats = player::stats::RecordData {
            average_score: 0,
            created_at: Datetime::now(),
            games_played: 0,
            highest_number_block: 0,
            highest_score: 0,
            least_moves_to_find_twenty_forty_eight: 0,
            sync_status: _2048::defs::SyncStatusData {
                created_at: Datetime::now(),
                hash: "".to_string(),
                synced_with_at_repo: true,
                updated_at: Datetime::now(),
            }
            .into(),
            times_twenty_forty_eight_been_found: 0,
            total_score: 0,
        };
        let string_data = serde_json::to_string(&new_player_stats).unwrap();
        let hash = const_xxh3(string_data.as_bytes());
        new_player_stats.sync_status.hash = format!("{:x}", hash);

        let new_player_stats_record: KnownRecord = new_player_stats.clone().into();
        let mut synced_with_at_repo = false;
        match &self.client {
            None => {}
            Some(client) => {
                let create_request = client
                    .api
                    .com
                    .atproto
                    .repo
                    .create_record(
                        atrium_api::com::atproto::repo::create_record::InputData {
                            collection: blue::_2048::player::Stats::NSID.parse().unwrap(),
                            record: new_player_stats_record.into(),
                            //TODO unwrap is not best, but ideally if we have a client we should have a did
                            repo: AtIdentifier::Did(self.users_did.clone().unwrap()),
                            rkey: Some(SELF_KEY.parse().unwrap()),
                            swap_commit: None,
                            validate: None,
                        }
                        .into(),
                    )
                    .await;
                match create_request {
                    Ok(_) => {
                        synced_with_at_repo = true;
                    }
                    Err(err) => {
                        //Just going to log errors "quietly" as I figure out how to handle them
                        log::error!("{:?}", err);
                    }
                }
            }
        }

        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        new_player_stats.sync_status.synced_with_at_repo = synced_with_at_repo;
        //Since it did not sync with the at repo we need to update the hash
        if !synced_with_at_repo {
            let string_data = serde_json::to_string(&new_player_stats).unwrap();
            let hash = const_xxh3(string_data.as_bytes());
            new_player_stats.sync_status.hash = format!("{:x}", hash);
        }
        match transaction_put(
            db,
            new_player_stats.clone(),
            STATS_STORE,
            Some(SELF_KEY.to_string()),
        )
        .await
        {
            Ok(_) => Ok(new_player_stats),
            Err(err) => Err(AtRepoSyncError::Error(err.to_string())),
        }
    }

    pub async fn update_a_player_stats(
        &self,
        mut new_stats: player::stats::RecordData,
    ) -> Result<(), AtRepoSyncError> {
        //TODO probably not most efficient but call a sync before
        self.sync_stats().await?;
        new_stats.sync_status.updated_at = Datetime::now();
        let string_data = serde_json::to_string(&new_stats).unwrap();
        let hash = const_xxh3(string_data.as_bytes());
        new_stats.sync_status.hash = format!("{:x}", hash);

        let new_player_stats_record: KnownRecord = new_stats.clone().into();
        let mut synced_with_at_repo = false;
        match &self.client {
            None => {}
            Some(client) => {
                let create_request = client
                    .api
                    .com
                    .atproto
                    .repo
                    .put_record(
                        atrium_api::com::atproto::repo::put_record::InputData {
                            collection: blue::_2048::player::Stats::NSID.parse().unwrap(),
                            record: new_player_stats_record.into(),
                            //TODO unwrap is not best, but ideally if we have a client we should have a did
                            repo: AtIdentifier::Did(self.users_did.clone().unwrap()),
                            rkey: SELF_KEY.parse().unwrap(),
                            swap_commit: None,
                            swap_record: None,
                            validate: None,
                        }
                        .into(),
                    )
                    .await;
                match create_request {
                    Ok(_) => {
                        synced_with_at_repo = true;
                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                        match atrium_xrpc::Error::from(err) {
                            Authentication(_) => {
                                return Err(AtRepoSyncError::AuthErrorNeedToReLogin);
                            }
                            _ => {
                                //Just going to log errors "quietly" as I figure out how to handle them
                            }
                        }
                    }
                }
            }
        }

        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        new_stats.sync_status.synced_with_at_repo = synced_with_at_repo;
        //Since it did not sync with the at repo we need to update the hash
        if !synced_with_at_repo {
            let string_data = serde_json::to_string(&new_stats).unwrap();
            let hash = const_xxh3(string_data.as_bytes());
            new_stats.sync_status.hash = format!("{:x}", hash);
        }
        match transaction_put(
            db,
            new_stats.clone(),
            STATS_STORE,
            Some(SELF_KEY.to_string()),
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(AtRepoSyncError::Error(err.to_string())),
        }
    }

    pub async fn get_local_player_stats(
        &self,
    ) -> Result<Option<player::stats::RecordData>, AtRepoSyncError> {
        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        match object_get::<player::stats::RecordData>(db, STATS_STORE, SELF_KEY).await {
            Ok(stats) => Ok(stats),
            Err(err) => Err(AtRepoSyncError::Error(err.to_string())),
        }
    }

    pub async fn sync_stats(&self) -> Result<(), AtRepoSyncError> {
        let local_stats = self.get_local_player_stats().await?;
        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        match self
            .get_remote_record::<blue::_2048::player::stats::RecordData>(
                blue::_2048::player::Stats::NSID,
                SELF_KEY.parse().unwrap(),
            )
            .await
        {
            // There is a remote profile
            Ok(remote_stats) => match local_stats {
                //There is a local profile and a remote one
                Some(local_profile) => {
                    if local_profile.sync_status.hash != remote_stats.sync_status.hash {
                        //Just taking remote over all rn
                        transaction_put(
                            db,
                            remote_stats.clone(),
                            STATS_STORE,
                            Some(SELF_KEY.to_string()),
                        )
                        .await
                        .map_err(|err| AtRepoSyncError::LocalRepoError(err.to_string()))
                    } else {
                        Ok(())
                    }
                }
                None => transaction_put(
                    db,
                    remote_stats.clone(),
                    STATS_STORE,
                    Some(SELF_KEY.to_string()),
                )
                .await
                .map_err(|err| AtRepoSyncError::LocalRepoError(err.to_string())),
            },
            //There is no remote profile or was unable to get it
            Err(_err) => {
                //Not right but assuming I just create a new one
                self.create_a_new_player_stats().await?;
                Ok(())
            }
        }
    }

    pub async fn create_a_new_game(
        &self,
        mut new_game: blue::_2048::game::RecordData,
        record_key: RecordKey,
        game_hash: String,
    ) -> Result<(), AtRepoSyncError> {
        let string_data = serde_json::to_string(&new_game).unwrap();
        let hash = const_xxh3(string_data.as_bytes());
        new_game.sync_status.hash = format!("{:x}", hash);

        let new_game_record: KnownRecord = new_game.clone().into();
        let mut synced_with_at_repo = false;
        match &self.client {
            None => {}
            Some(client) => {
                let create_request = client
                    .api
                    .com
                    .atproto
                    .repo
                    .create_record(
                        atrium_api::com::atproto::repo::create_record::InputData {
                            collection: blue::_2048::Game::NSID.parse().unwrap(),
                            record: new_game_record.into(),
                            //TODO unwrap is not best, but ideally if we have a client we should have a did
                            repo: AtIdentifier::Did(self.users_did.clone().unwrap()),
                            rkey: Some(record_key.clone()),
                            swap_commit: None,
                            validate: None,
                        }
                        .into(),
                    )
                    .await;
                match create_request {
                    Ok(_) => {
                        synced_with_at_repo = true;
                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                        match atrium_xrpc::Error::from(err) {
                            Authentication(_) => {
                                return Err(AtRepoSyncError::AuthErrorNeedToReLogin);
                            }
                            _ => {
                                return Err(AtRepoSyncError::Error(String::from(
                                    "Error saving the game remotely",
                                )));
                                //Just going to log errors "quietly" as I figure out how to handle them
                            }
                        }
                    }
                }
            }
        }

        let db = match Database::open(DB_NAME).await {
            Ok(db) => db,
            Err(err) => {
                return Err(AtRepoSyncError::Error(err.to_string()));
            }
        };

        // new_game.sync_status.synced_with_at_repo = synced_with_at_repo;
        //Since it did not sync with the at repo we need to update the hash
        new_game.sync_status.synced_with_at_repo = synced_with_at_repo;
        if !synced_with_at_repo {
            let string_data = serde_json::to_string(&new_game).unwrap();
            let hash = const_xxh3(string_data.as_bytes());
            new_game.sync_status.hash = format!("{:x}", hash);
        }

        let local_game_record = RecordStorageWrapper {
            rkey: record_key,
            record: new_game.clone(),
            index_hash: game_hash,
        };
        match transaction_put(db, local_game_record.clone(), GAME_STORE, None).await {
            Ok(_) => Ok(()),
            Err(err) => Err(AtRepoSyncError::Error(err.to_string())),
        }
    }

    pub async fn get_remote_games(
        &self,
        cursor: Option<String>,
        limit: Option<u8>,
    ) -> Result<
        (
            Rc<Vec<Rc<RecordStorageWrapper<blue::_2048::game::RecordData>>>>,
            Option<String>,
        ),
        AtRepoSyncError,
    > {
        let client = match self.client.as_ref() {
            None => {
                return Err(AtRepoSyncError::Error(String::from(
                    "There was no client setup.",
                )));
            }
            Some(client) => client,
        };
        // Some(LimitedNonZeroU16::try_from(2000_u16).unwrap()),
        let limit = match limit {
            None => None,
            Some(limit) => LimitedNonZeroU8::try_from(limit).ok(),
        };

        //HACK unwrapping the did for now since we know we have it since we have a client
        let did = self.users_did.clone().unwrap();

        match client
            .api
            .com
            .atproto
            .repo
            .list_records(
                atrium_api::com::atproto::repo::list_records::ParametersData {
                    collection: blue::_2048::Game::NSID.parse().unwrap(),
                    cursor,
                    limit,
                    repo: AtIdentifier::Did(did),
                    reverse: None,
                }
                .into(),
            )
            .await
        {
            Ok(result) => {
                Ok((Rc::from(
                    result
                        .records
                        .iter()
                        .map(|record| {
                            let rkey = parse_record_key(&record.uri)
                                .map_err(|e| AtRepoSyncError::Error(e))
                                .unwrap();
                            Rc::new(RecordStorageWrapper {
                                rkey,
                                record: record.value.clone().into(),
                                //Leaving empty for now. We can get the hash, but atm we're not syncing
                                //remote to local so would affect that
                                index_hash: "".to_string(),
                            })
                        })
                        .collect::<Vec<Rc<RecordStorageWrapper<blue::_2048::game::RecordData>>>>(),
                ), result.cursor.clone()))
            }
            Err(err) => Err(AtRepoSyncError::AtRepoCallError(err.to_string())),
        }
    }
}

pub fn parse_record_key(at_uri: &str) -> Result<RecordKey, String> {
    if let Some(record_key) = at_uri.split("/").last() {
        let record_key = record_key.parse::<RecordKey>()?;
        return Ok(record_key);
    }
    Err(String::from("Unable to parse record key"))
}
