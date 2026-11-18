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
  [ -n "${HDR:-}" ] && args+=(-H "$HDR")
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
# limite por minuto (só quando o servidor sobe com MB_KEY_RATE=KEY_RATE)
if [ -n "${KEY_RATE:-}" ]; then
  codes=""; for _ in $(seq "$KEY_RATE"); do codes="$codes $(curl -s -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $KEY" "$BASE/tables")"; done
  case "$codes" in *429*) echo "ok   chave trava ao passar de $KEY_RATE pedidos por minuto";; *) echo "FALHA chave sem limite: $codes"; fails=$((fails+1));; esac
  check 200 "a sessão não entra no limite da chave" GET /tables
fi
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
check 200 "leitor troca a própria senha"      POST /password "{\"current\":\"$PW\",\"new\":\"${PW}2\"}"
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
check 401 "senha errada"                      POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"errada-errada\"}"
check 200 "entrar"                            POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"

# ---- sessões, links de senha, editor SQL restrito ----
hdr=$(curl -s -D - -o /dev/null -b "$JAR" "$BASE/me" | tr -d '\r' | grep -i '^cache-control:')
[ "$hdr" = "cache-control: no-store" ] && echo "ok   API sem cache" || { echo "FALHA cache-control: '$hdr'"; fails=$((fails+1)); }
check 200 "lista sessões"                     GET  /sessions; has '"current":true'
check 400 "SQL não lê metadados"              POST /sql '{"sql":"SELECT * FROM masterbase.users"}'; has 'permissão'
check 400 "SQL não cria tabela"               POST /sql '{"sql":"CREATE TABLE x (a int)"}'
check 400 "SQL não cria função de gatilho"    POST /sql '{"sql":"CREATE FUNCTION public.f() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RETURN NEW; END$$"}'
check 400 "SQL não vira outro papel"          POST /sql '{"sql":"SET ROLE postgres"}'
check 200 "configurações mostram editor"      GET  /settings; has '"sql_editor":true'

check 200 "convite sem senha"                 POST /users '{"name":"Gil","email":"gil@smoke.test","role":"leitor"}'; has '"token":"'
TOKEN=$(sed -E 's/.*"token":"([0-9a-f]+)".*/\1/' /tmp/mb-body)
check 401 "convidado sem senha não entra"     POST /login '{"email":"gil@smoke.test","password":""}'
check 200 "link do convite vale"              POST /reset/check "{\"token\":\"$TOKEN\"}"; has 'gil@smoke.test'
check 404 "link inventado"                    POST /reset/check '{"token":"00ff"}'
check 400 "link com senha curta"              POST /reset "{\"token\":\"$TOKEN\",\"password\":\"123\"}"
check 200 "criar senha pelo link"             POST /reset "{\"token\":\"$TOKEN\",\"password\":\"$PW\"}"
check 200 "já entra com a senha nova"         GET  /me; has 'gil@smoke.test'
check 404 "link não vale duas vezes"          POST /reset "{\"token\":\"$TOKEN\",\"password\":\"$PW\"}"
check 200 "entrar como admin"                 POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
check 200 "link para quem esqueceu a senha"   POST /users/3/link; has '"token":"'
TOKEN=$(sed -E 's/.*"token":"([0-9a-f]+)".*/\1/' /tmp/mb-body)
check 404 "link de pessoa que não existe"     POST /users/999/link
check 200 "trocar senha pelo link"            POST /reset "{\"token\":\"$TOKEN\",\"password\":\"${PW}3\"}";
check 200 "entrar com a senha do link"        POST /login "{\"email\":\"lia@smoke.test\",\"password\":\"${PW}3\"}"
check 403 "leitor não gera link"              POST /users/1/link
check 200 "entrar como admin"                 POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"

check 200 "nome e duração da sessão"          PATCH /settings '{"name":"Smoke","session_days":3}'
check 400 "sessão de 0 dias"                  PATCH /settings '{"session_days":0}'
check 200 "nome aparece no me"                GET  /me; has '"project":"Smoke"'
check 200 "nome aparece no login"             GET  /setup; has '"name":"Smoke"'

