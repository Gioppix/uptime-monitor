-- Test user for check tests
-- Password: "password123" (hashed with Argon2)
INSERT INTO users_by_id (user_id, username, user_hashed_password)
VALUES (33333333-3333-3333-3333-333333333333, 'testuser',
        '$argon2id$v=19$m=19456,t=2,p=1$NnnAff1hFe4rqgY7R5mhLw$DxsGrvPxXRAeCLjP15po9TYomtXSuLGABk+K5ShnBuk');

INSERT INTO users_by_username (username, user_id, user_hashed_password)
VALUES ('testuser', 33333333-3333-3333-3333-333333333333,
        '$argon2id$v=19$m=19456,t=2,p=1$NnnAff1hFe4rqgY7R5mhLw$DxsGrvPxXRAeCLjP15po9TYomtXSuLGABk+K5ShnBuk');

-- Test session for testuser
INSERT INTO sessions (session_id, user_id, created_at, expires_at, logged_out)
VALUES (55555555-5555-5555-5555-555555555555, 33333333-3333-3333-3333-333333333333, currenttimestamp(), currenttimestamp() + 1d, false);

-- Test check that testuser has access to
INSERT INTO checks (check_id, region, bucket_version, bucket, check_name, url, http_method, 
                    check_frequency_seconds, timeout_seconds, expected_status_code, 
                    request_headers, request_body, is_enabled, created_at)
VALUES (44444444-4444-4444-4444-444444444444, 'us-east4-eqdc4a', {{BUCKET_VERSION}}, {{BUCKET}}, 
        'Test Check', 'https://example.com', 'GET', 60, 10, 200, 
        {}, null, true, currenttimestamp());

-- Grant testuser access to the check
INSERT INTO access_by_check (check_id, user_id, user_name, can_edit, can_see)
VALUES (44444444-4444-4444-4444-444444444444, 33333333-3333-3333-3333-333333333333, 'testuser', true, true);
