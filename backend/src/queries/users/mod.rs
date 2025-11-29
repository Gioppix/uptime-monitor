mod password;

use crate::database::preparer::CachedPreparedStatement;
use crate::queries::users::password::hash_password;
use anyhow::Result;
use scylla::{client::session::Session, statement::batch::Batch};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub user_hashed_password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PublicUser {
    pub user_id: Uuid,
    pub username: String,
}

static GET_USER_BY_ID_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT user_id,
           username,
           user_hashed_password
    FROM users_by_id
    WHERE user_id = ?
    ",
);

pub async fn get_user_by_id(session: &Session, user_id: Uuid) -> Result<Option<User>> {
    let result = GET_USER_BY_ID_QUERY
        .execute_unpaged(session, (user_id,))
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

static GET_USER_BY_USERNAME_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    SELECT user_id,
           username,
           user_hashed_password
    FROM users_by_username
    WHERE username = ?
    ",
);

pub async fn get_user_by_username(session: &Session, username: &str) -> Result<Option<User>> {
    let result = GET_USER_BY_USERNAME_QUERY
        .execute_unpaged(session, (username,))
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

static CREATE_USER_BY_ID_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO users_by_id (user_id, username, user_hashed_password)
    VALUES (?, ?, ?)
    ",
);

static CREATE_USER_BY_USERNAME_QUERY: CachedPreparedStatement = CachedPreparedStatement::new(
    "
    INSERT INTO users_by_username (username, user_id, user_hashed_password)
    VALUES (?, ?, ?)
    ",
);

pub async fn create_user(
    db: &Session,
    user_id: Uuid,
    username: &str,
    password: &str,
) -> Result<()> {
    let user_hashed_password = hash_password(password)?;

    let prepared_by_id = CREATE_USER_BY_ID_QUERY.get_prepared_statement(db).await?;
    let prepared_by_username = CREATE_USER_BY_USERNAME_QUERY
        .get_prepared_statement(db)
        .await?;

    let mut batch = Batch::default();
    batch.append_statement(prepared_by_id);
    batch.append_statement(prepared_by_username);

    db.batch(
        &batch,
        (
            (user_id, &username, &user_hashed_password),
            (&username, user_id, &user_hashed_password),
        ),
    )
    .await?;

    Ok(())
}

pub enum LoginResult {
    Ok(PublicUser),
    ErrorWrongPassword,
    ErrorNotFound,
}

pub async fn login_user(session: &Session, username: &str, password: &str) -> Result<LoginResult> {
    let user = get_user_by_username(session, username).await?;

    match user {
        None => Ok(LoginResult::ErrorNotFound),
        Some(user) => {
            let password_matches = password::verify_password(password, &user.user_hashed_password)?;
            if password_matches {
                Ok(LoginResult::Ok(PublicUser {
                    user_id: user.user_id,
                    username: user.username,
                }))
            } else {
                Ok(LoginResult::ErrorWrongPassword)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_get_user() -> Result<()> {
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

        // Test get_user_by_username
        let user = get_user_by_username(&session, "testuser1").await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.user_id, user_id);
        assert_eq!(user.username, "testuser1");
        assert_eq!(
            user.user_hashed_password,
            "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYfZJx.6YoK"
        );

        // Test non-existing username
        let user = get_user_by_username(&session, "nonexistentuser").await?;
        assert!(user.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_login_user() -> Result<()> {
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

        // Test login with correct password
        let login_result = login_user(&session, username, password).await?;
        assert!(matches!(login_result, LoginResult::Ok(_)));
        if let LoginResult::Ok(public_user) = login_result {
            assert_eq!(public_user.user_id, new_user_id);
            assert_eq!(public_user.username, username);
        }

        // Test login with wrong password
        let login_result = login_user(&session, username, "wrong_password").await?;
        assert!(matches!(login_result, LoginResult::ErrorWrongPassword));

        // Test login with non-existent user
        let login_result = login_user(&session, "nonexistentuser", "somepassword").await?;
        assert!(matches!(login_result, LoginResult::ErrorNotFound));

        Ok(())
    }
}
