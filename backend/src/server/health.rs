use actix_web::{HttpResponse, get};

#[utoipa::path(
    responses(
        (status = 200, description = "Health check")
    ),
    tags = ["health"]
)]
#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

#[utoipa::path(
    responses(
        (status = 200, description = "Home endpoint")
    ),
    tags = ["health"]
)]
#[get("/")]
pub async fn home() -> HttpResponse {
    HttpResponse::Ok().body("Monitor")
}
