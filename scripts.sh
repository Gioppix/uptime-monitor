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
        cargo run &
        PID=$!
        cd ..
        sleep 5
        curl -o backend/OpenAPI.json localhost:8801/api/openapi.json
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
  docker run -d \
    --name cassandra \
    -p 9042:9042 \
    -v cassandra1:/var/lib/cassandra \
    -e CASSANDRA_CLUSTER_NAME="CassandraCluster" \
    -e CASSANDRA_LISTEN_ADDRESS="localhost" \
    -e CASSANDRA_BROADCAST_ADDRESS="localhost" \
    -e CASSANDRA_RPC_ADDRESS="0.0.0.0" \
    -e CASSANDRA_BROADCAST_RPC_ADDRESS="localhost" \
    -e CASSANDRA_SEEDS="localhost" \
    -e MAX_HEAP_SIZE="800M" \
    -e HEAP_NEWSIZE="200M" \
    cassandra:5
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
