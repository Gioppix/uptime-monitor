# Create private network for node communication
resource "hcloud_network" "node_network" {
  name     = "node-network"
  ip_range = "10.0.0.0/16"
}

# Create network subnet
resource "hcloud_network_subnet" "node_subnet" {
  network_id   = hcloud_network.node_network.id
  type         = "cloud"
  network_zone = "eu-central"
  ip_range     = "10.0.1.0/24"
}

# Create firewall to expose SSH and database port
resource "hcloud_firewall" "database_firewall" {
  name = "database-firewall"

  # SSH access
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "22"
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # ScyllaDB CQL port (database connections)
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.scylla_cql_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # ScyllaDB SSL CQL port
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.scylla_ssl_cql_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # ScyllaDB shard-aware native port
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.scylla_shard_aware_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # ScyllaDB shard-aware SSL port
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.scylla_shard_aware_ssl_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # Backend API port
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.backend_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }
}

# Create firewall for monitoring server
resource "hcloud_firewall" "monitoring_firewall" {
  name = "monitoring-firewall"

  # SSH access
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "22"
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # Grafana
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.grafana_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # Prometheus
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.prometheus_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # Alertmanager
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.alertmanager_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # Loki
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = var.loki_port
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }
}

# Attach firewall to servers
resource "hcloud_firewall_attachment" "database_firewall_attachment" {
  firewall_id = hcloud_firewall.database_firewall.id
  server_ids  = [for server in hcloud_server.node : server.id]
}
