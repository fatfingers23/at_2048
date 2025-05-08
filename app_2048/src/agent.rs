use crate::at_repo_sync::{AtRepoSync, AtRepoSyncError};
use crate::idb::{DB_NAME, GAME_STORE, RecordStorageWrapper, StorageError, object_get_index};
use crate::oauth_client::oauth_client;
use atrium_api::agent::Agent;
use atrium_api::types::LimitedU32;
use atrium_api::types::string::{Datetime, Did, Tid};
use indexed_db_futures::database::Database;
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use twothousand_forty_eight::unified::game::GameState;
use twothousand_forty_eight::unified::hash::Hashable;
use twothousand_forty_eight::unified::reconstruction::Reconstructable;
use twothousand_forty_eight::v2::recording::SeededRecording;
use types_2048::blue;
use types_2048::blue::_2048::defs::SyncStatusData;
use types_2048::blue::_2048::game;
use wasm_bindgen::JsValue;
use yew_agent::Codec;
use yew_agent::prelude::*;

/// Postcard codec for worker messages serialization.
pub struct Postcard;

impl Codec for Postcard {
    fn encode<I>(input: I) -> JsValue
    where
        I: Serialize,
    {
        let buf = postcard::to_allocvec(&input).expect("can't serialize a worker message");
        Uint8Array::from(buf.as_slice()).into()
    }

