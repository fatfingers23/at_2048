use atrium_api::agent::atp_agent::AtpSession;
use atrium_api::com::atproto::sync::list_repos_by_collection::Repo;
use atrium_api::types::string::Did;
use atrium_api::types::{LimitedNonZeroU8, LimitedU8, TryIntoUnknown};
use atrium_api::{
    agent::atp_agent::AtpAgent,
    agent::atp_agent::store::MemorySessionStore,
    types::{Collection, LimitedNonZeroU16},
};
use atrium_common::resolver::Resolver;
use atrium_common::store::memory::MemoryStore;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig, DnsTxtResolver},
};
use atrium_oauth::DefaultHttpClient;
use atrium_xrpc_client::reqwest::ReqwestClient;
use clap::{Parser, Subcommand};
use hickory_resolver::TokioAsyncResolver;
use std::collections::HashMap;
use std::sync::Arc;
use twothousand_forty_eight::unified::validation::Validatable;
use twothousand_forty_eight::v2::recording::SeededRecording;

use types_2048::blue;

const RELAY_ENDPOINT: &str = "https://relay1.us-east.bsky.network";

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

#[derive(Debug)]
struct TempLeaderboardPlace {
    pub did: Did,
    pub handle: Option<String>,
    pub pds_url: String,
    pub top_score: Option<usize>,
    pub top_score_uri: Option<String>,
}
async fn create_a_temp_leaderboard() -> anyhow::Result<()> {
    log::info!("Creating a temp leaderboard...");
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
    let result = agent
        .api
        .com
        .atproto
        .sync
        .list_repos_by_collection(
            atrium_api::com::atproto::sync::list_repos_by_collection::ParametersData {
                collection: blue::_2048::Game::NSID.parse().unwrap(),
                cursor: None,
                limit: Some(LimitedNonZeroU16::try_from(2000_u16).unwrap()),
            }
            .into(),
        )
        .await;
    let output = match result {
        Ok(output) => output,
        Err(err) => {
            anyhow::bail!("{:?}", err)
        }
    };
    let mut resolve_count = 0;
    let mut hashmap_by_pds: HashMap<String, Vec<TempLeaderboardPlace>> = HashMap::new();
    for repo in &output.repos {
        resolve_count += 1;
        let resolved_did = match did_resolver.resolve(&repo.did).await {
            Ok(doc) => doc,
            Err(err) => {
                log::error!("Error resolving: {} {:?}", &repo.did.to_string(), err);
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
                });
            }
        }
        if resolve_count % 10 == 0 {
            log::info!("{} repos resolved", resolve_count);
        }
    }
    log::info!(
        "{} repos resolved. Getting games from the repos now.",
        resolve_count
    );

    let mut leaderboards: Vec<TempLeaderboardPlace> = Vec::new();
    for (pds_url, repos) in hashmap_by_pds.iter_mut() {
        log::info!("Getting {} repos from {},", repos.len(), pds_url);
        let pds_agent = AtpAgent::new(ReqwestClient::new(pds_url), MemorySessionStore::default());
        for repo in repos {
            match get_top_game(&pds_agent, &repo.did, &repo.handle).await {
                Ok(new_leaderboard_place) => {
                    leaderboards.push(new_leaderboard_place);
                }
                Err(err) => {
                    log::error!("Error getting top game: {}", err);
                }
            }
        }
    }

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

async fn get_top_game(
    atp_agent: &AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
    did: &Did,
    handle: &Option<String>,
) -> anyhow::Result<TempLeaderboardPlace> {
    let mut cursor = None;
    let mut keep_calling = true;
    let mut top_score = 0;
    let mut top_score_uri: Option<String> = None;

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
                        Ok(real_score) => {
                            if real_score > top_score {
                                top_score = real_score;
                                top_score_uri = Some(record.uri.clone());
                            }
                        }
                        Err(err) => {
                            log::error!("Error parsing game: {}", err);
                        }
                    }
                }
            }
            Err(e) => log::error!("Error getting top game: {}", e),
        };
    }

    Ok(TempLeaderboardPlace {
        did: did.clone(),
        handle: handle.clone(),
        pds_url: "https://relay1.us-east.bsky.network".to_string(),
        top_score: Some(top_score),
        top_score_uri: top_score_uri,
    })
}

fn parse_game_and_validate(game: &String) -> anyhow::Result<usize> {
    let history: SeededRecording = match game.parse() {
        Ok(history) => history,
        Err(err) => Err(anyhow::anyhow!("Error parsing game: {}", err))?,
    };

    return match history.validate() {
        Ok(valid_history) => {
            if valid_history.score > 0 {
                Ok(valid_history.score)
            } else {
                Err(anyhow::anyhow!("Invalid game: {}", game))
            }
        }
        Err(err) => {
            log::error!("Error validating game: {}", err);
            Err(anyhow::anyhow!("Invalid game: {}", game))
        }
    };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cli = Cli::parse();
    match &cli.command {
        Commands::Leaderboard(Leaderboard { subcommand }) => match subcommand {
            LeaderboardCommands::Temp => create_a_temp_leaderboard().await,
        },
    }
}
