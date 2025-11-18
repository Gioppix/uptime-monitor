use crate::worker::fetch::{self, ServiceCheck};
use chrono::{DateTime, Utc};
use reqwest::{Client, Method};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct CheckResult {
    pub result_id: Uuid,
    pub service_check_id: Uuid,
    pub check_started_at: DateTime<Utc>,
    pub response_time_micros: i64,
    pub status_code: Option<i32>,
    pub matches_expected: bool,
    pub response_body_fetched: bool,
    pub response_body: Option<String>,
}

pub async fn execute_check(client: &Client, check: &ServiceCheck) -> CheckResult {
    let method = match check.http_method {
        fetch::Method::Get => Method::GET,
        fetch::Method::Post => Method::POST,
        fetch::Method::Put => Method::PUT,
        fetch::Method::Delete => Method::DELETE,
        fetch::Method::Head => Method::HEAD,
    };

    let start = Instant::now();
    let check_started_at = Utc::now();

    let mut request = client
        .request(method, &check.url)
        .timeout(Duration::from_secs(check.timeout_seconds as u64));

    for (key, value) in &check.request_headers {
        request = request.header(key, value);
    }

    if !check.request_body.is_empty() {
        request = request.body(check.request_body.clone());
    }

    let result = request.send().await;
    let response_time_micros = start.elapsed().as_micros() as i64;

    match result {
        Ok(response) => {
            let status_code = response.status().as_u16() as i32;
            let matches_expected = status_code == check.expected_status_code;

            CheckResult {
                result_id: Uuid::new_v4(),
                service_check_id: check.check_id,
                check_started_at,
                response_time_micros,
                status_code: Some(status_code),
                matches_expected,
                response_body_fetched: false,
                response_body: None,
            }
        }
        Err(_) => {
            todo!()
        }
    }
}
