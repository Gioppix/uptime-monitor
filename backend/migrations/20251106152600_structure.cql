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

    PRIMARY KEY (service_id, check_id)
);

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
    day                   date,
    check_started_at      timestamp,
    response_time_micros  bigint,
    status_code           int,
    is_success            boolean,
    response_body_fetched boolean,
    response_body         text,

    PRIMARY KEY ((service_check_id, day), check_started_at, result_id)
) WITH CLUSTERING ORDER BY (check_started_at DESC)
   AND DEFAULT_TIME_TO_LIVE = 7776000;
