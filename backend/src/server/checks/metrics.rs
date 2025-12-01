use crate::{
    queries::{
        authorization::get_user_access_to_check,
        check_results::{MetricsResponse, get_check_metrics},
    },
    regions::Region,
    server::{AppState, auth::AuthenticatedUser},
};
use actix_web::{
    Error,
    error::{ErrorBadRequest, ErrorForbidden, ErrorInternalServerError},
    get,
    web::{Data, Json, Path, Query},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::env;
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct MetricsQuery {
    /// Start timestamp (ISO 8601)
    pub from: DateTime<Utc>,
    /// End timestamp (ISO 8601, exclusive)
    pub to: DateTime<Utc>,
    /// Comma-separated list of regions (optional, defaults to all)
    pub regions: Option<String>,
}

fn get_max_days() -> i64 {
    env::var("CHECK_RESULTS_MAX_DAYS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(90)
}

#[utoipa::path(
    summary = "Get check metrics",
    description = "Get aggregated uptime and performance metrics for a check over a time range",
    params(
        ("check_id" = Uuid, Path, description = "Check ID"),
        ("from" = DateTime<Utc>, Query, description = "Start timestamp (ISO 8601)"),
        ("to" = DateTime<Utc>, Query, description = "End timestamp (ISO 8601, exclusive)"),
        ("regions" = Option<String>, Query, description = "Comma-separated list of regions to filter by"),
    ),
    responses(
        (status = 200, description = "Metrics retrieved successfully", body = MetricsResponse),
        (status = 400, description = "Invalid query parameters"),
        (status = 403, description = "Forbidden - no access to check"),
        (status = 404, description = "Check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie_auth" = []),
        ("bearer_auth" = [])
    ),
    tags = ["checks"],
    operation_id = "getCheckMetrics"
)]
#[get("/{check_id}/metrics")]
pub async fn get_check_metrics_endpoint(
    check_id: Path<Uuid>,
    query: Query<MetricsQuery>,
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<MetricsResponse>, Error> {
    let check_id = check_id.into_inner();
    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: Check API key permissions
            todo!("API key check metrics not yet implemented")
        }
    };

    // Validate from < to
    if query.from >= query.to {
        return Err(ErrorBadRequest("'from' must be before 'to'"));
    }

    // Validate time range doesn't exceed max days
    let max_days = get_max_days();
    let duration = query.to - query.from;
    if duration.num_days() > max_days {
        return Err(ErrorBadRequest(format!(
            "Time range cannot exceed {} days",
            max_days
        )));
    }

    // Parse and validate regions
    let regions = match &query.regions {
        Some(regions_str) => {
            let region_strs: Vec<&str> = regions_str.split(',').map(|s| s.trim()).collect();
            if region_strs.is_empty() {
                return Err(ErrorBadRequest("regions parameter cannot be empty"));
            }
            let mut regions = Vec::new();
            for r in region_strs {
                regions.push(
                    Region::from_str(r)
                        .map_err(|_| ErrorBadRequest(format!("Invalid region: {}", r)))?,
                );
            }
            Some(regions)
        }
        None => None,
    };

    // Check user access
    let access = get_user_access_to_check(&app_state.database, user_id, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorForbidden("No access to this check"))?;

    if !access.can_see {
        return Err(ErrorForbidden("No permission to view this check"));
    }

    // Get metrics
    let metrics = get_check_metrics(&app_state.database, check_id, regions, query.from, query.to)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(Json(metrics))
}
