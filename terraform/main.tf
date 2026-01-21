terraform {
  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.56"
    }
  }
}

# Configure the Hetzner Cloud Provider
provider "hcloud" {
  token = var.hcloud_token
}

resource "hcloud_ssh_key" "default" {
  name       = "my-ssh-key"
  public_key = file("~/.ssh/id_rsa.pub")
}

# Create nodes for database and backend services
resource "hcloud_server" "node" {
  for_each = var.nodes

  name        = "node-${each.key}"
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
