#!/usr/bin/env bash
# push.sh — fast add/commit/push with secret scanning
# Usage: ./push.sh "commit message"
#        ./push.sh              (auto-generates message from changed files)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

cd "$(git rev-parse --show-toplevel)"

# 1. Check for secrets before staging
echo "🔍 Pre-check: scanning for secrets..."
PATTERNS=(
    'sk-[a-zA-Z0-9]{20,}'
    'AIza[0-9A-Za-z_-]{35}'
    'ghp_[a-zA-Z0-9]{36}'
    'AKIA[0-9A-Z]{16}'
    'nvapi-[a-zA-Z0-9]{20,}'
    'sk-or-v1-[a-zA-Z0-9]{20,}'
    'sk-ckUQ[a-zA-Z0-9]{20,}'
    'BRAIN_VAULT_PASSPHRASE\s*='
    'BRAIN_API_KEY\s*='
    'COHERE_API_KEY\s*='
    'password\s*[:=]\s*["\x27][^\s"]{8,}'
    'secret\s*[:=]\s*["\x27][^\s"]{8,}'
)

SECRET_FOUND=0
for pattern in "${PATTERNS[@]}"; do
    MATCHES=$(git diff --unified=0 2>/dev/null | grep -InE "$pattern" | grep "^+" | grep -v "^+++" || true)
    if [ -n "$MATCHES" ]; then
        echo -e "${RED}⚠ SECRET IN DIFF: $pattern${NC}"
        echo "$MATCHES" | head -3
        SECRET_FOUND=1
    fi
done

if [ "$SECRET_FOUND" -eq 1 ]; then
    echo -e "${RED}❌ COMMIT BLOCKED: Secrets detected! Remove them first.${NC}"
    exit 1
fi

# 2. Stage all
git add -A

# 3. Check if there's anything to commit
if git diff --cached --quiet 2>/dev/null; then
    echo -e "${YELLOW}Nothing to commit${NC}"
    exit 0
fi

# 4. Commit message
if [ $# -gt 0 ]; then
    MSG="$*"
else
    # Auto-generate from changed files
    FILES=$(git diff --cached --name-only)
    COUNT=$(echo "$FILES" | wc -l)
    FIRST=$(echo "$FILES" | head -1 | sed 's|.*/||')
    if [ "$COUNT" -eq 1 ]; then
        MSG="chore: update $FIRST"
    else
        MSG="chore: update $COUNT files"
    fi
fi

# 5. Commit
git commit -m "$MSG"
echo -e "${GREEN}✓ Committed: $MSG${NC}"

# 6. Push (pre-push hook will also scan)
if git push 2>&1; then
    echo -e "${GREEN}✓ Pushed to origin${NC}"
else
    echo -e "${RED}❌ Push failed${NC}"
    exit 1
fi
