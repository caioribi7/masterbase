# Masterbase MVP

Postgres com tela visual. Quem não sabe SQL cria tabela, liga tabelas, consulta e edita dados. Rust no servidor, Svelte na tela.

## Temos

**Páginas (todas do menu funcionam)**
- **Login.** Sem nenhuma conta, abre no cadastro. A primeira conta é a do administrador. E-mail e senha, mais Google e GitHub quando ligados (só para quem já tem conta). "Esqueci a senha" manda o link por e-mail; sem e-mail configurado, explica como pedir o link (ou rodar `masterbase senha EMAIL` no servidor). Link de convite e de senha nova abre a tela "Criar senha". Depois disso, cadastro fecha.
- **Início.** Tabelas, linhas, tamanho do banco, versão do Postgres. Barra por tabela. Últimas alterações.
- **Tabelas.** Grade tipo planilha:
  - filtro "é" ou "contém";
  - ordenar clicando no cabeçalho;
  - duplo clique edita a célula;
  - nova linha;
  - marcar várias linhas e apagar, com confirmação;
  - importar CSV (primeira linha = nomes das colunas; até 5.000 linhas; entra tudo ou nada);
  - exportar CSV;
  - paginação de 12 em 12;
  - no celular, fichas no lugar da grade e formulário da linha inteira.
- **Estrutura.** Sem SQL:
  - criar tabela (coluna `id` vem sozinha);
  - criar, editar (nome, tipo, valor padrão, obrigatório) e apagar colunas;
  - troca de tipo converte os valores e não muda nada se algum não servir;
  - marcar obrigatório ou "sem repetidos";
  - ligação com outra tabela (chave estrangeira);
  - renomear tabela;
  - apagar tabela digitando o nome.
- **Relações.** Diagrama das tabelas e das ligações. Passar o mouse acende as linhas. Clique abre a Estrutura.
- **Consultas.** Construtor visual:
  - escolher tabela e colunas;
  - condições: é, não é, contém, maior que, menor que, vazio, não vazio;
  - ordem e limite (até 500 linhas);
  - "Ver SQL" mostra o SELECT gerado;
  - salvar consulta e abrir de novo.
- **Editor SQL.** Só administradores. Texto livre, Ctrl+Enter roda, vários comandos (mostra o último), até 1.000 linhas, "só leitura" ligado por padrão (transação READ ONLY). Erro vem com a mensagem do Postgres. Comando que muda dados vai para a Atividade. Roda num papel do Postgres à parte (`MB_SQL_URL`, criado pelo instalador): lê e muda linhas de `public`, sem DDL, sem gatilhos, sem ver o schema `masterbase`.
- **Configurações.** Mudar o próprio nome e senha (trocar a senha encerra os outros aparelhos). Aparelhos conectados com "Encerrar" e "Sair dos outros aparelhos". Tamanho do texto e "menos movimento", por navegador. Administrador muda o nome da instância e a duração da sessão (1 a 90 dias). Endereço da documentação para IA (`/llms.txt`) com botão copiar. Administrador vê versão, banco, Postgres, endereço, quem acessa, pasta de backups e arquivo de configuração (sem a senha do banco).
- **Relações.** Arrastar com o mouse move o diagrama; arrastar em cima de uma tabela não abre a tabela.
- **Celular (até 700 px).** Menu vira barra de ícones embaixo; lista de tabelas vira faixa rolável em cima.
- **Acesso.** Pessoas e papéis:
  - administrador: tudo;
  - editor: muda linhas, não mexe na estrutura;
  - leitor: só vê e consulta;
  - criar pessoa (sem senha gera link de convite), mudar papel, link de senha nova (24 h, uso único), desativar e reativar;
  - cada um troca a própria senha.
- **API e chaves.** Chave para site ou sistema usar as tabelas:
  - "só ler" ou "ler e editar linhas";
  - aparece inteira uma vez só; o banco guarda o hash;
  - revogar;
  - exemplos prontos em curl e JavaScript.
