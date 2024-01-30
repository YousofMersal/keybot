use std::collections::HashMap;

use color_eyre::eyre::Result;
use sqlx::{
    migrate::MigrateDatabase,
    sqlite::{Sqlite, SqlitePoolOptions},
    Pool,
};
use tokio::io::AsyncBufReadExt;
use tracing::debug;

pub async fn connect_or_create(database_name: &str) -> Result<Pool<Sqlite>> {
    Sqlite::database_exists(&database_name).await?;

    if !Sqlite::database_exists(&database_name).await? {
        Sqlite::create_database(&database_name).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect(&database_name)
        .await?;

    Ok(pool)
}

pub async fn add_tables(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query!(
        r#"
CREATE TABLE IF NOT EXISTS keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    key_val VARCHAR(255) NOT NULL,
    claimed BOOLEAN DEFAULT FALSE NOT NULL,
    user_claim VARCHAR(255),
    claimed_at DATE,
    added_at DATE DEFAULT (datetime('now', 'localtime')),
    claim_round INTEGER,
    UNIQUE (key_val),
    FOREIGN KEY (user_claim) references users (id),
    FOREIGN KEY (claim_round) REFERENCES giveaway_rounds (round_id)
);"#
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
CREATE TABLE IF NOT EXISTS config (
    key VARCHAR(255) PRIMARY KEY NOT NULL,
    value VARCHAR(255) NOT NULL
);"#
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
CREATE TABLE IF NOT EXISTS giveaway_rounds (
    round_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    status VARCHAR(255) NOT NULL -- e.g., 'active', 'completed'
);"#
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    username VARCHAR(255) NOT NULL,
    UNIQUE (username)
);"#
    )
    .execute(pool)
    .await?;

    Ok(())
}

// claims a key for a user and returns the key and marks the key as claimed
pub async fn claim_key_with_user(pool: &Pool<Sqlite>, user: &str) -> Result<String> {
    let key = sqlx::query!(
        r#"
        SELECT key_val FROM keys WHERE claimed = FALSE LIMIT 1;
        "#
    )
    .fetch_one(pool)
    .await?;

    // add user to user table if they don't exist
    sqlx::query!(
        r#"
        INSERT OR IGNORE INTO users (username) VALUES (?);
        "#,
        user
    )
    .execute(pool)
    .await?;

    // sqlx::query!(r#"
    // SELECT
    // "#);

    sqlx::query!(
        r#"
UPDATE keys SET claimed = TRUE, user_claim = (select id from users where username = ?), claimed_at = datetime('now', 'localtime') WHERE key_val = ?;
        "#,
        user,
        key.key_val
    )
    .execute(pool)
    .await?;

    Ok(key.key_val)
}

pub async fn get_config_val(pool: &Pool<Sqlite>, key: &str) -> Result<String> {
    let val = sqlx::query!(
        r#"
        SELECT value FROM config WHERE key = ?;
        "#,
        key
    )
    .fetch_one(pool)
    .await?;

    Ok(val.value)
}

pub async fn set_round(
    pool: &Pool<Sqlite>,
    round: i64,
    config: &mut HashMap<String, String>,
) -> Result<()> {
    let mut transaction = pool.begin().await?;

    sqlx::query!(
        r#"
        UPDATE giveaway_rounds SET status = 'completed' WHERE status = 'active';
        "#
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query!(
        r#"
        INSERT OR REPLACE INTO giveaway_rounds (round_id, status) VALUES (?,'active');
        "#,
        round
    )
    .execute(&mut *transaction)
    .await?;

    if let Ok(_) = transaction.commit().await {
        config.insert("claim_round".to_owned(), round.to_string());
    } else {
        return Err(color_eyre::eyre::eyre!("Failed to commit transaction"));
    };

    Ok(())
}

pub async fn get_round(pool: &Pool<Sqlite>) -> Result<Option<i64>> {
    let round = sqlx::query!(
        r#"
        SELECT round_id FROM giveaway_rounds WHERE status = 'active';
        "#
    )
    .fetch_optional(pool)
    .await?;

    return match round {
        Some(round) => Ok(Some(round.round_id)),
        None => Ok(None),
    };
}

pub async fn set_config_val(pool: &Pool<Sqlite>, key: &str, value: &str) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT OR REPLACE INTO config (key, value) VALUES (?, ?);
        "#,
        key,
        value
    )
    .execute(pool)
    .await?;

    Ok(())
}

// read beta keys from a file and insert them into the database
pub async fn read_beta_keys_file(pool: &Pool<Sqlite>, file: &str) -> Result<()> {
    let file = tokio::fs::File::open(file).await?;
    let reader = tokio::io::BufReader::new(file);

    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        sqlx::query!(
            r#"
        INSERT OR IGNORE INTO keys (key_val) VALUES (?);
        "#,
            line
        )
        .execute(pool)
        .await?;
    }
    debug!("Done inserting keys into database");
    // let contents = tokio::fs::read_to_string(file).await?;
    // let s = contents.lines().map(String::from).into_iter();

    Ok(())
}
