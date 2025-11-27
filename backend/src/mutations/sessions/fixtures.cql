-- Insert test sessions
-- Valid session (expires in future)
INSERT INTO sessions (session_id, user_id, created_at, expires_at, logged_out)
VALUES (aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 11111111-1111-1111-1111-111111111111,
        toTimestamp(now()), '2099-12-31 23:59:59+0000', false);

-- Expired session (expires in past)
INSERT INTO sessions (session_id, user_id, created_at, expires_at, logged_out)
VALUES (bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 22222222-2222-2222-2222-222222222222,
        '2020-01-01 00:00:00+0000', '2020-02-01 00:00:00+0000', false);

-- Session with null expires_at (should be treated as expired)
INSERT INTO sessions (session_id, user_id, created_at, expires_at, logged_out)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 33333333-3333-3333-3333-333333333333,
        toTimestamp(now()), null, false);