# ---- colunas: valor padrão e tipo ----
check 200 "coluna com padrão (com aspas)"     POST /schema/clientes/columns '{"name":"status","kind":"texto","default":{"mode":"fixo","value":"d'"'"'água"}}'
check 200 "estrutura mostra o padrão"         GET  /schema; has "d'água"
check 200 "linha nova usa o padrão"           POST /tables/clientes/rows '{"values":{"nome":"Padrão SA"}}'
check 200 "coluna de data e hora com agora"   POST /schema/clientes/columns '{"name":"visto","kind":"data e hora","default":{"mode":"agora"}}'
check 400 "agora em coluna de texto"          PATCH /schema/clientes/columns/status '{"default":{"mode":"agora"}}'
check 200 "coluna de código"                  POST /schema/clientes/columns '{"name":"codigo","kind":"texto"}'
check 200 "texto vira número"                 PATCH /schema/clientes/columns/codigo '{"kind":"número","default":{"mode":"fixo","value":"0"}}'
check 400 "padrão que não é número"           PATCH /schema/clientes/columns/codigo '{"default":{"mode":"fixo","value":"abc"}}'
check 400 "tipo que não converte"             PATCH /schema/clientes/columns/nome '{"kind":"número"}'; has 'não combinam'
check 200 "nome continua texto"               GET  /schema; has '"label":"texto","link":null,"name":"nome"'
check 400 "ligação não muda de tipo"          PATCH /schema/pedidos/columns/cliente_id '{"kind":"texto"}'
check 200 "tirar padrão"                      PATCH /schema/clientes/columns/status '{"default":{"mode":"nenhum"}}'
check 400 "modo de padrão inventado"          PATCH /schema/clientes/columns/status '{"default":{"mode":"now(); drop"}}'

# ---- backups: quantos guardar ----
check 200 "guardar 3 diários e 0 semanais"    PATCH /backups/settings '{"keep_daily":3,"keep_weekly":0}'; has '"keep_daily":3'
check 400 "guardar 0 diários"                 PATCH /backups/settings '{"keep_daily":0}'
check 200 "cópia externa desligada"           GET  /backups/settings; has '"copy":{"dir":null'

check 200 "sair dos outros aparelhos"         DELETE /sessions; has '"ended"'
check 200 "sessão atual continua"             GET  /me
for _ in 1 2 3 4 5; do curl -s -o /dev/null -b "$JAR" -H 'content-type: application/json' -d '{"password":"errada"}' "$BASE/backups/key"; done
check 429 "chave trava após 5 senhas erradas" POST /backups/key "{\"password\":\"$PW\"}"

# ---- verificação em duas etapas ----
totp() { python3 -c "import hmac,hashlib,struct,time,base64,sys; k=base64.b32decode(sys.argv[1]+'='*(-len(sys.argv[1])%8)); c=int(time.time())//30; h=hmac.new(k,struct.pack('>Q',c),hashlib.sha1).digest(); o=h[19]&15; print('%06d'%((struct.unpack('>I',h[o:o+4])[0]&0x7fffffff)%1000000))" "$1"; }
check 200 "gerar QR do segundo fator"         POST /2fa/setup; has '"qr":"<?xml'; has 'otpauth://totp/'
SECRET=$(sed -E 's/.*"secret":"([A-Z2-7]+)".*/\1/' /tmp/mb-body)
check 400 "código errado não liga"            POST /2fa/enable '{"code":"000000"}'
CODE=$(totp "$SECRET")
check 200 "ligar o segundo fator"             POST /2fa/enable "{\"code\":\"$CODE\"}"; has '"codes":\['
REC1=$(grep -o '"[0-9a-f]\{5\}-[0-9a-f]\{5\}"' /tmp/mb-body | sed -n 1p | tr -d '"')
REC2=$(grep -o '"[0-9a-f]\{5\}-[0-9a-f]\{5\}"' /tmp/mb-body | sed -n 2p | tr -d '"')
check 200 "me mostra o segundo fator"         GET  /me; has '"totp":true'
check 200 "senha certa pede o código"         POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"; has '"second_factor":true'
TICKET=$(sed -E 's/.*"ticket":"([0-9a-f]+)".*/\1/' /tmp/mb-body)
check 400 "o mesmo código não vale duas vezes" POST /login/2fa "{\"ticket\":\"$TICKET\",\"code\":\"$CODE\"}"
check 200 "código de recuperação entra"       POST /login/2fa "{\"ticket\":\"$TICKET\",\"code\":\"$REC1\"}"
check 401 "ticket não vale duas vezes"        POST /login/2fa "{\"ticket\":\"$TICKET\",\"code\":\"$REC2\"}"
check 200 "senha de novo"                     POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
TICKET=$(sed -E 's/.*"ticket":"([0-9a-f]+)".*/\1/' /tmp/mb-body)
check 400 "recuperação já gasta não entra"    POST /login/2fa "{\"ticket\":\"$TICKET\",\"code\":\"$REC1\"}"
check 200 "outra recuperação entra"           POST /login/2fa "{\"ticket\":\"$TICKET\",\"code\":\"$REC2\"}"
check 200 "pessoas mostram 2 etapas"          GET  /users; has '"totp":true'
check 400 "desligar exige código"             POST /2fa/disable '{"code":"123456"}'
check 200 "admin desliga pela tela Acesso"     PATCH /users/1 '{"reset_2fa":true}'
check 200 "sem segundo fator"                 GET  /me; has '"totp":false'
check 200 "entra direto de novo"              POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"; has '"ok":true'
check 400 "passkey precisa de nome no MB_URL" POST /passkeys/login/options

