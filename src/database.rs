use sqlx::{PgPool, Pool, Postgres};

pub type Database = Pool<Postgres>;

pub async fn create_database_pool(database_url: &str) -> Result<Database, sqlx::Error> {
    let pool = PgPool::connect(database_url).await?;
    
    // Test the connection
    sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await?;
    
    println!("Connected to database successfully!");
    Ok(pool)
}