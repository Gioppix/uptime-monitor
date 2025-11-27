# ScyllaDB Configuration Template
# Managed by Terraform - changes will be overwritten

cluster_name: "Uptime Monitor Cluster"
authenticator: AllowAllAuthenticator
authorizer: AllowAllAuthorizer

# Network configuration passed via command-line arguments

# Ports
native_transport_port: ${scylla_port}
native_transport_port_ssl: null

# Storage
data_file_directories:
    - /var/lib/scylla/data

commitlog_directory: /var/lib/scylla/commitlog

# Performance
endpoint_snitch: SimpleSnitch
