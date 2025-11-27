terraform {
  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.56"
    }
  }
}


variable "hcloud_token" {
  sensitive = true
}

# Define nodes to deploy across datacenters
variable "nodes" {
  description = "List of nodes to deploy with their datacenters and seed status"
  type = list(object({
    datacenter = string
    is_seed    = bool
  }))
  default = [
    { datacenter = "fsn1-dc14", is_seed = true },
    { datacenter = "hel1-dc2", is_seed = true },
    { datacenter = "nbg1-dc3", is_seed = false }
  ]
}

# Control whether ScyllaDB is accessible via public IP
variable "scylla_public_access" {
  description = "Whether ScyllaDB should be accessible via public IP (true) or only private network (false)"
  type        = bool
  default     = false
}

# ScyllaDB CQL port (use 443 or 8443 to bypass network restrictions)
variable "scylla_port" {
  description = "Port for ScyllaDB CQL connections (default 9042, use 443 or 8443 if blocked)"
  type        = number
  default     = 9042
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

# Configure the Hetzner Cloud Provider
provider "hcloud" {
  token = var.hcloud_token
}

resource "hcloud_ssh_key" "default" {
  name       = "my-ssh-key"
  public_key = file("~/.ssh/id_rsa.pub")
}
