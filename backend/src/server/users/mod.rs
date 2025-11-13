use crate::{
    mutations::{
        sessions::{create_session, log_out_session},
        users::{LoginResult, PublicUser, create_user, get_user_by_id, login_user},
    },
    server::{
        AppState,
        auth::{AuthenticatedUser, UserSession, create_logout_cookie, create_session_cookie},
    },
};
use actix_web::{
    Error, HttpResponse,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound, ErrorUnauthorized},
    get, post,
    web::{Data, Json, Path},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_actix_web::{scope, service_config::ServiceConfig};
use uuid::Uuid;

pub fn configure_routes(config: &mut ServiceConfig) {
    config.service(
        scope::scope("/users")
            .service(get_user)
            .service(create_new_user)
            .service(login)
            .service(logout),
    );
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    username: String,
    password: String,
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
            // TODO: log error
            ErrorInternalServerError(e)
        })?
        .ok_or_else(|| ErrorNotFound("User not found"))?;

    Ok(Json(PublicUser {
        user_id: user.user_id,
        username: user.username,
    }))
}

#[utoipa::path(
    summary = "Create a new user",
    description = "Creates a new user account and establishes a session",
    responses(
        (status = 200, description = "User created successfully", body = PublicUser),
        (status = 500, description = "Internal server error")
    ),
    tags = ["users"],
    operation_id = "createUser"
)]
#[post("/new")]
async fn create_new_user(
    body: Json<CreateUserRequest>,
    app_state: Data<AppState>,
) -> Result<HttpResponse, Error> {
    let user_id = Uuid::new_v4();

    create_user(&app_state.database, user_id, &body.username, &body.password)
        .await
        .map_err(|e| {
            // TODO: log error
            ErrorInternalServerError(e)
        })?;

    // Create session
    let session_id = Uuid::new_v4();
    create_session(&app_state.database, user_id, session_id)
        .await
        .map_err(|e| {
            // TODO: log error
            ErrorInternalServerError(e)
        })?;

    // Create session cookie
    let cookie = create_session_cookie(session_id);

    Ok(HttpResponse::Ok().cookie(cookie).json(PublicUser {
        user_id,
        username: body.username.clone(),
    }))
}

#[utoipa::path(
    summary = "Login user",
    description = "Authenticates a user and establishes a session",
    responses(
        (status = 200, description = "Login successful", body = PublicUser),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    tags = ["users"],
    operation_id = "loginUser"
)]
#[post("/login")]
async fn login(body: Json<LoginRequest>, app_state: Data<AppState>) -> Result<HttpResponse, Error> {
    let result = login_user(&app_state.database, &body.username, &body.password)
        .await
        .map_err(|e| {
            // TODO: log error
            ErrorInternalServerError(e)
        })?;

    match result {
        LoginResult::Ok(public_user) => {
            // Create session
            let session_id = Uuid::new_v4();
            create_session(&app_state.database, public_user.user_id, session_id)
                .await
                .map_err(|e| {
                    // TODO: log error
                    ErrorInternalServerError(e)
                })?;

            // Create session cookie
            let cookie = create_session_cookie(session_id);

            Ok(HttpResponse::Ok().cookie(cookie).json(public_user))
        }
        LoginResult::ErrorWrongPassword | LoginResult::ErrorNotFound => {
            Err(ErrorUnauthorized("Invalid username or password"))
        }
    }
}

#[utoipa::path(
    summary = "Logout user",
    description = "Logs out the current user and invalidates their session",
    responses(
        (status = 200, description = "Logout successful"),
        (status = 500, description = "Internal server error")
    ),
    tags = ["users"],
    operation_id = "logoutUser"
)]
#[post("/logout")]
async fn logout(app_state: Data<AppState>, auth: AuthenticatedUser) -> Result<HttpResponse, Error> {
    match auth {
        AuthenticatedUser::Api(_) => Err(ErrorBadRequest("API keys cannot be logged out")),
        AuthenticatedUser::User(UserSession { session_id, .. }) => {
            log_out_session(&app_state.database, session_id)
                .await
                .map_err(|e| {
                    // TODO: log error
                    ErrorInternalServerError(e)
                })?;

            let logout_cookie = create_logout_cookie();

            Ok(HttpResponse::Ok()
                .cookie(logout_cookie)
                .json(serde_json::json!({ "message": "Logged out successfully" })))
        }
    }
}

#[cfg(test)]
mod user_endpoints_tests;
