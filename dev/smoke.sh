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

# ---- papéis, pessoas e chaves ----
check 200 "me traz papel e projeto"           GET  /me; has '"role":"admin"'
check 200 "criar editor"                      POST /users "{\"name\":\"Edu\",\"email\":\"edu@smoke.test\",\"password\":\"$PW\",\"role\":\"editor\"}"
check 200 "criar leitor"                      POST /users "{\"name\":\"Lia\",\"email\":\"lia@smoke.test\",\"password\":\"$PW\",\"role\":\"leitor\"}"
check 409 "e-mail repetido"                   POST /users "{\"name\":\"Lia\",\"email\":\"lia@smoke.test\",\"password\":\"$PW\",\"role\":\"leitor\"}"
check 400 "papel inventado"                   POST /users "{\"name\":\"X\",\"email\":\"x@smoke.test\",\"password\":\"$PW\",\"role\":\"root\"}"
check 400 "não se rebaixa sozinho"            PATCH /users/1 '{"role":"leitor"}'
check 200 "listar pessoas"                    GET  /users; has 'lia@smoke.test'
check 200 "criar chave leitora"               POST /keys '{"name":"site","role":"leitor"}'; has '"token":"mb_'
KEY=$(grep -o 'mb_[0-9a-f]*' /tmp/mb-body)
check 400 "chave não pode ser admin"          POST /keys '{"name":"x","role":"admin"}'
check 200 "lista não mostra a chave inteira"  GET  /keys; ! grep -q "$KEY" /tmp/mb-body || { echo "FALHA chave vazou na lista"; fails=$((fails+1)); }

# com chave (sem cookie)
key() { # key <status> <descrição> <método> <caminho> [json]
  local args=(-s -o /tmp/mb-body -w '%{http_code}' -H "Authorization: Bearer $KEY" -X "$3")
  [ -n "${5:-}" ] && args+=(-H 'content-type: application/json' -d "$5")
  local got; got=$(curl "${args[@]}" "$BASE$4")
  if [ "$got" = "$1" ]; then echo "ok   $2"; else echo "FALHA $2: esperado $1, veio $got $(cat /tmp/mb-body)"; fails=$((fails+1)); fi
}
key 200 "chave lê tabela"                     GET  /tables/clientes
key 403 "chave leitora não escreve"           POST /tables/clientes/rows '{"values":{"nome":"x"}}'
key 403 "chave não mexe em estrutura"         POST /schema '{"name":"hack","columns":[]}'
key 403 "chave não lista pessoas"             GET  /users
key 403 "chave não faz backup"                POST /backups
key 403 "chave não vê a chave de backup"      POST /backups/key '{"password":"x"}'
key 403 "chave não muda o backup automático"  PATCH /backups/settings '{"enabled":false}'
key 403 "chave não roda SQL"                   POST /sql '{"sql":"SELECT 1"}'
KEY="mb_000000000000000000000000000000000000000000000000"
key 401 "chave inventada"                     GET  /tables

# ---- consultas ----
Q='{"table":"clientes","columns":["nome","cidade"],"filters":[{"col":"plano","op":"é","val":"pro"},{"col":"id","op":"menor que","val":"6"}],"sort":"nome","desc":false,"limit":50}'
check 200 "rodar consulta"                    POST /query "$Q"; has '"sql":"SELECT'
check 400 "consulta com operador falso"       POST /query '{"table":"clientes","filters":[{"col":"id","op":"; drop","val":"1"}]}'
check 400 "consulta com valor de tipo errado" POST /query '{"table":"clientes","filters":[{"col":"id","op":"maior que","val":"abc"}]}'
check 404 "consulta em metadados"             POST /query '{"table":"users"}'
check 200 "salvar consulta"                   POST /queries "{\"name\":\"pro\",\"spec\":$Q}"
check 200 "listar consultas"                  GET  /queries; has '"name":"pro"'
check 200 "apagar consulta"                   DELETE /queries/1

# ---- importar ----
check 200 "importar 2 linhas"                 POST /tables/fornecedores/import '{"columns":["nome"],"rows":[["Moinho Sul"],["Laticínios Serra"]]}'; has '"imported":2'
check 400 "importar coluna que não existe"    POST /tables/fornecedores/import '{"columns":["xyz"],"rows":[["a"]]}'

