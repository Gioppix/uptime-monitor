#!/bin/bash
set -e

export CLUSTER_NAME="${CLUSTER_NAME:-CassandraCluster}"
export MAX_HEAP_SIZE="${MAX_HEAP_SIZE:-2G}"
export HEAP_NEWSIZE="${HEAP_NEWSIZE:-512M}"
export LISTEN_ADDR="${LISTEN_ADDR:-::}"
export SEEDS="${SEEDS:-::1}"
export DATA_DIR="${DATA_DIR:-/var/lib/cassandra}"

export BROADCAST_ADDR="${BROADCAST_ADDR:-${NODE_IP:-::1}}"
export RPC_ADDR="${RPC_ADDR:-::}"
export BROADCAST_RPC_ADDR="${BROADCAST_RPC_ADDR:-${NODE_IP:-::1}}"

# Configure Cassandra memory settings
mkdir -p /etc/cassandra/jvm-server.options.d
cat > /etc/cassandra/jvm-server.options.d/memory.options <<EOF
-Xms${MAX_HEAP_SIZE}
-Xmx${MAX_HEAP_SIZE}
-Xmn${HEAP_NEWSIZE}
EOF

# Configure cassandra.yaml
sed -i "s/^cluster_name:.*/cluster_name: '${CLUSTER_NAME}'/" /etc/cassandra/cassandra.yaml
sed -i "s/^listen_address:.*/listen_address: ${LISTEN_ADDR}/" /etc/cassandra/cassandra.yaml
sed -i "s/^broadcast_address:.*/broadcast_address: ${BROADCAST_ADDR}/" /etc/cassandra/cassandra.yaml
sed -i "s/^rpc_address:.*/rpc_address: ${RPC_ADDR}/" /etc/cassandra/cassandra.yaml
sed -i "s/^broadcast_rpc_address:.*/broadcast_rpc_address: ${BROADCAST_RPC_ADDR}/" /etc/cassandra/cassandra.yaml
sed -i "s/- seeds:.*/- seeds: \"${SEEDS}\"/" /etc/cassandra/cassandra.yaml

# Start Cassandra
exec /usr/local/bin/docker-entrypoint.sh cassandra -f
