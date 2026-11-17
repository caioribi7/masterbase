# Masterbase MVP

Postgres com tela visual. Quem não sabe SQL cria tabela, liga tabelas, consulta e edita dados. Rust no servidor, Svelte na tela.

## Temos

**Páginas (todas do menu funcionam)**
- **Login.** Sem nenhuma conta, abre no cadastro. A primeira conta administra. Depois disso, cadastro fecha.
- **Início.** Tabelas, linhas, tamanho do banco, versão do Postgres. Barra por tabela. Últimas alterações.
- **Tabelas.** Grade tipo planilha:
  - filtro "é" ou "contém";
  - ordenar clicando no cabeçalho;
  - duplo clique edita a célula;
  - nova linha;
  - marcar várias linhas e apagar, com confirmação;
  - importar CSV (primeira linha = nomes das colunas; até 5.000 linhas; entra tudo ou nada);
  - exportar CSV;
  - paginação de 12 em 12.
- **Estrutura.** Sem SQL:
  - criar tabela (coluna `id` vem sozinha);
  - criar, renomear e apagar colunas;
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
- **Acesso.** Pessoas e papéis:
  - administra: tudo;
  - editor: muda linhas, não mexe na estrutura;
  - leitor: só vê e consulta;
  - criar pessoa, mudar papel, dar senha nova, desativar e reativar;
  - cada um troca a própria senha.
- **API e chaves.** Chave para site ou sistema usar as tabelas:
  - "só ler" ou "ler e editar linhas";
  - aparece inteira uma vez só; o banco guarda o hash;
  - revogar;
  - exemplos prontos em curl e JavaScript.
- **Backups.** Fazer backup agora, baixar, apagar, restaurar (digitando "restaurar"). Antes de restaurar, guarda um backup do estado atual.
- **Atividade.** Quem mudou o quê e quando. Agrupado por dia. Filtro por tabela. Carregar mais.

**Tipos em palavra simples**
- texto, número, decimal, sim/não, data, data e hora, dados (jsonb), id único (uuid), ligação.
- Tipos do seed também: opção (enum), e-mail (domain).

**Segurança**
- Senha em argon2id. Sessão em cookie httpOnly, Secure, SameSite=Strict. Banco guarda só o hash do token. Expira em 7 dias.
- Login sem usuário gasta o mesmo tempo. Não revela e-mail.
- 5 senhas erradas no mesmo e-mail: bloqueio de 15 min (429).
- Cadastro só para a primeira conta, com trava contra corrida.
- Papel conferido no servidor em toda rota que escreve. A tela só esconde botão.
- Ninguém muda o próprio papel nem se desativa (não tem como se trancar para fora).
- Trocar ou redefinir senha, ou desativar conta, encerra as sessões da pessoa.
- Chave de API nunca administra: não mexe em estrutura, pessoas, chaves nem backups.
- Nome de tabela e coluna existente: conferido no catálogo. Nome novo: validado e sempre entre aspas. Tipo e operador: lista fixa. Valor: sempre parametrizado.
- Consultas: não existe SQL livre. O servidor monta o SELECT.
- Backup: nome de arquivo só no formato que o servidor gera (nada de `../`). Senha do banco vai por variável de ambiente, não aparece na lista de processos. Restaurar roda em uma transação.
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
- `cargo test`: 9 testes (aspas, rótulos, tipos, nomes, bloqueio de login, SELECT das consultas, nome de backup, leitura da URL do banco).
- `dev/wsl.sh smoke`: 83 checagens de ponta a ponta na API, num banco separado (`masterbase_smoke`, porta 8081). Cobre papéis, chaves, consultas, importar e restaurar backup.
- Instalador no WSL: instala, cria banco, `masterbase` serve a tela; segunda vez mantém a config. `configurar` testado com terminal simulado.
- Navegador (1440 px): todas as páginas abertas; criar tabela com ligação, nova linha, ordenar, apagar, rodar consulta, fazer backup.

## Falta

- Aba SQL livre nas Consultas. Precisa de um usuário do Postgres só de leitura e restrito a `public`; com o dono do banco seria possível ler as senhas.
- Regras por linha ("vendedor só vê os próprios pedidos"). Hoje o papel vale para o banco inteiro.
- Backup automático (agendado) e backup fora do servidor.
- Convite por e-mail, link mágico e passkey. Precisam de SMTP/WebAuthn. Hoje quem administra cria a pessoa e passa a senha.
- "Meu servidor" no login: só visual. Falta conectar em Postgres externo e ter vários projetos.
- Valor padrão na coluna (ex.: "agora" em data e hora). Mudar tipo de coluna existente.
- Arrastar para ligar tabelas no diagrama. Hoje a ligação nasce na Estrutura.
- Gráfico rápido do resultado da consulta.
- Limite de login por IP e persistente.
- TLS no sqlx para banco remoto. Cookie Secure exige HTTPS fora do localhost.
- Usuário do Postgres com permissão mínima. Hoje usa o dono do banco.
- Contagem com `count(*)`: lenta acima de ~1M linhas. Download de backup carrega o arquivo na memória.
- Tela de celular (abaixo de ~700 px).
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
