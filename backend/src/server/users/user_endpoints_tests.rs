use crate::server::start_server_test;
use crate::server::users::{CreateUserRequest, LoginRequest, PublicUser};
use reqwest::StatusCode;
use uuid::Uuid;

const FIXTURES: &str = include_str!("fixtures.cql");

fn extract_session_cookie(response: &reqwest::Response) -> Option<String> {
    response
        .cookies()
        .find(|c| c.name() == "session_id")
        .map(|c| format!("{}={}", c.name(), c.value()))
}

#[tokio::test]
async fn test_cascading_user_flows() {
    let (port, _) = start_server_test(Some(FIXTURES)).await;
    let client = reqwest::Client::new();
    let base_url = format!("http://localhost:{}", port);

    // Test 1: Get existing user
    let response = client
        .get(format!(
            "{}/users/info/33333333-3333-3333-3333-333333333333",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let user: PublicUser = response.json().await.unwrap();
    assert_eq!(user.username, "testuser");

    // Test 2: Get non-existent user (404)
    let response = client
        .get(format!(
            "{}/users/info/99999999-9999-9999-9999-999999999999",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Test 3: Create new user
    let new_username = format!("newuser_{}", Uuid::new_v4());
    let response = client
        .post(format!("{}/users/new", base_url))
        .json(&CreateUserRequest {
            username: new_username.clone(),
            password: "secure_pass".to_string(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let session_cookie = extract_session_cookie(&response).expect("No session cookie");
    let new_user: PublicUser = response.json().await.unwrap();
    assert_eq!(new_user.username, new_username);
    let new_user_id = new_user.user_id;

    // Test 4: Logout (should work since create_new_user sets session cookie)
    let response = client
        .post(format!("{}/users/logout", base_url))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test 5: Login with fixture user (correct password)
    let response = client
        .post(format!("{}/users/login", base_url))
        .json(&LoginRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let session_cookie = extract_session_cookie(&response).expect("No session cookie");
    let user: PublicUser = response.json().await.unwrap();
    assert_eq!(user.username, "testuser");

    // Test 6: Logout again
    let response = client
        .post(format!("{}/users/logout", base_url))
        .header("Cookie", &session_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test 7: Login with wrong password (401)
    let response = client
        .post(format!("{}/users/login", base_url))
        .json(&LoginRequest {
            username: "testuser".to_string(),
            password: "wrong_password".to_string(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test 8: Login with non-existent user
    let response = client
        .post(format!("{}/users/login", base_url))
        .json(&LoginRequest {
            username: "nonexistent".to_string(),
            password: "password".to_string(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test 9: Login with newly created user
    let response = client
        .post(format!("{}/users/login", base_url))
        .json(&LoginRequest {
            username: new_username.clone(),
            password: "secure_pass".to_string(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test 10: Verify authenticated access to get_user still works
    let response = client
        .get(format!("{}/users/info/{}", base_url, new_user_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched_user: PublicUser = response.json().await.unwrap();
    assert_eq!(fetched_user.user_id, new_user_id);
    assert_eq!(fetched_user.username, new_username);
}

#[tokio::test]
async fn test_logout_requires_auth() {
    let (port, _) = start_server_test(None).await;
    let client = reqwest::Client::new();

    // Attempt logout without being logged in
    let response = client
        .post(format!("http://localhost:{}/users/logout", port))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
