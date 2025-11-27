resource "hcloud_server" "node" {
  for_each = { for idx, node in var.nodes : idx => node }

  name        = "node-${each.value.datacenter}"
  image       = "ubuntu-24.04"
  server_type = "cx23"
  datacenter  = each.value.datacenter
  ssh_keys    = [hcloud_ssh_key.default.id]

  user_data = <<-EOF
    #cloud-config
    resize_rootfs: false
    write_files:
      - content: |
          # Disable growroot
        path: /etc/growroot-disabled

    runcmd:
      - [ sgdisk, -e, /dev/sda ]
      - [ partprobe ]
      - [ parted, -s, /dev/sda, mkpart, primary, xfs, "8GiB", "100%" ]
      - [ partprobe ]
      - [ mkfs.xfs, /dev/sda2 ]
      - [ growpart, /dev/sda, "1" ]
      - [ resize2fs, /dev/sda1 ]
      - [ mkdir, -p, /data ]
      - [ mount, /dev/sda2, /data ]
      - [ chown, -R, "999:1000", /data ]
      - [ chmod, -R, "755", /data ]
      - [ apt-get, update ]
      - [ apt-get, install, -y, docker.io, docker-compose ]
      - [ systemctl, enable, docker ]
      - [ systemctl, start, docker ]

    mounts:
     - [ /dev/sda2, /data ]
  EOF
}

# Attach servers to private network
resource "hcloud_server_network" "node_network_attachment" {
  for_each = hcloud_server.node

  server_id  = each.value.id
  network_id = hcloud_network.node_network.id
}

# Local to get seed node IPs
locals {
  seed_nodes = [
    for idx, node in var.nodes : hcloud_server_network.node_network_attachment[tostring(idx)].ip
    if node.is_seed
  ]
  seed_list = join(",", local.seed_nodes)

  # Render scylla.yaml for each node
  scylla_configs = {
    for idx, server in hcloud_server.node : idx => templatefile("${path.module}/../database/scylla.yaml.tpl", {
      scylla_port = var.scylla_port
    })
  }

  # Render docker-compose.yml for each node
  docker_compose_configs = {
    for idx, server in hcloud_server.node : idx => templatefile("${path.module}/../database/docker-compose.yml", {
      listen_address        = hcloud_server_network.node_network_attachment[idx].ip
      rpc_address           = var.scylla_public_access ? "0.0.0.0" : hcloud_server_network.node_network_attachment[idx].ip
      broadcast_address     = hcloud_server_network.node_network_attachment[idx].ip
      broadcast_rpc_address = var.scylla_public_access ? server.ipv4_address : hcloud_server_network.node_network_attachment[idx].ip
      seeds                 = local.seed_list
    })
  }
}

resource "null_resource" "deploy_docker_compose" {
  for_each = hcloud_server.node

  triggers = {
    docker_compose_hash = md5(local.docker_compose_configs[each.key])
    config_hash         = md5(local.scylla_configs[each.key])
    server_id           = each.value.id
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
      "mkdir -p /root/scylla"
    ]
  }

  provisioner "file" {
    content     = local.docker_compose_configs[each.key]
    destination = "/root/scylla/docker-compose.yml"
  }

  provisioner "file" {
    content     = local.scylla_configs[each.key]
    destination = "/root/scylla/scylla.yaml"
  }

  provisioner "remote-exec" {
    inline = [
      "set -e",
      "cd /root/scylla",
      "docker-compose down || true",
      "docker-compose up -d"
    ]
  }

  depends_on = [hcloud_server_network.node_network_attachment]
}
