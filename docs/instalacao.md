# Instalação num servidor

Para rodar o Masterbase num servidor, com HTTPS e ligando sozinho. Para só experimentar no seu computador, o [README](../README.md#começar) basta.

> O instalador foi testado no Ubuntu 24.04 (WSL). O serviço do systemd e o Caddy seguem o formato padrão dessas ferramentas, mas **ainda não foram testados num servidor real**: confira antes de confiar.

## O que precisa

- Ubuntu ou Debian (o instalador usa `apt`). Em outro Linux, veja [instalar à mão](#instalar-à-mão).
- Um usuário com `sudo`.
- Um domínio apontando para o servidor, para ter HTTPS.

## 1. Instalar

```bash
curl -fsSL https://raw.githubusercontent.com/caioribi7/masterbase/main/install.sh | sh
```

O instalador:

- instala o Postgres, se ainda não tiver;
- cria o banco `masterbase` com uma senha aleatória;
- baixa o Masterbase da [página de releases](https://github.com/caioribi7/masterbase/releases) e confere a soma de verificação;
- coloca o comando em `/usr/local/bin/masterbase`;
- salva a configuração em `~/.config/masterbase/masterbase.env`, que só o seu usuário consegue ler.

Rodar de novo atualiza o programa e mantém a configuração.

## 2. Configurar

```bash
masterbase configurar
```

São três perguntas: o endereço do banco (o instalador já preencheu), a porta e quem vai abrir o painel.

| Resposta | Quando usar |
|---|---|
| Só este computador | Uso pessoal, no seu notebook. |
| Outras máquinas da rede local | Rede de casa ou do escritório, sem HTTPS. A senha passa aberta pela rede: só em rede de confiança. |
| Pela internet, com HTTPS na frente | Servidor na nuvem. O Masterbase escuta só em `127.0.0.1` e o Caddy ou o Nginx atende a internet. |

## 3. Deixar rodando sozinho

```bash
sudo masterbase servico
```

Isso cria o serviço `masterbase` no systemd, rodando com o seu usuário, e liga na hora. A partir daí ele sobe junto com o servidor.

```bash
systemctl status masterbase          # está rodando?
journalctl -u masterbase -f          # mensagens do Masterbase
```

## 4. HTTPS

**É obrigatório para acesso pela internet.** Fora da rede local, o cookie de sessão só é enviado por HTTPS, então sem ele o login não se mantém.

O caminho mais curto é o [Caddy](https://caddyserver.com), que tira o certificado sozinho. `/etc/caddy/Caddyfile`:

```
dados.suaempresa.com.br {
    reverse_proxy 127.0.0.1:8080
}
```

Com nginx ou outro proxy, o que importa é: o HTTPS termina nele e ele repassa para `127.0.0.1:8080`.

## 5. Primeiro acesso

Abra o endereço do painel. A tela pede a primeira conta. **Faça isso logo depois de subir o servidor:** enquanto não houver nenhuma conta, quem chegar primeiro vira quem administra.

Daí em diante, siga o [guia de uso](guia-de-uso.md).

## Acompanhar pelo terminal

Enquanto roda, o Masterbase escreve no terminal uma linha por pedido à API e uma por mudança:

```
14:02:11Z POST /api/login 200 41ms
14:02:11Z ana@padaria.com login {}
14:03:40Z ana@padaria.com sql {"sql":"UPDATE clientes SET plano = 'pro' WHERE id = 3"}
14:03:40Z POST /api/sql 200 12ms
14:05:02Z login recusado para joao@padaria.com
```

A hora é UTC (o `Z` no fim). O caminho não inclui o que vem depois do `?`, e senhas nunca aparecem. Como serviço, as mesmas linhas vão para o journal: `journalctl -u masterbase -f`. Para desligar, `MB_LOG=off`.

## Backups

O Masterbase faz um backup por dia sozinho e apaga os antigos (ficam 7 diários e 4 semanais). Os arquivos ficam em `~/.local/share/masterbase/backups`, no mesmo servidor. Isso protege contra erro humano (apagou a tabela errada), não contra perder o servidor. Copie essa pasta para outro lugar com a ferramenta que você já usa (rsync, restic, o backup do provedor): os arquivos são criptografados, então podem ir para qualquer lugar.

**Guarde a chave de recuperação logo depois de instalar:**

```bash
masterbase chave-backup
```

Ela é criada na primeira vez que o Masterbase sobe e fica em `MB_BACKUP_KEY`, no arquivo de configuração. Sem ela, os backups não abrem fora deste servidor. Para recuperar numa máquina nova: instale, copie o arquivo e rode `masterbase recuperar arquivo.dump.age`. O formato é o [age](https://age-encryption.org), então `age -d -i chave.txt arquivo.dump.age > arquivo.dump` também abre.

Esses backups têm só as suas tabelas. Para guardar também contas e histórico, faça um `pg_dump` do banco inteiro por fora.

## Atualizar

Rode o instalador de novo e reinicie:

```bash
curl -fsSL https://raw.githubusercontent.com/caioribi7/masterbase/main/install.sh | sh
sudo systemctl restart masterbase
```

Mudanças nas tabelas internas do Masterbase são aplicadas sozinhas quando o serviço sobe. Faça um backup antes.

## Configuração à mão

O arquivo `~/.config/masterbase/masterbase.env` tem uma linha por item. Variáveis de ambiente com o mesmo nome valem mais que o arquivo.

| Variável | Padrão | Para que serve |
|---|---|---|
| `DATABASE_URL` | (obrigatória) | Conexão com o Postgres. Caracteres como `@`, `:` ou `/` na senha vão em formato de URL (`%40`, `%3A`, `%2F`). |
| `MB_ADDR` | `127.0.0.1:8080` | Endereço e porta do painel. |
| `MB_BACKUP_DIR` | `backups` | Pasta dos backups. |
| `MB_BACKUP_KEY` | (criada sozinha) | Chave que criptografa os backups. Não troque: backups feitos com a anterior deixam de abrir aqui. |
| `MB_INSECURE_COOKIE` | (vazio) | `1` aceita login sem HTTPS. Só para rede local. |
| `MB_LOG` | (ligado) | `off` desliga as linhas de acompanhamento no terminal. |
| `MB_CONFIG` | `~/.config/masterbase/masterbase.env` | Outro arquivo de configuração. |
| `MB_WEB_DIR` | (vazio) | Serve a tela de uma pasta em vez da que vem dentro do programa. Só para desenvolver. |

## Instalar à mão

Em outro Linux, ou para compilar você mesmo:

1. Instale o Postgres 16 com `pg_dump` e `pg_restore` na mesma máquina (os backups usam os dois).
2. Crie o banco:
   ```bash
   sudo -u postgres psql -c "CREATE ROLE masterbase LOGIN PASSWORD 'uma-senha-longa-e-aleatoria'"
   sudo -u postgres psql -c "CREATE DATABASE masterbase OWNER masterbase"
   ```
3. Baixe o programa da [página de releases](https://github.com/caioribi7/masterbase/releases) ou compile (precisa de Rust e Node 20+):
   ```bash
   git clone https://github.com/caioribi7/masterbase.git && cd masterbase
   cd web && npm ci && npm run build && cd ..
   cd server && cargo build --release      # sai em server/target/release/masterbase
   ```
4. Copie para `/usr/local/bin/masterbase` e siga do passo 2 acima.

## Usar um banco que já existe

Dá para apontar `DATABASE_URL` para um banco com tabelas prontas: elas aparecem na tela. Cuidados:

- o usuário da conexão precisa poder criar o schema `masterbase` nesse banco;
- tabelas sem chave primária aparecem só para leitura;
- o Masterbase mostra o schema `public`. Tabelas em outros schemas não aparecem.

## Limites que você deve conhecer

- O Masterbase conecta como dono do banco. Quem administra o Masterbase tem, na prática, controle total das tabelas.
- A conexão com o Postgres não usa TLS. Mantenha o banco na mesma máquina ou numa rede privada.
- O bloqueio de tentativas de login é por e-mail e fica na memória: reiniciar o serviço zera.
- Um servidor atende um banco. Não há vários projetos.

A lista completa está em [MVP.md](../MVP.md#falta).
