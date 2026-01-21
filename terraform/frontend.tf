# Frontend deployment configuration

# Calculate hash of all git-tracked files in frontend/
data "external" "frontend_source_hash" {
  program = ["bash", "-c", <<-EOF
    cd ${path.module}/..
    HASH=$(git ls-files frontend/ | sort | xargs -I {} md5sum {} | md5sum | cut -d' ' -f1)
    echo "{\"hash\": \"$HASH\"}"
  EOF
  ]
}

locals {
  frontend_image_name = "uptime-frontend:latest"
  frontend_image_tar  = "/tmp/frontend-image.tar"

  # Generate docker-compose config for each node with private IP
  frontend_compose_configs = {
    for idx, node in var.nodes : idx => templatefile("${path.module}/../frontend/docker-compose.yml", {
      frontend_image_name = local.frontend_image_name
      frontend_port       = var.frontend_internal_port
      private_api_url     = "http://${hcloud_server_network.node_network_attachment[idx].ip}:${var.backend_port}"
      backend_url         = var.domain != "" ? "https://${var.api_subdomain}.${idx}.${var.domain}" : "http://${hcloud_server.node[idx].ipv4_address}:${var.backend_port}"
      origin              = var.domain != "" ? "https://${idx}.${var.domain}" : "http://${hcloud_server.node[idx].ipv4_address}"
    })
  }
}

# Build Docker image locally for amd64
resource "null_resource" "build_frontend_image" {
  triggers = {
    frontend_hash = data.external.frontend_source_hash.result.hash
  }

  provisioner "local-exec" {
    command     = "docker build --platform linux/amd64 -t ${local.frontend_image_name} ."
    working_dir = "${path.module}/../frontend"
  }

  provisioner "local-exec" {
    command = "docker save ${local.frontend_image_name} -o ${local.frontend_image_tar}"
  }
}

resource "null_resource" "deploy_frontend" {
  for_each = hcloud_server.node

  triggers = {
    build_id     = null_resource.build_frontend_image.id
    compose_hash = md5(local.frontend_compose_configs[each.key])
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
      "mkdir -p /root/frontend"
    ]
  }

  # Copy pre-built image
  provisioner "file" {
    source      = local.frontend_image_tar
    destination = "/tmp/frontend-image.tar"
  }

  # Load image
  provisioner "remote-exec" {
    inline = [
      "set -e",
      "echo 'Loading pre-built Docker image...'",
      "docker load -i /tmp/frontend-image.tar",
      "rm /tmp/frontend-image.tar"
    ]
  }

  # Copy docker-compose configuration
  provisioner "file" {
    content     = local.frontend_compose_configs[each.key]
    destination = "/root/frontend/docker-compose.yml"
  }

  # Start the frontend service
  provisioner "remote-exec" {
    inline = [
      "set -e",
      "cd /root/frontend",
      "docker-compose down || true",
      "docker-compose up -d"
    ]
  }

  depends_on = [
    hcloud_server_network.node_network_attachment,
    null_resource.deploy_backend,
    null_resource.build_frontend_image
  ]
}
