-- Test user for authentication flows
-- Password: "password123" (hashed with Argon2)
INSERT INTO users_by_id (user_id, username, user_hashed_password)
VALUES (33333333-3333-3333-3333-333333333333, 'testuser',
        '$argon2id$v=19$m=19456,t=2,p=1$NnnAff1hFe4rqgY7R5mhLw$DxsGrvPxXRAeCLjP15po9TYomtXSuLGABk+K5ShnBuk');

INSERT INTO users_by_username (username, user_id, user_hashed_password)
VALUES ('testuser', 33333333-3333-3333-3333-333333333333,
        '$argon2id$v=19$m=19456,t=2,p=1$NnnAff1hFe4rqgY7R5mhLw$DxsGrvPxXRAeCLjP15po9TYomtXSuLGABk+K5ShnBuk');
