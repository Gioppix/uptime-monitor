-- Check ID: aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa (all successful)
-- Successful checks in fsn1
INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'fsn1', '2025-11-29', '2025-11-29 10:00:00+0000', 100000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'fsn1', '2025-11-29', '2025-11-29 11:00:00+0000', 150000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'fsn1', '2025-11-29', '2025-11-29 12:00:00+0000', 120000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'fsn1', '2025-11-29', '2025-11-29 13:00:00+0000', 200000, 200, true, false, null);

-- Successful checks in hel1
INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'hel1', '2025-11-29', '2025-11-29 10:00:00+0000', 110000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'hel1', '2025-11-29', '2025-11-29 11:00:00+0000', 130000, 200, true, false, null);

-- Successful checks in nbg1
INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'nbg1', '2025-11-29', '2025-11-29 10:00:00+0000', 90000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa, 'nbg1', '2025-11-29', '2025-11-29 11:00:00+0000', 95000, 200, true, false, null);

-- Check ID: bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb (70% success rate)
-- Successful checks
INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 10:00:00+0000', 100000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 11:00:00+0000', 110000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 12:00:00+0000', 120000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 13:00:00+0000', 130000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 14:00:00+0000', 140000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 15:00:00+0000', 150000, 200, true, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 16:00:00+0000', 160000, 200, true, false, null);

-- Failed checks (30%)
INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 17:00:00+0000', 500000, 500, false, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 18:00:00+0000', 600000, 503, false, false, null);

INSERT INTO check_results (result_id, service_check_id, region, day, check_started_at, response_time_micros, status_code, matches_expected, response_body_fetched, response_body)
VALUES (uuid(), bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb, 'fsn1', '2025-11-29', '2025-11-29 19:00:00+0000', 550000, 404, false, false, null);

-- Check ID: cccccccc-cccc-cccc-cccc-cccccccccccc (hourly aggregates)
INSERT INTO check_results_hourly (service_check_id, region, hour, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'fsn1', '2025-11-29 10:00:00+0000', 95, 5, 125000, 80000, 300000, 120000, 250000, 280000, 95.00, '2025-11-29 11:00:00+0000');

INSERT INTO check_results_hourly (service_check_id, region, hour, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'fsn1', '2025-11-29 11:00:00+0000', 98, 2, 115000, 75000, 280000, 110000, 230000, 270000, 98.00, '2025-11-29 13:00:00+0000');

INSERT INTO check_results_hourly (service_check_id, region, hour, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'fsn1', '2025-11-29 12:00:00+0000', 98, 2, 115000, 75000, 280000, 110000, 230000, 270000, 98.00, '2025-11-29 13:00:00+0000');

INSERT INTO check_results_hourly (service_check_id, region, hour, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'nbg1', '2025-11-29 12:00:00+0000', 98, 2, 115000, 75000, 280000, 110000, 230000, 270000, 98.00, '2025-11-29 13:00:00+0000');

-- Check ID: cccccccc-cccc-cccc-cccc-cccccccccccc (daily aggregates)
INSERT INTO check_results_daily (service_check_id, region, day, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'fsn1', '2025-11-27', 95, 5, 125000, 80000, 300000, 120000, 250000, 280000, 95.00, '2025-11-29 11:00:00+0000');

INSERT INTO check_results_daily (service_check_id, region, day, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'nbg1', '2025-11-27', 95, 5, 125000, 80000, 300000, 120000, 250000, 280000, 95.00, '2025-11-29 11:00:00+0000');

INSERT INTO check_results_daily (service_check_id, region, day, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'fsn1', '2025-11-28', 98, 2, 115000, 75000, 280000, 110000, 230000, 270000, 98.00, '2025-11-29 13:00:00+0000');

INSERT INTO check_results_daily (service_check_id, region, day, successful_checks, failed_checks, avg_response_time_micros, min_response_time_micros, max_response_time_micros, p50_response_time_micros, p95_response_time_micros, p99_response_time_micros, uptime_percent, computed_at)
VALUES (cccccccc-cccc-cccc-cccc-cccccccccccc, 'fsn1', '2025-11-29', 98, 2, 115000, 75000, 280000, 110000, 230000, 270000, 98.00, '2025-11-29 13:00:00+0000');
