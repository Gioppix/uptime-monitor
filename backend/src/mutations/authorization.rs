use anyhow::Result;
use scylla::client::session::Session;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckAccess {
    pub can_edit: bool,
    pub can_see: bool,
}

/// Get user's access to a check
pub async fn get_user_access_to_check(
    session: &Session,
    user_id: Uuid,
    check_id: Uuid,
) -> Result<Option<CheckAccess>> {
    let query = "
        SELECT can_edit,
               can_see
        FROM access_by_check
        WHERE check_id = ?
          AND user_id = ?
    ";

    let result = session
        .query_unpaged(query, (check_id, user_id))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(bool, bool)>()?;

    if let Some(row) = rows.into_iter().next() {
        let (can_edit, can_see) = row?;
        Ok(Some(CheckAccess { can_edit, can_see }))
    } else {
        Ok(None)
    }
}

/// Grant access to a check for a user
pub async fn grant_check_access(
    session: &Session,
    check_id: Uuid,
    user_id: Uuid,
    user_name: &str,
    access: CheckAccess,
) -> Result<()> {
    let query = "
        INSERT INTO access_by_check (check_id,
                                     user_id,
                                     user_name,
                                     can_edit,
                                     can_see)
        VALUES (?, ?, ?, ?, ?)
    ";

    session
        .query_unpaged(
            query,
            (
                check_id,
                user_id,
                user_name,
                access.can_edit,
                access.can_see,
            ),
        )
        .await?;

    Ok(())
}

/// Get all checks a user has access to
pub async fn get_user_checks(session: &Session, user_id: Uuid) -> Result<Vec<(Uuid, CheckAccess)>> {
    let query = "
        SELECT check_id,
               can_edit,
               can_see
        FROM access_by_check
        WHERE user_id = ?
    ";

    let result = session
        .query_unpaged(query, (user_id,))
        .await?
        .into_rows_result()?;

    let rows = result.rows::<(Uuid, bool, bool)>()?;

    let checks = rows
        .into_iter()
        .filter_map(|r| r.ok())
        .map(|(check_id, can_edit, can_see)| (check_id, CheckAccess { can_edit, can_see }))
        .collect();

    Ok(checks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::create_test_database;

    #[tokio::test]
    async fn test_check_access() -> Result<()> {
        let (session, _keyspace) = create_test_database(None).await?;

        let user_id = Uuid::new_v4();
        let check_id = Uuid::new_v4();

        // No access initially
        assert!(
            get_user_access_to_check(&session, user_id, check_id)
                .await?
                .is_none()
        );

        // Grant view access only
        grant_check_access(
            &session,
            check_id,
            user_id,
            "testuser",
            CheckAccess {
                can_edit: false,
                can_see: true,
            },
        )
        .await?;

        let access = get_user_access_to_check(&session, user_id, check_id)
            .await?
            .unwrap();
        assert!(!access.can_edit);
        assert!(access.can_see);

        // Grant edit access
        grant_check_access(
            &session,
            check_id,
            user_id,
            "testuser",
            CheckAccess {
                can_edit: true,
                can_see: true,
            },
        )
        .await?;

        let access = get_user_access_to_check(&session, user_id, check_id)
            .await?
            .unwrap();
        assert!(access.can_edit);
        assert!(access.can_see);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_user_checks() -> Result<()> {
        let (session, _keyspace) = create_test_database(None).await?;

        let user_id = Uuid::new_v4();
        let check1_id = Uuid::new_v4();
        let check2_id = Uuid::new_v4();

        grant_check_access(
            &session,
            check1_id,
            user_id,
            "testuser",
            CheckAccess {
                can_edit: true,
                can_see: true,
            },
        )
        .await?;
        grant_check_access(
            &session,
            check2_id,
            user_id,
            "testuser",
            CheckAccess {
                can_edit: false,
                can_see: true,
            },
        )
        .await?;

        let checks = get_user_checks(&session, user_id).await?;
        assert_eq!(checks.len(), 2);

        Ok(())
    }
}
