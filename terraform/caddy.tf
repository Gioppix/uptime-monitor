# Caddy reverse proxy configuration for SSL/HTTPS

locals {
  caddy_config = {
    for idx, node in var.nodes : idx => templatefile("${path.module}/../infra/Caddyfile", {
      domain        = var.domain
      api_subdomain = var.api_subdomain
      backend_port  = var.backend_port
      frontend_port = var.frontend_internal_port
      region        = var.nodes[tonumber(idx)].region
      is_primary    = var.nodes[tonumber(idx)].region == var.primary_region
    })
  }
}

# Install and configure Caddy on each node
resource "null_resource" "setup_caddy" {
  for_each = hcloud_server.node

  triggers = {
    caddy_config_hash = md5(local.caddy_config[each.key])
    server_id         = each.value.id
  }

  connection {
    host        = each.value.ipv4_address
    user        = "root"
    private_key = file("~/.ssh/id_rsa")
  }

  # Install Caddy
  provisioner "remote-exec" {
    inline = [
      "export DEBIAN_FRONTEND=noninteractive",
      "apt-get update",
      "apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl",
      "curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg",
      "curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' > /etc/apt/sources.list.d/caddy-stable.list",
      "apt-get update",
      "apt-get install -y caddy",
    ]
  }

  # Upload Caddyfile
  provisioner "file" {
    content     = local.caddy_config[each.key]
    destination = "/etc/caddy/Caddyfile"
  }

  # Restart Caddy
  provisioner "remote-exec" {
    inline = [
      "systemctl enable caddy",
      "systemctl restart caddy"
    ]
  }

  depends_on = [
    null_resource.deploy_backend,
    null_resource.deploy_frontend
  ]
}
