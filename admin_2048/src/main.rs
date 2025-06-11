use atrium_api::agent::atp_agent::AtpSession;
use atrium_api::types::string::{Did, Nsid};
use atrium_api::types::{LimitedNonZeroU8, TryIntoUnknown};
use atrium_api::{
    agent::atp_agent::AtpAgent,
    agent::atp_agent::store::MemorySessionStore,
    types::{Collection, LimitedNonZeroU16},
};
use atrium_common::resolver::Resolver;
use atrium_common::store::memory::MemoryStore;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::AtprotoHandleResolverConfig,
};
use atrium_oauth::DefaultHttpClient;
use atrium_xrpc_client::reqwest::ReqwestClient;
use backend_shared::cache::{Cache, DID_DOC_KEY_PREFIX, RedisFetchErrors};
use backend_shared::database::Database;
use backend_shared::game_util::parse_game_and_validate;
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::collections::HashMap;
use std::sync::Arc;
use types_2048::blue;

const RELAY_ENDPOINT: &str = "https://relay1.us-west.bsky.network";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Admin actions for leaderboards
    Leaderboard(Leaderboard),
    //
    /// Backfill commands
    Backfill {
        #[command(flatten)]
        command: BackfillCommands,
    },
}

#[derive(Parser, Debug)]
#[command(name = "leaderboard", about = "Actions for leaderboards")]
struct Leaderboard {
    /// Type of lexicon generation
    #[command(subcommand)]
    subcommand: LeaderboardCommands,
}

#[derive(Subcommand, Debug)]
enum LeaderboardCommands {
    /// Generates the temp leaderboard
    Temp,
}