# ---- backups ----
check 200 "configuração do backup"            GET  /backups/settings; has '"enabled":true'; has '"encrypted":true'; has '"key_saved":false'
check 200 "criar backup"                      POST /backups; has 'backup-'
BK=$(grep -o 'backup-[0-9-]*[.]dump[.]age' /tmp/mb-body | head -1)
[ -n "$BK" ] || { echo "FALHA backup saiu sem criptografia: $(cat /tmp/mb-body)"; fails=$((fails+1)); }
check 200 "listar backups"                    GET  /backups; has "$BK"
check 200 "baixar backup"                     GET  "/backups/$BK"
head -c 21 /tmp/mb-body | grep -q 'age-encryption.org/v1' || { echo "FALHA backup baixado não está criptografado"; fails=$((fails+1)); }
! grep -qa 'clientes' /tmp/mb-body || { echo "FALHA nome de tabela legível no backup"; fails=$((fails+1)); }
check 404 "baixar caminho malicioso"          GET  '/backups/..%2F..%2Fetc%2Fpasswd'
check 200 "apagar linhas depois do backup"    DELETE /tables/produtos/rows '{"pks":[1,2]}'
check 400 "restaurar sem confirmar"           POST "/backups/$BK/restore" '{"confirm":"x"}'
check 200 "restaurar"                         POST "/backups/$BK/restore" '{"confirm":"restaurar"}'
check 200 "linhas voltaram"                   GET  '/tables/produtos?col=id&val=1'; has '"total":1'
check 200 "guardou o estado antes"            GET  /backups; has '"kind":"antes"'
check 400 "chave com senha errada"            POST /backups/key '{"password":"errada"}'
check 200 "ver chave com a senha"             POST /backups/key "{\"password\":\"$PW\"}"; has 'AGE-SECRET-KEY-1'
check 200 "mudar horário e desligar"          PATCH /backups/settings '{"minute":90,"enabled":false,"key_saved":true}'; has '"minute":90'; has '"key_saved":true'; has '"next":null'
check 400 "horário inválido"                  PATCH /backups/settings '{"minute":1440}'
check 200 "religar"                           PATCH /backups/settings '{"enabled":true}'; has '"enabled":true'
check 200 "apagar backup"                     DELETE "/backups/$BK"

# ---- editor SQL e configurações ----
check 200 "SQL lê"                            POST /sql '{"sql":"SELECT id, nome FROM clientes ORDER BY id LIMIT 2","read_only":true}'; has '"columns":\["id","nome"\]'
check 200 "SQL vários comandos"               POST /sql '{"sql":"SELECT 1; SELECT 2 AS dois","read_only":true}'; has '"statements":2'
check 400 "SQL só leitura barra UPDATE"       POST /sql '{"sql":"UPDATE clientes SET cidade = cidade","read_only":true}'; has 'leitura'
check 400 "SQL com erro mostra o Postgres"    POST /sql '{"sql":"SELEC 1","read_only":true}'; has 'SELEC'
check 200 "SQL escreve"                       POST /sql '{"sql":"UPDATE clientes SET cidade = cidade WHERE id = 1"}'; has '"affected":1'
check 200 "SQL registra na atividade"         GET  /activity; has '"action":"sql"'
check 200 "configurações"                     GET  /settings; has '"version"'
! grep -q 'postgres://\|postgresql://' /tmp/mb-body || { echo "FALHA configurações mostram a URL do banco"; fails=$((fails+1)); }
check 200 "mudar meu nome"                    PATCH /me '{"name":"Dona Smoke"}'
check 200 "nome mudou"                        GET  /me; has 'Dona Smoke'
check 400 "nome vazio"                        PATCH /me '{"name":"  "}'

# ---- papéis na prática ----
check 200 "sair (admin)"                      POST /logout
check 200 "entrar como leitor"                POST /login "{\"email\":\"lia@smoke.test\",\"password\":\"$PW\"}"
check 200 "leitor lê"                         GET  /tables/clientes
check 403 "leitor não edita"                  PATCH /tables/clientes/rows '{"pk":1,"column":"cidade","value":"X"}'
check 403 "leitor não apaga"                  DELETE /tables/clientes/rows '{"pks":[2]}'
check 403 "leitor não cria tabela"            POST /schema '{"name":"z","columns":[]}'
check 403 "leitor não vê pessoas"             GET  /users
check 403 "leitor não roda SQL"               POST /sql '{"sql":"SELECT 1","read_only":true}'
check 403 "leitor não vê configurações"       GET  /settings
check 200 "leitor troca a própria ! grep -q 'postgres://|postgresql://' /tmp/mb-body"      POST /password "{\"current\":\"$PW\",\"new\":\"${PW}2\"}"
check 200 "sair (leitor)"                     POST /logout
check 200 "entrar como editor"                POST /login "{\"email\":\"edu@smoke.test\",\"password\":\"$PW\"}"
check 200 "editor edita"                      PATCH /tables/clientes/rows '{"pk":1,"column":"cidade","value":"Curitiba"}'
check 403 "editor não apaga tabela"           DELETE /schema/produtos '{"confirm":"produtos"}'
check 403 "editor não cria chave"             POST /keys '{"name":"x","role":"leitor"}'
check 403 "editor não roda SQL"               POST /sql '{"sql":"SELECT 1","read_only":true}'
check 200 "sair (editor)"                     POST /logout
check 200 "entrar como admin"                 POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
check 200 "desativar editor"                  PATCH /users/2 '{"active":false}'
check 200 "sair (admin) de novo"              POST /logout
check 401 "desativado não entra"              POST /login "{\"email\":\"edu@smoke.test\",\"password\":\"$PW\"}"

check 200 "entrar de novo como admin" POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
check 200 "sair"                              POST /logout
check 401 "sessão encerrada"                  GET  /me
check 401 "! grep -q 'postgres://|postgresql://' /tmp/mb-body errada"                      POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"errada-errada\"}"
check 200 "entrar"                            POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"

code=$(curl -s -o /tmp/mb-body -w '%{http_code}' "${BASE%/api}/llms.txt")
[ "$code" = 200 ] && grep -q 'Authorization: Bearer' /tmp/mb-body && echo "ok   documentação para IA em /llms.txt" || { echo "FALHA /llms.txt: $code"; fails=$((fails+1)); }

echo; [ $fails -eq 0 ] && echo "tudo certo" || { echo "$fails falha(s)"; exit 1; }
