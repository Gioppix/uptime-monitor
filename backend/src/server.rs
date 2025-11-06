use actix_cors::Cors;
use actix_web::{
    App, HttpResponse, HttpServer, Result, get,
    web::{self, Data},
};
use std::{net::TcpListener, sync::Arc};

pub type AppState = Arc<AppStateInner>;

#[derive(Debug)]
pub struct AppStateInner {}

#[get("/health")]
async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json("ok"))
}

#[get("/")]
async fn home() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("Monitor"))
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    let cors = Cors::default()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST"])
        .allowed_headers(vec!["Content-Type", "Authorization"])
        .max_age(3600);

    // cfg.service(messaging::configure_routes()).service(
    //     web::scope("/frontend")
    //         .wrap(cors)
    //         .configure(pfp::configure_routes_cors)
    //         .configure(document_upload::configure_routes_cors)
    //         .configure(templater::configure_routes_cors)
    //         .configure(demos::configure_routes_cors),
    // );
}

#[cfg(test)]
pub fn start_server_test() -> u16 {
    let state = AppStateInner {};

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

pub async fn start_server(state: AppState, listener: TcpListener) -> std::io::Result<()> {
    let data = Data::new(state);
    HttpServer::new(move || {
        App::new()
            .service(home)
            .service(health)
            .configure(configure_app)
            .app_data(data.clone())
    })
    .listen(listener)
    .expect("Failed to bind port")
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::init_logging;

    #[tokio::test]
    async fn test_health_endpoint() {
        init_logging();

        let port = start_server_test();

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
