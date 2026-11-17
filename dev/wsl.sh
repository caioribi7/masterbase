#!/usr/bin/env bash
# Dev no WSL (Ubuntu, root). Uso: dev/wsl.sh reset | run | smoke
# Senha do banco vem de .dev/database_url (gerada no setup, fora do git).
set -euo pipefail
cd "$(dirname "$0")/.."
URL="$(sed 's/:54329/:5432/' .dev/database_url)"
PW="$(sed -E 's#.*://masterbase:([^@]+)@.*#\1#' .dev/database_url)"
service postgresql start >/dev/null

case "${1:-}" in
  reset)
    su postgres -c "psql -q -c 'DROP DATABASE IF EXISTS masterbase'"
    su postgres -c "psql -q -c \"DO \\\$\\\$BEGIN CREATE ROLE masterbase LOGIN PASSWORD '$PW'; EXCEPTION WHEN duplicate_object THEN NULL; END\\\$\\\$\" -c 'CREATE DATABASE masterbase OWNER masterbase'"
    PGPASSWORD="$PW" psql -h 127.0.0.1 -U masterbase -d masterbase -q -v ON_ERROR_STOP=1 -f dev/seed.sql
    echo "banco recriado com dados de exemplo"
    ;;
  run)
    . /root/.cargo/env
    cd server
    DATABASE_URL="$URL" MB_WEB_DIR=../web/dist MB_BACKUP_DIR=../.dev/backups CARGO_TARGET_DIR=/root/mb-target exec cargo run
    ;;
  smoke)
    # Banco e porta próprios: não mexe nos dados de dev.
    su postgres -c "psql -q -c 'DROP DATABASE IF EXISTS masterbase_smoke' -c 'CREATE DATABASE masterbase_smoke OWNER masterbase'"
    PGPASSWORD="$PW" psql -h 127.0.0.1 -U masterbase -d masterbase_smoke -q -v ON_ERROR_STOP=1 -f dev/seed.sql
    . /root/.cargo/env
    (cd server && CARGO_TARGET_DIR=/root/mb-target cargo build -q)
    DATABASE_URL="${URL%/*}/masterbase_smoke" MB_ADDR=127.0.0.1:8081 MB_BACKUP_DIR=/tmp/mb-smoke-backups /root/mb-target/debug/masterbase >/tmp/mb-smoke.log 2>&1 &
    pid=$!
    for _ in $(seq 30); do curl -s -o /dev/null http://127.0.0.1:8081/api/setup && break; sleep 0.5; done
    bash dev/smoke.sh http://127.0.0.1:8081; rc=$?
    kill $pid
    exit $rc
    ;;
  *) echo "uso: dev/wsl.sh reset | run | smoke"; exit 1 ;;
esac
