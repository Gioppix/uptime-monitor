# Uptime Monitor

_Attention: there are no nodes in Falkenstein as the region was unavailable during development. Checks there will not happen._

Goals:

- API first
- Low footprint
- Reliable

## Installation

Create a `terraform/prod.tfvars` file containing the needed variables, especially `hcloud_token` (taken from a Hetzner project).

Initialize the database:

```
$IP= <get from terraform output, a random one is ok>

# The port might need to be changed
docker run --rm --entrypoint cqlsh scylladb/scylla $IP 8443 -e "CREATE KEYSPACE IF NOT EXISTS default_keyspace WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 2};"
```

Run migrations:

```
docker run --rm --entrypoint cqlsh -v $(pwd)/backend/migrations:/migrations scylladb/scylla $IP 8443 -k default_keyspace -f /migrations/20251106152600_structure.cql
```

## Notes

If on macOS, enable Rosetta in Docker to speedup `amd64` builds.
