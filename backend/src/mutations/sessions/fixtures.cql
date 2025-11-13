-- Insert test sessions
-- Valid session (expires in future)
INSERT INTO sessions (session_id, user_id, created_at, expires_at)
VALUES (aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 11111111-1111-1111-1111-111111111111,
        '2025-01-01 00:00:00', '2026-01-01 00:00:00');

-- Expired session (expires in past)
INSERT INTO sessions (session_id, user_id, created_at, expires_at)
VALUES (bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 22222222-2222-2222-2222-222222222222,
        '2024-01-01 00:00:00', '2024-02-01 00:00:00');
