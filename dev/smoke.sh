#!/usr/bin/env bash
# Teste de ponta a ponta da API. Precisa de banco recém-criado (dev/wsl.sh reset) e servidor rodando.
# Uso: bash dev/smoke.sh [http://127.0.0.1:8080]
set -uo pipefail
BASE="${1:-http://127.0.0.1:8080}/api"
JAR="$(mktemp)"; trap 'rm -f "$JAR"' EXIT
fails=0

# check <status esperado> <descrição> <método> <caminho> [json]
check() {
  local want=$1 what=$2 method=$3 path=$4 body=${5:-}
  local args=(-s -o /tmp/mb-body -w '%{http_code}' -b "$JAR" -c "$JAR" -X "$method")
  [ -n "$body" ] && args+=(-H 'content-type: application/json' -d "$body")
  local got; got=$(curl "${args[@]}" "$BASE$path")
  if [ "$got" = "$want" ]; then echo "ok   $what"; else echo "FALHA $what: esperado $want, veio $got $(cat /tmp/mb-body)"; fails=$((fails+1)); fi
}
has() { grep -q "$1" /tmp/mb-body || { echo "FALHA resposta sem '$1': $(cat /tmp/mb-body)"; fails=$((fails+1)); }; }

PW="smoke-$(date +%s)-senha"
check 200 "cadastro aberto sem conta"        GET  /setup;  has '"signup":true'
check 401 "sem sessão"                        GET  /tables
check 400 "senha curta"                       POST /signup '{"name":"A","email":"a@b.c","password":"123"}'
check 200 "primeira conta"                    POST /signup "{\"name\":\"Dona\",\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
check 200 "cadastro fecha depois"             GET  /setup;  has '"signup":false'
check 403 "segunda conta barrada"             POST /signup "{\"name\":\"Outra\",\"email\":\"outra@smoke.test\",\"password\":\"$PW\"}"
check 200 "me"                                GET  /me;     has 'dona@smoke.test'

check 200 "listar tabelas"                    GET  /tables; has 'clientes'
check 200 "filtro contém"                     GET  '/tables/clientes?col=cidade&val=curi&op=contém'
check 200 "ordenar desc"                      GET  '/tables/clientes?sort=nome&desc=true'
check 400 "ordenar coluna falsa"              GET  '/tables/clientes?sort=x%22y'
check 404 "tabela falsa"                      GET  '/tables/x%22;drop'
check 404 "metadados escondidos"              GET  /tables/users

check 200 "criar tabela"                      POST /schema '{"name":"entregas","columns":[{"name":"cliente","kind":"ligação","link":"clientes","required":true},{"name":"endereço","kind":"texto"},{"name":"feita","kind":"sim/não"}]}'
check 400 "criar tabela repetida"             POST /schema '{"name":"entregas","columns":[]}'
check 400 "tipo inventado"                    POST /schema '{"name":"z","columns":[{"name":"a","kind":"text; drop"}]}'
check 200 "estrutura mostra ligação"          GET  /schema; has '"link":"clientes"'
check 200 "nova linha com ligação"            POST /tables/entregas/rows '{"values":{"cliente":"1","endereço":"Rua A, 10"}}'
check 400 "ligação para cliente inexistente"  POST /tables/entregas/rows '{"values":{"cliente":"99999"}}'
check 400 "obrigatório vazio"                 POST /tables/entregas/rows '{"values":{"endereço":"x"}}'
check 400 "apagar cliente em uso"             DELETE /tables/clientes/rows '{"pks":[1]}'
check 200 "adicionar coluna"                  POST /schema/entregas/columns '{"name":"valor","kind":"decimal"}'
check 400 "coluna repetida"                   POST /schema/entregas/columns '{"name":"valor","kind":"decimal"}'
check 200 "renomear coluna"                   PATCH /schema/entregas/columns/valor '{"name":"frete"}'
check 400 "mexer na chave"                    PATCH /schema/entregas/columns/id '{"name":"x"}'
check 200 "editar célula decimal"             PATCH /tables/entregas/rows '{"pk":1,"column":"frete","value":"12.50"}'; has '12.5'
check 400 "decimal inválido"                  PATCH /tables/entregas/rows '{"pk":1,"column":"frete","value":"doze"}'
check 200 "apagar coluna"                     DELETE /schema/entregas/columns/frete
check 400 "apagar clientes (tem ligação)"     DELETE /schema/clientes '{"confirm":"clientes"}'
check 200 "renomear tabela"                   PATCH /schema/entregas '{"name":"entregas2"}'
check 400 "apagar sem confirmar"              DELETE /schema/entregas2 '{"confirm":"errado"}'
check 200 "apagar linhas"                     DELETE /tables/entregas2/rows '{"pks":[1]}'; has '"deleted":1'
check 200 "apagar tabela"                     DELETE /schema/entregas2 '{"confirm":"entregas2"}'

check 200 "atividade"                         GET  /activity; has 'drop_table'
check 200 "atividade por tabela"              GET  '/activity?table=entregas2'; has 'rename_table'
check 200 "início"                            GET  /overview; has '"version"'

check 200 "sair"                              POST /logout
check 401 "sessão encerrada"                  GET  /me
check 401 "senha errada"                      POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"errada-errada\"}"
check 200 "entrar"                            POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"

echo; [ $fails -eq 0 ] && echo "tudo certo" || { echo "$fails falha(s)"; exit 1; }