#[derive(Parser, Debug)]
#[command(name = "backfill", about = "Backfill actions")]
struct BackfillCommands {
    #[arg(value_enum)]
    action: BackfillAction,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum BackfillAction {
    Games,
}

#[derive(Debug)]
struct TempLeaderboardPlace {
    pub did: Did,
    pub handle: Option<String>,
    pub pds_url: String,
    pub top_score: Option<usize>,
    pub top_score_uri: Option<String>,
    pub games_played: usize,
}
async fn create_a_temp_leaderboard(
    agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    did_resolver: &CommonDidResolver<DefaultHttpClient>,
    cache: &mut Cache,
) -> anyhow::Result<()> {
    log::info!("Creating a temp leaderboard...");

    let (resolve_count, mut hashmap_by_pds) = get_repos(
        &did_resolver,
        &agent,
        cache,
        blue::_2048::Game::NSID.parse().unwrap(),
    )
    .await;
    log::info!(
        "{} repos resolved. Getting games from the repos now.",
        resolve_count
    );

    let mut global_games_played = 0;

    let mut leaderboards: Vec<TempLeaderboardPlace> = Vec::new();
    for (pds_url, repos) in hashmap_by_pds.iter_mut() {
        log::info!("Getting {} repos from {},", repos.len(), pds_url);
        let pds_agent = AtpAgent::new(ReqwestClient::new(pds_url), MemorySessionStore::default());
        for repo in repos {
            match get_top_game(&pds_agent, &repo.did, &repo.handle).await {
                Ok(new_leaderboard_place) => {
                    global_games_played += new_leaderboard_place.games_played;
                    leaderboards.push(new_leaderboard_place);
                }
                Err(err) => {
                    log::error!("Error getting top game: {}", err);
                    log::error!("Skipping repo: {}", repo.did.to_string());
                    continue;
                }
            }
        }
    }

    log::info!("{} games played", global_games_played);

    // Sort leaderboards by top score in descending order
    leaderboards.sort_by(|a, b| b.top_score.cmp(&a.top_score));

    // Print top 10 entries
    for (index, entry) in leaderboards.iter().enumerate() {
        if let (Some(score), Some(_)) = (entry.top_score, entry.top_score_uri.clone()) {
            let player = match &entry.handle {
                Some(handle) => handle.replace("at://", "@"),
                None => format!("@{}", entry.did.to_string()),
            };

            println!("{}. {:} {}", index + 1, score, player);
        }
    }

    Ok(())
}

async fn backfill_games(
    agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    did_resolver: &CommonDidResolver<DefaultHttpClient>,
    database: Database,
    cache: &mut Cache,
) -> anyhow::Result<()> {
    log::info!("Creating a temp leaderboard...");

    let (resolve_count, mut hashmap_by_pds) = get_repos(
        &did_resolver,
        &agent,
        cache,
        blue::_2048::Game::NSID.parse().unwrap(),
    )
    .await;
    log::info!(
        "{} repos resolved. Getting games from the repos now.",
        resolve_count
    );

    let mut global_games_played = 0;

    let mut leaderboards: Vec<TempLeaderboardPlace> = Vec::new();
    for (pds_url, repos) in hashmap_by_pds.iter_mut() {
        log::info!("Getting {} repos from {},", repos.len(), pds_url);
        let pds_agent = AtpAgent::new(ReqwestClient::new(pds_url), MemorySessionStore::default());
        for repo in repos {
            match save_a_repos_games(&pds_agent, &database, &repo.did).await {
                Ok(()) => {}
                Err(err) => {
                    log::error!("Error getting top game: {}", err);
                    log::error!("Skipping repo: {}", repo.did.to_string());
                    continue;
                }
            }
        }
    }

    log::info!("{} games played", global_games_played);

    // Sort leaderboards by top score in descending order
    leaderboards.sort_by(|a, b| b.top_score.cmp(&a.top_score));

    // Print top 10 entries
    for (index, entry) in leaderboards.iter().enumerate() {
        if let (Some(score), Some(_)) = (entry.top_score, entry.top_score_uri.clone()) {
            let player = match &entry.handle {
                Some(handle) => handle.replace("at://", "@"),
                None => format!("@{}", entry.did.to_string()),
            };

            println!("{}. {:} {}", index + 1, score, player);
        }
    }

    Ok(())
}

async fn save_a_repos_games(
    atp_agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    database: &Database,
    did: &Did,
) -> anyhow::Result<()> {
    let mut cursor = None;
    let mut keep_calling = true;
    while keep_calling {
        log::info!("Getting top game for {}", did.clone().to_string());
        match atp_agent
            .api
            .com
            .atproto
            .repo
            .list_records(
                atrium_api::com::atproto::repo::list_records::ParametersData {
                    collection: types_2048::blue::_2048::Game::NSID.parse().unwrap(),
                    cursor: cursor.clone(),
                    limit: Some(LimitedNonZeroU8::<100>::try_from(100_u8).unwrap()),
                    repo: did.clone().into(),
                    reverse: None,
                }
                .into(),
            )
            .await
        {
            Ok(output) => {
                if output.records.len() == 100 {
                    cursor = output.cursor.clone();
                } else {
                    keep_calling = false;
                    cursor = None;
                }

                for record in &output.records {
                    let game: types_2048::blue::_2048::game::RecordData =
                        types_2048::blue::_2048::game::RecordData::from(record.value.clone());

                    match parse_game_and_validate(&game.seeded_recording) {
                        Ok(result) => {
                            let uri = record.uri.clone();
                            if let Err(error) = database
                                .insert_game(&game, result.hash, result.score as i32, did, &uri)
                                .await
                            {
                                log::error!("Error inserting game: {}", error);
                                continue;
                            }
                        }
                        Err(err) => {
                            log::error!("Error parsing game: {}", err);
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Error getting top game: {}", e);
                break;
            }
        };
    }

    Ok(())
}

async fn get_repos(
    did_resolver: &CommonDidResolver<DefaultHttpClient>,
    agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    cache: &mut Cache,
    nsid: Nsid,
) -> (i32, HashMap<String, Vec<TempLeaderboardPlace>>) {
    let result = agent
        .api
        .com
        .atproto
        .sync
        .list_repos_by_collection(
            atrium_api::com::atproto::sync::list_repos_by_collection::ParametersData {
                collection: nsid,
                cursor: None,
                limit: Some(LimitedNonZeroU16::try_from(2000_u16).unwrap()),
            }
            .into(),
        )
        .await;
    let output = match result {
        Ok(output) => output,
        Err(err) => {
            return (0, HashMap::new());
        }
    };
    let mut resolve_count = 0;
    let mut hashmap_by_pds: HashMap<String, Vec<TempLeaderboardPlace>> = HashMap::new();
    for repo in &output.repos {
        resolve_count += 1;

        // We check the cache first to see if the DidDocument is there already
        // Not really needed here besides multiple re-runs, but more so proof of concept for later usage
        let key = format!("{}:{}", DID_DOC_KEY_PREFIX, repo.did.to_string());
        //43200 = 8hours
        let resolved_did = cache
            .get_or_set(key.as_str(), 43_200, || async {
                did_resolver
                    .resolve(&repo.did)
                    .await
                    .map_err(|e| RedisFetchErrors::Other(e.to_string()))
            })
            .await;
        let resolved_did = match resolved_did {
            Ok(doc) => doc,
            Err(err) => {
                log::error!("Here: Error resolving: {} {:?}", &repo.did.to_string(), err);
                continue;
            }
        };
        let handle = resolved_did.also_known_as.unwrap().get(0).unwrap().clone();
        let pds_url = match resolved_did.service.as_ref().and_then(|services| {
            services
                .iter()
                .find(|service| service.r#type == "AtprotoPersonalDataServer")
                .map(|service| service.service_endpoint.clone())
        }) {
            None => {
                log::error!("No pds url found for {}", &repo.did.to_string());
                continue;
            }
            Some(url) => url,
        };

        match hashmap_by_pds.get_mut(&pds_url) {
            None => {
                hashmap_by_pds.insert(
                    pds_url.clone(),
                    vec![TempLeaderboardPlace {
                        did: repo.did.clone(),
                        handle: Some(handle),
                        pds_url,
                        top_score: None,
                        top_score_uri: None,
                        games_played: 0,
                    }],
                );
            }
            Some(already_exists) => {
                already_exists.push(TempLeaderboardPlace {
                    did: repo.did.clone(),
                    handle: Some(handle),
                    pds_url: pds_url.clone(),
                    top_score: None,
                    top_score_uri: None,
                    games_played: 0,
                });
            }
        }
        if resolve_count % 10 == 0 {
            log::info!("{} repos resolved", resolve_count);
        }
    }
    (resolve_count, hashmap_by_pds)
}

async fn get_top_game(
    atp_agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    did: &Did,
    handle: &Option<String>,
) -> anyhow::Result<TempLeaderboardPlace> {
    let mut cursor = None;
    let mut keep_calling = true;
    let mut top_score = 0;
    let mut top_score_uri: Option<String> = None;
    let mut games_played: usize = 0;
    while keep_calling {
        log::info!("Getting top game for {}", did.clone().to_string());
        match atp_agent
            .api
            .com
            .atproto
            .repo
            .list_records(
                atrium_api::com::atproto::repo::list_records::ParametersData {
                    collection: types_2048::blue::_2048::Game::NSID.parse().unwrap(),
                    cursor: cursor.clone(),
                    limit: Some(LimitedNonZeroU8::<100>::try_from(100_u8).unwrap()),
                    repo: did.clone().into(),
                    reverse: None,
                }
                .into(),
            )
            .await
        {
            Ok(output) => {
                if output.records.len() == 100 {
                    cursor = output.cursor.clone();
                } else {
                    keep_calling = false;
                    cursor = None;
                }
                games_played += output.records.len();

                for record in &output.records {
                    let game: types_2048::blue::_2048::game::RecordData =
                        types_2048::blue::_2048::game::RecordData::from(record.value.clone());

                    match parse_game_and_validate(&game.seeded_recording) {
                        Ok(result) => {
                            // let uri = record.uri.clone();
                            // if let Err(error) = db
                            //     .insert_game(&game, result.hash, result.score as i32, did, &uri)
                            //     .await
                            // {
                            //     log::error!("Error inserting game: {}", error);
                            // }
                            if result.score > top_score {
                                top_score = result.score;
                                top_score_uri = Some(record.uri.clone());
                            }
                        }
                        Err(err) => {
                            log::error!("Error parsing game: {}", err);
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Error getting top game: {}", e);
                break;
            }
        };
    }

    Ok(TempLeaderboardPlace {
        did: did.clone(),
        handle: handle.clone(),
        pds_url: atp_agent.get_endpoint().await,
        top_score: Some(top_score),
        top_score_uri: top_score_uri,
        games_played,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database = Database::new(&db_url).await?;

    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let mut cache = Cache::new(&redis_url).await?;

    let http_client = Arc::new(DefaultHttpClient::default());

    //finds the did document from the users did
    let did_resolver = CommonDidResolver::new(CommonDidResolverConfig {
        plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
        http_client: Arc::clone(&http_client),
    });

    let agent = AtpAgent::new(
        ReqwestClient::new(RELAY_ENDPOINT),
        MemorySessionStore::default(),
    );

    let cli = Cli::parse();
    match &cli.command {
        Commands::Leaderboard(Leaderboard { subcommand }) => match subcommand {
            LeaderboardCommands::Temp => {
                create_a_temp_leaderboard(&agent, &did_resolver, &mut cache).await
            }
        },
        Commands::Backfill { command, .. } => match command.action {
            BackfillAction::Games => {
                backfill_games(&agent, &did_resolver, database, &mut cache).await
            }
        },
    }
}
