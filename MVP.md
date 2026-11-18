# Masterbase MVP

Postgres com tela visual. Quem não sabe SQL cria tabela, liga tabelas, consulta e edita dados. Rust no servidor, Svelte na tela.

## Temos

**Páginas (todas do menu funcionam)**
- **Login.** Sem nenhuma conta, abre no cadastro. A primeira conta é a do administrador. E-mail e senha, passkey (digital, rosto ou PIN do aparelho), link de entrar por e-mail (15 min, uso único), mais Google e GitHub quando ligados (só para quem já tem conta). Com verificação em duas etapas ligada, depois da senha pede o código de 6 dígitos do app autenticador ou um código de recuperação. "Esqueci a senha" manda o link por e-mail; sem e-mail configurado, explica como pedir o link (ou rodar `masterbase senha EMAIL` no servidor). Link de convite e de senha nova abre a tela "Criar senha". Depois disso, cadastro fecha.
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
  - tabela acima de 1 milhão de linhas mostra "cerca de" (estimativa do Postgres) em vez de contar uma por uma;
  - no celular, fichas no lugar da grade e formulário da linha inteira.
- **Estrutura.** Sem SQL:
  - criar tabela (coluna `id` vem sozinha);
  - criar, editar (nome, tipo, valor padrão, obrigatório) e apagar colunas;
  - troca de tipo converte os valores e não muda nada se algum não servir;
  - marcar obrigatório ou "sem repetidos";
  - ligação com outra tabela (chave estrangeira);
  - renomear tabela;
  - apagar tabela digitando o nome;
  - quem vê as linhas: todo mundo, ou cada pessoa só as linhas com o e-mail dela numa coluna (ex.: vendedor só vê os próprios pedidos).
- **Projetos.** O cartão do projeto no menu troca de banco. O administrador conecta outro banco Postgres pelo endereço (`postgres://...`); o servidor testa a conexão antes de guardar e recusa o próprio banco do Masterbase. Tabelas, Estrutura, Relações, Consultas, Editor SQL e Início passam a mostrar o banco escolhido. Contas, chaves, atividade e backups ficam sempre no banco principal. Tirar um projeto só desconecta: o banco continua intacto.
- **Relações.** Diagrama das tabelas e das ligações. Passar o mouse acende as linhas. Clique abre a Estrutura. Administrador arrasta da borda de uma tabela até outra para criar a ligação.
- **Consultas.** Construtor visual:
  - escolher tabela e colunas;
  - condições: é, não é, contém, maior que, menor que, vazio, não vazio;
  - ordem e limite (até 500 linhas);
  - "Ver SQL" mostra o SELECT gerado;
  - salvar consulta e abrir de novo (cada projeto tem as suas);
  - gráfico rápido: escolher a coluna para agrupar e o que medir (contagem ou soma), 12 maiores e "outros".
- **Editor SQL.** Só administradores. Texto livre, Ctrl+Enter roda, vários comandos (mostra o último), até 1.000 linhas, "só leitura" ligado por padrão (transação READ ONLY). Erro vem com a mensagem do Postgres. Comando que muda dados vai para a Atividade. No banco principal roda num papel do Postgres à parte (`MB_SQL_URL`, criado pelo instalador): lê e muda linhas de `public`, sem DDL, sem gatilhos, sem ver o schema `masterbase`. Em outro projeto, roda com o usuário do endereço que o administrador conectou.
- **Configurações.** Mudar o próprio nome e senha (trocar a senha encerra os outros aparelhos). Aparelhos conectados com "Encerrar" e "Sair dos outros aparelhos". Tamanho do texto e "menos movimento", por navegador. Verificação em duas etapas (QR para o app, 10 códigos de recuperação, desligar pedindo o código). Passkeys: adicionar com nome, ver quando foi usada, apagar. Administrador muda o nome da instância e a duração da sessão (1 a 90 dias). Endereço da documentação para IA (`/llms.txt`) com botão copiar. Administrador vê versão, banco, Postgres, endereço, quem acessa, pasta de backups e arquivo de configuração (sem a senha do banco).
- **Relações.** Arrastar com o mouse move o diagrama; arrastar em cima de uma tabela não abre a tabela.
- **Celular (até 700 px).** Menu vira barra de ícones embaixo; lista de tabelas vira faixa rolável em cima. Tabelas e Consultas mostram fichas; Relações vira lista de ligações; o Editor SQL tem botão de rodar embaixo e resultado em fichas.
- **Acesso.** Pessoas e papéis:
  - administrador: tudo;
  - editor: muda linhas, não mexe na estrutura;
  - leitor: só vê e consulta;
  - criar pessoa (sem senha gera link de convite, que vai por e-mail quando o SMTP está ligado), mudar papel, link de senha nova (24 h, uso único), desativar e reativar;
  - desligar as duas etapas de quem perdeu o celular e os códigos;
  - cada um troca a própria senha.
