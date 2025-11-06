#!/bin/bash

set -eh

cache_queries() {
  echo "Caching SQL queries..."

  # `all-targets` is needed to cache tests
  (cd backend && cargo sqlx prepare -- --all-targets)
}

format_toml() {
  taplo fmt **/*.toml
}

update_pub_env() {
  echo "Updating public env file..."
  ./update_env.py backend/
}

run_automatic_tests() {
  echo "Running automatic tests..."
  (cd backend && cargo test --workspace)
}

pre_commit() {
  cache_queries && format_toml && update_pub_env && run_automatic_tests
}