    fn decode<O>(input: JsValue) -> O
    where
        O: for<'de> Deserialize<'de>,
    {
        let data = Uint8Array::from(input).to_vec();
        postcard::from_bytes(&data).expect("can't deserialize a worker message")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StorageRequest {
    // GameCompleted(RecordStorageWrapper<game::RecordData>),
    GameCompleted(String, Option<Did>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StorageResponse {
    Success,
    AlreadySynced,
    Error(StorageError),
    RepoError(AtRepoSyncError),
}

#[oneshot]
pub async fn StorageTask(request: StorageRequest) -> StorageResponse {
    let _db = match Database::open(DB_NAME).await {
        Ok(db) => db,
        Err(err) => {
            return StorageResponse::Error(StorageError::OpenDbError(err.to_string()));
        }
    };

    let response = match request {
        StorageRequest::GameCompleted(game_history, did) => {
            // transaction_put(db, game, GAME_STORE, None).await
            handle_game_completed(game_history, did).await
        }
    };
    response.unwrap_or_else(|error| StorageResponse::RepoError(error))
}

pub async fn handle_game_completed(
    game_history: String,
    did: Option<Did>,
) -> Result<StorageResponse, AtRepoSyncError> {
    let seeded_recording: SeededRecording = match game_history.clone().parse() {
        Ok(seeded_recording) => seeded_recording,
        Err(err) => {
            return Err(AtRepoSyncError::Error(err.to_string()));
        }
    };
    let at_repo_sync = match did {
        None => AtRepoSync::new_local_repo(),
        Some(did) => {
            let oauth_client = oauth_client().await;
            let session = match oauth_client.restore(&did).await {
                Ok(session) => session,
                Err(err) => {
                    log::error!("{:?}", err);
                    return Err(AtRepoSyncError::Error(err.to_string()));
                }
            };

            let agent = Agent::new(session);

            AtRepoSync::new_logged_in_repo(agent, did)
        }
    };

    let db = match Database::open(DB_NAME).await {
        Ok(db) => db,
        Err(err) => {
            return Err(AtRepoSyncError::Error(err.to_string()));
        }
    };

    let already_saved: Option<RecordStorageWrapper<game::RecordData>> =
        object_get_index(db, GAME_STORE, &seeded_recording.game_hash())
            .await
            .map_err(|err| AtRepoSyncError::Error(err.to_string()))?;
    if let Some(already_saved) = already_saved {
        if already_saved.record.sync_status.synced_with_at_repo || !at_repo_sync.can_remote_sync() {
            log::info!("already saved or cannot sync");
            return Ok(StorageResponse::AlreadySynced);
        } else {
            //TODO sync with remote repo idk what I want to do here yet
        }
        log::info!("already saved");
        return Ok(StorageResponse::AlreadySynced);
    }

    let gamestate = match GameState::from_reconstructable_ruleset(&seeded_recording) {
        Ok(gamestate) => gamestate,
        Err(e) => {
            log::error!("Error reconstructing game: {:?}", e.to_string());
            return Err(AtRepoSyncError::Error(e.to_string()));
        }
    };

    let record = blue::_2048::game::RecordData {
        completed: gamestate.over,
        created_at: Datetime::now(),
        current_score: gamestate.score_current as i64,
        seeded_recording: game_history,
        sync_status: SyncStatusData {
            created_at: Datetime::now(),
            hash: "".to_string(),
            //Defaults to true till proven it is not synced
            synced_with_at_repo: true,
            updated_at: Datetime::now(),
        }
        .into(),
        won: gamestate.won,
    };

    // if at_repo_sync.can_remote_sync() {
    let stats_sync = at_repo_sync.sync_stats().await;
    if stats_sync.is_err() {
        if at_repo_sync.can_remote_sync() {
            match stats_sync.err() {
                None => {}
                Some(err) => {
                    log::error!("Error syncing stats: {:?}", err);
                    if let AtRepoSyncError::AuthErrorNeedToReLogin = err {
                        return Err(AtRepoSyncError::AuthErrorNeedToReLogin);
                    }
                }
            }
        }
    }

    let mut stats = match at_repo_sync.get_local_player_stats().await {
        Ok(stats) => match stats {
            None => {
                return Err(AtRepoSyncError::Error(
                    "No stats found. Good chance they were never created if syncing is off. Or something much worse now."
                        .to_string(),
                ));
            }
            Some(stats) => stats,
        },
        Err(err) => {
            return Err(err);
        }
    };

    let highest_block_this_game = gamestate
        .board
        .tiles
        .iter()
        .flatten()
        .filter_map(|tile| *tile)
        .map(|x| x.value)
        .max()
        .unwrap_or(0) as i64;

    //Update the stats
    stats.games_played += 1;
    stats.total_score += gamestate.score_current as i64;
    stats.average_score = stats.total_score / stats.games_played;
    if highest_block_this_game > stats.highest_number_block {
        stats.highest_number_block = highest_block_this_game;
    }

    if gamestate.score_current as i64 > stats.highest_score {
        stats.highest_score = gamestate.score_current as i64;
    }

    let reconstruction = match seeded_recording.reconstruct() {
        Ok(reconstruction) => reconstruction,
        Err(err) => {
            return Err(AtRepoSyncError::Error(err.to_string()));
        }
    };

    let mut twenty_48_this_game: Vec<usize> = vec![];
    let mut turns_till_2048 = 0;
    let mut turns = 0;
    for board_in_the_moment in reconstruction.history {
        turns += 1;

        for tile in board_in_the_moment
            .tiles
            .iter()
            .flatten()
            .filter_map(|tile| *tile)
        {
            if tile.value as i64 > stats.highest_number_block {
                stats.highest_number_block = tile.value as i64;
            }

            if tile.value as i64 == 2048 && twenty_48_this_game.contains(&tile.id) == false {
                if turns_till_2048 == 0 {
                    turns_till_2048 = turns;
                    if turns < stats.least_moves_to_find_twenty_forty_eight {
                        stats.least_moves_to_find_twenty_forty_eight = turns;
                    }
                    // stats.least_moves_to_find_twenty_forty_eight
                }
                stats.times_twenty_forty_eight_been_found += 1;
                twenty_48_this_game.push(tile.id);
            }
        }
    }

    at_repo_sync.update_a_player_stats(stats).await?;

    let tid = Tid::now(LimitedU32::MIN);
    at_repo_sync
        .create_a_new_game(record, tid, seeded_recording.game_hash())
        .await?;

    Ok(StorageResponse::Success)
}
