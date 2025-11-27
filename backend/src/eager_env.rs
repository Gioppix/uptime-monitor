use std::env;
use std::sync::LazyLock;

macro_rules! define_env_vars {
    ($(($name:ident, $env_name:expr, $type:ty)),* $(,)?) => {
        $(
            pub static $name: LazyLock<$type> = LazyLock::new(|| {
                let val = env::var($env_name).unwrap_or_else(|_| {
                    panic!("Missing required environment variable: {}", $env_name)
                });
                val.parse::<$type>().unwrap_or_else(|_| {
                    panic!(
                        "Failed to parse environment variable {} with value '{}' as {}",
                        $env_name,
                        val,
                        stringify!($type)
                    )
                })
            });
        )*

        /// Force initialization of all environment variables at startup
        /// Call this early in main() to fail fast if any env vars are missing
        pub fn check_env() {
            $(
                let _ = *$name;
            )*
        }
    };
}

// Define all environment variables
define_env_vars!(
    (PORT, "PORT", u16),
    (DATABASE_NODE_URLS, "DATABASE_NODE_URLS", String),
    (DATABASE_KEYSPACE, "DATABASE_KEYSPACE", String),
    (
        DATABASE_CONCURRENT_REQUESTS,
        "DATABASE_CONCURRENT_REQUESTS",
        usize
    ),
    (DATABASE_CONNECTIONS, "DATABASE_CONNECTIONS", usize),
    (COOKIE_KEY, "COOKIE_KEY", String),
    (DEV_MODE, "DEV_MODE", bool),
    (SESSION_DURATION_DAYS, "SESSION_DURATION_DAYS", i64),
    (FRONTEND_PUBLIC_URL, "FRONTEND_PUBLIC_URL", String),
    (
        HEARTBEAT_INTERVAL_SECONDS,
        "HEARTBEAT_INTERVAL_SECONDS",
        u64
    ),
    (CURRENT_BUCKET_VERSION, "CURRENT_BUCKET_VERSION", u32),
    (CURRENT_BUCKETS_COUNT, "CURRENT_BUCKETS_COUNT", u32),
    (REPLICATION_FACTOR, "REPLICATION_FACTOR", u32),
    (
        MAX_CONCURRENT_HEALTH_CHECKS,
        "MAX_CONCURRENT_HEALTH_CHECKS",
        usize
    ),
    (REPLICAS_COMMON_KEY, "REPLICAS_COMMON_KEY", String),
    (RAILWAY_REPLICA_REGION, "RAILWAY_REPLICA_REGION", String),
);
