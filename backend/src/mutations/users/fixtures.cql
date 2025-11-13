-- Insert test users
INSERT INTO users_by_id (user_id, username, user_hashed_password)
VALUES (11111111-1111-1111-1111-111111111111, 'testuser1',
        '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYfZJx.6YoK');

INSERT INTO users_by_id (user_id, username, user_hashed_password)
VALUES (22222222-2222-2222-2222-222222222222, 'testuser2',
        '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYfZJx.6YoK');

INSERT INTO users_by_username (username, user_id, user_hashed_password)
VALUES ('testuser1', 11111111-1111-1111-1111-111111111111,
        '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYfZJx.6YoK');

INSERT INTO users_by_username (username, user_id, user_hashed_password)
VALUES ('testuser2', 22222222-2222-2222-2222-222222222222,
        '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYfZJx.6YoK');
