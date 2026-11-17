#!/usr/bin/env bash
# Dev no WSL (Ubuntu, root). Uso: dev/wsl.sh reset | run
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
    DATABASE_URL="$URL" MB_WEB_DIR=../web/dist CARGO_TARGET_DIR=/root/mb-target exec cargo run
    ;;
  *) echo "uso: dev/wsl.sh reset | run"; exit 1 ;;
esac
