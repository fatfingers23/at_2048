use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;

pub struct Database{
    pool: Pool<Postgres>
}

#[derive(Error,Debug)]
pub enum DatabaseError{
    SqlxError(#[from] sqlx::Error),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::SqlxError(e) => write!(f, "Database error: {}", e),
        }
    }
}


impl Database{
    pub async fn new(db_url: &str)-> Result<Self, DatabaseError>{
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url).await?;
        
        Ok(Self{
            pool
        })
    }
}