# ---- regra por linha: vendedor só vê os próprios pedidos ----
check 200 "tabela de vendas"                  POST /schema '{"name":"vendas","columns":[{"name":"item","kind":"texto"},{"name":"vendedor","kind":"texto"}]}'
check 200 "venda da Vera"                     POST /tables/vendas/rows '{"values":{"item":"mesa","vendedor":"VERA@smoke.test"}}'
check 200 "venda do Rui"                      POST /tables/vendas/rows '{"values":{"item":"cadeira","vendedor":"rui@smoke.test"}}'
check 400 "regra em coluna que não existe"    PUT  /schema/vendas/owner '{"column":"x"}'
check 200 "ligar a regra"                     PUT  /schema/vendas/owner '{"column":"vendedor"}'
check 200 "estrutura mostra a regra"          GET  /schema; has '"owner":"vendedor"'
check 400 "coluna do dono não se apaga"       DELETE /schema/vendas/columns/vendedor
check 200 "admin vê todas"                    GET  /tables/vendas; has '"total":2'
check 200 "criar vendedora"                   POST /users "{\"name\":\"Vera\",\"email\":\"vera@smoke.test\",\"password\":\"$PW\",\"role\":\"editor\"}"
check 200 "sair (admin, regra)"               POST /logout
check 200 "entrar como vendedora"             POST /login "{\"email\":\"vera@smoke.test\",\"password\":\"$PW\"}"
check 200 "vê só a própria venda"             GET  /tables/vendas; has '"total":1'; has '"owner":"vendedor"'; has 'mesa'
grep -q cadeira /tmp/mb-body && { echo "FALHA vendedora viu venda alheia"; fails=$((fails+1)); }
check 200 "contagem só das dela"              GET  /tables; has '"name":"vendas","rows":1'
check 200 "consulta também filtra"            POST /query '{"table":"vendas"}'; has 'mesa'
grep -q cadeira /tmp/mb-body && { echo "FALHA consulta mostrou venda alheia"; fails=$((fails+1)); }
check 200 "venda nova é dela"                 POST /tables/vendas/rows '{"values":{"item":"sofá","vendedor":"rui@smoke.test"}}'; has '"vendedor":"vera@smoke.test"'
check 404 "não edita venda alheia"            PATCH /tables/vendas/rows '{"pk":2,"column":"item","value":"x"}'
check 400 "não troca o dono"                  PATCH /tables/vendas/rows '{"pk":1,"column":"vendedor","value":"rui@smoke.test"}'
check 200 "não apaga venda alheia"            DELETE /tables/vendas/rows '{"pks":[2]}'; has '"deleted":0'
check 200 "importar vira dela"                POST /tables/vendas/import '{"columns":["item"],"rows":[["banco"]]}'
check 200 "agora são 3 dela"                  GET  /tables/vendas; has '"total":3'
check 200 "sair (vendedora)"                  POST /logout
check 200 "admin volta"                       POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
check 200 "renomear a coluna leva a regra"    PATCH /schema/vendas/columns/vendedor '{"name":"dono"}'
check 200 "regra segue o nome novo"           GET  /schema; has '"owner":"dono"'
check 200 "desligar a regra"                  PUT  /schema/vendas/owner '{"column":null}'
check 200 "admin vê as 4"                     GET  /tables/vendas; has '"total":4'

