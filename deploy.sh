#!/bin/bash
set -euo pipefail

PROJECT_DIR="/opt/pg2bitable"
REPO_URL="https://github.com/garyzheng0714-lang/PostgreSQL-to-larkbase.git"
BRANCH="${1:-main}"

echo "=== Step 1: Clone / Pull repository ==="
if [ -d "$PROJECT_DIR" ]; then
    cd "$PROJECT_DIR"
    if [ -n "$(git status --porcelain)" ]; then
        echo "WARNING: Local changes detected, stashing..."
        git stash
    fi
    git fetch origin
    git checkout "$BRANCH"
    git reset --hard "origin/$BRANCH"
else
    git clone -b "$BRANCH" "$REPO_URL" "$PROJECT_DIR"
    cd "$PROJECT_DIR"
fi

echo "=== Step 2: Build frontend ==="
cd "$PROJECT_DIR/frontend"
export VITE_API_BASE_URL=""
npm ci
npm run build

echo "=== Step 3: Build and start backend ==="
cd "$PROJECT_DIR"
docker compose down --remove-orphans 2>/dev/null || true
docker compose build
docker compose up -d

echo "=== Step 4: Verify ==="
sleep 3
docker compose ps
curl -sf http://127.0.0.1:18082/meta.json | python3 -m json.tool

echo "=== Deploy complete ==="
