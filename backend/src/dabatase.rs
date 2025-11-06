use crate::env_u32;
use anyhow::Result;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

const DATABASE_MIN_CONNECTIONS: u32 = env_u32!("DATABASE_MIN_CONNECTIONS");
const DATABASE_MAX_CONNECTIONS: u32 = env_u32!("DATABASE_MAX_CONNECTIONS");

pub async fn connect_db(database_url: &str) -> Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .min_connections(DATABASE_MIN_CONNECTIONS)
        .max_connections(DATABASE_MAX_CONNECTIONS)
        .connect(database_url)
        .await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    #[sqlx::test]
    async fn database_check(pool: PgPool) {
        let row = sqlx::query!(
            r#"
            SELECT $1 as "string!"
            "#,
            "hello"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.string, "hello");
    }
}
