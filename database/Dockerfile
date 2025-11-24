FROM scylladb/scylla:latest

# Set environment variables for Scylla configuration
ENV SCYLLA_CLUSTER_NAME=CassandraCluster
ENV SCYLLA_LISTEN_ADDRESS=
ENV SCYLLA_BROADCAST_ADDRESS=
ENV SCYLLA_RPC_ADDRESS=
ENV SCYLLA_BROADCAST_RPC_ADDRESS=
ENV SCYLLA_SEEDS=
ENV SCYLLA_DEVELOPER_MODE=1
ENV EXTRA_ARGS=""

EXPOSE 9042
EXPOSE 9160
EXPOSE 10000
EXPOSE 9180
EXPOSE 7000
EXPOSE 7001

ENTRYPOINT ["/bin/bash", "-c", "\
    ARGS=\"\"; \
    if [ -n \"$SCYLLA_CLUSTER_NAME\" ]; then ARGS=\"$ARGS --cluster-name=$SCYLLA_CLUSTER_NAME\"; fi; \
    if [ -n \"$SCYLLA_LISTEN_ADDRESS\" ]; then ARGS=\"$ARGS --listen-address=$SCYLLA_LISTEN_ADDRESS\"; fi; \
    if [ -n \"$SCYLLA_BROADCAST_ADDRESS\" ]; then ARGS=\"$ARGS --broadcast-address=$SCYLLA_BROADCAST_ADDRESS\"; fi; \
    if [ -n \"$SCYLLA_RPC_ADDRESS\" ]; then ARGS=\"$ARGS --rpc-address=$SCYLLA_RPC_ADDRESS\"; fi; \
    if [ -n \"$SCYLLA_BROADCAST_RPC_ADDRESS\" ]; then ARGS=\"$ARGS --broadcast-rpc-address=$SCYLLA_BROADCAST_RPC_ADDRESS\"; fi; \
    if [ -n \"$SCYLLA_SEEDS\" ]; then ARGS=\"$ARGS --seeds=$SCYLLA_SEEDS\"; fi; \
    if [ -n \"$SCYLLA_DEVELOPER_MODE\" ]; then ARGS=\"$ARGS --developer-mode=$SCYLLA_DEVELOPER_MODE\"; fi; \
    exec /docker-entrypoint.py $ARGS $EXTRA_ARGS \"$@\""]
