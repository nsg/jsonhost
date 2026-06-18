# Systemd Deployment

Instructions for deploying jsonhost as a systemd service on Linux. jsonhost is
stateless — it stores nothing locally and forwards everything to a stathost
backend — so deployment is just the binary plus a config file.

## Prerequisites

Download and extract the Linux binary from [releases](https://github.com/nsg/jsonhost/releases):

```bash
tar xzf jsonhost-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 755 jsonhost /usr/local/bin/
```

## Installation

### 1. Create the service user

```bash
sudo useradd -r -s /bin/false jsonhost
```

### 2. Create the config directory

```bash
sudo mkdir -p /etc/jsonhost
```

### 3. Install configuration and service

```bash
sudo cp systemd/jsonhost.toml /etc/jsonhost/
sudo cp systemd/jsonhost.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now jsonhost
```

Edit `/etc/jsonhost/jsonhost.toml` to point `stathost_url` at your stathost
instance.

## Collections and tokens

jsonhost holds no state and no secrets. Collections, tokens, and document
storage all live in stathost — create a bucket there named after each
collection. See the [stathost docs](https://github.com/nsg/stathost) for bucket
and token management.
