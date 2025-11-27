output "nodes" {
  value = {
    for key, server in hcloud_server.node : key => {
      name       = server.name
      datacenter = server.datacenter
      public_ip  = server.ipv4_address
      private_ip = hcloud_server_network.node_network_attachment[key].ip
      is_seed    = var.nodes[tonumber(key)].is_seed
    }
  }
  description = "Information about all deployed nodes"
}

output "seed_nodes" {
  value       = local.seed_list
  description = "Comma-separated list of seed node IPs"
}

output "monitoring" {
  value = {
    name             = hcloud_server.monitoring.name
    public_ip        = hcloud_server.monitoring.ipv4_address
    private_ip       = hcloud_server_network.monitoring_network_attachment.ip
    grafana_url      = "http://${hcloud_server.monitoring.ipv4_address}:${var.grafana_port}"
    prometheus_url   = "http://${hcloud_server.monitoring.ipv4_address}:${var.prometheus_port}"
    alertmanager_url = "http://${hcloud_server.monitoring.ipv4_address}:${var.alertmanager_port}"
    loki_url         = "http://${hcloud_server.monitoring.ipv4_address}:${var.loki_port}"
  }
  description = "Monitoring server information"
}
