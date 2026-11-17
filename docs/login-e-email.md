# Login com Google e GitHub, e envio de e-mail

As duas coisas são opcionais e grátis. Sem elas o Masterbase funciona igual: entra com e-mail e senha, e quem administra copia os links de convite e de senha nova.

O jeito mais rápido é rodar `masterbase configurar`, escolher a opção 3 (pela internet) e responder às perguntas. Ele pede o endereço público, os dados do e-mail e as chaves do Google e do GitHub, e grava tudo no arquivo de configuração. Depois reinicie:

```bash
sudo systemctl restart masterbase
```

Em **Configurações**, o quadro do servidor mostra o que ficou ligado.

## Endereço público

Os dois recursos precisam saber o endereço do painel, porque é ele que vai nos links dos e-mails e na volta do login:

```
MB_URL=https://painel.seudominio.com
```

Ainda sem domínio? Dá para testar com o [sslip.io](https://sslip.io), que transforma o IP da VPS num nome: para o IP `203.0.113.5`, use `https://203-0-113-5.sslip.io` no Caddy e no `MB_URL`. O Caddy emite o certificado HTTPS do mesmo jeito. Quando comprar o domínio, troque o endereço aqui e nos apps do Google e do GitHub.

## Entrar com Google e GitHub

Só entra quem **já tem conta** no Masterbase. O Google ou o GitHub confirma o e-mail, e o Masterbase procura uma conta ativa com esse e-mail. Ninguém cria conta sozinho por aqui: quem administra continua adicionando as pessoas em **Acesso**.

Para quem foi convidado, isso dispensa criar senha: basta entrar com o Google ou o GitHub do mesmo e-mail.

### GitHub (uns 2 minutos)

1. Abra [github.com/settings/developers](https://github.com/settings/developers) → **OAuth Apps** → **New OAuth App**.
2. Preencha:
   - **Application name:** o nome que as pessoas vão ver, por exemplo "Masterbase da Empresa".
   - **Homepage URL:** o seu `MB_URL`.
   - **Authorization callback URL:** `MB_URL` + `/api/oauth/github/callback`.
3. Clique em **Register application** e depois em **Generate a new client secret**.
4. Copie para a configuração:

```
MB_GITHUB_ID=Ov23li...
MB_GITHUB_SECRET=...
```

### Google (uns 5 minutos)

1. Abra o [Google Cloud Console](https://console.cloud.google.com) e crie um projeto. É grátis e não pede cartão.
2. Em **Google Auth Platform** → **Visão geral**, clique em **Começar**: nome do app, seu e-mail de suporte e público **Externo**.
3. Em **Público-alvo**, clique em **Publicar app**. Como o Masterbase pede só o e-mail, o Google não exige verificação. Sem publicar, só os e-mails da lista de teste conseguem entrar.
4. Em **Clientes** → **Criar cliente**: tipo **Aplicativo da Web**. Em **URIs de redirecionamento autorizados**, ponha `MB_URL` + `/api/oauth/google/callback`.
5. Copie o ID e a chave secreta:

```
MB_GOOGLE_ID=1234-abc.apps.googleusercontent.com
MB_GOOGLE_SECRET=GOCSPX-...
```

O Google não aceita IP puro como endereço de volta: use um domínio ou o sslip.io. O GitHub aceita qualquer endereço.

## Envio de e-mail

Com o e-mail ligado:

- o convite de pessoa nova chega sozinho na caixa dela;
- o botão **Link de senha** em Acesso também manda o link por e-mail;
- a tela de login ganha o **Esqueci a senha** de verdade: a pessoa digita o e-mail e recebe o link.

Qualquer SMTP serve. Três opções grátis:

| Serviço | Precisa de domínio? | Limite grátis | Quando usar |
| --- | --- | --- | --- |
| **Gmail** com senha de app | Não | ~500 por dia | Para começar hoje, sem cadastro novo. |
| **Brevo** | Não (confirma só o remetente) | 300 por dia | Quando quiser um remetente que não seja seu Gmail. |
| **Resend** | Sim | 3.000 por mês | Depois de comprar o domínio. Chega melhor na caixa de entrada. |

### Gmail (o mais simples)

1. Ligue a verificação em duas etapas na conta Google.
2. Abra [myaccount.google.com/apppasswords](https://myaccount.google.com/apppasswords) e crie uma senha de app com o nome "Masterbase". Ela tem 16 letras.
3. Configure:

```
MB_SMTP_HOST=smtp.gmail.com
MB_SMTP_PORT=465
MB_SMTP_USER=voce@gmail.com
MB_SMTP_PASS=abcdefghijklmnop
```

Os e-mails saem como "Masterbase <voce@gmail.com>".

### Brevo ou Resend

Crie a conta, gere a chave SMTP e use os dados que o painel deles mostrar:

```
# Brevo
MB_SMTP_HOST=smtp-relay.brevo.com
MB_SMTP_PORT=587
MB_SMTP_USER=seu-login@smtp-brevo.com
MB_SMTP_PASS=chave-smtp
MB_MAIL_FROM=Masterbase <painel@seudominio.com>

# Resend
MB_SMTP_HOST=smtp.resend.com
MB_SMTP_PORT=465
MB_SMTP_USER=resend
MB_SMTP_PASS=re_...
MB_MAIL_FROM=Masterbase <painel@seudominio.com>
```

Com domínio próprio, siga o passo a passo do serviço para cadastrar os registros SPF e DKIM no DNS. Sem eles, parte dos e-mails cai no spam.

## Todas as variáveis

| Variável | Para quê |
| --- | --- |
| `MB_URL` | Endereço público do painel. Obrigatório para os dois recursos. |
| `MB_GOOGLE_ID`, `MB_GOOGLE_SECRET` | Liga o botão do Google. |
| `MB_GITHUB_ID`, `MB_GITHUB_SECRET` | Liga o botão do GitHub. |
| `MB_SMTP_HOST` | Servidor SMTP. Liga o e-mail. |
| `MB_SMTP_PORT` | 465 (TLS direto, padrão) ou 587 (STARTTLS). |
| `MB_SMTP_USER`, `MB_SMTP_PASS` | Login no SMTP. |
| `MB_MAIL_FROM` | Remetente. Sem ele, usa `MB_SMTP_USER`. |

## Segurança

- O login pelo Google ou GitHub só aceita e-mail **verificado** no provedor.
- A volta do provedor é amarrada ao navegador que começou o login, por um código guardado em cookie por 10 minutos. Um link de volta forjado é recusado.
- "Esqueci a senha" responde igual para e-mail com e sem conta, e aceita 3 pedidos por e-mail e 20 por IP a cada 15 minutos.
- Os links dos e-mails usam sempre o `MB_URL`, nunca o endereço que veio no pedido. Assim ninguém consegue mandar para você um link que aponta para outro site.
- O e-mail só sai sem criptografia para um servidor SMTP na própria máquina (`localhost`).
