mod password;

use anyhow::Result;
use scylla::{client::session::Session, statement::batch::Batch};
use uuid::Uuid;

use crate::mutations::users::password::hash_password;

pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub user_hashed_password: String,
}

pub async fn get_user_by_id(session: &Session, user_id: Uuid) -> Result<Option<User>> {
    let query = "SELECT user_id, username, user_hashed_password FROM users_by_id WHERE user_id = ?";

    let result = session
        .query_unpaged(query, (user_id,))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(Uuid, String, String)>()?;

    if let Some(row) = rows.into_iter().next() {
        let (user_id, username, user_hashed_password) = row?;
        Ok(Some(User {
            user_id,
            username,
            user_hashed_password,
        }))
    } else {
        Ok(None)
    }
}

pub async fn create_user(
    session: &Session,
    user_id: Uuid,
    username: &str,
    password: &str,
) -> Result<()> {
    let user_hashed_password = hash_password(password)?;

    let query_by_id =
        "INSERT INTO users_by_id (user_id, username, user_hashed_password) VALUES (?, ?, ?)";
    let query_by_username =
        "INSERT INTO users_by_username (username, user_id, user_hashed_password) VALUES (?, ?, ?)";

    let mut batch = Batch::default();
    batch.append_statement(query_by_id);
    batch.append_statement(query_by_username);

    session
        .batch(
            &batch,
            (
                (user_id, &username, &user_hashed_password),
                (&username, user_id, &user_hashed_password),
            ),
        )
        .await?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_get_user_by_id() -> Result<()> {
        let (session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        // Test existing user
        let user_id = uuid!("11111111-1111-1111-1111-111111111111");
        let user = get_user_by_id(&session, user_id).await?;

        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.user_id, user_id);
        assert_eq!(user.username, "testuser1");
        assert_eq!(
            user.user_hashed_password,
            "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYfZJx.6YoK"
        );

        // Test non-existing user
        let non_existing_id = uuid!("99999999-9999-9999-9999-999999999999");
        let user = get_user_by_id(&session, non_existing_id).await?;
        assert!(user.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_user() -> Result<()> {
        let (session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let new_user_id = Uuid::new_v4();
        let username = "newuser";
        let password = "super_secure";

        create_user(&session, new_user_id, username, password).await?;

        // Verify user was created in users_by_id
        let user = get_user_by_id(&session, new_user_id).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.user_id, new_user_id);
        assert_eq!(user.username, username);

        // Verify user was created in users_by_username
        let query = "SELECT user_id, username, user_hashed_password FROM users_by_username WHERE username = ?";
        let result = session
            .query_unpaged(query, (&username,))
            .await?
            .into_rows_result()?;
        let rows = result.rows::<(Uuid, String, String)>()?;
        let row = rows.into_iter().next().unwrap()?;
        assert_eq!(row.0, new_user_id);
        assert_eq!(row.1, username);

        Ok(())
    }
}