# ---- vários projetos: outro banco no mesmo painel (PROJ_URL = banco de teste à parte) ----
if [ -n "${PROJ_URL:-}" ]; then
  check 400 "endereço que não é postgres"       POST /projects '{"name":"x","url":"mysql://a@b/c"}'
  check 400 "banco que não responde"            POST /projects '{"name":"x","url":"postgres://a:b@127.0.0.1:1/x"}'
  check 200 "conectar outro banco"              POST /projects "{\"name\":\"Loja\",\"url\":\"$PROJ_URL\"}"
  PID=$(sed -E 's/.*"id":([0-9]+).*/\1/' /tmp/mb-body)
  check 200 "lista não mostra a senha"          GET  /projects; has '"name":"Loja"'
  grep -q "$PROJ_URL" /tmp/mb-body && { echo "FALHA lista de projetos mostrou o endereço"; fails=$((fails+1)); }
  HDR="X-MB-Project: $PID"
  check 200 "me mostra o projeto aberto"        GET  /me; has '"project":"Loja"'
  check 200 "tabelas do outro banco"            GET  /tables; has '"name":"itens","rows":2'
  grep -q clientes /tmp/mb-body && { echo "FALHA projeto mostrou tabela do principal"; fails=$((fails+1)); }
  check 200 "linha nova no outro banco"         POST /tables/itens/rows '{"values":{"nome":"arruela"}}'
  check 200 "coluna nova no outro banco"        POST /schema/itens/columns '{"name":"dono","kind":"texto"}'
  check 200 "regra por linha no projeto"        PUT  /schema/itens/owner '{"column":"dono"}'
  check 200 "consulta salva no projeto"         POST /queries '{"name":"todos os itens","spec":{"table":"itens"}}'
  check 200 "SQL no projeto"                    POST /sql '{"sql":"SELECT count(*) AS n FROM itens","read_only":true}'; has '"3"'
  check 200 "início do projeto"                 GET  /overview; has '"name":"itens"'
  HDR="X-MB-Project: 99999"
  check 404 "projeto que não existe"            GET  /tables
  HDR=""
  check 200 "principal continua igual"          GET  /tables; has 'clientes'
  grep -q itens /tmp/mb-body && { echo "FALHA principal mostrou tabela do projeto"; fails=$((fails+1)); }
  check 200 "consulta do projeto fica lá"       GET  /queries
  grep -q 'todos os itens' /tmp/mb-body && { echo "FALHA consulta do projeto apareceu no principal"; fails=$((fails+1)); }
  check 200 "regra do projeto fica lá"          GET  /schema
  grep -q '"owner":"dono"' /tmp/mb-body && { echo "FALHA regra do projeto apareceu no principal"; fails=$((fails+1)); }
  check 200 "atividade marca o projeto"         GET  /activity; has "\"project\":$PID"
  check 200 "sair (admin, projetos)"            POST /logout
  check 200 "vendedora entra"                   POST /login "{\"email\":\"vera@smoke.test\",\"password\":\"$PW\"}"
  check 403 "editor não conecta banco"          POST /projects '{"name":"x","url":"postgres://a:b@127.0.0.1:1/x"}'
  check 403 "editor não tira projeto"           DELETE /projects/$PID
  check 200 "sair (vendedora)"                  POST /logout
  check 200 "admin volta (projetos)"            POST /login "{\"email\":\"dona@smoke.test\",\"password\":\"$PW\"}"
  check 200 "tirar o projeto"                   DELETE /projects/$PID
  HDR="X-MB-Project: $PID"
  check 404 "projeto tirado não abre"           GET  /tables
  HDR=""
fi