- **API e chaves.** Chave para site ou sistema usar as tabelas:
  - "só ler" ou "ler e editar linhas";
  - aparece inteira uma vez só; o banco guarda o hash;
  - revogar;
  - exemplos prontos em curl e JavaScript;
  - em outro projeto, manda junto o cabeçalho `X-MB-Project`.
- **Backups.** Automático todo dia no horário escolhido (padrão 03:00 em Brasília), guardando os N últimos e um por semana de mais M (padrão 7 e 4, muda na tela). Cópia de cada backup para outra pasta (`MB_BACKUP_COPY_DIR`) ou por comando (`MB_BACKUP_HOOK`, ex.: rclone); a tela mostra a última cópia. Download sai em stream. Fazer backup agora, baixar, apagar, restaurar (digitando "restaurar"). Antes de restaurar, guarda um backup do estado atual. Tudo criptografado com age (X25519); a chave de recuperação aparece com a senha de quem administra e por `masterbase chave-backup`. `masterbase chave-backup tirar` deixa no servidor só a parte pública (ou `MB_BACKUP_RECIPIENT` desde o começo): o servidor criptografa, mas não abre; para restaurar, cola a chave na tela. `masterbase recuperar` restaura num servidor novo. Backups cobrem só o banco principal.
- **Atividade.** Quem mudou o quê e quando. Agrupado por dia. Filtro por tabela. Carregar mais.

**Tipos em palavra simples**
- texto, número, decimal, sim/não, data, data e hora, dados (jsonb), id único (uuid), ligação.
- Tipos do seed também: opção (enum), e-mail (domain).

**Segurança**
- Senha em argon2id. Sessão em cookie httpOnly, Secure, SameSite=Strict. Banco guarda só o hash do token. Expira em 7 dias (ajustável de 1 a 90); o cookie dura o mesmo. Cada sessão guarda navegador e "visto por último".
- Login sem usuário gasta o mesmo tempo. Não revela e-mail.
- 5 senhas erradas no mesmo e-mail ou 30 do mesmo IP: bloqueio de 15 min (429). Fica no banco (`masterbase.attempts`), então reiniciar o servidor não zera. IP real vem do `X-Forwarded-For` só se o pedido chega de 127.0.0.1 (proxy local).
- Ver a chave dos backups, código das duas etapas, "esqueci a senha" e link de entrar também têm limite (3 e-mails por endereço, 20 por IP).
- Duas etapas: TOTP de 30 s (RFC 6238) com uma janela de folga, o mesmo código não vale duas vezes, códigos de recuperação guardados como hash. Senha certa gera um ticket de 5 minutos; só o código transforma o ticket em sessão. Vale para senha, Google, GitHub, link de senha e link de entrar.
- Passkey: WebAuthn com ES256, verificação do usuário obrigatória, desafio de uso único, origem e `rpId` conferidos com `MB_URL`, contador de assinatura conferido.
- Regra por linha: conferida no servidor em ler, contar, consultar, editar, apagar, inserir e importar. Linha nova e importada sempre levam o e-mail de quem cria; a coluna do dono não pode ser trocada por quem tem a regra.
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
- Projetos: o endereço (com a senha do outro banco) fica no banco principal e nunca volta para a tela.

