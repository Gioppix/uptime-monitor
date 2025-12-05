use actix_web::{
    HttpRequest, HttpResponse, post,
    web::{Data, Json},
};
use log::error;
use utoipa_actix_web::service_config::ServiceConfig;

use crate::{
    collab::internode::{BroadcastBody, messages::InterNodeMessage},
    eager_env,
    server::AppState,
};

pub fn configure_routes(config: &mut ServiceConfig) {
    config.service(internal);
}

#[utoipa::path(
    responses(
        (status = 200, description = "Internal endpoint success"),
        (status = 401, description = "Unauthorized - invalid or missing password"),
    ),
    tags = ["internal"],
    security(
        ("internal_bearer" = [])
    )
)]
#[post("/internal")]
pub async fn internal(
    req: HttpRequest,
    app_state: Data<AppState>,
    body: Json<BroadcastBody>,
) -> HttpResponse {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if token != Some(&*eager_env::BACKEND_INTERNAL_PASSWORD) {
        log::warn!("unauthorized call to internal endpoint");
        return HttpResponse::Unauthorized().body("Invalid or missing internal password");
    }

    let messages = body.into_inner();

    let mut check_ids = Vec::new();
    let mut shutting_process_ids = Vec::new();

    for msg in messages {
        log::info!("Received message: {msg:?}");

        match msg {
            InterNodeMessage::ServiceCheckMutation { check_id } => {
                check_ids.push(check_id);
            }
            InterNodeMessage::ShuttingDown { process_id } => {
                shutting_process_ids.push(process_id);
            }
        }
    }

    let task_updates_res = app_state.task_updates.send(check_ids.into_iter().collect());
    if let Err(error) = task_updates_res {
        error!("Error sending task updates to worker: {error}");
    }

    HttpResponse::Ok().finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::start_server_test;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_internal_endpoint() {
        let (port, _) = start_server_test(None).await;
        let client = reqwest::Client::new();
        let url = format!("http://localhost:{}/internal", port);

        // No token - should be unauthorized
        let response = client
            .post(&url)
            .json(&Vec::<InterNodeMessage>::new())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);

        // Wrong token - should be unauthorized
        let response = client
            .post(&url)
            .header("Authorization", "Bearer wrong_password")
            .json(&Vec::<InterNodeMessage>::new())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);

        // Correct token - should succeed
        let messages = vec![InterNodeMessage::ServiceCheckMutation {
            check_id: Uuid::new_v4(),
        }];
        let response = client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", *eager_env::BACKEND_INTERNAL_PASSWORD),
            )
            .json(&messages)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }
}
