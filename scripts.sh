#!/bin/bash

set -eh

format_toml() {
  taplo fmt **/*.toml
}

update_pub_env() {
  echo "Updating public env file..."
  ./update_env.py backend/
  ./update_env.py frontend/
}

run_automatic_tests() {
  echo "Running automatic tests..."
  (cd backend && cargo test --workspace)
}

generate_openapi_docs() {
    echo "Generating OpenAPI documentation..."
    (
        cd backend
        # Find an available port
        PORT=$(python3 -c "import socket; s=socket.socket(); s.bind(('', 0)); print(s.getsockname()[1]); s.close()")
        echo "Using port: $PORT"
        echo "Running server..."
        TEMPFILE=$(mktemp)
        PORT=$PORT cargo run 2>&1 > "$TEMPFILE" &
        PID=$!
        cd ..
        echo "Waiting for server to be ready on port $PORT..."
        while ! grep -q ":$PORT" "$TEMPFILE"; do
            if ! ps -p $PID > /dev/null 2>&1; then
                echo "Error: Server process died"
                cat "$TEMPFILE"
                rm "$TEMPFILE"
                exit 1
            fi
            sleep 0.1
        done
        rm "$TEMPFILE"
        echo "Server ready"
        curl -o backend/OpenAPI.json localhost:$PORT/api/openapi.json
        kill $PID
        jq '.' backend/OpenAPI.json > backend/OpenAPI.json.tmp && mv backend/OpenAPI.json.tmp backend/OpenAPI.json
    )
    (cd frontend && npm run generate:api)
}


pre_commit() {
  generate_openapi_docs && format_toml && update_pub_env && run_automatic_tests
}


start_cassandra() {
  echo "Starting Cassandra Docker container..."
  # docker network create cassandra-net
  docker run -d \
    --name cassandra1 \
    --network cassandra-net \
    -p 9042:9042 \
    -v cassandra1:/var/lib/cassandra \
    -e CASSANDRA_CLUSTER_NAME="CassandraCluster" \
    -e CASSANDRA_BROADCAST_ADDRESS="cassandra1" \
    -e CASSANDRA_SEEDS="cassandra1" \
    -e MAX_HEAP_SIZE="800M" \
    -e HEAP_NEWSIZE="200M" \
    cassandra:5
}

start_scylla() {
  echo "Starting ScyllaDB container from custom Dockerfile..."
  docker network create scylla-net 2>/dev/null || true
  docker build -t scylla-custom -f database/Dockerfile .
  docker run -d \
    --name scylla1 \
    --network scylla-net \
    -p 9042:9042 \
    -p 10000:10000 \
    -v scylla1:/var/lib/scylla \
    -e SCYLLA_CLUSTER_NAME="TestScyllaCluster" \
    -e SCYLLA_LISTEN_ADDRESS="scylla1" \
    -e SCYLLA_BROADCAST_ADDRESS="scylla1" \
    -e SCYLLA_RPC_ADDRESS="scylla1" \
    -e SCYLLA_BROADCAST_RPC_ADDRESS="scylla1" \
    -e SCYLLA_SEEDS="scylla1" \
    -e SCYLLA_DEVELOPER_MODE="1" \
    -e EXTRA_ARGS="--reactor-backend=epoll --smp=1" \
    scylla-custom
}

reset_dev_database() {
  echo "Resetting development database..."

  echo "Dropping keyspace 'default_keyspace'..."
  docker exec -i cassandra1 cqlsh -e "DROP KEYSPACE IF EXISTS default_keyspace;"

  echo "Creating keyspace 'default_keyspace'..."
  docker exec -i cassandra1 cqlsh -e "CREATE KEYSPACE IF NOT EXISTS default_keyspace WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1};"

  echo "Running migrations..."
  for migration in backend/migrations/*.cql; do
    if [ -f "$migration" ]; then
      echo "Applying migration: $(basename $migration)"
      docker exec -i cassandra1 cqlsh -k default_keyspace < "$migration"
    fi
  done

  echo "Database reset complete."
}

new_migration() {
  if [ -z "$1" ]; then
    echo "Error: Migration name required"
    echo "Usage: new_migration <name>"
    return 1
  fi

  local timestamp=$(date +%Y%m%d%H%M%S)
  local filename="${timestamp}_${1}.cql"

  touch "backend/migrations/$filename"
  echo "Created migration file: $filename"
}