- **Backups.** Automático todo dia no horário escolhido (padrão 03:00 em Brasília), guardando os N últimos e um por semana de mais M (padrão 7 e 4, muda na tela). Cópia de cada backup para outra pasta (`MB_BACKUP_COPY_DIR`) ou por comando (`MB_BACKUP_HOOK`, ex.: rclone); a tela mostra a última cópia. Download sai em stream. Fazer backup agora, baixar, apagar, restaurar (digitando "restaurar"). Antes de restaurar, guarda um backup do estado atual. Tudo criptografado com age (X25519); a chave de recuperação aparece com a senha de quem administra e por `masterbase chave-backup`. `masterbase recuperar` restaura num servidor novo.
- **Atividade.** Quem mudou o quê e quando. Agrupado por dia. Filtro por tabela. Carregar mais.

**Tipos em palavra simples**
- texto, número, decimal, sim/não, data, data e hora, dados (jsonb), id único (uuid), ligação.
- Tipos do seed também: opção (enum), e-mail (domain).

**Segurança**
- Senha em argon2id. Sessão em cookie httpOnly, Secure, SameSite=Strict. Banco guarda só o hash do token. Expira em 7 dias (ajustável de 1 a 90); o cookie dura o mesmo. Cada sessão guarda navegador e "visto por último".
- Login sem usuário gasta o mesmo tempo. Não revela e-mail.
- 5 senhas erradas no mesmo e-mail ou 30 do mesmo IP: bloqueio de 15 min (429). IP real vem do `X-Forwarded-For` só se o pedido chega de 127.0.0.1 (proxy local).
- Ver a chave dos backups também trava após 5 senhas erradas.
- Link de senha: 32 bytes aleatórios, só o hash no banco, 24 h, uso único (DELETE ... RETURNING contra corrida), checado por POST para não cair no log.
- Cadastro só para a primeira conta, com trava contra corrida.
- Papel conferido no servidor em toda rota que escreve. A tela só esconde botão.
- Ninguém muda o próprio papel nem se desativa (não tem como se trancar para fora).
- Trocar a própria senha encerra as outras sessões; redefinir pelo link ou desativar encerra todas.
- Chave de API nunca administra: não mexe em estrutura, pessoas, chaves nem backups.
- Cada chave faz até 300 pedidos por minuto (`MB_KEY_RATE`). Passou disso, 429 até o minuto virar.
- Nome de tabela e coluna existente: conferido no catálogo. Nome novo: validado e sempre entre aspas. Tipo e operador: lista fixa. Valor: sempre parametrizado.
- Consultas: não existe SQL livre. O servidor monta o SELECT.
- Editor SQL num papel sem superusuário, sem CREATE e sem TRIGGER; o servidor confere isso ao subir e desliga o editor se o papel tiver poder demais.
- Valor padrão de coluna: lista fixa de modos; valor fixo vira literal com aspas dobradas e cast para o tipo do catálogo.
- Respostas da API com `Cache-Control: no-store`.
- Backup: nome de arquivo só no formato que o servidor gera (nada de `../`). Senha do banco vai por variável de ambiente, não aparece na lista de processos. Restaurar roda em uma transação. A saída do `pg_dump` é criptografada enquanto é escrita (nunca há cópia aberta no disco), em arquivo `.part` com permissão 600, renomeado só no fim. Chave errada é recusada antes de tocar no banco.
- Apagar tabela sem CASCADE. Se outra tabela liga nela, o Postgres recusa e a tela explica.
- Erro do Postgres vira frase clara. Resto vira 500 e vai pro log.
- Consulta com mais de 15 s é cancelada. Corpo de requisição até 256 KB (8 MB na importação).
- CSP `self`, nosniff, no-referrer, sem iframe. `unsafe` proibido no crate.
- Metadados no schema `masterbase`. A tela só vê `public`.

