use crate::{
    mutations::users::{PublicUser, get_user_by_id},
    server::AppState,
};
use actix_web::{
    Error,
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::{Data, Json, Path},
};
use utoipa_actix_web::{scope, service_config::ServiceConfig};
use uuid::Uuid;

pub fn configure_routes(config: &mut ServiceConfig) {
    config.service(scope::scope("/users").service(get_user));
}

#[utoipa::path(
    summary = "Get user by ID",
    description = "Retrieves a user's public information by their unique identifier",
    responses(
        (status = 200, description = "User found successfully", body = PublicUser),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tags = ["users"],
    operation_id = "getUser"
)]
#[get("/{user_id}")]
async fn get_user(
    user_id: Path<Uuid>,
    app_state: Data<AppState>,
) -> Result<Json<PublicUser>, Error> {
    let user = get_user_by_id(&app_state.database, user_id.into_inner())
        .await
        .map_err(|e| {
            // TODO: report error
            ErrorInternalServerError(e)
        })?
        .ok_or_else(|| ErrorNotFound("User not found"))?;

    Ok(Json(PublicUser {
        user_id: user.user_id,
        username: user.username,
    }))
}
