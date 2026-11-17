# Segurança

## Avisar de uma falha

**Não abra uma issue pública.** Use o aviso privado do GitHub: aba **Security** deste repositório → **Report a vulnerability**.

Conte:

- o que a falha permite fazer;
- os passos para repetir;
- a versão ou o commit em que você testou.

Você recebe uma resposta em até 7 dias. Quando a correção sair, seu nome entra nos agradecimentos, se você quiser.

## Versões atendidas

O projeto está na versão 0.x. Só o código mais recente da branch `main` recebe correções.

## O que conta como falha

- Ler ou mudar dados sem estar logado, ou além do que o papel permite.
- Alcançar o schema `masterbase` (contas, sessões, chaves) pela tela ou pela API.
- Injeção de SQL em qualquer rota.
- Sequestro de sessão, ou vazamento de senha, token de sessão ou chave de API.
- Ler ou escrever arquivos fora da pasta de backups.

## Limites conhecidos

Já sabemos destes e eles estão em [MVP.md](MVP.md#falta). Não precisam de aviso privado:

- o servidor conecta como dono do banco;
- a conexão com o Postgres não usa TLS;
- o bloqueio de tentativas de login é por e-mail, em memória, e não por IP;
- não há limite de requisições por chave de API;
- enquanto nenhuma conta existe, quem acessar primeiro cria a conta de administrador.
