use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use scylla::client::session::Session;
use uuid::Uuid;

use crate::env_u32;

pub struct UserSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

const SESSION_DURATION_DAYS: u32 = env_u32!("SESSION_DURATION_DAYS");

pub async fn create_session(
    db_session: &Session,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<UserSession> {
    let now = Utc::now();
    let expires_at = now + Duration::days(SESSION_DURATION_DAYS as i64);

    let query =
        "INSERT INTO sessions (session_id, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)";

    db_session
        .query_unpaged(query, (session_id, user_id, now, expires_at))
        .await?;

    Ok(UserSession {
        session_id,
        user_id,
        created_at: now,
        expires_at,
    })
}

pub async fn get_session(db_session: &Session, session_id: Uuid) -> Result<Option<UserSession>> {
    let query =
        "SELECT session_id, user_id, created_at, expires_at FROM sessions WHERE session_id = ?";

    let result = db_session
        .query_unpaged(query, (session_id,))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(Uuid, Uuid, DateTime<Utc>, DateTime<Utc>)>()?;

    if let Some(row) = rows.into_iter().next() {
        let (session_id, user_id, created_at, expires_at) = row?;
        Ok(Some(UserSession {
            session_id,
            user_id,
            created_at,
            expires_at,
        }))
    } else {
        Ok(None)
    }
}

pub async fn is_session_valid(db_session: &Session, session_id: Uuid) -> Result<bool> {
    let maybe_user_session = get_session(db_session, session_id).await?;

    if let Some(user_session) = maybe_user_session {
        Ok(user_session.expires_at > Utc::now())
    } else {
        Ok(false)
    }
}

pub async fn delete_session(db_session: &Session, session_id: Uuid) -> Result<()> {
    let query = "DELETE FROM sessions WHERE session_id = ?";
    db_session.query_unpaged(query, (session_id,)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;
    use uuid::uuid;

    const FIXTURES: &str = include_str!("fixtures.cql");

    #[tokio::test]
    async fn test_create_and_get_session() -> Result<()> {
        let (db_session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let user_id = uuid!("11111111-1111-1111-1111-111111111111");
        let session_id = Uuid::new_v4();

        // Create session
        let created = create_session(&db_session, user_id, session_id).await?;
        assert_eq!(created.session_id, session_id);
        assert_eq!(created.user_id, user_id);

        // Get session
        let retrieved = get_session(&db_session, session_id).await?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id, session_id);
        assert_eq!(retrieved.user_id, user_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() -> Result<()> {
        let (db_session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let session_id = uuid!("99999999-9999-9999-9999-999999999999");
        let retrieved = get_session(&db_session, session_id).await?;
        assert!(retrieved.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_session_validity() -> Result<()> {
        let (db_session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        // Valid session from fixtures
        let valid_session_id = uuid!("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        assert!(is_session_valid(&db_session, valid_session_id).await?);

        // Expired session from fixtures
        let expired_session_id = uuid!("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        assert!(!is_session_valid(&db_session, expired_session_id).await?);

        // Non-existent session
        let nonexistent_id = uuid!("99999999-9999-9999-9999-999999999999");
        assert!(!is_session_valid(&db_session, nonexistent_id).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_session() -> Result<()> {
        let (db_session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let user_id = uuid!("11111111-1111-1111-1111-111111111111");
        let session_id = Uuid::new_v4();

        // Create and verify session exists
        create_session(&db_session, user_id, session_id).await?;
        assert!(get_session(&db_session, session_id).await?.is_some());

        // Delete session
        delete_session(&db_session, session_id).await?;

        // Verify session is deleted
        assert!(get_session(&db_session, session_id).await?.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_sessions_per_user() -> Result<()> {
        let (db_session, _keyspace) = create_test_database(Some(FIXTURES)).await?;

        let user_id = uuid!("11111111-1111-1111-1111-111111111111");
        let session_id1 = Uuid::new_v4();
        let session_id2 = Uuid::new_v4();

        // Create two sessions for same user
        create_session(&db_session, user_id, session_id1).await?;
        create_session(&db_session, user_id, session_id2).await?;

        // Both sessions should exist and be valid
        let sess1 = get_session(&db_session, session_id1).await?;
        let sess2 = get_session(&db_session, session_id2).await?;

        assert!(sess1.is_some());
        assert!(sess2.is_some());
        assert_eq!(sess1.unwrap().user_id, user_id);
        assert_eq!(sess2.unwrap().user_id, user_id);

        Ok(())
    }
}
