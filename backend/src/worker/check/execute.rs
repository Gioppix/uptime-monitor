use crate::worker::fetch::{self, ServiceCheck};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use log::trace;
use reqwest::{Client, Method, header};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use url::Url;
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

fn is_safe_ip(ip: &IpAddr, accept_local: bool) -> bool {
    if accept_local {
        return true;
    }

    match ip {
        IpAddr::V4(ipv4) => {
            !ipv4.is_private()
                && !ipv4.is_loopback()
                && !ipv4.is_link_local()
                && !ipv4.is_broadcast()
                && !ipv4.is_documentation()
                && !ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            !ipv6.is_loopback()
                && !ipv6.is_unspecified()
                && !ipv6.is_unique_local()
                && !ipv6.is_unicast_link_local()
        }
    }
}

/// Validates the URL's resolved IP addresses and transforms the URL to use the IP directly.
/// Returns the transformed URL and the original host for the Host header.
pub async fn validate_and_transform_url(url: &Url, accept_local: bool) -> Result<(Url, String)> {
    let original_host = url.host_str().context("URL missing host")?.to_string();

    // Should always work for http(s)
    let port = url
        .port_or_known_default()
        .context("Unable to determine port")?;

    // Resolve DNS
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((original_host.as_str(), port))
        .await
        .context("DNS resolution failed")?
        .collect();

    if addrs.is_empty() {
        bail!("No IP addresses resolved for host: {}", original_host);
    }

    // Find first safe IP
    let safe_addr = addrs
        .iter()
        .find(|addr| is_safe_ip(&addr.ip(), accept_local))
        .context(format!(
            "All resolved IPs for {} are private/internal: {:?}",
            original_host,
            addrs.iter().map(|a| a.ip()).collect::<Vec<_>>()
        ))?;

    trace!("DNS validated: {} -> {}", original_host, safe_addr.ip());

    // Create new URL with IP address
    let mut ip_url = url.clone();
    let ip_host = match safe_addr.ip() {
        IpAddr::V4(ipv4) => ipv4.to_string(),
        IpAddr::V6(ipv6) => format!("[{}]", ipv6), // IPv6 needs brackets in URLs
    };

    ip_url
        .set_host(Some(&ip_host))
        .context("Failed to set IP address in URL")?;

    Ok((ip_url, original_host))
}

pub async fn execute_check(
    client: &Client,
    check: &ServiceCheck,
    accept_local: bool,
) -> Result<CheckResult> {
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

    // Validate URL and transform to use IP address
    let (_ip_url, original_host) = validate_and_transform_url(&check.url, accept_local)
        .await
        .context("URL validation failed")?;

    let start = Instant::now();
    let check_started_at = Utc::now();

    // TODO: use `ip_url` or fix
    // code: -67843, message: "The certificate was not trusted."
    let mut request = client
        .request(method, check.url.clone())
        .header(header::HOST, original_host) // Set original host for virtual hosting and TLS/SNI
        .timeout(Duration::from_secs(check.timeout_seconds as u64));

    for (key, value) in &check.request_headers {
        request = request.header(key, value);
    }

    if let Some(body) = &check.request_body
        && !body.is_empty()
    {
        request = request.body(body.clone());
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
            } else {
                trace!("Service check encountered error: {:?}", error);
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

    trace!(
        "Health check completed: {} - status: {:?}, matches: {}, time: {}μs",
        check.check_name, status_code, matches_expected, response_time_micros
    );

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
            region: Region::Hel1,
            check_name: String::from("test_check"),
            url: server.url("/").parse().unwrap(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 30,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: None,
            is_enabled: true,
            created_at: Utc::now(),
        };

        let result = execute_check(&client, &check, true).await;
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
            region: Region::Hel1,
            check_name: String::from("test_check"),
            url: server.url("/slow").parse().unwrap(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 1,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: None,
            is_enabled: true,
            created_at: Utc::now(),
        };

        let start = Instant::now();
        let result = execute_check(&client, &check, true).await.unwrap();
        let duration = start.elapsed();

        // Should timeout early
        assert!(duration < Duration::from_secs(2));

        assert_eq!(result.service_check_id, check.check_id);
        assert_eq!(result.status_code, None);
        assert!(!result.matches_expected);

        mock.assert();
    }

    #[tokio::test]
    async fn test_execute_check_example_com() {
        let client = Client::new();
        let check = ServiceCheck {
            check_id: Uuid::new_v4(),
            region: Region::Hel1,
            check_name: String::from("test_check"),
            url: "https://example.com/".parse().unwrap(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 10,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: None,
            is_enabled: true,
            created_at: Utc::now(),
        };

        execute_check(&client, &check, false).await.unwrap();
    }

    #[tokio::test]
    async fn test_execute_check_local_url_rejected() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200).body("OK");
        });

        let client = Client::new();
        let check = ServiceCheck {
            check_id: Uuid::new_v4(),
            region: Region::Hel1,
            check_name: String::from("test_check"),
            url: server.url("/").parse().unwrap(),
            http_method: Method::Get,
            check_frequency_seconds: 60,
            timeout_seconds: 30,
            expected_status_code: 200,
            request_headers: HashMap::new(),
            request_body: None,
            is_enabled: true,
            created_at: Utc::now(),
        };

        let result = execute_check(&client, &check, false).await;
        assert!(result.is_err());

        mock.assert_calls(0);
    }

    #[tokio::test]
    async fn test_validate_and_transform_url_success() {
        let url: Url = "https://example.com/path".parse().unwrap();
        let result = validate_and_transform_url(&url, false).await;

        assert!(result.is_ok());
        let (ip_url, original_host) = result.unwrap();

        // Should have replaced hostname with IP
        assert_ne!(ip_url.host_str().unwrap(), "example.com");
        // Should preserve original host
        assert_eq!(original_host, "example.com");
        // Should preserve path
        assert_eq!(ip_url.path(), "/path");
        // Should preserve scheme
        assert_eq!(ip_url.scheme(), "https");
    }

    #[tokio::test]
    async fn test_validate_and_transform_url_blocks_private_ip() {
        let url: Url = "http://localhost/admin".parse().unwrap();
        let result = validate_and_transform_url(&url, false).await;

        // Should fail because localhost resolves to 127.0.0.1 (private)
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("private") || error_msg.contains("internal"));
    }

    #[tokio::test]
    async fn test_is_safe_ip() {
        // Public IPs should be safe
        assert!(is_safe_ip(&"8.8.8.8".parse().unwrap(), false));
        assert!(is_safe_ip(&"1.1.1.1".parse().unwrap(), false));

        // Private IPs should not be safe
        assert!(!is_safe_ip(&"127.0.0.1".parse().unwrap(), false));
        assert!(!is_safe_ip(&"192.168.1.1".parse().unwrap(), false));
        assert!(!is_safe_ip(&"10.0.0.1".parse().unwrap(), false));
        assert!(!is_safe_ip(&"172.16.0.1".parse().unwrap(), false));
        assert!(!is_safe_ip(&"0.0.0.0".parse().unwrap(), false));

        // IPv6 localhost should not be safe
        assert!(!is_safe_ip(&"::1".parse().unwrap(), false));
        // IPv6 link-local should not be safe
        assert!(!is_safe_ip(&"fe80::1".parse().unwrap(), false));

        // All IPs should be safe when accept_local is true
        assert!(is_safe_ip(&"127.0.0.1".parse().unwrap(), true));
        assert!(is_safe_ip(&"192.168.1.1".parse().unwrap(), true));
        assert!(is_safe_ip(&"10.0.0.1".parse().unwrap(), true));
    }
}
