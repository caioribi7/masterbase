# Instalação num servidor

Para rodar o Masterbase de verdade, com HTTPS e reiniciando sozinho. Para só experimentar no seu computador, o [README](../README.md#começar) basta.

> Ainda não existe instalador nem imagem Docker. Os passos abaixo são manuais. Os arquivos de exemplo do systemd e do Caddy seguem o formato padrão dessas ferramentas, mas **não foram testados num servidor real**: confira antes de confiar.

## O que precisa

- Linux (testado no Ubuntu 24.04).
- Postgres 16, com `pg_dump` e `pg_restore` instalados na mesma máquina do Masterbase (os backups usam os dois). Versões mais antigas do Postgres não foram testadas.
- Rust estável e Node 20+, só para compilar.
- Um domínio apontando para o servidor, para ter HTTPS.

## 1. Banco

```bash
sudo -u postgres psql -c "CREATE ROLE masterbase LOGIN PASSWORD 'uma-senha-longa-e-aleatoria'"
sudo -u postgres psql -c "CREATE DATABASE masterbase OWNER masterbase"
```

Use um banco só para o Masterbase. Ele cria dois espaços dentro desse banco: `public` (suas tabelas) e `masterbase` (contas, sessões, chaves, histórico).

## 2. Compilar

```bash
git clone https://github.com/caioribi7/masterbase.git /opt/masterbase
cd /opt/masterbase/web && npm ci && npm run build
cd /opt/masterbase/server && cargo build --release
```

O resultado é um binário (`server/target/release/masterbase`) e uma pasta de arquivos estáticos (`web/dist`).

## 3. Configurar

Crie `/etc/masterbase.env`, legível só pelo root (`chmod 600`):

```bash
DATABASE_URL=postgresql://masterbase:uma-senha-longa-e-aleatoria@127.0.0.1:5432/masterbase
MB_ADDR=127.0.0.1:8080
MB_WEB_DIR=/opt/masterbase/web/dist
MB_BACKUP_DIR=/var/lib/masterbase/backups
```

Deixe `MB_ADDR` em `127.0.0.1`. Quem atende a internet é o proxy do passo 5, não o Masterbase direto.

Se a senha do banco tiver caracteres como `@`, `:` ou `/`, escreva-os em formato de URL (`%40`, `%3A`, `%2F`).

## 4. Serviço do sistema

`/etc/systemd/system/masterbase.service`:

```ini
[Unit]
Description=Masterbase
After=network.target postgresql.service

[Service]
User=masterbase
EnvironmentFile=/etc/masterbase.env
ExecStart=/opt/masterbase/server/target/release/masterbase
Restart=on-failure
StateDirectory=masterbase

[Install]
WantedBy=multi-user.target
```

```bash
sudo useradd --system --home /var/lib/masterbase masterbase
sudo mkdir -p /var/lib/masterbase/backups && sudo chown -R masterbase: /var/lib/masterbase
sudo systemctl enable --now masterbase
sudo journalctl -u masterbase -f     # deve mostrar "Masterbase em http://127.0.0.1:8080"
```

Na primeira subida o Masterbase cria as próprias tabelas sozinho.

## 5. HTTPS

**É obrigatório fora do seu computador.** O cookie de sessão só é enviado por HTTPS; sem isso, o login não se mantém.

O jeito mais curto é o [Caddy](https://caddyserver.com), que tira o certificado sozinho. `/etc/caddy/Caddyfile`:

```
dados.suaempresa.com.br {
    reverse_proxy 127.0.0.1:8080
}
```

Com nginx ou outro proxy, o que importa é: terminar o HTTPS nele e repassar para `127.0.0.1:8080`.

## 6. Primeiro acesso

Abra `https://dados.suaempresa.com.br`. A tela pede a criação da primeira conta. **Faça isso logo depois de subir o servidor:** enquanto não existe nenhuma conta, quem chegar primeiro vira quem administra.

Daí em diante, siga o [guia de uso](guia-de-uso.md).

## Backups

Os backups da página Backups ficam em `MB_BACKUP_DIR`, no mesmo servidor. Isso protege contra erro humano (apagou a tabela errada), não contra perder o servidor. Copie essa pasta para outro lugar com a ferramenta que você já usa (rsync, restic, o backup do provedor).

Esses backups têm só as suas tabelas. Para guardar também contas e histórico, faça um `pg_dump` do banco inteiro por fora.

## Atualizar

```bash
cd /opt/masterbase && git pull
cd web && npm ci && npm run build
cd ../server && cargo build --release
sudo systemctl restart masterbase
```

Mudanças nas tabelas internas do Masterbase são aplicadas sozinhas quando o serviço sobe. Faça um backup antes.

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
