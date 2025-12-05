mod auth;
mod checks;
mod health;
mod internal;
mod openapi;
mod users;

use crate::{
    collab::heartbeat::HeartbeatManager, database::Database, eager_env, server::health::*,
};
use actix_cors::Cors;
use actix_web::{App, HttpServer, http::Method, web::Data};
use std::{collections::BTreeSet, net::TcpListener, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

pub type AppState = Arc<AppStateInner>;
pub type TaskUpdateType = BTreeSet<Uuid>;

pub struct AppStateInner {
    pub process_id: Uuid,
    pub database: Arc<Database>,
    pub task_updates: UnboundedSender<TaskUpdateType>,
    pub heartbeat_manager: Arc<HeartbeatManager>,
}

pub async fn start_server(state: AppState, listener: TcpListener) -> std::io::Result<()> {
    let data = Data::new(state);

    HttpServer::new(move || {
        // Parse FRONTEND_PUBLIC_URL as comma-separated list of allowed origins
        let allowed_origins = eager_env::FRONTEND_PUBLIC_URL
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        let mut cors = Cors::default()
            .supports_credentials()
            .allowed_methods(vec![
                Method::GET,
                Method::POST,
                Method::PATCH,
                Method::DELETE,
            ])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(60 * 60 * 12);

        // Add each allowed origin
        for origin in allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        App::new()
            .wrap(cors)
            .into_utoipa_app()
            .openapi(openapi::ApiDoc::openapi())
            .service(home)
            .service(health)
            .configure(users::configure_routes)
            .configure(checks::configure_routes)
            .configure(internal::configure_routes)
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
    use std::time::Duration;

    use crate::{database::testing::create_test_database, regions::Region};
    use tokio::sync::mpsc;

    let (task_updates, _rx) = mpsc::unbounded_channel();

    let (database, _) = create_test_database(fixtures)
        .await
        .expect("error creating database");
    let database = Arc::new(database);

    let process_id = Uuid::new_v4();
    let state = AppStateInner {
        process_id,
        task_updates,
        heartbeat_manager: Arc::new(
            HeartbeatManager::new(
                process_id,
                Region::Fsn1,
                Duration::from_secs(99999),
                database.clone(),
            )
            .await
            .unwrap(),
        ),
        database,
    };
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
