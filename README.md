<p align="center">
  <img src="brand/masterbase-icon.svg" width="96" alt="Masterbase">
</p>

<h1 align="center">Masterbase</h1>

<p align="center">
  Postgres com tela visual, para quem não sabe SQL.<br>
  Código aberto. Roda no seu servidor.
</p>

<p align="center">
  <a href="docs/guia-de-uso.md">Guia de uso</a> ·
  <a href="docs/instalacao.md">Instalação</a> ·
  <a href="docs/api.md">API</a> ·
  <a href="MVP.md">O que tem e o que falta</a>
</p>

![Página Tabelas](docs/img/tabelas.jpg)

O Masterbase fica na frente de um banco Postgres comum. Você cria tabelas, liga uma na outra, edita os dados como numa planilha e monta consultas escolhendo opções. O banco continua sendo Postgres de verdade: dá para conectar qualquer outra ferramenta nele quando precisar.

> **Estado do projeto:** versão inicial (0.1). Funciona de ponta a ponta e tem testes, mas ainda não tem instalador nem imagem Docker. Veja [o que falta](MVP.md#falta) antes de usar com dados importantes.

## O que dá para fazer

| | |
|---|---|
| **Tabelas** | Ver, filtrar, ordenar, editar célula com duplo clique, adicionar e apagar linhas, importar e exportar CSV. |
| **Estrutura** | Criar tabelas e colunas com tipos em português (texto, número, data, sim/não...). Marcar coluna como obrigatória ou sem repetidos. |
| **Relações** | Ligar uma tabela a outra e ver o diagrama das ligações. |
| **Consultas** | Montar "mostrar clientes onde plano é pro" clicando. Ver o SQL gerado. Salvar para usar de novo. |
| **Acesso** | Três papéis: quem administra, editor e leitor. |
| **API e chaves** | Chave para um site ou sistema ler e gravar nas tabelas. |
| **Backups** | Fazer, baixar e restaurar cópias das tabelas. |
| **Atividade** | Registro de quem mudou o quê e quando. |

<table>
  <tr>
    <td><img src="docs/img/estrutura.jpg" alt="Página Estrutura"></td>
    <td><img src="docs/img/relacoes.jpg" alt="Página Relações"></td>
  </tr>
  <tr>
    <td><img src="docs/img/consultas.jpg" alt="Página Consultas"></td>
    <td><img src="docs/img/inicio.jpg" alt="Página Início"></td>
  </tr>
</table>

## Começar

Você precisa de **Postgres 16** (versões mais antigas não foram testadas), **Rust** (estável) e **Node 20 ou mais novo**. Em Linux, macOS ou WSL:

```bash
git clone https://github.com/caioribi7/masterbase.git
cd masterbase

# 1. Um banco para o Masterbase (troque a senha)
sudo -u postgres psql -c "CREATE ROLE masterbase LOGIN PASSWORD 'troque-esta-senha'"
sudo -u postgres psql -c "CREATE DATABASE masterbase OWNER masterbase"

# 2. A tela
cd web && npm ci && npm run build && cd ..

# 3. O servidor
cd server
DATABASE_URL="postgresql://masterbase:troque-esta-senha@127.0.0.1:5432/masterbase" cargo run --release
```

Abra `http://127.0.0.1:8080`. Como o banco é novo, a primeira tela é o cadastro: **a primeira conta criada administra tudo**. Depois dela, o cadastro fecha e novas pessoas entram pela página Acesso.

Quer ver com dados de exemplo? Antes do passo 3, carregue a padaria fictícia:

```bash
PGPASSWORD='troque-esta-senha' psql -h 127.0.0.1 -U masterbase -d masterbase -f dev/seed.sql
```

Para colocar num servidor de verdade (HTTPS, serviço do sistema, backups), siga o guia de [instalação](docs/instalacao.md).

## Configuração

Tudo por variável de ambiente. Há um modelo em [`.env.example`](.env.example).

| Variável | Padrão | Para que serve |
|---|---|---|
| `DATABASE_URL` | (obrigatória) | Conexão com o Postgres. |
| `MB_ADDR` | `127.0.0.1:8080` | Endereço e porta em que o servidor escuta. |
| `MB_WEB_DIR` | `../web/dist` | Pasta com a tela já compilada. |
| `MB_BACKUP_DIR` | `backups` | Onde os backups são guardados. |

## Segurança

O Masterbase guarda dados dos outros, então isso vem primeiro:

- Senhas em argon2id. Sessão em cookie `HttpOnly`, `Secure`, `SameSite=Strict`. O banco guarda só o hash do token de sessão e das chaves de API.
- Cinco senhas erradas bloqueiam aquele e-mail por 15 minutos.
- O papel de cada pessoa é conferido no servidor em toda ação que escreve.
- Nenhum SQL é montado com texto digitado. Nomes de tabela e coluna são conferidos no catálogo do Postgres, tipos e operadores vêm de listas fixas, valores sempre vão como parâmetro.
- As contas e o histórico ficam num schema separado (`masterbase`), que a tela e a API não alcançam.
- `unsafe` é proibido no código Rust.

Achou uma falha? Não abra issue pública. Veja [SECURITY.md](SECURITY.md).

## Como é feito

```
server/   Rust: Axum (HTTP) + sqlx (Postgres). Um binário.
web/      Svelte 5 + Vite. Vira arquivos estáticos que o servidor entrega.
dev/      Dados de exemplo e teste de ponta a ponta.
docs/     Guias.
brand/    Logo e regras de interface.
```

O servidor não tem estado além do banco: as tabelas que você cria ficam no schema `public`, e o que é do Masterbase (contas, sessões, chaves, consultas salvas, atividade) fica no schema `masterbase`, criado sozinho na primeira subida.

## Desenvolver

```bash
cd server && cargo test          # testes do servidor
cd web && npm run dev            # tela com recarga ao vivo em localhost:5173
bash dev/smoke.sh                # 83 checagens na API (banco novo com dev/seed.sql)
```

Leia [CONTRIBUTING.md](CONTRIBUTING.md) antes de mandar um pull request.

## Licença

[MIT](LICENSE).
