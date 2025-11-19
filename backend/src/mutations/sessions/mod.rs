use crate::env_u32;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use scylla::client::session::Session;
use uuid::Uuid;

pub struct UserSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    logged_out: bool,
}

const SESSION_DURATION_DAYS: u32 = env_u32!("SESSION_DURATION_DAYS");

pub async fn create_session(
    db_session: &Session,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<UserSession> {
    let now = Utc::now();
    let expires_at = now + Duration::days(SESSION_DURATION_DAYS as i64);

    let query = "
        INSERT INTO sessions (session_id,
                              user_id,
                              created_at,
                              expires_at)
        VALUES (?, ?, ?, ?)
    ";

    db_session
        .query_unpaged(query, (session_id, user_id, now, expires_at))
        .await?;

    Ok(UserSession {
        session_id,
        user_id,
        created_at: Some(now),
        expires_at: Some(expires_at),
        logged_out: false,
    })
}

async fn get_session(db_session: &Session, session_id: Uuid) -> Result<Option<UserSession>> {
    let query = "
        SELECT session_id,
               user_id,
               created_at,
               expires_at,
               logged_out
        FROM sessions
        WHERE session_id = ?
    ";

    let result = db_session
        .query_unpaged(query, (session_id,))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(
        Uuid,
        Uuid,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<bool>,
    )>()?;

    if let Some(row) = rows.into_iter().next() {
        let (session_id, user_id, created_at, expires_at, logger_out) = row?;
        Ok(Some(UserSession {
            session_id,
            user_id,
            created_at,
            expires_at,
            logged_out: logger_out.unwrap_or(false),
        }))
    } else {
        Ok(None)
    }
}

pub async fn get_valid_session_user_id(
    db_session: &Session,
    session_id: Uuid,
) -> Result<Option<Uuid>> {
    let maybe_user_session = get_session(db_session, session_id).await?;

    if let Some(user_session) = maybe_user_session {
        let is_expired = match user_session.expires_at {
            Some(expires_at) => expires_at <= Utc::now(),
            None => true,
        };

        if !is_expired && !user_session.logged_out {
            Ok(Some(user_session.user_id))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub async fn log_out_session(db_session: &Session, session_id: Uuid) -> Result<()> {
    let query = "UPDATE sessions SET logged_out = true WHERE session_id = ?";
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
    async fn test_session_operations() -> Result<()> {
        let (db_session, _keyspace) = create_test_database(Some(FIXTURES)).await?;
        let user_id = uuid!("11111111-1111-1111-1111-111111111111");
        let session_id = Uuid::new_v4();

        // Test: Create and retrieve session
        let created = create_session(&db_session, user_id, session_id).await?;
        assert_eq!(created.session_id, session_id);
        assert_eq!(created.user_id, user_id);
        assert!(!created.logged_out);
        assert!(created.expires_at.is_some());

        let retrieved = get_session(&db_session, session_id).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, user_id);

        // Test: Valid session returns user_id
        assert_eq!(
            get_valid_session_user_id(&db_session, session_id).await?,
            Some(user_id)
        );

        // Test: Logout invalidates session
        log_out_session(&db_session, session_id).await?;
        assert_eq!(
            get_valid_session_user_id(&db_session, session_id).await?,
            None
        );

        // Test: Multiple sessions per user
        let session_id2 = Uuid::new_v4();
        create_session(&db_session, user_id, session_id2).await?;
        assert!(
            get_valid_session_user_id(&db_session, session_id2)
                .await?
                .is_some()
        );

        // Test: Nonexistent session
        let nonexistent = uuid!("99999999-9999-9999-9999-999999999999");
        assert!(get_session(&db_session, nonexistent).await?.is_none());
        assert!(
            get_valid_session_user_id(&db_session, nonexistent)
                .await?
                .is_none()
        );

        // Test: Valid session from fixtures
        let valid_id = uuid!("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        assert!(
            get_valid_session_user_id(&db_session, valid_id)
                .await?
                .is_some()
        );

        // Test: Expired session from fixtures
        let expired_id = uuid!("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        assert!(
            get_valid_session_user_id(&db_session, expired_id)
                .await?
                .is_none()
        );

        // Test: Null expires_at treated as expired
        let null_expiry_id = uuid!("cccccccc-cccc-cccc-cccc-cccccccccccc");
        let session = get_session(&db_session, null_expiry_id).await?.unwrap();
        assert_eq!(session.expires_at, None);
        assert!(
            get_valid_session_user_id(&db_session, null_expiry_id)
                .await?
                .is_none()
        );

        Ok(())
    }
}
