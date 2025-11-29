use crate::{eager_env, queries::sessions::get_valid_session_user_id, server::AppState};
use actix_web::{
    FromRequest, HttpRequest,
    cookie::{Cookie, SameSite},
    dev::Payload,
    error::{ErrorInternalServerError, ErrorUnauthorized},
};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub const SESSION_COOKIE_NAME: &str = "session_id";

#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum AuthenticatedUser {
    /// `user_id` from cookie session
    User(UserSession),
    /// `api_key_id` from Authorization header
    Api(Uuid),
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // Check Authorization header first
            if let Some(auth_header) = req.headers().get("Authorization")
                && let Ok(_) = auth_header.to_str()
            {
                todo!("validate API auth")
            }

            // If no Authorization header, check for session cookie
            let session_cookie = req.cookie(SESSION_COOKIE_NAME);

            if let Some(cookie) = session_cookie {
                let session_id_str = cookie.value();

                let session_id = match Uuid::parse_str(session_id_str) {
                    Ok(session_id) => session_id,
                    Err(_) => return Err(ErrorUnauthorized("Invalid session ID format")),
                };

                // Get app state to access database
                let app_state = match req.app_data::<actix_web::web::Data<AppState>>() {
                    Some(state) => state,
                    None => return Err(ErrorInternalServerError("App state not found")),
                };

                let maybe_user_id =
                    get_valid_session_user_id(&app_state.database, session_id).await;

                match maybe_user_id {
                    Ok(Some(user_id)) => {
                        return Ok(AuthenticatedUser::User(UserSession {
                            user_id,
                            session_id,
                        }));
                    }
                    Ok(None) => {
                        return Err(ErrorUnauthorized("Session expired or invalid"));
                    }
                    Err(e) => return Err(ErrorInternalServerError(e)),
                }
            }

            Err(ErrorUnauthorized("No valid authentication provided"))
        })
    }
}

pub fn create_session_cookie(session_id: Uuid) -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE_NAME, session_id.to_string())
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .path("/")
        .domain(&*eager_env::COOKIE_DOMAIN)
        .finish()
}

/// Creates a cookie that expires immediately
pub fn create_logout_cookie() -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE_NAME, "")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .path("/")
        .domain(&*eager_env::COOKIE_DOMAIN)
        .max_age(actix_web::cookie::time::Duration::seconds(0))
        .finish()
}
