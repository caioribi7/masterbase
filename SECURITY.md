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
- Ver a chave de recuperação dos backups sem a senha de um administrador, ou abrir um backup sem a chave.
- Pelo editor SQL, criar ou apagar tabelas, ler o schema `masterbase` ou ganhar mais permissão que o papel de `MB_SQL_URL`.
- Usar um link de senha vencido, já usado ou de outra pessoa.

## Limites conhecidos

Já sabemos destes e eles estão em [MVP.md](MVP.md#falta). Não precisam de aviso privado:

- o servidor conecta como dono do banco (o editor SQL usa um papel à parte, só com acesso a linhas de `public`);
- o limite de pedidos por chave de API fica em memória e zera ao reiniciar (o de tentativas de login fica no banco);
- com a chave dos backups no servidor (o padrão), quem invade o servidor também abre os backups; `masterbase chave-backup tirar` tira a chave de lá;
- o endereço de um projeto conectado (com a senha daquele banco) fica guardado no banco principal;
- quem tem acesso ao painel vê todos os projetos conectados;
- enquanto nenhuma conta existe, quem acessar primeiro cria a conta de administrador.
