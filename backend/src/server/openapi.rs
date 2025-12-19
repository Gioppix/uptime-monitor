use crate::queries::check_results::GraphGranularity;
use crate::server::auth::SESSION_COOKIE_NAME;
use utoipa::OpenApi;
use utoipa::openapi::{
    OpenApi as OpenApiSpec,
    security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme},
};

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health-related endpoints."),
        (name = "users", description = "User-related endpoints."),
        (name = "checks", description = "Health check management endpoints."),
        (name = "internal", description = "Internal endpoints for backend-to-backend communication."),
    ),
    modifiers(&SecurityAddon),
    components(schemas(GraphGranularity)), // Auto registering fails
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut OpenApiSpec) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "cookie_auth",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE_NAME))),
            );
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
            components.add_security_scheme(
                "internal_bearer",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
