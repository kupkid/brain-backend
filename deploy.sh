#!/usr/bin/env bash
set -euo pipefail

# deploy.sh — Build on GitHub Actions, download, deploy to VPS
# Usage: ./deploy.sh [commit_message]
# Requires: gh CLI (authenticated), systemctl access

BINARY="brain-backend"
SERVICE="brain-backend"
ARTIFACT_NAME="brain-backend-linux-amd64"
DEPLOY_DIR="/tmp/brain-deploy"

echo "==> Pushing changes..."
if [ -n "${1:-}" ]; then
    git add -A && git commit -m "$1" && git push origin main
else
    git push origin main
fi

echo "==> Triggering server build..."
RUN_URL=$(gh workflow run server.yml --json url -q '.url')
echo "    Build: $RUN_URL"

echo "==> Waiting for build to complete (~5 min)..."
gh run watch --exit-status 2>/dev/null || {
    # Fallback: poll every 15s
    while true; do
        STATUS=$(gh run list --workflow=server.yml --limit=1 --json status,conclusion -q '.[0].conclusion')
        if [ "$STATUS" = "success" ]; then break
        elif [ "$STATUS" = "failure" ]; then echo "BUILD FAILED"; exit 1; fi
        echo "    Still building... ($STATUS)"
        sleep 15
    done
}

RUN_ID=$(gh run list --workflow=server.yml --limit=1 --json databaseId -q '.[0].databaseId')
echo "==> Downloading artifact (run $RUN_ID)..."
mkdir -p "$DEPLOY_DIR"
gh run download "$RUN_ID" -n "$ARTIFACT_NAME" -D "$DEPLOY_DIR/"

echo "==> Deploying..."
chmod 755 "$DEPLOY_DIR/$BINARY"
cp "$DEPLOY_DIR/$BINARY" "/root/projects/brain-backend/target/release/$BINARY"

echo "==> Restarting $SERVICE..."
systemctl restart "$SERVICE"
sleep 2

if systemctl is-active --quiet "$SERVICE"; then
    echo "==> OK! $SERVICE is running"
    curl -s http://localhost:3000/health
    echo
else
    echo "==> FAILED! Check: journalctl -u $SERVICE -n 20"
    exit 1
fi

rm -rf "$DEPLOY_DIR"
echo "==> Deploy complete!"
