# KotobaDB Deployment Guide

This guide provides comprehensive instructions for deploying KotobaDB in various environments, from development to production.

## Table of Contents

- [Quick Start](#quick-start)
- [System Requirements](#system-requirements)
- [Single Node Deployment](#single-node-deployment)
- [Cluster Deployment](#cluster-deployment)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Cloud Deployment](#cloud-deployment)
- [Configuration](#configuration)
- [Monitoring and Observability](#monitoring-and-observability)
- [Backup and Recovery](#backup-and-recovery)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Local Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/your-org/kotoba.git
cd kotoba
cargo build --release

# Run with default settings
./target/release/kotoba --config config/dev.toml
```

### Docker

```bash
# Pull the image
docker pull kotoba/kotoba:latest

# Run with default configuration
docker run -p 8080:8080 -v kotoba-data:/data kotoba/kotoba

# Or with custom configuration
docker run -p 8080:8080 \
  -v kotoba-data:/data \
  -v ./config:/config \
  kotoba/kotoba --config /config/production.toml
```

## System Requirements

### Minimum Requirements

- **CPU**: 2 cores
- **RAM**: 4GB
- **Storage**: 10GB SSD
- **OS**: Linux 4.4+, macOS 10.12+, Windows 10+

### Recommended Production Requirements

- **CPU**: 4+ cores
- **RAM**: 8GB+
- **Storage**: 100GB+ SSD
- **Network**: 1Gbps

### Storage Considerations

KotobaDB supports multiple storage engines:

#### LSM-Tree Engine (Recommended for Production)
- **Pros**: High write throughput, persistent, ACID compliant
- **Cons**: Higher memory usage, compaction overhead
- **Use case**: General purpose, high-write workloads

#### Memory Engine
- **Pros**: Extremely fast, simple
- **Cons**: Not persistent, data lost on restart
- **Use case**: Caching, development, temporary data

### Memory Requirements

```
Base memory: 256MB
Per connection: 2-4MB
Cache: Configurable (default 1GB)
LSM buffers: 256MB
Total recommended: 2-4GB for moderate workloads
```

## Single Node Deployment

### Binary Installation

```bash
# Download the latest release
wget https://github.com/your-org/kotoba/releases/latest/download/kotoba-linux-x64.tar.gz

# Extract
tar xzf kotoba-linux-x64.tar.gz
cd kotoba

# Create configuration
cat > config.toml << EOF
[database]
path = "/var/lib/kotoba/data"
engine = "lsm"

[server]
host = "0.0.0.0"
port = 8080

[cache]
max_size = "1GB"
ttl = "1h"
EOF

# Run
./kotoba --config config.toml
```

### Systemd Service

```bash
# Create systemd service file
sudo tee /etc/systemd/system/kotoba.service > /dev/null <<EOF
[Unit]
Description=KotobaDB Graph Database
After=network.target

[Service]
Type=simple
User=kotoba
Group=kotoba
ExecStart=/usr/local/bin/kotoba --config /etc/kotoba/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

# Create user and directories
sudo useradd -r -s /bin/false kotoba
sudo mkdir -p /var/lib/kotoba /etc/kotoba
sudo chown -R kotoba:kotoba /var/lib/kotoba /etc/kotoba

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable kotoba
sudo systemctl start kotoba

# Check status
sudo systemctl status kotoba
```

### Configuration File

```toml
[database]
# Storage engine: "lsm" or "memory"
engine = "lsm"
# Data directory (for LSM engine)
path = "/var/lib/kotoba/data"

[server]
# Bind address
host = "0.0.0.0"
# Port
port = 8080
# Worker threads
workers = 4

[cache]
# Maximum cache size
max_size = "1GB"
# Cache TTL
ttl = "1h"
# Enable compression
compression = true

[logging]
# Log level: "error", "warn", "info", "debug", "trace"
level = "info"
# Log format: "json" or "text"
format = "text"

[security]
# Enable TLS
tls_enabled = false
# Certificate path (if TLS enabled)
cert_path = "/etc/kotoba/cert.pem"
# Key path (if TLS enabled)
key_path = "/etc/kotoba/key.pem"

[monitoring]
# Enable metrics endpoint
metrics_enabled = true
# Metrics port
metrics_port = 9090

[backup]
# Backup directory
backup_dir = "/var/backups/kotoba"
# Backup schedule (cron format)
schedule = "0 2 * * *"
# Retention period (days)
retention_days = 30
```

## Cluster Deployment

### Cluster Architecture

KotobaDB cluster consists of:
- **Leader nodes**: Handle writes and coordinate cluster
- **Follower nodes**: Handle reads and replicate data
- **Witness nodes**: Participate in consensus without storing data

### Minimum Cluster Setup

```bash
# Node 1 (Leader)
./kotoba --config cluster-node1.toml

# Node 2 (Follower)
./kotoba --config cluster-node2.toml

# Node 3 (Follower)
./kotoba --config cluster-node3.toml
```

### Cluster Configuration

```toml
# cluster-node1.toml
[cluster]
enabled = true
node_id = "node-1"
bind_addr = "10.0.0.1:8080"
advertise_addr = "10.0.0.1:8080"

[cluster.nodes]
node-1 = "10.0.0.1:8080"
node-2 = "10.0.0.2:8080"
node-3 = "10.0.0.3:8080"

[cluster.raft]
# Raft configuration
heartbeat_timeout = "1000ms"
election_timeout = "10000ms"
commit_timeout = "50ms"
max_append_entries = 64
snapshot_interval = "120s"
snapshot_threshold = 8192

# cluster-node2.toml
[cluster]
enabled = true
node_id = "node-2"
bind_addr = "10.0.0.2:8080"
advertise_addr = "10.0.0.2:8080"

[cluster.nodes]
node-1 = "10.0.0.1:8080"
node-2 = "10.0.0.2:8080"
node-3 = "10.0.0.3:8080"
```

### Load Balancing

```nginx
# nginx.conf
upstream kotoba_cluster {
    server 10.0.0.1:8080;
    server 10.0.0.2:8080;
    server 10.0.0.3:8080;
}

server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://kotoba_cluster;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Docker Deployment

### Single Container

```dockerfile
FROM rust:1.70-slim as builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kotoba /usr/local/bin/kotoba

EXPOSE 8080
VOLUME ["/data"]

CMD ["kotoba", "--config", "/data/config.toml"]
```

```bash
# Build and run
docker build -t kotoba .
docker run -p 8080:8080 -v kotoba-data:/data kotoba
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  kotoba:
    image: kotoba/kotoba:latest
    ports:
      - "8080:8080"
    volumes:
      - kotoba-data:/data
    environment:
      - KOTOBA_CONFIG=/data/config.toml
    configs:
      - source: kotoba-config
        target: /data/config.toml
    depends_on:
      - redis

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

volumes:
  kotoba-data:
  redis-data:

configs:
  kotoba-config:
    file: ./config/docker.toml
```

## Kubernetes Deployment

### Basic Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kotoba
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kotoba
  template:
    metadata:
      labels:
        app: kotoba
    spec:
      containers:
      - name: kotoba
        image: kotoba/kotoba:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: data
          mountPath: /data
        env:
        - name: KOTOBA_CONFIG
          value: "/data/config.toml"
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: kotoba-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: kotoba-service
spec:
  selector:
    app: kotoba
  ports:
  - port: 8080
    targetPort: 8080
  type: LoadBalancer
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: kotoba-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 100Gi
```

### StatefulSet for Clustering

```yaml
# statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: kotoba-cluster
spec:
  serviceName: kotoba-cluster
  replicas: 3
  selector:
    matchLabels:
      app: kotoba
  template:
    metadata:
      labels:
        app: kotoba
    spec:
      containers:
      - name: kotoba
        image: kotoba/kotoba:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 8081
          name: raft
        volumeMounts:
        - name: data
          mountPath: /data
        env:
        - name: POD_IP
          valueFrom:
            fieldRef:
              fieldPath: status.podIP
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        command: ["/usr/local/bin/kotoba"]
        args: ["--cluster", "--bind-addr", "$(POD_IP):8080"]
  volumeClaimTemplates:
  - metadata:
    name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 100Gi
```

## Cloud Deployment

### AWS

#### EC2 Instance

```bash
# Install dependencies
sudo yum update -y
sudo yum install -y git gcc openssl-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Build and install
git clone https://github.com/your-org/kotoba.git
cd kotoba
cargo build --release
sudo cp target/release/kotoba /usr/local/bin/

# Configure systemd
sudo tee /etc/systemd/system/kotoba.service > /dev/null <<EOF
[Unit]
Description=KotobaDB
After=network.target

[Service]
Type=simple
User=ec2-user
ExecStart=/usr/local/bin/kotoba --config /home/ec2-user/config.toml
Restart=always

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable kotoba
sudo systemctl start kotoba
```

#### ECS/Fargate

```json
{
  "family": "kotoba-task",
  "taskRoleArn": "arn:aws:iam::123456789012:role/ecsTaskExecutionRole",
  "executionRoleArn": "arn:aws:iam::123456789012:role/ecsTaskExecutionRole",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "256",
  "memory": "512",
  "containerDefinitions": [
    {
      "name": "kotoba",
      "image": "kotoba/kotoba:latest",
      "essential": true,
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "KOTOBA_CONFIG",
          "value": "/app/config/production.toml"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/kotoba",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

### Google Cloud

#### Cloud Run

```yaml
# cloud-run.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: kotoba
spec:
  template:
    spec:
      containers:
      - image: gcr.io/your-project/kotoba:latest
        ports:
        - name: http1
          containerPort: 8080
        env:
        - name: KOTOBA_CONFIG
          value: "/app/config/production.toml"
        resources:
          limits:
            cpu: 1000m
            memory: 2Gi
```

#### GKE (Google Kubernetes Engine)

```yaml
# gke-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kotoba
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kotoba
  template:
    metadata:
      labels:
        app: kotoba
    spec:
      containers:
      - name: kotoba
        image: gcr.io/your-project/kotoba:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
      volumes:
      - name: data
        gcePersistentDisk:
          pdName: kotoba-disk
          fsType: ext4
```

### Azure

#### AKS (Azure Kubernetes Service)

```yaml
# aks-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kotoba
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kotoba
  template:
    metadata:
      labels:
        app: kotoba
    spec:
      containers:
      - name: kotoba
        image: kotoba.azurecr.io/kotoba:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
      volumes:
      - name: data
        azureDisk:
          kind: Managed
          diskName: kotoba-disk
          diskURI: /subscriptions/.../disks/kotoba-disk
---
apiVersion: v1
kind: Service
metadata:
  name: kotoba-service
spec:
  selector:
    app: kotoba
  ports:
  - port: 8080
    targetPort: 8080
  type: LoadBalancer
```

## Configuration

### Environment Variables

```bash
# Database
export KOTOBA_DATABASE_ENGINE=lsm
export KOTOBA_DATABASE_PATH=/var/lib/kotoba

# Server
export KOTOBA_SERVER_HOST=0.0.0.0
export KOTOBA_SERVER_PORT=8080
export KOTOBA_SERVER_WORKERS=4

# Cache
export KOTOBA_CACHE_MAX_SIZE=1GB
export KOTOBA_CACHE_TTL=1h

# Logging
export KOTOBA_LOG_LEVEL=info
export RUST_LOG=kotoba=info

# Security
export KOTOBA_TLS_CERT=/path/to/cert.pem
export KOTOBA_TLS_KEY=/path/to/key.pem

# Monitoring
export KOTOBA_METRICS_ENABLED=true
export KOTOBA_METRICS_PORT=9090
```

### Advanced Configuration

```toml
[database]
engine = "lsm"
path = "/var/lib/kotoba/data"
max_file_size = "2GB"
compression = "snappy"
sync_writes = true

[lsm]
# LSM-specific settings
write_buffer_size = "64MB"
max_write_buffer_number = 3
min_write_buffer_number_to_merge = 1
compression_type = "snappy"
bloom_filter_bits_per_key = 10

[cache]
max_size = "2GB"
ttl = "2h"
compression = true
eviction_policy = "lru"

[cluster]
enabled = true
node_id = "node-1"
bind_addr = "0.0.0.0:8080"
advertise_addr = "10.0.0.1:8080"
seed_nodes = ["10.0.0.1:8080", "10.0.0.2:8080"]

[cluster.raft]
heartbeat_timeout = "1000ms"
election_timeout = "10000ms"
commit_timeout = "50ms"
snapshot_interval = "120s"
snapshot_threshold = 8192

[monitoring]
enabled = true
metrics_port = 9090
health_check_interval = "30s"

[security]
tls_enabled = true
cert_path = "/etc/kotoba/cert.pem"
key_path = "/etc/kotoba/key.pem"
client_auth = "optional"
ca_cert_path = "/etc/kotoba/ca.pem"

[backup]
enabled = true
backup_dir = "/var/backups/kotoba"
schedule = "0 2 * * *"
retention_days = 30
compression = true
encryption_key = "your-encryption-key"

[logging]
level = "info"
format = "json"
file_path = "/var/log/kotoba/kotoba.log"
max_file_size = "100MB"
max_files = 5
```

## Monitoring and Observability

### Metrics

KotobaDB exposes Prometheus metrics at `/metrics`:

```prometheus
# Database metrics
kotoba_db_nodes_total{type="User"} 1000
kotoba_db_edges_total{type="follows"} 2500
kotoba_db_operations_total{type="read"} 50000

# Performance metrics
kotoba_db_query_duration_seconds{quantile="0.5"} 0.001
kotoba_db_query_duration_seconds{quantile="0.95"} 0.005
kotoba_db_query_duration_seconds{quantile="0.99"} 0.01

# System metrics
kotoba_memory_bytes 1073741824
kotoba_cpu_usage_percent 45.2
```

### Health Checks

```bash
# Health endpoint
curl http://localhost:8080/health

# Readiness endpoint
curl http://localhost:8080/ready

# Metrics endpoint
curl http://localhost:8080/metrics
```

### Logging

```json
{
  "timestamp": "2024-01-01T12:00:00Z",
  "level": "INFO",
  "target": "kotoba::db",
  "message": "Node created",
  "node_id": 123,
  "operation": "create_node",
  "duration_ms": 1.5
}
```

### Monitoring Dashboard

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'kotoba'
    static_configs:
      - targets: ['localhost:9090']
```

## Backup and Recovery

### Automated Backups

```bash
# Configure backup
cat > backup-config.toml << EOF
[backup]
enabled = true
backup_dir = "/var/backups/kotoba"
schedule = "0 2 * * *"  # Daily at 2 AM
retention_days = 30
compression = true
encryption = true
EOF

# Manual backup
kotoba --backup /path/to/backup.tar.gz

# List backups
kotoba --list-backups

# Restore from backup
kotoba --restore /path/to/backup.tar.gz
```

### Point-in-Time Recovery

```bash
# Restore to specific timestamp
kotoba --restore --timestamp "2024-01-01T12:00:00Z"

# Restore to specific commit
kotoba --restore --commit abc123def456
```

### Disaster Recovery

```bash
# Full cluster backup
kotoba-cluster --backup-cluster /path/to/cluster-backup

# Restore cluster
kotoba-cluster --restore-cluster /path/to/cluster-backup
```

## Performance Tuning

### Memory Tuning

```toml
[cache]
max_size = "4GB"          # Increase for large datasets
ttl = "4h"               # Longer TTL for hot data
compression = true       # Enable compression

[lsm]
write_buffer_size = "128MB"    # Larger buffers for high write throughput
max_write_buffer_number = 4   # More buffers for concurrent writes
```

### CPU Tuning

```toml
[server]
workers = 8             # Match CPU cores
max_connections = 1000  # Connection pool size

[database]
max_concurrent_operations = 100  # Limit concurrent operations
```

### Storage Tuning

```toml
[lsm]
compression_type = "lz4"           # Faster compression
bloom_filter_bits_per_key = 10    # Bloom filter tuning
max_file_size = "2GB"              # Larger SST files

[storage]
sync_writes = false                # Disable for better performance
direct_io = true                   # Enable direct I/O
```

### Network Tuning

```toml
[server]
tcp_keepalive = "60s"
tcp_timeout = "30s"
max_request_size = "10MB"

[cluster]
heartbeat_timeout = "500ms"        # Faster failure detection
max_append_entries = 128           # Larger batch sizes
```

## Troubleshooting

### Common Issues

#### High Memory Usage

```bash
# Check memory usage
ps aux | grep kotoba

# Check cache statistics
curl http://localhost:9090/metrics | grep cache

# Reduce cache size
echo "cache.max_size = '512MB'" >> config.toml
systemctl restart kotoba
```

#### Slow Queries

```bash
# Enable query profiling
curl -X POST http://localhost:8080/debug/profile/start

# Run slow query
# Check logs for profiling information

# Check indexes
curl http://localhost:8080/debug/indexes

# Add missing indexes
curl -X POST http://localhost:8080/indexes \
  -H "Content-Type: application/json" \
  -d '{"name": "user_email_idx", "target": {"node_property": {"node_type": "User", "property": "email"}}, "type": "btree"}'
```

#### Connection Issues

```bash
# Check connectivity
telnet localhost 8080

# Check logs
journalctl -u kotoba -f

# Check configuration
kotoba --validate-config config.toml
```

#### Cluster Issues

```bash
# Check cluster status
curl http://localhost:8080/cluster/status

# Check node connectivity
curl http://localhost:8080/cluster/nodes

# Force leader election
curl -X POST http://localhost:8080/cluster/reelect
```

### Performance Troubleshooting

```bash
# Enable detailed metrics
curl -X POST http://localhost:9090/debug/metrics/enable

# Collect system statistics
curl http://localhost:9090/debug/system

# Generate flame graph
curl http://localhost:9090/debug/profile/cpu > flame.svg
```

### Log Analysis

```bash
# Search for errors
grep "ERROR" /var/log/kotoba/kotoba.log | tail -10

# Find slow operations
grep "duration_ms.*[0-9]\{4,\}" /var/log/kotoba/kotoba.log

# Analyze query patterns
grep "MATCH" /var/log/kotoba/kotoba.log | head -20
```

This comprehensive deployment guide covers everything from basic installation to advanced production deployments. For specific use cases or additional configuration options, refer to the API documentation or community forums.
