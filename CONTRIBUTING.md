# Contribuir

Obrigado por querer ajudar. O Masterbase é pequeno de propósito; estas regras existem para ele continuar assim.

## Antes de escrever código

- **Bug:** abra uma issue com os passos para repetir.
- **Função nova:** abra uma issue antes do pull request. Dá para saber se combina com o projeto sem você gastar um fim de semana.
- **Falha de segurança:** não abra issue. Veja [SECURITY.md](SECURITY.md).

A lista do que falta está em [MVP.md](MVP.md#falta). São bons lugares para começar.

## Rodar no seu computador

Precisa de Postgres 16, Rust estável e Node 20+.

```bash
# banco de desenvolvimento com dados de exemplo
sudo -u postgres psql -c "CREATE ROLE masterbase LOGIN PASSWORD 'dev'"
sudo -u postgres psql -c "CREATE DATABASE masterbase OWNER masterbase"
PGPASSWORD=dev psql -h 127.0.0.1 -U masterbase -d masterbase -f dev/seed.sql

# servidor
cd server && DATABASE_URL=postgresql://masterbase:dev@127.0.0.1:5432/masterbase cargo run

# tela com recarga ao vivo (outro terminal)
cd web && npm ci && npm run dev      # abre em http://localhost:5173
```

No Windows, o build do Rust pode ser bloqueado pelo Controle Inteligente de Aplicativos. Use o WSL; `dev/wsl.sh` tem os atalhos (`reset`, `run`, `smoke`).

## Testes

Todo pull request precisa passar nos três:

```bash
cd server && cargo test       # testes de unidade
cd web && npm run build       # a tela compila sem avisos
bash dev/smoke.sh             # ponta a ponta na API
```

O `smoke.sh` precisa de um banco recém-criado com `dev/seed.sql` e do servidor rodando nele (ele cria a primeira conta). Passe o endereço se não for `http://127.0.0.1:8080`.

Mudou ou criou uma rota? Adicione as checagens dela em `dev/smoke.sh`, incluindo o caso que deve ser recusado.

## Regras do código

**Servidor (Rust)**

- Nenhum texto do usuário entra em SQL. Nome de tabela ou coluna existente passa por `catalog::columns` e `ident`. Nome novo passa por `valid_name`. Valor vai sempre com `.bind()`.
- Toda rota que escreve começa com `user.editor()?` ou `user.admin()?`.
- Mensagem de erro para a pessoa é em português e diz o que fazer. Detalhe técnico vai para o log.
- Sem `unsafe` (o compilador recusa).
- Dependência nova só com motivo forte.

**Tela (Svelte)**

- Siga [brand/ui-direcao.md](brand/ui-direcao.md): vidro escuro, luz lavanda, **uma única ação lima por tela**.
- Texto da interface em português simples. Quem usa não sabe SQL: "ligação", não "chave estrangeira".
- Nada carregado de fora (fontes, scripts). A política de segurança do servidor bloqueia.

**Geral**

- A menor mudança que resolve. Sem abstração para um uso só.
- Comentário explica o porquê, não o quê.

## Pull request

- Um assunto por pull request.
- Diga o que mudou, por quê, e como você testou.
- Mudança visual: inclua uma captura de tela.
- Mensagens de commit em português, no imperativo curto ("Corrige filtro em coluna de data").

Ao contribuir, você concorda em licenciar sua contribuição sob a [licença MIT](LICENSE) do projeto.
