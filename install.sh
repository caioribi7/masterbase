#!/bin/sh
# Instala o Masterbase: Postgres, o programa e a configuração.
#   curl -fsSL https://raw.githubusercontent.com/caioribi7/masterbase/main/install.sh | sh
# Depois é só digitar: masterbase
#
# Funciona em Ubuntu/Debian (apt) e macOS (Homebrew). Testado em Ubuntu 24.04.
# MB_BINARIO=/caminho/masterbase usa um binário local em vez de baixar (para testar).
set -eu

REPO="caioribi7/masterbase"
say() { printf '\033[1m==>\033[0m %s\n' "$1"; }
die() { printf '\033[31mErro:\033[0m %s\n' "$1" >&2; exit 1; }

if [ "$(id -u)" = 0 ]; then
  SUDO=""
else
  command -v sudo >/dev/null || die "precisa de sudo para instalar o Postgres"
  SUDO="sudo"
fi

OS=$(uname -s)
case "$(uname -m)" in
  x86_64|amd64) ARCH=x86_64 ;;
  aarch64|arm64) ARCH=aarch64 ;;
  *) die "processador $(uname -m) ainda não tem versão pronta" ;;
esac
case "$OS" in
  Linux) ASSET="masterbase-linux-$ARCH" ;;
  Darwin) ASSET="masterbase-macos-$ARCH" ;;
  *) die "sistema $OS não suportado" ;;
esac

# Roda SQL como dono do Postgres.
pg_admin() {
  if [ "$OS" = Darwin ]; then
    psql -d postgres -qtA -v ON_ERROR_STOP=1 -c "$1"
  elif [ -n "$SUDO" ]; then
    (cd /tmp && sudo -u postgres psql -qtA -v ON_ERROR_STOP=1 -c "$1")
  else
    (cd /tmp && runuser -u postgres -- psql -qtA -v ON_ERROR_STOP=1 -c "$1")
  fi
}

# 1. Postgres
if [ "$OS" = Darwin ]; then
  command -v brew >/dev/null || die "instale o Homebrew antes: https://brew.sh"
  if ! brew list postgresql@16 >/dev/null 2>&1; then
    say "Instalando Postgres 16"
    brew install postgresql@16
  fi
  brew services start postgresql@16 >/dev/null
  PATH="$(brew --prefix postgresql@16)/bin:$PATH"
elif command -v apt-get >/dev/null; then
  if ! command -v pg_dump >/dev/null || ! [ -d /etc/postgresql ]; then
    say "Instalando Postgres"
    $SUDO apt-get update -qq
    $SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql curl >/dev/null
  fi
  $SUDO systemctl enable --now postgresql >/dev/null 2>&1 || $SUDO service postgresql start >/dev/null
else
  die "instalação automática só em Ubuntu/Debian e macOS. Instale Postgres 16, baixe o programa em https://github.com/$REPO/releases e rode: masterbase configurar"
fi

for _ in 1 2 3 4 5 6 7 8 9 10; do pg_admin "SELECT 1" >/dev/null 2>&1 && break; sleep 1; done
pg_admin "SELECT 1" >/dev/null || die "o Postgres não respondeu"

# 2. Banco e configuração
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/masterbase/masterbase.env"
if [ -f "$CONFIG" ]; then
  say "Configuração já existe em $CONFIG, mantendo"
else
  say "Criando o banco masterbase"
  PASS=$(od -An -tx1 -N24 /dev/urandom | tr -d ' \n')
  if [ "$(pg_admin "SELECT 1 FROM pg_roles WHERE rolname = 'masterbase'")" = 1 ]; then
    pg_admin "ALTER ROLE masterbase LOGIN PASSWORD '$PASS'" >/dev/null
  else
    pg_admin "CREATE ROLE masterbase LOGIN PASSWORD '$PASS'" >/dev/null
  fi
  if [ "$(pg_admin "SELECT 1 FROM pg_database WHERE datname = 'masterbase'")" != 1 ]; then
    pg_admin "CREATE DATABASE masterbase OWNER masterbase" >/dev/null
  fi
  mkdir -p "$(dirname "$CONFIG")"
  (
    umask 077
    cat > "$CONFIG" <<EOF
# Configuração do Masterbase. Edite com \`masterbase configurar\`.
DATABASE_URL=postgresql://masterbase:$PASS@127.0.0.1:5432/masterbase
MB_ADDR=127.0.0.1:8080
MB_BACKUP_DIR=${XDG_DATA_HOME:-$HOME/.local/share}/masterbase/backups
EOF
  )
fi

# Papel do editor SQL: entra no banco, mas só lê e muda linhas (o Masterbase dá as permissões ao subir).
# Sem ele o editor SQL fica desligado. Instalações antigas ganham o papel ao rodar o instalador de novo.
if ! grep -q '^MB_SQL_URL=' "$CONFIG"; then
  DB_URL=$(sed -n 's/^DATABASE_URL=//p' "$CONFIG")
  case "$DB_URL" in
    postgresql://masterbase:*@127.0.0.1:5432/masterbase)
      say "Criando o acesso do editor SQL"
      SQL_PASS=$(od -An -tx1 -N24 /dev/urandom | tr -d ' \n')
      if [ "$(pg_admin "SELECT 1 FROM pg_roles WHERE rolname = 'masterbase_sql'")" = 1 ]; then
        pg_admin "ALTER ROLE masterbase_sql LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD '$SQL_PASS'" >/dev/null
      else
        pg_admin "CREATE ROLE masterbase_sql LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD '$SQL_PASS'" >/dev/null
      fi
      pg_admin "GRANT CONNECT ON DATABASE masterbase TO masterbase_sql" >/dev/null
      (umask 077; echo "MB_SQL_URL=postgresql://masterbase_sql:$SQL_PASS@127.0.0.1:5432/masterbase" >> "$CONFIG"; chmod 600 "$CONFIG")
      ;;
    *) say "Banco próprio: para ligar o editor SQL, veja docs/instalacao.md (MB_SQL_URL)" ;;
  esac
fi

# 3. O programa
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
if [ -n "${MB_BINARIO:-}" ]; then
  cp "$MB_BINARIO" "$TMP/masterbase"
else
  say "Baixando o Masterbase"
  URL="https://github.com/$REPO/releases/latest/download/$ASSET"
  curl -fsSL "$URL" -o "$TMP/masterbase" || die "não deu para baixar $URL"
  curl -fsSL "$URL.sha256" -o "$TMP/sum" || die "não deu para baixar a soma de verificação"
  EXPECTED=$(cut -d' ' -f1 "$TMP/sum")
  if command -v sha256sum >/dev/null; then GOT=$(sha256sum "$TMP/masterbase" | cut -d' ' -f1)
  else GOT=$(shasum -a 256 "$TMP/masterbase" | cut -d' ' -f1); fi
  [ "$EXPECTED" = "$GOT" ] || die "o arquivo baixado não confere com a soma de verificação"
fi
$SUDO mkdir -p /usr/local/bin
$SUDO install -m 755 "$TMP/masterbase" /usr/local/bin/masterbase

echo
say "Pronto. Para abrir, digite:"
echo
echo "    masterbase"
echo
echo "Num servidor, para deixar rodando sozinho:"
echo "    masterbase configurar"
echo "    sudo masterbase servico"
