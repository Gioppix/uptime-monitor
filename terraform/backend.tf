# Backend deployment configuration

# Calculate hash of all git-tracked files in backend/
data "external" "backend_source_hash" {
  program = ["bash", "-c", <<-EOF
    cd ${path.module}/..
    HASH=$(git ls-files backend/ | sort | xargs -I {} md5sum {} | md5sum | cut -d' ' -f1)
    echo "{\"hash\": \"$HASH\"}"
  EOF
  ]
}

locals {
  backend_image_name = "uptime-backend:latest"
  backend_image_tar  = "/tmp/backend-image.tar"

  # CORS allowed origins for each node
  cors_allowed_origins = {
    for idx, node in var.nodes : idx => join(",", concat(
      # Main domain origins (if domain is provided)
      var.domain != "" ? [
        "https://${var.domain}",
        "https://www.${var.domain}",
        "https://${var.api_subdomain}.${var.domain}",
        "http://${var.domain}",
        "http://www.${var.domain}",
        "http://${var.api_subdomain}.${var.domain}"
      ] : [],
      # Regional subdomains (all regions)
      var.domain != "" ? flatten([
        for region_idx, region_node in var.nodes : [
          "https://${region_node.region}.${var.domain}",
          "https://${var.api_subdomain}.${region_node.region}.${var.domain}",
          "http://${region_node.region}.${var.domain}",
          "http://${var.api_subdomain}.${region_node.region}.${var.domain}"
        ]
      ]) : [],
      # IP-based origins (for direct access and development)
      [
        "http://${hcloud_server.node[idx].ipv4_address}:${var.frontend_port}",
        "http://${hcloud_server.node[idx].ipv4_address}",
        "http://${hcloud_server_network.node_network_attachment[idx].ip}:${var.frontend_port}",
        "http://${hcloud_server_network.node_network_attachment[idx].ip}",
        "http://localhost:${var.frontend_port}",
        "http://localhost"
      ]
    ))
  }

  # Render .env file for each node
  backend_env_configs = {
    for idx, node in var.nodes : idx => <<-EOF
SELF_IP="${hcloud_server_network.node_network_attachment[idx].ip}"

DATABASE_NODE_URLS="${join(",", [for node_idx, node in hcloud_server_network.node_network_attachment : "${node.ip}:${var.scylla_cql_port}"])}"
DATABASE_KEYSPACE="default_keyspace"
DATABASE_CONCURRENT_REQUESTS="10"

BACKEND_INTERNAL_PASSWORD="xxxx"
COOKIE_KEY="xxxx"
COOKIE_DOMAIN="${var.domain != "" ? ".${var.domain}" : ""}"

DATABASE_CONNECTIONS="2"

DEV_MODE="false"

SESSION_DURATION_DAYS="7"

FRONTEND_PUBLIC_URL="${local.cors_allowed_origins[idx]}"

HEARTBEAT_INTERVAL_SECONDS="15"

CURRENT_BUCKET_VERSION='1'
CURRENT_BUCKETS_COUNT='20'
REPLICATION_FACTOR='2'

MAX_CONCURRENT_HEALTH_CHECKS="100"

RUST_LOG=warn,backend=info

REGION='${node.region}'
EOF
  }

  # Docker compose config that uses pre-built image
  backend_compose_config = templatefile("${path.module}/../backend/docker-compose.yml", {
    backend_image_name = local.backend_image_name
    backend_port       = var.backend_port
  })
}

# Build Docker image locally for amd64
resource "null_resource" "build_backend_image" {
  triggers = {
    backend_hash = data.external.backend_source_hash.result.hash
  }

  provisioner "local-exec" {
    command     = "docker build --platform linux/amd64 -t ${local.backend_image_name} ."
    working_dir = "${path.module}/../backend"
  }

  provisioner "local-exec" {
    command = "docker save ${local.backend_image_name} -o ${local.backend_image_tar}"
  }
}

resource "null_resource" "deploy_backend" {
  for_each = hcloud_server.node

  triggers = {
    build_id     = null_resource.build_backend_image.id
    compose_hash = md5(local.backend_compose_config)
    env_hash     = md5(local.backend_env_configs[each.key])
    server_id    = each.value.id
  }

  connection {
    type        = "ssh"
    user        = "root"
    host        = each.value.ipv4_address
    private_key = file("~/.ssh/id_rsa")
  }

  provisioner "remote-exec" {
    inline = [
      "set -e",
      "echo 'Waiting for cloud-init to complete...'",
      "cloud-init status --wait",
      "echo 'Cloud-init completed!'",
      "mkdir -p /root/backend"
    ]
  }

  # Copy pre-built image
  provisioner "file" {
    source      = local.backend_image_tar
    destination = "/tmp/backend-image.tar"
  }

  # Load image
  provisioner "remote-exec" {
    inline = [
      "set -e",
      "echo 'Loading pre-built Docker image...'",
      "docker load -i /tmp/backend-image.tar",
      "rm /tmp/backend-image.tar"
    ]
  }

  # Copy configuration files
  provisioner "file" {
    content     = local.backend_env_configs[each.key]
    destination = "/root/backend/.env"
  }

  provisioner "file" {
    content     = local.backend_compose_config
    destination = "/root/backend/docker-compose.yml"
  }

  # Start the backend service
  provisioner "remote-exec" {
    inline = [
      "set -e",
      "cd /root/backend",
      "docker-compose down || true",
      "docker-compose up -d"
    ]
  }

  depends_on = [
    hcloud_server_network.node_network_attachment,
    null_resource.deploy_database,
    null_resource.build_backend_image
  ]
}
