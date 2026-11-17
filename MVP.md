# Masterbase MVP

Postgres com tela visual. Rust no servidor, Svelte na tela.

## Temos

**Servidor** (`server/`, Rust + Axum + sqlx)
- Cadastro e login. Senha em argon2id.
- Sessão em cookie httpOnly, Secure, SameSite=Strict. Banco guarda só hash SHA-256 do token. Expira em 7 dias.
- Login sem usuário gasta o mesmo tempo. Não revela e-mail cadastrado.
- Tabelas do schema `public`: listar, ler paginado (12 por página), filtrar ("coluna é valor"), editar célula, inserir linha.
- Nome de tabela e coluna conferido no catálogo antes de entrar no SQL. Valor sempre parametrizado.
- Valor inválido devolve 400 com mensagem clara, não 500.
- Tipo do Postgres em palavra simples: número, texto, data, opção, e-mail, sim/não.
- Auditoria: login, cadastro, edição, inserção vão para `masterbase.activity`.
- Cabeçalhos: CSP só `self`, nosniff, no-referrer, sem iframe.
- `unsafe` proibido no crate.
- Migração automática ao subir.
- Testes: escape de identificador, rótulo de tipo.

**Tela** (`web/`, Svelte 5 + Vite, 60 KB de JS)
- Login e cadastro iguais ao Paper. Vidro, foco lima, um botão lima.
- Tabelas: barra lateral, lista de tabelas com contagem, grade, filtro, paginação.
- Duplo clique edita célula. Enter salva, Esc cancela.
- Nova linha inline. Campo vazio usa valor padrão.
- Exportar página atual em CSV.
- Fontes servidas localmente. Nada chama Google.

**Dev**
- Postgres 17 portátil em `.dev/pgdata`, porta 54329. Senha aleatória em `.dev/database_url`. Fora do git.
- `dev/seed.sql`: dados da Padaria Aurora.

## Falta

**Bloqueio atual**
- Controle Inteligente de Aplicativos do Windows bloqueia o build do Rust. Servidor escrito, **não compilado nem testado**.
- Solução escolhida: WSL. Rodar `wsl --install` como administrador e reiniciar.

**Próximo**
- Compilar no WSL e testar ponta a ponta.
- Limite de tentativas no login (força bruta).
- "Meu servidor": hoje só visual. Falta conectar a um Postgres externo.
- Link por e-mail e chave de acesso (passkey): botões desativados.
- TLS no sqlx (`tls-rustls`) para banco remoto.
- Binário único com a tela embutida, e imagem Docker.
- Usuário do Postgres com permissão mínima. Hoje usa o dono do banco.
- Contagem de linhas usa `count(*)`. Lento acima de ~1M linhas.

**Páginas sem tela ainda**
- Início, Estrutura, Relações, Consultas, Acesso, API e chaves, Backups, Atividade, Configurações.
- Seletor de projetos. Responsivo da grade no celular.

## Rodar

Postgres:
```
~/pgsql/bin/pg_ctl.exe -D .dev/pgdata -o "-p 54329 -c listen_addresses=127.0.0.1" -l .dev/pg.log start
```

Servidor (no WSL, depois de instalado):
```
cd server && DATABASE_URL="$(cat ../.dev/database_url)" cargo run
```

Tela:
```
cd web && npm install && npm run dev
```
Abre `http://localhost:5173`. Vite repassa `/api` para `127.0.0.1:8080`.
