mod health;
mod users;

use crate::{database::Database, server::health::*};
use actix_cors::Cors;
use actix_web::{App, HttpServer, web::Data};
use std::{net::TcpListener, sync::Arc};
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

pub type AppState = Arc<AppStateInner>;

#[derive(Debug)]
pub struct AppStateInner {
    pub database: Database,
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health-related endpoints.")
    )
)]
pub struct ApiDoc;

pub async fn start_server(state: AppState, listener: TcpListener) -> std::io::Result<()> {
    let data = Data::new(state);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(60 * 60 * 12);

        App::new()
            .wrap(cors)
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .service(home)
            .service(health)
            .configure(users::configure_routes)
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
pub async fn start_server_test() -> u16 {
    use crate::database::testing::create_test_database;

    let (database, _) = create_test_database(None)
        .await
        .expect("error creating database");

    let state = AppStateInner { database };

    let listener = TcpListener::bind("0.0.0.0:0").expect("failed to bind to random port");
    let port = listener
        .local_addr()
        .expect("failed to get local addr")
        .port();

    tokio::spawn(async {
        start_server(Arc::new(state), listener).await.unwrap();
    });

    port
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let port = start_server_test().await;

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
