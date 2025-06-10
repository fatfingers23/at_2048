use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use thiserror::Error;
use twothousand_forty_eight::unified::hash::Hashable;
use twothousand_forty_eight::v2::recording::SeededRecording;
use types_2048::blue::_2048::game;

pub struct Database {
    pool: Pool<Postgres>,
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Failed to parse SeededRecording: {0}")]
    ParseError(String),
}

impl Database {
    pub async fn new(db_url: &str) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await?;

        Ok(Self { pool })
    }

    /// Inserts a game record into the database
    ///
    /// # Arguments
    ///
    /// * `record` - The game record to insert
    /// * `did` - The DID of the user who created the game
    /// * `at_uri` - The AT URI of the game record
    ///
    /// # Returns
    ///
    /// The ID of the inserted game record
    pub async fn insert_game(
        &self,
        record: &game::RecordData,
        game_hash: String,
        validated_score: i32,
        did: &str,
        at_uri: &String,
    ) -> Result<i64, DatabaseError> {
        // Convert record to JSONB
        let record_json = json!(record);

        // Insert the game record
        let id = sqlx::query!(
            r#"
            INSERT INTO games (game_hash, did, at_uri, record, score)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            game_hash,
            did,
            at_uri,
            record_json,
            validated_score,
        )
        .fetch_one(&self.pool)
        .await?
        .id;

        Ok(id)
    }
}
