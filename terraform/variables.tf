variable "hcloud_token" {
  description = "Hetzner Cloud API token"
  sensitive   = true
}

# Define nodes to deploy across datacenters
variable "nodes" {
  description = "List of nodes to deploy with their regions, datacenters and seed status"
  type = list(object({
    region     = string
    datacenter = string
    is_seed    = bool
  }))
  default = [
    { region = "fsn1", datacenter = "fsn1-dc14", is_seed = true },
    { region = "hel1", datacenter = "hel1-dc2", is_seed = true },
    { region = "nbg1", datacenter = "nbg1-dc3", is_seed = false }
  ]
}

# Control whether ScyllaDB is accessible via public IP
variable "scylla_public_access" {
  description = "Whether ScyllaDB should be accessible via public IP (true) or only private network (false)"
  type        = bool
  default     = false
}

# ScyllaDB CQL port (use 443 or 8443 to bypass network restrictions)
variable "scylla_cql_port" {
  description = "Port for ScyllaDB CQL connections (default 9042, use 443 or 8443 if blocked)"
  type        = number
  default     = 9042
}

# ScyllaDB SSL CQL port
variable "scylla_ssl_cql_port" {
  description = "Port for ScyllaDB SSL CQL connections"
  type        = number
  default     = 50001
}

# ScyllaDB shard-aware native port
variable "scylla_shard_aware_port" {
  description = "Port for ScyllaDB shard-aware native transport"
  type        = number
  default     = 50002
}

# ScyllaDB shard-aware SSL port
variable "scylla_shard_aware_ssl_port" {
  description = "Port for ScyllaDB shard-aware SSL transport"
  type        = number
  default     = 50003
}

# Monitoring stack ports
variable "grafana_port" {
  description = "Port for Grafana web interface"
  type        = number
  default     = 8080
}

variable "prometheus_port" {
  description = "Port for Prometheus"
  type        = number
  default     = 8081
}

variable "alertmanager_port" {
  description = "Port for Alertmanager"
  type        = number
  default     = 8082
}

variable "loki_port" {
  description = "Port for Loki"
  type        = number
  default     = 8083
}

# Backend port
variable "backend_port" {
  description = "Port for the backend API"
  type        = number
  default     = 40000
}

# Frontend port
variable "frontend_port" {
  description = "Port for the frontend web interface"
  type        = number
  default     = 80
}
