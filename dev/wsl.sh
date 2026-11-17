#!/usr/bin/env bash
# Dev no WSL (Ubuntu, root). Uso: dev/wsl.sh reset | run | smoke
# Senha do banco vem de .dev/database_url (gerada no setup, fora do git).
set -euo pipefail
cd "$(dirname "$0")/.."
URL="$(sed 's/:54329/:5432/' .dev/database_url)"
PW="$(sed -E 's#.*://masterbase:([^@]+)@.*#\1#' .dev/database_url)"
service postgresql start >/dev/null
# Papel do editor SQL (só linhas). Em dev usa a mesma senha do dono.
su postgres -c "psql -q" >/dev/null <<SQL
DO \$\$BEGIN CREATE ROLE masterbase_sql LOGIN PASSWORD '$PW'; EXCEPTION WHEN duplicate_object THEN NULL; END\$\$;
SQL
SQL_URL="$(echo "$URL" | sed 's#://masterbase:#://masterbase_sql:#')"

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
    DATABASE_URL="$URL" MB_SQL_URL="$SQL_URL" MB_WEB_DIR=../web/dist MB_BACKUP_DIR=../.dev/backups CARGO_TARGET_DIR=/root/mb-target exec cargo run
    ;;
  smoke)
    # Banco e porta próprios: não mexe nos dados de dev.
    su postgres -c "psql -q -c 'DROP DATABASE IF EXISTS masterbase_smoke' -c 'CREATE DATABASE masterbase_smoke OWNER masterbase'"
    PGPASSWORD="$PW" psql -h 127.0.0.1 -U masterbase -d masterbase_smoke -q -v ON_ERROR_STOP=1 -f dev/seed.sql
    . /root/.cargo/env
    (cd server && CARGO_TARGET_DIR=/root/mb-target cargo build -q)
    rm -rf /tmp/mb-smoke-backups /tmp/mb-smoke.env; : > /tmp/mb-smoke-mail
    # SMTP falso e um GitHub de mentira: testa e-mail e o começo do login com GitHub.
    python3 dev/smtp-sink.py 2525 /tmp/mb-smoke-mail & sink=$!
    MB_URL=http://127.0.0.1:8081 MB_SMTP_HOST=127.0.0.1 MB_SMTP_PORT=2525 MB_MAIL_FROM=painel@smoke.test MB_GITHUB_ID=smoke-id MB_GITHUB_SECRET=smoke-secret     DATABASE_URL="${URL%/*}/masterbase_smoke" MB_SQL_URL="${SQL_URL%/*}/masterbase_smoke" MB_CONFIG=/tmp/mb-smoke.env MB_ADDR=127.0.0.1:8081 MB_BACKUP_DIR=/tmp/mb-smoke-backups /root/mb-target/debug/masterbase >/tmp/mb-smoke.log 2>&1 &
    pid=$!
    for _ in $(seq 30); do curl -s -o /dev/null http://127.0.0.1:8081/api/setup && break; sleep 0.5; done
    SINK=/tmp/mb-smoke-mail bash dev/smoke.sh http://127.0.0.1:8081; rc=$?
    kill $pid $sink
    exit $rc
    ;;
  *) echo "uso: dev/wsl.sh reset | run | smoke"; exit 1 ;;
esac
