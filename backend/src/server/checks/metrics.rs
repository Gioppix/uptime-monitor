use crate::{
    queries::{
        authorization::get_user_access_to_check,
        check_results::{
            GraphGranularity, MetricsResponse, MetricsResponseDate, get_check_metrics,
            get_check_metrics_graph, is_rounded_to_granularity,
        },
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
use strum::IntoEnumIterator;
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

const CHECK_RESULTS_MAX_DAYS: u32 = 90;

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
    let duration = query.to - query.from;
    if duration.num_days() > CHECK_RESULTS_MAX_DAYS.into() {
        return Err(ErrorBadRequest(format!(
            "Time range cannot exceed {} days",
            CHECK_RESULTS_MAX_DAYS
        )));
    }

    let regions = parse_regions(query.regions.as_ref()).map_err(ErrorBadRequest)?;

    // Check user access
    let access = get_user_access_to_check(&app_state.database, user_id, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorForbidden("No access to this check"))?;

    if !access.can_see {
        return Err(ErrorForbidden("No permission to view this check"));
    }

    // Get metrics
    let metrics = get_check_metrics(
        &app_state.database,
        check_id,
        &regions,
        query.from,
        query.to,
    )
    .await
    .map_err(ErrorInternalServerError)?;

    Ok(Json(metrics))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MetricsGraphQuery {
    #[serde(flatten)]
    pub query: MetricsQuery,
    pub granularity: GraphGranularity,
}

#[utoipa::path(
    summary = "Get check metrics graph",
    description = "Get time-series metrics data for a check with specified granularity",
    params(
        ("check_id" = Uuid, Path, description = "Check ID"),
        ("from" = DateTime<Utc>, Query, description = "Start timestamp, included (ISO 8601, must be rounded to granularity)"),
        ("to" = DateTime<Utc>, Query, description = "End timestamp, excluded (ISO 8601, exclusive, must be rounded to granularity)"),
        ("regions" = Option<String>, Query, description = "Comma-separated list of regions to filter by"),
        ("granularity" = GraphGranularity, Query, description = "Time granularity for data points"),
    ),
    responses(
        (status = 200, description = "Metrics graph data retrieved successfully", body = Vec<MetricsResponseDate>),
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
    operation_id = "getCheckMetricsGraph"
)]
#[get("/{check_id}/metrics/graph")]
pub async fn get_check_metrics_graph_endpoint(
    check_id: Path<Uuid>,
    query: Query<MetricsGraphQuery>,
    app_state: Data<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<MetricsResponseDate>>, Error> {
    if query.query.from >= query.query.to {
        return Err(ErrorBadRequest("'from' must be before 'to'"));
    }

    if !is_rounded_to_granularity(query.query.from, query.granularity) {
        return Err(ErrorBadRequest(
            "'from' timestamp must be rounded to the specified granularity",
        ));
    }
    if !is_rounded_to_granularity(query.query.to, query.granularity) {
        return Err(ErrorBadRequest(
            "'to' timestamp must be rounded to the specified granularity",
        ));
    }

    let check_id = check_id.into_inner();
    let user_id = match auth {
        AuthenticatedUser::User(session) => session.user_id,
        AuthenticatedUser::Api(_) => {
            // TODO: Check API key permissions
            todo!("API key check metrics not yet implemented")
        }
    };

    // Validate time range doesn't exceed max days
    let duration = query.query.to - query.query.from;
    if duration.num_days() > CHECK_RESULTS_MAX_DAYS.into() {
        return Err(ErrorBadRequest(format!(
            "Time range cannot exceed {} days",
            CHECK_RESULTS_MAX_DAYS
        )));
    }

    let regions = parse_regions(query.query.regions.as_ref()).map_err(ErrorBadRequest)?;

    // Check user access
    let access = get_user_access_to_check(&app_state.database, user_id, check_id)
        .await
        .map_err(ErrorInternalServerError)?
        .ok_or_else(|| ErrorForbidden("No access to this check"))?;

    if !access.can_see {
        return Err(ErrorForbidden("No permission to view this check"));
    }

    // Get metrics
    let metrics = get_check_metrics_graph(
        &app_state.database,
        check_id,
        &regions,
        query.query.from,
        query.query.to,
        query.granularity,
    )
    .await
    .map_err(ErrorInternalServerError)?;

    Ok(Json(metrics))
}

fn parse_regions(regions_str: Option<&String>) -> Result<Vec<Region>, &'static str> {
    match regions_str {
        Some(regions_str) => {
            let region_strings: Vec<&str> = regions_str.split(',').map(|s| s.trim()).collect();
            if region_strings.is_empty() {
                return Err("regions parameter cannot be empty");
            }
            let mut regions = Vec::new();
            for r in region_strings {
                regions.push(serde_plain::from_str(r).map_err(|_| "Invalid region")?);
            }
            Ok(regions)
        }
        None => Ok(Region::iter().collect()),
    }
}
