pub mod metrics;

use std::sync::Arc;

use crate::{
    collab::{
        get_bucket_for_check,
        heartbeat::HeartbeatManager,
        internode::{MessageWithFilters, messages::InterNodeMessage, standard_broadcast},
    },
    queries::{
        authorization::{
            CheckAccess, get_user_access_to_check, get_user_checks, grant_check_access,
        },
        checks::{Check, create_check, delete_check, get_check_by_id, update_check},
        users::get_user_by_id,
    },
    server::{AppState, auth::AuthenticatedUser},
};
use actix_web::{
    Error, HttpResponse, delete,
    error::{ErrorForbidden, ErrorInternalServerError, ErrorNotFound},
    get, patch, post,
    web::{Data, Json, Path},
};
use log::error;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_actix_web::{scope, service_config::ServiceConfig};
use uuid::Uuid;

pub fn configure_routes(config: &mut ServiceConfig) {
    config.service(
        scope::scope("/checks")
            .service(create_check_endpoint)
            .service(get_check_endpoint)
            .service(list_my_checks)
            .service(update_check_endpoint)
            .service(delete_check_endpoint)
            .service(metrics::get_check_metrics_endpoint)
            .service(metrics::get_check_metrics_graph_endpoint),
    );
}

fn broadcast_check_mutation(heartbeat_manager: Arc<HeartbeatManager>, check_id: Uuid) {
    tokio::spawn(async move {
        let bucket = get_bucket_for_check(check_id).1 as u32;
        let result = standard_broadcast(
            &heartbeat_manager,
            vec![MessageWithFilters {
                message: InterNodeMessage::ServiceCheckMutation { check_id },
                filter_bucket: Some(bucket),
            }],
        )
        .await;

        if let Err(e) = result {
            error!("Failed to broadcast check mutation: {}", e);
        }
    });
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckWithAccess {
    #[serde(flatten)]
    pub check: Check,
    #[serde(flatten)]
    pub access: CheckAccess,
}

#[utoipa::path(
    summary = "Create a new check",
    description = "Creates a new check across multiple regions. The creator automatically gets full access (can_edit and can_see).",
    request_body = Check,
    responses(
        (status = 200, description = "Check created successfully", body = Check),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie_auth" = []),
        ("bearer_auth" = [])
    ),
    tags = ["checks"],
    operation_id = "createCheck"
)]
#[post("/")]
async fn create_check_endpoint(
    body: Json<Check>,
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Check>, Error> {
    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: Check API key permissions
            todo!("API key check creation not yet implemented")
        }
    };

    // Get user info for username
    let user = get_user_by_id(&app_state.database, user_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorNotFound("User not found"))?;

    let check = create_check(&app_state.database, body.regions.clone(), body.data.clone())
        .await
        .map_err(ErrorInternalServerError)?;

    // Grant full access to creator
    grant_check_access(
        &app_state.database,
        check.check_id,
        user_id,
        &user.username,
        CheckAccess {
            can_edit: true,
            can_see: true,
        },
    )
    .await
    .map_err(ErrorInternalServerError)?;

    broadcast_check_mutation(app_state.heartbeat_manager.clone(), check.check_id);

    Ok(Json(check))
}

#[utoipa::path(
    summary = "Get check by ID",
    description = "Retrieves a check by its ID. User must have access to view the check.",
    responses(
        (status = 200, description = "Check found", body = CheckWithAccess),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 403, description = "Forbidden - no access to this check"),
        (status = 404, description = "Check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie_auth" = []),
        ("bearer_auth" = [])
    ),
    tags = ["checks"],
    operation_id = "getCheck"
)]
#[get("/{check_id}")]
async fn get_check_endpoint(
    check_id: Path<Uuid>,
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<CheckWithAccess>, Error> {
    let check_id = check_id.into_inner();

    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: Check API key permissions
            todo!("API key access not yet implemented")
        }
    };

    // Check if user has access
    let access = get_user_access_to_check(&app_state.database, user_id, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorForbidden("No access to this check"))?;

    if !access.can_see {
        return Err(ErrorForbidden("No access to this check"));
    }

    let check = get_check_by_id(&app_state.database, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorNotFound("Check not found"))?;

    Ok(Json(CheckWithAccess { check, access }))
}

