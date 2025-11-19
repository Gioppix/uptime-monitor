use crate::worker::fetch::{self, ServiceCheck};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use log::trace;
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

pub async fn execute_check(client: &Client, check: &ServiceCheck) -> Result<CheckResult> {
    trace!(
        "Executing health check task: {} {} {}",
        check.check_name, check.check_frequency_seconds, check.check_id
    );

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

    let (status_code, matches_expected) = match result {
        Ok(response) => {
            let status_code = response.status().as_u16() as i32;
            let matches_expected = status_code == check.expected_status_code;
            (Some(status_code), matches_expected)
        }
        Err(error) => {
            // Only mark as genuine failure for errors that indicate the service is down/unhealthy
            // Exclude errors that indicate problems with our check implementation itself
            let genuine_fail =
                error.is_timeout() || error.is_connect() || error.is_request() || error.is_body();

            if !genuine_fail {
                bail!("health check error");
            }

            // This never matches the expected code
            (None, false)
        }
    };

    let result = CheckResult {
        result_id: Uuid::new_v4(),
        service_check_id: check.check_id,
        check_started_at,
        response_time_micros,
        status_code,
        matches_expected,
        response_body_fetched: false,
        response_body: None,
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        regions::Region,
        worker::fetch::{Method, ServiceCheck},
    };
    use httpmock::prelude::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_execute_check_success() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200).body("OK");
        });

        let client = Client::new();
        let check = ServiceCheck {
            check_id: Uuid::new_v4(),
            region: Region::UsEast,
            check_name: String::from("test_check"),
            url: server.url("/"),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 30,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: String::new(),
            is_enabled: true,
            created_at: Utc::now(),
        };

        let result = execute_check(&client, &check).await;
        assert!(result.is_ok());

        let check_result = result.unwrap();
        assert_eq!(check_result.service_check_id, check.check_id);
        assert_eq!(check_result.status_code, Some(200));
        assert!(check_result.matches_expected);
        assert!(check_result.response_time_micros > 0);

        mock.assert();
    }

    #[tokio::test]
    async fn test_execute_check_timeout() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/slow");
            then.status(200).delay(Duration::from_secs(5)).body("");
        });

        let client = Client::new();
        let check = ServiceCheck {
            check_id: Uuid::new_v4(),
            region: Region::UsEast,
            check_name: String::from("test_check"),
            url: server.url("/slow"),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 1,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: String::new(),
            is_enabled: true,
            created_at: Utc::now(),
        };

        let start = Instant::now();
        let result = execute_check(&client, &check).await;
        let duration = start.elapsed();
        assert!(result.is_ok());

        // Should timeout early
        assert!(duration < Duration::from_secs(2));

        let check_result = result.unwrap();
        assert_eq!(check_result.service_check_id, check.check_id);
        assert_eq!(check_result.status_code, None);
        assert!(!check_result.matches_expected);

        mock.assert();
    }
}
