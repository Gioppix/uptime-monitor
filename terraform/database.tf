# Local to get seed node IPs
locals {
  seed_nodes = [
    for idx, node in var.nodes : hcloud_server_network.node_network_attachment[idx].ip
    if node.is_seed
  ]
  seed_list = join(",", local.seed_nodes)

  # Render scylla.yaml for each node
  scylla_configs = {
    for idx, server in hcloud_server.node : idx => templatefile("${path.module}/../database/scylla.yaml", {
      scylla_port             = var.scylla_cql_port
      scylla_ssl_port         = var.scylla_ssl_cql_port
      scylla_shard_aware_port = var.scylla_shard_aware_port
      scylla_shard_aware_ssl  = var.scylla_shard_aware_ssl_port
      datacenter              = var.nodes[idx].datacenter
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

  # Render cassandra-rackdc.properties for each node
  rackdc_configs = {
    for idx, server in hcloud_server.node : idx => templatefile("${path.module}/../database/cassandra-rackdc.properties", {
      datacenter = var.nodes[idx].datacenter
    })
  }
}

resource "null_resource" "deploy_database" {
  for_each = hcloud_server.node

  triggers = {
    docker_compose_hash = md5(local.docker_compose_configs[each.key])
    config_hash         = md5(local.scylla_configs[each.key])
    rackdc_hash         = md5(local.rackdc_configs[each.key])
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

  provisioner "file" {
    content     = local.rackdc_configs[each.key]
    destination = "/root/scylla/cassandra-rackdc.properties"
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
