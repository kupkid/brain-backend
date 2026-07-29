# Brain Backend — Deployment Guide

## Requirements

- **OS**: Linux (Debian/Ubuntu recommended)
- **RAM**: 512MB minimum, 1GB recommended
- **Disk**: 1GB for binary + database
- **Network**: Port 8642 (HTTP), 443 (HTTPS via reverse proxy)

## Quick Deploy

### 1. Build Release Binary

```bash
# On build machine
cargo build --release
# Binary: target/release/brain-backend (3.8MB)
```

### 2. Upload to Server

```bash
scp target/release/brain-backend user@server:/opt/brain-backend/
```

### 3. Create Systemd Service

```bash
sudo tee /etc/systemd/system/brain-backend.service <<EOF
[Unit]
Description=Brain Backend Agent Runtime
After=network.target

[Service]
Type=simple
User=brain
Group=brain
WorkingDirectory=/opt/brain-backend
ExecStart=/opt/brain-backend/brain-backend
Restart=on-failure
RestartSec=5
Environment=BRAIN_VAULT_PASSPHRASE=your-secret-here
Environment=BRAIN_DATA_DIR=/var/lib/brain-backend
Environment=BRAIN_LISTEN_ADDR=127.0.0.1
Environment=BRAIN_LISTEN_PORT=8642
Environment=RUST_LOG=info

# Security
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/brain-backend
PrivateTmp=true

# Resource limits
MemoryMax=256M
CPUQuota=50%

[Install]
WantedBy=multi-user.target
EOF
```

### 4. Setup Data Directory

```bash
sudo mkdir -p /var/lib/brain-backend
sudo chown brain:brain /var/lib/brain-backend
```

### 5. Start Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable brain-backend
sudo systemctl start brain-backend
```

## Reverse Proxy (nginx)

```nginx
server {
    listen 443 ssl http2;
    server_name brain.example.com;

    ssl_certificate /etc/letsencrypt/live/brain.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/brain.example.com/privkey.pem;

    # WebSocket upgrade
    location /v1/runs/ {
        proxy_pass http://127.0.0.1:8642;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
    }

    # API endpoints
    location /v1/ {
        proxy_pass http://127.0.0.1:8642;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Health check
    location /health {
        proxy_pass http://127.0.0.1:8642;
    }
}
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.77 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/brain-backend /usr/local/bin/
RUN useradd -m -s /bin/bash brain
USER brain
WORKDIR /home/brain
EXPOSE 8642
ENTRYPOINT ["brain-backend"]
```

### docker-compose.yml

```yaml
version: '3.8'
services:
  brain-backend:
    build: .
    ports:
      - "8642:8642"
    environment:
      BRAIN_VAULT_PASSPHRASE: ${VAULT_PASSPHRASE}
      BRAIN_DATA_DIR: /data
      RUST_LOG: info
    volumes:
      - brain-data:/data
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 256M
          cpus: '0.5'

volumes:
  brain-data:
```

## Monitoring

### Health Check

```bash
curl http://localhost:8642/v1/health
```

### Logs

```bash
# Systemd logs
journalctl -u brain-backend -f

# Docker logs
docker logs -f brain-backend
```

### Metrics (Future)

- Request count by endpoint
- Response time p50/p95/p99
- Active runs count
- Memory usage
- SQLite WAL size

## Backup

### Database Backup

```bash
# Stop service
sudo systemctl stop brain-backend

# Backup database
cp /var/lib/brain-backend/brain.db /backup/brain-$(date +%Y%m%d).db

# Start service
sudo systemctl start brain-backend
```

### Automated Backup (cron)

```bash
# Add to crontab
0 2 * * * /usr/local/bin/backup-brain.sh
```

```bash
#!/bin/bash
# backup-brain.sh
BACKUP_DIR="/backup/brain"
DATA_DIR="/var/lib/brain-backend"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup database
sqlite3 $DATA_DIR/brain.db ".backup '$BACKUP_DIR/brain_$DATE.db'"

# Keep only last 7 days
find $BACKUP_DIR -name "brain_*.db" -mtime +7 -delete
```

## Performance Tuning

### SQLite WAL Mode

Already configured in DDL:
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
```

### Memory Limits

- Default: 256MB systemd limit
- Binary idle: ~5MB RSS
- Per-run: ~10MB (context + LLM response)
- 10 concurrent runs: ~100MB total

### Disk Usage

- Binary: 3.8MB
- Database (empty): 1MB
- Database (10K memories): ~50MB
- Database (100K memories): ~500MB
- vec0 index: ~4x memory size

## Security Checklist

- [ ] Run as non-root user
- [ ] Enable TLS (reverse proxy)
- [ ] Set strong vault passphrase
- [ ] Restrict firewall (only 443 open)
- [ ] Enable automatic updates
- [ ] Monitor logs for anomalies
- [ ] Regular backups
- [ ] Limit resource usage (memory, CPU)

## Troubleshooting

### Service Won't Start

```bash
# Check logs
journalctl -u brain-backend -n 50

# Check if port is in use
lsof -i :8642

# Check permissions
ls -la /var/lib/brain-backend/
```

### Database Locked

```bash
# Check WAL mode
sqlite3 /var/lib/brain-backend/brain.db "PRAGMA journal_mode;"

# Check busy timeout
sqlite3 /var/lib/brain-backend/brain.db "PRAGMA busy_timeout;"
```

### High Memory Usage

```bash
# Check process RSS
ps aux | grep brain-backend

# Check SQLite page count
sqlite3 /var/lib/brain-backend/brain.db "PRAGMA page_count;"
```
