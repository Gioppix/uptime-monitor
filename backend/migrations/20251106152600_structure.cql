CREATE TABLE IF NOT EXISTS users_by_id
(
    user_id              uuid,
    user_hashed_password text,
    username             text,

    PRIMARY KEY (user_id)
);

CREATE TABLE IF NOT EXISTS sessions
(
    session_id uuid,
    user_id    uuid,
    created_at timestamp,
    expires_at timestamp,
    logged_out boolean,

    PRIMARY KEY (session_id)
);

CREATE TABLE IF NOT EXISTS users_by_username
(
    username             text,
    user_id              uuid,
    user_hashed_password text,

    PRIMARY KEY (username)
);

CREATE TABLE IF NOT EXISTS checks
(
    check_id                uuid,
    region                  text,
    bucket_version          smallint,
    bucket                  int,

    check_name              text,
    url                     text,
    http_method             text,
    check_frequency_seconds int,
    timeout_seconds         int,
    expected_status_code    int,
    request_headers         map<text, text>,
    request_body            text,
    is_enabled              boolean,
    created_at              timestamp,

    PRIMARY KEY ((region, bucket_version, bucket), check_id)
);

CREATE TABLE IF NOT EXISTS access_by_check
(
    check_id  uuid,
    user_id   uuid,
    user_name text,
    can_edit  boolean,
    can_see   boolean,

    PRIMARY KEY (check_id, user_id)
);

CREATE INDEX IF NOT EXISTS access_by_check_by_user ON access_by_check (user_id);

CREATE TABLE IF NOT EXISTS check_results
(
    result_id             uuid,
    service_check_id      uuid,
    region                text,
    day                   date,
    check_started_at      timestamp,
    response_time_micros  bigint,
    status_code           int,
    matches_expected      boolean,
    response_body_fetched boolean,
    response_body         text,

    PRIMARY KEY ((service_check_id, region, day), check_started_at)
) WITH CLUSTERING ORDER BY (check_started_at DESC)
   AND DEFAULT_TIME_TO_LIVE = 7776000;


CREATE TABLE IF NOT EXISTS workers_heartbeats
(
    region              text,
    time_bucket_minutes bigint,
    timestamp           timestamp,
    position            int,
    process_id          uuid,
    address             text,

    PRIMARY KEY ((region, time_bucket_minutes), timestamp)
) WITH CLUSTERING ORDER BY (timestamp DESC)
   AND DEFAULT_TIME_TO_LIVE = 2592000;

CREATE TABLE IF NOT EXISTS workers_metadata
(
    process_id uuid,
    replica_id text,
    git_sha    text,

    PRIMARY KEY (process_id)
);

CREATE TABLE IF NOT EXISTS check_results_daily
(
    service_check_id         uuid,
    region                   text,
    day                      date,

    successful_checks        int,
    failed_checks            int,

    avg_response_time_micros bigint,
    min_response_time_micros bigint,
    max_response_time_micros bigint,
    p50_response_time_micros bigint,
    p95_response_time_micros bigint,
    p99_response_time_micros bigint,

    status_code_distribution map<int, int>,

    uptime_percent           float,

    computed_at              timestamp,

    PRIMARY KEY ((service_check_id, region), day)
) WITH CLUSTERING ORDER BY (day DESC)
   AND DEFAULT_TIME_TO_LIVE = 31536000; -- 1 year

CREATE TABLE IF NOT EXISTS check_results_hourly
(
    service_check_id         uuid,
    region                   text,
    hour                     timestamp, -- First instant of the relevant hour, e.g. 2025-12-16T17:00:00.000Z

    successful_checks        int,
    failed_checks            int,

    avg_response_time_micros bigint,
    min_response_time_micros bigint,
    max_response_time_micros bigint,
    p50_response_time_micros bigint,
    p95_response_time_micros bigint,
    p99_response_time_micros bigint,

    uptime_percent           float,

    computed_at              timestamp,

    PRIMARY KEY ((service_check_id, region), hour) -- We should break down this partition, but 100k entries is 11 years
) WITH CLUSTERING ORDER BY (hour DESC)
   AND DEFAULT_TIME_TO_LIVE = 2592000;
