#!/bin/bash
# Has to be run from the root folder
# Uses IPv6 by default (::, ::1)

docker rm -f $(docker ps -a -q --filter ancestor=cassandra-test) 2>/dev/null || true
docker build -t cassandra-test -f Cassandra/Dockerfile .

DATA_DIR=./cassandra-data
mkdir -p $DATA_DIR

docker run -it --rm -d \
  --name cassandra-node1 \
  -p 9042:9042 \
  -v $DATA_DIR:/var/lib/cassandra \
  -e CLUSTER_NAME=TestCluster \
  -e IS_SEED=true \
  -e MAX_HEAP_SIZE=2G \
  -e HEAP_NEWSIZE=512M \
  cassandra-test
