# Masterbase MVP

Postgres com tela visual. Quem não sabe SQL cria tabela, liga tabelas e edita dados. Rust no servidor, Svelte na tela.

## Temos

**Páginas prontas**
- **Login.** Sem nenhuma conta, abre no cadastro. A primeira conta administra. Depois disso, cadastro fecha.
- **Início.** Tabelas, linhas, tamanho do banco, versão do Postgres. Barra por tabela. Últimas alterações.
- **Tabelas.** Grade tipo planilha:
  - filtro "é" ou "contém";
  - ordenar clicando no cabeçalho;
  - duplo clique edita a célula;
  - nova linha;
  - marcar várias linhas e apagar, com confirmação;
  - exportar CSV;
  - paginação de 12 em 12.
- **Estrutura.** Sem SQL:
  - criar tabela (coluna `id` vem sozinha);
  - criar, renomear e apagar colunas;
  - marcar obrigatório ou "sem repetidos";
  - **ligação com outra tabela** (chave estrangeira);
  - renomear tabela;
  - apagar tabela digitando o nome.
- **Atividade.** Quem mudou o quê e quando. Agrupado por dia. Filtro por tabela. Carregar mais.
- Relações, Consultas, Acesso, API e chaves, Backups: aparecem apagados no menu.

**Tipos em palavra simples**
- texto, número, decimal, sim/não, data, data e hora, dados (jsonb), id único (uuid), ligação.
- Tipos do seed também: opção (enum), e-mail (domain).

**Segurança**
- Senha em argon2id. Sessão em cookie httpOnly, Secure, SameSite=Strict. Banco guarda só o hash do token. Expira em 7 dias. Sessões vencidas são limpas no login.
- Login sem usuário gasta o mesmo tempo. Não revela e-mail.
- 5 senhas erradas no mesmo e-mail: bloqueio de 15 min (429).
- Cadastro só para a primeira conta, com trava na tabela contra corrida.
- Nome de tabela e coluna existente: conferido no catálogo. Nome novo: validado e sempre entre aspas. Tipo: lista fixa. Valor: sempre parametrizado.
- Apagar tabela sem CASCADE. Se outra tabela liga nela, o Postgres recusa e a tela explica.
- Erro do Postgres vira frase clara (coluna repetida, ligação inválida, obrigatório vazio...). Resto vira 500 e vai pro log.
- Consulta com mais de 15 s é cancelada. Corpo de requisição até 256 KB.
- CSP `self` (fontes inclusas, sem data:), nosniff, no-referrer, sem iframe. `unsafe` proibido no crate.
- Metadados no schema `masterbase`. A tela só vê `public`.

**Código**
- `server/src/`: `main` (rotas), `auth`, `data` (linhas), `schema` (estrutura), `activity` (auditoria e início), `catalog` (nomes e tipos), `error`.
- `web/src/`: `Shell` (menu), `Home`, `Tables`, `Structure`, `Activity`, `Login`, `Modal`, `Icon`, `lib.js` (rotas por hash, ícones, frases).
- Tela: 90 KB de JS (31 KB gzip). Fontes locais.

**Testes**
- `cargo test`: 5 testes (aspas, rótulos, tipos, nomes, bloqueio de login).
- `dev/wsl.sh smoke`: 40 checagens de ponta a ponta na API, num banco separado (`masterbase_smoke`, porta 8081).
- Navegador (1440 e 1000 px):
  - Início;
  - criar tabela com ligação;
  - nova linha ligada;
  - ordenar;
  - marcar e apagar;
  - Atividade.

## Falta

**Próximo**
- Página Relações: diagrama das ligações.
- Página Consultas: "mostrar X onde Y" + aba SQL.
- Acesso: convidar pessoas e definir papéis. Hoje só existe a primeira conta.
- Valor padrão na coluna (ex.: "agora" em data e hora).
- Importar CSV.
- Limite de login por IP e persistente.
- "Meu servidor": hoje é só visual. Falta conectar em Postgres externo.
- Link por e-mail e chave de acesso (passkey): botões desativados.
- TLS no sqlx para banco remoto. Cookie Secure exige HTTPS fora do localhost.
- Binário único com a tela embutida. Imagem Docker.
- Usuário do Postgres com permissão mínima. Hoje usa o dono do banco.
- Contagem com `count(*)`. Lenta acima de ~1M linhas.
- Tela de celular (abaixo de ~700 px).

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

Tela com recarga ao vivo: `cd web && npm run dev`, abre `http://localhost:5173`.