**Instalar**
- `install.sh`: um comando. Postgres via apt, dnf, pacman ou Homebrew, banco com senha aleatória, binário da página de releases com soma conferida, config em `~/.config/masterbase/masterbase.env` (600).
- Comando `masterbase`: abre o painel e o navegador. `configurar` (banco, porta, só este computador / rede local / internet com HTTPS). `servico` (unit do systemd com o usuário de quem chamou).
- Binário único com a tela embutida: 4,7 MB estático (musl), ~4 MB de memória parado.
- `.github/workflows/release.yml`: tag `v*` compila Linux x86_64/arm64 e macOS arm64/x86_64 e publica. Releases 0.3.0 e 0.3.1 publicadas.

**Código**
- `server/src/`: `main` (rotas e comandos), `config` (arquivo, perguntas, serviço), `auth` (login, sessão, papéis), `access` (pessoas e chaves), `data` (linhas, importar), `schema` (estrutura), `query` (consultas), `backup`, `activity` (auditoria e início), `catalog` (nomes e tipos), `rules` (regra por linha), `projects` (vários bancos), `mfa` (duas etapas), `passkey`, `oauth` (Google e GitHub), `mail` (SMTP), `limits` (tentativas), `error`.
- `web/src/`: `Shell` (menu), uma página por arquivo (`Home`, `Tables`, `Structure`, `Relations`, `Queries`, `Access`, `Keys`, `Backups`, `Activity`, `Login`), `Modal`, `Icon`, `lib.js`, `session.svelte.js`.

**Testes**
- `cargo test`: 20 testes (aspas, rótulos, tipos, nomes, SELECT das consultas com e sem regra por linha, nomes e datas de backup, horário do automático, rotação 7+4, chave, leitura da URL do banco, TOTP pelo vetor da RFC, base32, registro e assinatura de passkey, limite por chave, e-mails do GitHub).
- `dev/wsl.sh smoke`: 250 checagens de ponta a ponta na API, num banco separado (`masterbase_smoke`, porta 8081) e um segundo banco (`masterbase_smoke2`) como projeto. Cobre papéis, chaves, consultas, editor SQL, configurações, importar, backup criptografado, restaurar, chave de recuperação, duas etapas, link de entrar, e-mail (SMTP falso), começo do login com GitHub, regra por linha, projetos e `/llms.txt`.
- Passkey testada no navegador com autenticador virtual (cadastrar, sair, entrar).
- Instalador no WSL: instala, cria banco, `masterbase` serve a tela; segunda vez mantém a config. `configurar` testado com terminal simulado.
- Navegador (1440 px): todas as páginas abertas; criar tabela com ligação, nova linha, ordenar, apagar, rodar consulta, fazer backup.

## Falta

- `servico` (systemd) e o instalador no macOS ainda não rodaram numa máquina de verdade.
- Ainda não rodou em servidor de produção com dados reais.
- Projetos: todo mundo com acesso vê todos os projetos (não tem papel por projeto). Backups só do banco principal.
- Regra por linha compara com o e-mail da conta. Não tem regra por grupo ou por papel.
- Passkey só funciona com `MB_URL` com nome (não IP), e com HTTPS fora do localhost.
- Com a chave dos backups no servidor (o padrão), quem invade o servidor abre os backups. `chave-backup tirar` resolve, mas aí a chave fica com você.

**Decidido assim**
- Sem login com Microsoft.
- A Estrutura e os backups usam o dono do banco: criar tabela e fazer `pg_dump` precisam disso. Só o editor SQL, que aceita texto livre, roda num papel restrito.

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
