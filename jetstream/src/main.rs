use async_trait::async_trait;
use atrium_api::agent::Configure;
use atrium_api::agent::atp_agent::store::MemorySessionStore;
use atrium_api::agent::atp_agent::{AtpAgent, AtpSession};
use atrium_api::types::Collection;
use atrium_api::types::string::Did;
use atrium_common::resolver::Resolver;
use atrium_common::store::memory::MemoryStore;
use atrium_identity::did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL};
use atrium_oauth::DefaultHttpClient;
use atrium_xrpc_client::reqwest::ReqwestClient;
use backend_shared::atproto_util::{get_and_validate_record, parse_did_doc};
use backend_shared::cache::{Cache, DID_DOC_KEY_PREFIX, RedisFetchErrors};
use backend_shared::database::Database;
use dotenv::dotenv;
use rocketman::types::event::Operation;
use rocketman::{
    connection::JetstreamConnection,
    handler,
    ingestion::LexiconIngestor,
    options::JetstreamOptions,
    types::event::Event,
};
use serde_json::Value;
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};
use types_2048::blue::_2048::Game;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database = Database::new(&db_url).await.map_err(anyhow::Error::from)?;

    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let cache = Cache::new(&redis_url).await?;

    let http_client = Arc::new(DefaultHttpClient::default());
    let did_resolver = CommonDidResolver::new(CommonDidResolverConfig {
        plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
        http_client: Arc::clone(&http_client),
    });

    let agent = AtpAgent::new(
        ReqwestClient::new("https://bsky.social"),
        MemorySessionStore::default(),
    );

    // init the builder
    let opts = JetstreamOptions::builder()
        // your EXACT nsids
        .wanted_collections(vec![Game::NSID.to_string()])
        .build();
    // create the jetstream connector
    let jetstream = JetstreamConnection::new(opts);

    // create your ingestors
    let mut ingestors: HashMap<String, Box<dyn LexiconIngestor + Send + Sync + 'static>> =
        HashMap::new();

    ingestors.insert(
        // your EXACT nsid
        Game::NSID.to_string(),
        Box::new(GameIngestor {
            database,
            did_resolver,
            cache: Arc::new(tokio::sync::Mutex::new(cache)),
            agent,
        }),
    );

    // tracks the last message we've processed
    let cursor: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));

    // get channels
    let msg_rx = jetstream.get_msg_rx();
    let reconnect_tx = jetstream.get_reconnect_tx();

    // spawn a task to process messages from the queue.
    // this is a simple implementation, you can use a more complex one based on needs.
    let c_cursor = cursor.clone();
    tokio::spawn(async move {
        while let Ok(message) = msg_rx.recv_async().await {
            if let Err(e) =
                handler::handle_message(message, &ingestors, reconnect_tx.clone(), c_cursor.clone())
                    .await
            {
                eprintln!("Error processing message: {}", e);
            };
        }
    });

    // connect to jetstream
    // retries internally, but may fail if there is an extreme error.
    if let Err(e) = jetstream.connect(cursor.clone()).await {
        eprintln!("Failed to connect to Jetstream: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

pub struct GameIngestor {
    database: Database,
    did_resolver: CommonDidResolver<DefaultHttpClient>,
    cache: Arc<tokio::sync::Mutex<Cache>>,
    agent: AtpAgent<MemoryStore<(), AtpSession>, ReqwestClient>,
}

/// A cool ingestor implementation. Will just print the message. Does not do verification.
#[async_trait]
impl LexiconIngestor for GameIngestor {
    async fn ingest(&self, message: Event<Value>) -> anyhow::Result<()> {
        if let Some(commit) = &message.commit {
            match commit.operation {
                Operation::Update | Operation::Delete => {
                    log::info!("Someone is updating/deleting records?: {commit:?}");
                    return Err(anyhow::anyhow!("Was a update/delete"));
                }
                //This is empty we just wanted to make sure it was not a update/delete
                Operation::Create => {}
            }

            if let Some(record) = &commit.record {
                let status_at_proto_record = serde_json::from_value::<
                    types_2048::blue::_2048::game::RecordData,
                >(record.clone())?;

                if let Some(ref cid) = commit.cid {
                    //The verification you are about to see here is a bit over the top and mostly just done to learn more about verifying records
                    let parsed_did: Did = message
                        .did
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid did"))?;
                    let key = format!("{}:{}", DID_DOC_KEY_PREFIX, message.did);

                    let mut cache = self.cache.lock().await;
                    //43200 = 8hours
                    let resolved_did = match cache
                        .get_or_set(key.as_str(), 43_200, || async {
                            self.did_resolver
                                .resolve(&parsed_did)
                                .await
                                .map_err(|e| RedisFetchErrors::Other(e.to_string()))
                        })
                        .await
                    {
                        Ok(doc) => doc,
                        Err(err) => {
                            return Err(anyhow::anyhow!(
                                "Error resolving: {} {:?}",
                                &message.did,
                                err
                            ));
                        }
                    };

                    let parsed_did_doc = match parse_did_doc(resolved_did) {
                        Ok(parsed_doc) => parsed_doc,
                        Err(e) => {
                            return Err(anyhow::anyhow!("Error parsing did doc: {:?}", e));
                        }
                    };
                    self.agent
                        .configure_endpoint(parsed_did_doc.pds_url.clone());
                    let record = get_and_validate_record::<types_2048::blue::_2048::Game>(
                        &self.agent,
                        &parsed_did_doc,
                        cid.parse()?,
                        commit.rkey.parse().unwrap(),
                        types_2048::blue::_2048::Game::NSID,
                    )
                    .await;
                    log::info!("Record: {:?}", record);

                    return Ok(());
                }
            }
        }

        Err(anyhow::anyhow!("Un expected message: {:?}", message))
    }
}
