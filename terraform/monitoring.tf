# ============================================
# ScyllaDB Monitoring Stack
# ============================================

# Create monitoring server
resource "hcloud_server" "monitoring" {
  name        = "scylla-monitoring"
  image       = "ubuntu-24.04"
  server_type = "cx23"
  datacenter  = "nbg1-dc3" # Deploy in same datacenter as first node
  ssh_keys    = [hcloud_ssh_key.default.id]

  user_data = <<-EOF
    #cloud-config
    runcmd:
      - [ apt-get, update ]
      - [ apt-get, install, -y, docker.io, wget ]
      - [ systemctl, enable, docker ]
      - [ systemctl, start, docker ]
  EOF
}

# Attach monitoring server to private network
resource "hcloud_server_network" "monitoring_network_attachment" {
  server_id  = hcloud_server.monitoring.id
  network_id = hcloud_network.node_network.id
}

# Attach firewall to monitoring server
resource "hcloud_firewall_attachment" "monitoring_firewall_attachment" {
  firewall_id = hcloud_firewall.monitoring_firewall.id
  server_ids  = [hcloud_server.monitoring.id]
}

# Generate Prometheus configuration for ScyllaDB nodes
locals {
  prometheus_scylla_config = yamlencode([
    for idx, node in var.nodes : {
      targets = [
        hcloud_server_network.node_network_attachment[tostring(idx)].ip
      ]
      labels = {
        cluster = "uptime-monitor"
        dc      = node.datacenter
      }
    }
  ])

  prometheus_node_exporter_config = yamlencode([
    for idx, node in var.nodes : {
      targets = [
        hcloud_server_network.node_network_attachment[tostring(idx)].ip
      ]
      labels = {
        cluster = "uptime-monitor"
        dc      = node.datacenter
      }
    }
  ])

  # Empty Scylla Manager config (not using manager yet)
  prometheus_manager_config = yamlencode([])

  monitoring_install_script = <<-SCRIPT
    #!/bin/bash
    set -e

    # Download and extract Scylla Monitoring Stack
    cd /root
    if [ ! -d "scylla-monitoring-4.12.2" ]; then
      wget -q https://github.com/scylladb/scylla-monitoring/archive/4.12.2.tar.gz
      tar -xzf 4.12.2.tar.gz
      rm 4.12.2.tar.gz
    fi

    cd scylla-monitoring-4.12.2

    # Write Prometheus configuration files
    cat > prometheus/scylla_servers.yml <<'SCYLLA_CONFIG'
${local.prometheus_scylla_config}
SCYLLA_CONFIG

    cat > prometheus/node_exporter_servers.yml <<'NODE_CONFIG'
${local.prometheus_node_exporter_config}
NODE_CONFIG

    cat > prometheus/scylla_manager_servers.yml <<'MANAGER_CONFIG'
${local.prometheus_manager_config}
MANAGER_CONFIG

    # Create prometheus data directory
    mkdir -p /root/prometheus_data
    chmod -R 777 /root/prometheus_data

    # Stop any existing monitoring stack
    ./kill-all.sh || true

    # Start monitoring stack with persistent data and custom ports
    ./start-all.sh \
      -d /root/prometheus_data \
      -g ${var.grafana_port} \
      -p ${var.prometheus_port} \
      -m ${var.alertmanager_port} \
      --loki-port ${var.loki_port}

    echo "Scylla Monitoring Stack installed and started successfully"
  SCRIPT
}

# Install and configure monitoring stack
resource "null_resource" "deploy_monitoring" {
  triggers = {
    # Trigger on configuration changes
    scylla_config        = md5(local.prometheus_scylla_config)
    node_exporter_config = md5(local.prometheus_node_exporter_config)
    manager_config       = md5(local.prometheus_manager_config)
    install_script       = md5(local.monitoring_install_script)
    # Trigger on server recreation
    server_id = hcloud_server.monitoring.id
  }

  connection {
    type        = "ssh"
    user        = "root"
    host        = hcloud_server.monitoring.ipv4_address
    private_key = file("~/.ssh/id_rsa")
  }

  provisioner "remote-exec" {
    inline = [
      "set -e",
      "echo 'Waiting for cloud-init to complete...'",
      "cloud-init status --wait",
      "echo 'Cloud-init completed!'"
    ]
  }

  provisioner "file" {
    content     = local.monitoring_install_script
    destination = "/tmp/install_monitoring.sh"
  }

  provisioner "remote-exec" {
    inline = [
      "chmod +x /tmp/install_monitoring.sh",
      "/tmp/install_monitoring.sh"
    ]
  }

  depends_on = [
    hcloud_server_network.monitoring_network_attachment,
    hcloud_server_network.node_network_attachment,
    null_resource.deploy_docker_compose
  ]
}
