#!/bin/bash
set -e

export CLUSTER_NAME="${CLUSTER_NAME:-ScyllaCluster}"
export MEM="${MEM:-2G}"
export SMP="${SMP:-2}"
export LISTEN_ADDR="${LISTEN_ADDR:-::}"
export API_ADDR="${API_ADDR:-::}"
export SEEDS="${SEEDS:-::1}"
export DATA_DIR="${DATA_DIR:-/var/lib/scylla}"

export BROADCAST_ADDR="${BROADCAST_ADDR:-${NODE_IP:-::1}}"
export RPC_ADDR="${RPC_ADDR:-${LISTEN_ADDR}}"
export BROADCAST_RPC_ADDR="${BROADCAST_RPC_ADDR:-${NODE_IP:-::1}}"

if [ "${IS_SEED}" = "true" ] && [[ ! "${SEEDS}" =~ "${BROADCAST_ADDR}" ]]; then
    export SEEDS="${BROADCAST_ADDR}"
fi

exec /docker-entrypoint.py \
    --memory "${MEM}" \
    --smp "${SMP}" \
    --listen-address "${LISTEN_ADDR}" \
    --broadcast-address "${BROADCAST_ADDR}" \
    --rpc-address "${RPC_ADDR}" \
    --broadcast-rpc-address "${BROADCAST_RPC_ADDR}" \
    --api-address "${API_ADDR}" \
    --seed-provider-parameters "seeds=${SEEDS}" \
    --enable-ipv6-dns-lookup 1 \
    --listen-interface-prefer-ipv6 1 \
    --rpc-interface-prefer-ipv6 1 \
    --overprovisioned 1 \
    --workdir "${DATA_DIR}"