#[utoipa::path(
    summary = "List my checks",
    description = "Lists all checks the authenticated user has access to",
    responses(
        (status = 200, description = "List of checks", body = Vec<CheckWithAccess>),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie_auth" = []),
        ("bearer_auth" = [])
    ),
    tags = ["checks"],
    operation_id = "listMyChecks"
)]
#[get("/")]
async fn list_my_checks(
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<CheckWithAccess>>, Error> {
    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: API keys should list associated checks
            todo!("API key check listing not yet implemented")
        }
    };

    let check_accesses = get_user_checks(&app_state.database, user_id)
        .await
        .map_err(ErrorInternalServerError)?;

    let mut checks_with_access = Vec::new();

    for (check_id, access) in check_accesses {
        if let Some(check) = get_check_by_id(&app_state.database, check_id)
            .await
            .map_err(ErrorInternalServerError)?
        {
            checks_with_access.push(CheckWithAccess { check, access });
        }
    }

    Ok(Json(checks_with_access))
}

#[utoipa::path(
    summary = "Update check",
    description = "Updates a check. User must have edit access to the check.",
    request_body = Check,
    responses(
        (status = 200, description = "Check updated successfully", body = Check),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 403, description = "Forbidden - no edit access to check"),
        (status = 404, description = "Check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie_auth" = []),
        ("bearer_auth" = [])
    ),
    tags = ["checks"],
    operation_id = "updateCheck"
)]
#[patch("/{check_id}")]
async fn update_check_endpoint(
    check_id: Path<Uuid>,
    body: Json<Check>,
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Check>, Error> {
    let check_id = check_id.into_inner();

    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: Check API key permissions
            todo!("API key check update not yet implemented")
        }
    };

    // Check if user has edit access
    let access = get_user_access_to_check(&app_state.database, user_id, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorForbidden("No access to this check"))?;

    if !access.can_edit {
        return Err(ErrorForbidden("No edit access to this check"));
    }

    // Verify check exists
    let _existing_check = get_check_by_id(&app_state.database, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorNotFound("Check not found"))?;

    // Use the check from the request but ensure check_id matches
    let mut check = body.into_inner();
    check.check_id = check_id;

    update_check(&app_state.database, check.clone())
        .await
        .map_err(ErrorInternalServerError)?;

    broadcast_check_mutation(app_state.heartbeat_manager.clone(), check_id);

    Ok(Json(check))
}

#[utoipa::path(
    summary = "Delete check",
    description = "Deletes a check from all regions. User must have edit access to the check.",
    responses(
        (status = 200, description = "Check deleted successfully"),
        (status = 401, description = "Unauthorized - authentication required"),
        (status = 403, description = "Forbidden - no edit access to check"),
        (status = 404, description = "Check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie_auth" = []),
        ("bearer_auth" = [])
    ),
    tags = ["checks"],
    operation_id = "deleteCheck"
)]
#[delete("/{check_id}")]
async fn delete_check_endpoint(
    check_id: Path<Uuid>,
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, Error> {
    let check_id = check_id.into_inner();

    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: Check API key permissions
            todo!("API key check deletion not yet implemented")
        }
    };

    // Check if user has edit access
    let access = get_user_access_to_check(&app_state.database, user_id, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorForbidden("No access to this check"))?;

    if !access.can_edit {
        return Err(ErrorForbidden("No edit access to this check"));
    }

    // Verify check exists
    let _check = get_check_by_id(&app_state.database, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorNotFound("Check not found"))?;

    delete_check(&app_state.database, check_id)
        .await
        .map_err(ErrorInternalServerError)?;

    broadcast_check_mutation(app_state.heartbeat_manager.clone(), check_id);

    Ok(HttpResponse::Ok().json(serde_json::json!({ "message": "Check deleted successfully" })))
}

#[cfg(test)]
mod check_endpoints_tests;
