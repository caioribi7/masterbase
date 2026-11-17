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
- Roda no WSL (Ubuntu 24.04): Rust + Postgres 16, porta 5432.
- Senha aleatória em `.dev/database_url`. Fora do git.
- `dev/seed.sql`: dados da Padaria Aurora.
- Metadados no schema `masterbase` (inclui `_sqlx_migrations`). Tela só vê `public`.

**Testado**
- API: cadastro, login, logout, senha errada, e-mail repetido, filtro, edição, inserção, valor inválido, chave primária bloqueada, injeção em nome de tabela e coluna.
- Navegador: login, filtro, edição por duplo clique, nova linha, paginação.

## Falta

**Próximo**
- Limite de tentativas no login (força bruta).
- "Meu servidor": hoje só visual. Falta conectar a um Postgres externo.
- Link por e-mail e chave de acesso (passkey): botões desativados.
- TLS no sqlx (`tls-rustls`) para banco remoto.
- Binário único com a tela embutida, e imagem Docker.
- Usuário do Postgres com permissão mínima. Hoje usa o dono do banco.
- Contagem de linhas usa `count(*)`. Lento acima de ~1M linhas.

**Páginas sem tela ainda**
- Início, Estrutura, Relações, Consultas, Acesso, API e chaves, Backups, Atividade, Configurações.
- Seletor de projetos. Responsivo: abaixo de ~900 px os botões do topo apertam.

## Rodar

Build do Rust fica no WSL (Windows bloqueia executável sem assinatura).

Tela (Windows):
```
cd web && npm install && npm run build
```

Banco + servidor (WSL, root):
```
bash dev/wsl.sh reset   # recria banco com dados de exemplo
bash dev/wsl.sh run     # servidor em 127.0.0.1:8080
```
Abre `http://127.0.0.1:8080`.

Tela com recarga ao vivo: `cd web && npm run dev`, abre `http://localhost:5173`.
