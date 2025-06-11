use async_trait::async_trait;
use atrium_api::types::Collection;
use backend_shared::database::Database;
use dotenv::dotenv;
use rocketman::{
    connection::JetstreamConnection,
    handler,
    ingestion::LexiconIngestor,
    options::JetstreamOptions,
    types::event::{Commit, Event},
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, sync::Mutex};
use types_2048::blue::_2048::Game;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database = Database::new(&db_url).await.map_err(anyhow::Error::from)?;

    // init the builder
    let opts = JetstreamOptions::builder()
        // your EXACT nsids
        .wanted_collections(vec![Game::NSID.to_string()])
        .build();
    // create the jetstream connector
    let jetstream = JetstreamConnection::new(opts);

    // create your ingestors
    let mut ingestors: HashMap<String, Box<dyn LexiconIngestor + Send + Sync>> = HashMap::new();
    ingestors.insert(
        // your EXACT nsid
        Game::NSID.to_string(),
        Box::new(GameIngestor { database }),
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
}

/// A cool ingestor implementation. Will just print the message. Does not do verification.
#[async_trait]
impl LexiconIngestor for GameIngestor {
    async fn ingest(&self, message: Event<Value>) -> anyhow::Result<()> {
        if let Some(Commit {
            record: Some(record),
            ..
        }) = message.commit
        {
            if let Some(Value::String(text)) = record.get("text") {
                println!("{text:?}");
            }
        }
        Ok(())
    }
}
