mod auth;
mod checks;
mod health;
mod openapi;
mod users;

use crate::{database::Database, env_str, server::health::*};
use actix_cors::Cors;
use actix_web::{App, HttpServer, http::Method, web::Data};
use std::{net::TcpListener, sync::Arc};
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

const FRONTEND_PUBLIC_URL: &str = env_str!("FRONTEND_PUBLIC_URL");

pub type AppState = Arc<AppStateInner>;

#[derive(Debug)]
pub struct AppStateInner {
    pub database: Arc<Database>,
}

pub async fn start_server(state: AppState, listener: TcpListener) -> std::io::Result<()> {
    let data = Data::new(state);

    HttpServer::new(move || {
        let cors = Cors::default()
            .supports_credentials()
            .allowed_origin(FRONTEND_PUBLIC_URL)
            .allowed_methods(vec![
                Method::GET,
                Method::POST,
                Method::PATCH,
                Method::DELETE,
            ])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(60 * 60 * 12);

        App::new()
            .wrap(cors)
            .into_utoipa_app()
            .openapi(openapi::ApiDoc::openapi())
            .service(home)
            .service(health)
            .configure(users::configure_routes)
            .configure(checks::configure_routes)
            .app_data(data.clone())
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api/openapi.json", api)
            })
            .into_app()
    })
    .listen(listener)
    .expect("Failed to bind port")
    .run()
    .await
}

#[cfg(test)]
pub async fn start_server_test(fixtures: Option<&str>) -> (u16, AppState) {
    use crate::database::testing::create_test_database;

    let (database, _) = create_test_database(fixtures)
        .await
        .expect("error creating database");
    let database = Arc::new(database);

    let state = AppStateInner { database };
    let app_state: AppState = Arc::new(state);

    let listener = TcpListener::bind("0.0.0.0:0").expect("failed to bind to random port");
    let port = listener
        .local_addr()
        .expect("failed to get local addr")
        .port();

    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        start_server(app_state.clone(), listener).await.unwrap();
    });

    (port, app_state_clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let (port, _) = start_server_test(None).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://localhost:{}/health", port))
            .send()
            .await
            .unwrap();

        let status = response.status();
        assert_eq!(status, 200);
    }
}
