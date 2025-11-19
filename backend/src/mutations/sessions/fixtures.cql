-- Insert test sessions
-- Valid session (expires in future)
INSERT INTO sessions (session_id, user_id, created_at, expires_at)
VALUES (aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 11111111-1111-1111-1111-111111111111,
        currenttimestamp(), currenttimestamp() + 365d);

-- Expired session (expires in past)
INSERT INTO sessions (session_id, user_id, created_at, expires_at)
VALUES (bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 22222222-2222-2222-2222-222222222222,
        currenttimestamp() - 365d, currenttimestamp() - 334d);

-- Session with null expires_at (should be treated as expired)
INSERT INTO sessions (session_id, user_id, created_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 33333333-3333-3333-3333-333333333333,
        currenttimestamp());
