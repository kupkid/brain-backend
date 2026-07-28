#!/bin/bash
# Brain Backend Watchdog — monitors server, restarts on crash
# Usage: nohup bash watchdog.sh &

BRAIN_DIR="/root/projects/brain-backend"
LOG="/root/projects/brain-backend/watchdog.log"
CHECK_INTERVAL=10

export PATH="$HOME/.cargo/bin:$PATH"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" >> "$LOG"
}

is_running() {
    pgrep -f "brain-backend" > /dev/null 2>&1 && \
    curl -sf http://localhost:8642/health > /dev/null 2>&1
}

recover_stalled() {
    # Mark crashed runs as failed
    if [ -f "$HOME/.brain/brain.db" ]; then
        sqlite3 "$HOME/.brain/brain.db" \
            "UPDATE runs SET status='failed' WHERE status='running';" 2>/dev/null
        log "Recovered stalled runs"
    fi
}

restart_server() {
    log "Restarting brain-backend..."
    pkill -f "brain-backend" 2>/dev/null
    sleep 2
    
    cd "$BRAIN_DIR" || exit 1
    
    # Set env vars
    export BRAIN_DATA_DIR="$HOME/.brain"
    export BRAIN_VAULT_PASSPHRASE="${BRAIN_VAULT_PASSPHRASE:-test123}"
    export BRAIN_LISTEN_PORT=8642
    export RUST_LOG=brain_backend=info
    
    nohup ./target/release/brain-backend >> /root/projects/brain-backend/server.log 2>&1 &
    SERVER_PID=$!
    log "Server restarted, PID=$SERVER_PID"
    
    # Wait for it to be ready
    for i in $(seq 1 15); do
        sleep 1
        if curl -sf http://localhost:8642/health > /dev/null 2>&1; then
            log "Server ready after ${i}s"
            return 0
        fi
    done
    log "WARNING: Server not ready after 15s"
    return 1
}

log "Watchdog started (PID=$$)"

# Main loop
while true; do
    if ! is_running; then
        log "Server DOWN — attempting recovery"
        recover_stalled
        restart_server
    fi
    sleep "$CHECK_INTERVAL"
done
