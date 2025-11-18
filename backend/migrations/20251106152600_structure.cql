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

CREATE TABLE IF NOT EXISTS workspaces
(
    workspace_id   uuid,
    workspace_name text,
    is_public      boolean,

    PRIMARY KEY (workspace_id)
);

CREATE TABLE IF NOT EXISTS services
(
    workspace_id uuid,
    service_id   uuid,
    service_name text,

    PRIMARY KEY (workspace_id, service_id)
);

CREATE TABLE IF NOT EXISTS service_checks
(
    check_id                uuid,
    region                  text,
    bucket_version          smallint,
    bucket                  int,

    service_id              uuid,
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

CREATE INDEX IF NOT EXISTS service_checks_by_service ON service_checks (service_id) USING 'SAI';

CREATE TABLE IF NOT EXISTS access_by_user
(
    user_id      uuid,
    user_name    text,
    can_edit     boolean,
    can_see      boolean,
    workspace_id uuid,

    PRIMARY KEY (user_id, workspace_id)
);

CREATE TABLE IF NOT EXISTS access_by_workspace
(
    workspace_id uuid,
    user_id      uuid,
    user_name    text,
    can_edit     boolean,
    can_see      boolean,

    PRIMARY KEY (workspace_id, user_id)
);

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
