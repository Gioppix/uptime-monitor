use crate::collab::get_bucket_for_check;
use crate::queries::checks::{Check, CheckData};
use crate::regions::Region;
use crate::server::checks::CheckWithAccess;
use crate::server::start_server_test;
use crate::worker::Method;
use chrono::Utc;
use reqwest::StatusCode;
use std::collections::HashMap;
use uuid::{Uuid, uuid};

const FIXTURES_TEMPLATE: &str = include_str!("fixtures.cql");

fn get_fixtures() -> String {
    let check_id = uuid!("44444444-4444-4444-4444-444444444444");
    let (bucket_version, bucket) = get_bucket_for_check(check_id);

    FIXTURES_TEMPLATE
        .replace("{{BUCKET_VERSION}}", &bucket_version.to_string())
        .replace("{{BUCKET}}", &bucket.to_string())
}

#[tokio::test]
async fn test_check_endpoints() {
    let fixtures = get_fixtures();
    let (port, _) = start_server_test(Some(&fixtures)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://localhost:{}", port);

    // Use pre-created session from fixtures
    let session_cookie = format!(
        "session_id={}",
        uuid!("55555555-5555-5555-5555-555555555555")
    );

    // === Test unauthenticated access (should all return 401) ===

    // Get check without auth
    let response = client
        .get(format!(
            "{}/checks/44444444-4444-4444-4444-444444444444",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Create check without auth
    let check_data = CheckData {
        check_name: "Test Check".to_string(),
        url: "https://example.com".to_string(),
        http_method: Method::Get,
        check_frequency_seconds: 60,
        timeout_seconds: 10,
        expected_status_code: 200,
        request_headers: HashMap::new(),
        request_body: None,
        is_enabled: true,
        created_at: Utc::now(),
    };

    let test_check = Check {
        check_id: Uuid::new_v4(),
        regions: vec![Region::Hel1],
        data: check_data,
    };

    let response = client
        .post(format!("{}/checks/", base_url))
        .json(&test_check)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // List checks without auth
    let response = client
        .get(format!("{}/checks/", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Update check without auth
    let response = client
        .patch(format!(
            "{}/checks/44444444-4444-4444-4444-444444444444",
            base_url
        ))
        .json(&test_check)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Delete check without auth
    let response = client
        .delete(format!(
            "{}/checks/44444444-4444-4444-4444-444444444444",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // === Test authenticated CRUD operations ===

    // Get existing check from fixtures
    let response = client
        .get(format!(
            "{}/checks/44444444-4444-4444-4444-444444444444",
            base_url
        ))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let check_with_access: CheckWithAccess = response.json().await.unwrap();
    assert_eq!(
        check_with_access.check.check_id,
        Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap()
    );
    assert!(check_with_access.access.can_edit);
    assert!(check_with_access.access.can_see);

    // Get non-existent check (should return 403)
    let response = client
        .get(format!(
            "{}/checks/99999999-9999-9999-9999-999999999999",
            base_url
        ))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Create new check
    let check_data = CheckData {
        check_name: "New Test Check".to_string(),
        url: "https://newcheck.com".to_string(),
        http_method: Method::Post,
        check_frequency_seconds: 60,
        timeout_seconds: 10,
        expected_status_code: 200,
        request_headers: HashMap::new(),
        request_body: Some(r#"{"test": "data"}"#.to_string()),
        is_enabled: true,
        created_at: Utc::now(),
    };

    let new_check = Check {
        check_id: Uuid::new_v4(),
        regions: vec![Region::Fsn1],
        data: check_data.clone(),
    };

    let response = client
        .post(format!("{}/checks/", base_url))
        .header("Cookie", &session_cookie)
        .json(&new_check)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created_check: Check = response.json().await.unwrap();
    let new_check_id = created_check.check_id;

    // List my checks (should include both fixture and new check)
    let response = client
        .get(format!("{}/checks/", base_url))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let checks: Vec<CheckWithAccess> = response.json().await.unwrap();
    assert!(checks.len() >= 2);
    assert!(
        checks.iter().any(|c| c.check.check_id
            == Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap())
    );
    assert!(checks.iter().any(|c| c.check.check_id == new_check_id));

    // Update the newly created check
    let updated_data = CheckData {
        check_name: "Updated Test Check".to_string(),
        url: "https://updated.com".to_string(),
        http_method: Method::Get,
        check_frequency_seconds: 120,
        timeout_seconds: 15,
        expected_status_code: 201,
        request_headers: HashMap::new(),
        request_body: None,
        is_enabled: true,
        created_at: Utc::now(),
    };

    let updated_check = Check {
        check_id: new_check_id,
        regions: vec![Region::Nbg1],
        data: updated_data,
    };

    let response = client
        .patch(format!("{}/checks/{}", base_url, new_check_id))
        .header("Cookie", &session_cookie)
        .json(&updated_check)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let returned_check: Check = response.json().await.unwrap();
    assert_eq!(returned_check.check_id, new_check_id);
    assert_eq!(returned_check.data.url, "https://updated.com");

    // Get the updated check to verify changes
    let response = client
        .get(format!("{}/checks/{}", base_url, new_check_id))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched_check: CheckWithAccess = response.json().await.unwrap();
    assert_eq!(fetched_check.check.data.url, "https://updated.com");

    // Delete the newly created check
    let response = client
        .delete(format!("{}/checks/{}", base_url, new_check_id))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify check is deleted (should be 404 or 403)
    let response = client
        .get(format!("{}/checks/{}", base_url, new_check_id))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::FORBIDDEN
    );

    // Verify list no longer includes deleted check
    let response = client
        .get(format!("{}/checks/", base_url))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let checks: Vec<CheckWithAccess> = response.json().await.unwrap();
    assert!(!checks.iter().any(|c| c.check.check_id == new_check_id));
}