**Instalar**
- `install.sh`: um comando. Postgres via apt ou Homebrew, banco com senha aleatória, binário da página de releases com soma conferida, config em `~/.config/masterbase/masterbase.env` (600).
- Comando `masterbase`: abre o painel e o navegador. `configurar` (banco, porta, só este computador / rede local / internet com HTTPS). `servico` (unit do systemd com o usuário de quem chamou).
- Binário único com a tela embutida: 4,7 MB estático (musl), ~4 MB de memória parado.
- `.github/workflows/release.yml`: tag `v*` compila Linux x86_64/arm64 e macOS arm64/x86_64 e publica.

**Código**
- `server/src/`: `main` (rotas e comandos), `config` (arquivo, perguntas, serviço), `auth` (login, sessão, papéis), `access` (pessoas e chaves), `data` (linhas, importar), `schema` (estrutura), `query` (consultas), `backup`, `activity` (auditoria e início), `catalog` (nomes e tipos), `error`.
- `web/src/`: `Shell` (menu), uma página por arquivo (`Home`, `Tables`, `Structure`, `Relations`, `Queries`, `Access`, `Keys`, `Backups`, `Activity`, `Login`), `Modal`, `Icon`, `lib.js`, `session.svelte.js`.

**Testes**
- `cargo test`: 13 testes (aspas, rótulos, tipos, nomes, bloqueio de login, SELECT das consultas, nomes e datas de backup, horário do automático, rotação 7+4, chave, leitura da URL do banco).
- `dev/wsl.sh smoke`: 178 checagens de ponta a ponta na API, num banco separado (`masterbase_smoke`, porta 8081). Cobre papéis, chaves, consultas, editor SQL, configurações, importar, backup criptografado, restaurar, chave de recuperação, horário do automático e `/llms.txt`.
- Instalador no WSL: instala, cria banco, `masterbase` serve a tela; segunda vez mantém a config. `configurar` testado com terminal simulado.
- Navegador (1440 px): todas as páginas abertas; criar tabela com ligação, nova linha, ordenar, apagar, rodar consulta, fazer backup.

## Falta

- Regras por linha ("vendedor só vê os próprios pedidos"). Hoje o papel vale para o banco inteiro.
- A chave dos backups fica no mesmo servidor: protege o arquivo que sai dele, não o servidor invadido.
- Link mágico e passkey.
- Login com Microsoft e segundo fator (TOTP).
- Vários projetos (vários bancos) num Masterbase só.
- Arrastar para ligar tabelas no diagrama. Hoje a ligação nasce na Estrutura.
- Gráfico rápido do resultado da consulta.
- Limite de login persistente (hoje por e-mail e por IP, em memória). 
- TLS no sqlx para banco remoto. Cookie Secure exige HTTPS fora do localhost.
- A tela Estrutura e os backups usam o dono do banco. Só o editor SQL tem papel restrito.
- Contagem com `count(*)`: lenta acima de ~1M linhas.
- Celular: Tabelas e Consultas mostram fichas, Relações vira lista de ligações. O Editor SQL funciona, mas foi pensado para tela grande.
- Instalador: só apt e Homebrew. Fedora/Arch à mão. `servico` e o macOS não testados. Primeiro release ainda não publicado.

## Rodar

O build do Rust roda no WSL, porque o Windows bloqueia executável sem assinatura.

Tela (Windows):
```
cd web && npm install && npm run build
```

Banco e servidor (WSL, root):
```
bash dev/wsl.sh reset   # recria o banco com os dados de exemplo
bash dev/wsl.sh run     # servidor em 127.0.0.1:8080
bash dev/wsl.sh smoke   # teste de ponta a ponta
```
Abre `http://127.0.0.1:8080`. Banco novo abre no cadastro da primeira conta.

Backups ficam em `MB_BACKUP_DIR` (no dev: `.dev/backups`). O servidor precisa ter `pg_dump` e `pg_restore`.

Tela com recarga ao vivo: `cd web && npm run dev`, abre `http://localhost:5173`.
