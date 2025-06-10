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

    ///
    /// Inserts a new game record into the `games` database table.
    ///
    /// # Parameters
    /// - `record`: A reference to a `RecordData` object representing the game record data to be inserted.
    /// - `game_hash`: A `String` representing the unique hash identifier for the game.
    /// - `validated_score`: An `i32` representing the validated score associated with the game.
    /// - `did`: A string slice (`&str`) representing the decentralized identifier (DID) for the game.
    /// - `at_uri`: A reference to a `String` containing the at://uri associated with the game record.
    ///
    /// # Returns
    /// - `Result<i64, DatabaseError>`: On success, returns the `id` of the inserted game record as an `i64`.
    ///   On failure, returns a `DatabaseError`.
    ///
    /// # Errors
    /// This function will return an error if the database operation fails such as:
    /// - Failure to connect or query the database.
    /// - Any issues during the `INSERT` operation.
    ///
    /// # Notes
    /// - The `record` is converted to a JSONB format before being inserted into the database.
    /// - The function is asynchronous and should be awaited.
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
