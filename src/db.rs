use anyhow::Result;
use sqlx::{Pool, Sqlite, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};

pub type Db = Pool<Sqlite>;

pub async fn setup_db() -> Db {
    Sqlite::create_database("sqlite:db/db.sqlite")
        .await
        .expect("failed to create database");

    let db = SqlitePoolOptions::new()
        .connect("sqlite:db/db.sqlite")
        .await
        .unwrap();

    run_migrations(&db).await.expect("failed to run migrations");

    db
}

async fn run_migrations(db: &Db) -> Result<()> {
    sqlx::migrate!("./migrations").run(db).await?;

    Ok(())
}

#[cfg(test)]
pub async fn test_db() -> Db {
    let db = SqlitePoolOptions::new().connect(":memory:").await.unwrap();

    run_migrations(&db).await.expect("failed to run migrations");

    db
}