# ---- e-mail e login com GitHub (só quando o servidor sobe com o SMTP falso: SINK=arquivo) ----
if [ -n "${SINK:-}" ]; then
  check 200 "e-mail e GitHub ligados"         GET  /setup; has '"mail":true'; has '"oauth":\["github"\]'
  grep -q 'To: gil@smoke.test' "$SINK" && grep -q 'Seu acesso ao' "$SINK" && echo "ok   convite chegou por e-mail" || { echo "FALHA convite sem e-mail"; fails=$((fails+1)); }
  : > "$SINK"
  check 200 "esqueci a senha"                 POST /forgot '{"email":"gil@smoke.test"}'; has '"ok":true'
  check 200 "esqueci a senha, e-mail sem conta" POST /forgot '{"email":"ninguem@smoke.test"}'; has '"ok":true'
  sleep 1
  TOKEN=$(grep -o 'senha/[0-9a-f]\{64\}' "$SINK" | head -1 | cut -d/ -f2)
  [ "$(grep -c '^To:' "$SINK")" = 1 ] && [ -n "$TOKEN" ] && echo "ok   link chegou só para quem tem conta" || { echo "FALHA e-mails: $(cat "$SINK")"; fails=$((fails+1)); }
  check 200 "link do e-mail vale"             POST /reset/check "{\"token\":\"$TOKEN\"}"; has 'gil@smoke.test'
  check 200 "esqueci a senha, 2ª vez"         POST /forgot '{"email":"ninguem@smoke.test"}'
  check 200 "esqueci a senha, 3ª vez"         POST /forgot '{"email":"ninguem@smoke.test"}'
  check 429 "esqueci a senha trava na 4ª"     POST /forgot '{"email":"ninguem@smoke.test"}'

  : > "$SINK"
  check 200 "entrar com link por e-mail"      POST /magic '{"email":"gil@smoke.test"}'; has '"ok":true'
  sleep 1
  MAGIC=$(grep -o 'link/[0-9a-f]\{64\}' "$SINK" | head -1 | cut -d/ -f2)
  [ -n "$MAGIC" ] && grep -q 'Entrar no' "$SINK" && echo "ok   link de entrar chegou" || { echo "FALHA link de entrar: $(cat "$SINK")"; fails=$((fails+1)); }
  code=$(curl -s -o /tmp/mb-body -w '%{http_code}' -c /tmp/mb-magic-jar -H 'content-type: application/json' -d "{\"token\":\"$MAGIC\"}" "$BASE/magic/use")
  [ "$code" = 200 ] && grep -q 'mb_session' /tmp/mb-magic-jar && echo "ok   link de entrar abre sessão" || { echo "FALHA link de entrar: $code $(cat /tmp/mb-body)"; fails=$((fails+1)); }
  code=$(curl -s -o /dev/null -w '%{http_code}' -H 'content-type: application/json' -d "{\"token\":\"$MAGIC\"}" "$BASE/magic/use")
  [ "$code" = 404 ] && echo "ok   link de entrar não vale duas vezes" || { echo "FALHA link reusado: $code"; fails=$((fails+1)); }

  loc() { curl -s -o /dev/null -D - -b "$JAR" -c "$JAR" "$BASE$1" | tr -d '\r' | sed -n 's/^location: //Ip'; }
  L=$(loc /oauth/github)
  case "$L" in https://github.com/login/oauth/authorize?client_id=smoke-id\&redirect_uri=http%3A%2F%2F127.0.0.1%3A8081%2Fapi%2Foauth%2Fgithub%2Fcallback*state=*) echo "ok   botão do GitHub leva ao GitHub";; *) echo "FALHA GitHub: $L"; fails=$((fails+1));; esac
  grep -q mb_oauth "$JAR" && echo "ok   state guardado em cookie" || { echo "FALHA sem cookie de state"; fails=$((fails+1)); }
  L=$(loc '/oauth/github/callback?code=x&state=forjado')
  [ "$L" = "/#/entrar?erro=falhou" ] && echo "ok   volta com state forjado é recusada" || { echo "FALHA state forjado: $L"; fails=$((fails+1)); }
  L=$(loc '/oauth/github/callback?code=x&state=y')
  [ "$L" = "/#/entrar?erro=cancelado" ] && echo "ok   state não serve duas vezes" || { echo "FALHA state reusado: $L"; fails=$((fails+1)); }
  L=$(loc /oauth/google)
  [ "$L" = "/#/entrar?erro=desligado" ] && echo "ok   Google desligado não redireciona" || { echo "FALHA Google: $L"; fails=$((fails+1)); }
fi

code=$(curl -s -o /tmp/mb-body -w '%{http_code}' "${BASE%/api}/llms.txt")
[ "$code" = 200 ] && grep -q 'Authorization: Bearer' /tmp/mb-body && echo "ok   documentação para IA em /llms.txt" || { echo "FALHA /llms.txt: $code"; fails=$((fails+1)); }

echo; [ $fails -eq 0 ] && echo "tudo certo" || { echo "$fails falha(s)"; exit 1; }
