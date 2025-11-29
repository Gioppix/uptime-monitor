# DNS management with Hetzner Cloud DNS

# Create DNS zone for the parent domain
resource "hcloud_zone" "main" {
  name = "giovannifeltrin.com"
  mode = "primary"
  ttl  = 3600
}

locals {
  # Find the primary region server IP
  primary_ip = [
    for idx, server in hcloud_server.node :
    server.ipv4_address if var.nodes[tonumber(idx)].region == var.primary_region
  ][0]
}

# Per-region subdomains (e.g., fsn1.uptime.giovannifeltrin.com)
resource "hcloud_zone_rrset" "uptime_region" {
  for_each = hcloud_server.node

  zone = hcloud_zone.main.name
  name = "${var.nodes[tonumber(each.key)].region}.uptime"
  type = "A"
  ttl  = 300
  records = [
    { value = each.value.ipv4_address }
  ]
}

# Per-region API subdomains (e.g., api.fsn1.uptime.giovannifeltrin.com)
resource "hcloud_zone_rrset" "uptime_api_region" {
  for_each = hcloud_server.node

  zone = hcloud_zone.main.name
  name = "api.${var.nodes[tonumber(each.key)].region}.uptime"
  type = "A"
  ttl  = 300
  records = [
    { value = each.value.ipv4_address }
  ]
}

# Main domain points to primary region
resource "hcloud_zone_rrset" "uptime_main" {
  zone = hcloud_zone.main.name
  name = "uptime"
  type = "A"
  ttl  = 300
  records = [
    { value = local.primary_ip }
  ]
}

# Main API domain points to primary region
resource "hcloud_zone_rrset" "uptime_api_main" {
  zone = hcloud_zone.main.name
  name = "api.uptime"
  type = "A"
  ttl  = 300
  records = [
    { value = local.primary_ip }
  ]
}
