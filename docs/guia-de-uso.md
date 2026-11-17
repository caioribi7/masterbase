# Guia de uso

Para quem vai usar o Masterbase no dia a dia. Não precisa saber SQL. Se você ainda vai instalar, comece pela [instalação](instalacao.md).

- [Primeiro acesso](#primeiro-acesso)
- [Criar uma tabela](#criar-uma-tabela)
- [Tipos de coluna](#tipos-de-coluna)
- [Ligar uma tabela a outra](#ligar-uma-tabela-a-outra)
- [Trabalhar com os dados](#trabalhar-com-os-dados)
- [Importar uma planilha](#importar-uma-planilha)
- [Consultas](#consultas)
- [Editor SQL](#editor-sql)
- [Dar acesso a outras pessoas](#dar-acesso-a-outras-pessoas)
- [Chaves de API](#chaves-de-api)
- [Backups](#backups)
- [Atividade](#atividade)
- [Configurações](#configurações)
- [Problemas comuns](#problemas-comuns)

## Primeiro acesso

Num Masterbase recém-instalado, a tela de entrada pede nome, e-mail e senha (10 caracteres ou mais). **Essa primeira conta é a do administrador.** Depois que ela existe, a tela passa a mostrar só o login, e novas pessoas entram por convite seu (veja [Dar acesso a outras pessoas](#dar-acesso-a-outras-pessoas)).

Se você errar a senha cinco vezes, o e-mail fica bloqueado por 15 minutos.

## Criar uma tabela

Uma tabela guarda um tipo de coisa: clientes, pedidos, produtos.

1. Abra **Estrutura** no menu.
2. Clique em **Nova tabela**.
3. Dê um nome (por exemplo, `clientes`).
4. Adicione as colunas: o nome de cada uma e o tipo.
5. Clique em **Criar tabela**.

Toda tabela ganha sozinha uma coluna `id`, que numera as linhas. É por ela que o Masterbase sabe qual linha editar ou apagar, então ela não pode ser mudada.

Depois de criada, na mesma página você pode:

- adicionar colunas (**Nova coluna**);
- renomear uma coluna (o lápis ao lado do nome);
- apagar uma coluna (a lixeira no fim da linha), o que apaga os valores dela em todas as linhas;
- renomear ou apagar a tabela. Para apagar, o Masterbase pede que você digite o nome dela.

### Regras de uma coluna

- **Obrigatório:** a linha não pode ser salva com esse campo vazio. Só dá para ligar essa regra numa coluna em que todas as linhas já estão preenchidas.
- **Sem repetidos:** duas linhas não podem ter o mesmo valor. Bom para e-mail, CPF, código de produto.

## Tipos de coluna

| Tipo | Use para | Exemplo |
|---|---|---|
| texto | Nomes, endereços, observações | `Padaria Aurora` |
| número | Quantidades inteiras | `42` |
| decimal | Dinheiro, peso, medidas | `19.90` |
| sim/não | Coisas que só têm dois estados | `true` ou `false` |
| data | Um dia | `2026-11-17` |
| data e hora | Um momento exato | `2026-11-17 14:30` |
| dados | Informação estruturada (JSON) | `{"cor": "azul"}` |
| id único | Código que nunca se repete (UUID) | |
| ligação com… | Apontar para uma linha de outra tabela | o `id` de um cliente |

Ao digitar, use ponto como separador decimal (`19.90`) e datas no formato ano-mês-dia (`2026-11-17`). Na tela elas aparecem no formato brasileiro.

## Ligar uma tabela a outra

Um pedido é de um cliente. Em vez de repetir o nome do cliente em cada pedido, o pedido aponta para a linha do cliente.

1. Em **Estrutura**, abra a tabela que vai apontar (`pedidos`).
2. Clique em **Nova coluna**.
3. Escolha o tipo **ligação com…** e depois a tabela de destino (`clientes`).
4. Salve.

Agora, em cada pedido, essa coluna guarda o `id` do cliente. O Masterbase passa a proteger a ligação:

- não deixa salvar um pedido apontando para um cliente que não existe;
- não deixa apagar um cliente que ainda tem pedidos;
- não deixa apagar a tabela `clientes` enquanto `pedidos` aponta para ela.

A página **Relações** mostra todas as tabelas e as linhas de ligação entre elas. Passe o mouse numa tabela para acender as ligações dela.

## Trabalhar com os dados

A página **Tabelas** funciona como uma planilha.

- **Editar:** clique duas vezes numa célula, mude o valor e aperte Enter. Esc cancela.
- **Adicionar:** **Nova linha**, preencha e clique em **Salvar linha**. Campo deixado em branco usa o valor padrão da coluna.
- **Apagar:** marque as caixinhas das linhas e clique em **Apagar**. Não dá para desfazer.
- **Filtrar:** **Filtrar**, escolha a coluna, a condição (`é` ou `contém`) e o valor.
- **Ordenar:** clique no nome da coluna. Um clique: crescente. Dois: decrescente. Três: tira a ordem.
- **Exportar:** baixa a página atual em CSV, que abre no Excel ou no Google Planilhas.

## Importar uma planilha

1. Na planilha, a primeira linha precisa ter **exatamente os nomes das colunas** da tabela.
2. Salve como CSV.
3. Em **Tabelas**, abra a tabela e clique em **Importar**.

Regras:

- até 5.000 linhas por arquivo;
- se uma linha tiver erro (data inválida, campo obrigatório vazio), **nada é importado** e a tela diz o motivo. Corrija e importe de novo;
- não inclua a coluna `id`: ela é preenchida sozinha;
- vírgula ou ponto e vírgula como separador, os dois funcionam.

## Consultas

Uma consulta responde uma pergunta sobre os dados. Em **Consultas**:

1. Escolha a tabela.
2. Clique nas colunas que quer ver (as acesas aparecem no resultado).
3. Clique em **Condição** para cada filtro. Várias condições valem juntas ("e").
4. Escolha a ordem e o máximo de linhas (até 500).
5. Clique em **Rodar**.

Condições disponíveis: `é`, `não é`, `contém`, `maior que`, `menor que`, `vazio`, `não vazio`.

**Ver SQL** mostra o comando que o Masterbase montou. Serve para aprender e para copiar para outra ferramenta. **Salvar** guarda a consulta na lista da esquerda.

## Editor SQL

Para quem sabe SQL. Só administradores veem a página **Editor SQL**.

Escreva o comando e clique em **Rodar** (ou Ctrl+Enter). Dá para mandar vários comandos separados por `;`: todos rodam juntos e a tela mostra o resultado do último. Aparecem até 1.000 linhas.

- **Só leitura** vem ligado. Assim, nada é gravado: um UPDATE ou DELETE dá erro. Desligue quando quiser mudar dados de verdade.
- Tudo roda numa transação. Se um comando falha, nenhum vale.
- Comando que passa de 15 segundos é cancelado.
- Comandos que mudam dados ficam registrados na Atividade.

O editor roda com permissão total sobre o banco. Antes de um DROP ou um UPDATE grande, faça um backup.

## Dar acesso a outras pessoas

Em **Acesso**, clique em **Nova pessoa**, preencha nome, e-mail, uma senha inicial e o papel. Passe a senha para a pessoa por um canal seguro; ela pode trocar depois em **Configurações**.

| Papel | Pode |
|---|---|
| Administrador | Tudo: estrutura, pessoas, chaves, backups e editor SQL. |
| Editor | Ver os dados, mudar, adicionar e apagar linhas, importar, salvar consultas. |
| Leitor | Ver os dados e rodar consultas. |

Na lista você também pode mudar o papel de alguém, dar uma senha nova (isso desconecta a pessoa de todos os aparelhos) e **desativar** a conta. Conta desativada não entra mais, mas o que ela fez continua no histórico.

Você não consegue mudar o próprio papel nem se desativar. É de propósito: evita que o banco fique sem administrador.

## Chaves de API

Uma chave deixa um site ou sistema usar as tabelas sem passar pela tela. Em **API e chaves**:

1. **Nova chave**.
2. Dê um nome que diga onde ela vai ser usada (`site da padaria`).
3. Escolha se ela **só lê** ou **lê e edita linhas**.
4. **Copie a chave na hora.** Ela aparece uma única vez.

Se uma chave vazar, clique em **Revogar**: ela para de funcionar na hora. Como usar a chave no código está em [API](api.md). Para pedir a uma IA que faça a integração, mande o endereço que aparece em **Configurações** (termina em `/llms.txt`).

## Backups

O Masterbase faz um backup sozinho todo dia, às 03:00 no começo (mude o horário na própria página). Ficam os 7 mais recentes e um por semana das 4 semanas anteriores; os mais velhos são apagados sozinhos. Se o servidor estava desligado no horário, o backup sai assim que ele ligar.

**Fazer backup agora** guarda uma cópia na hora, por exemplo antes de uma mudança grande. Esses manuais não são apagados sozinhos.

Na lista, cada backup mostra o tipo: **Automático**, **Manual** ou **Antes de restaurar** (a cópia que o Masterbase guarda antes de uma restauração).

- **Baixar:** traz o arquivo para o seu computador. Faça isso de vez em quando: se o servidor quebrar, os backups que estão só nele vão junto.
- **Restaurar:** as tabelas voltam a ser como eram naquele backup. Antes, o Masterbase guarda sozinho uma cópia do estado atual, para você poder voltar atrás.
- **Apagar:** remove o arquivo do servidor.

O backup tem as tabelas e os dados. Contas, chaves e histórico de atividade não entram.

### Criptografia e chave de recuperação

Todo backup sai criptografado. Quem pegar o arquivo (um pendrive perdido, uma pasta na nuvem) não consegue ler nada sem a **chave de recuperação**.

A chave fica no servidor, e é por isso que o Masterbase restaura sem pedir nada. Mas se o servidor se perder, ela vai junto. Então guarde uma cópia:

1. Em **Backups**, clique em **Ver chave** e digite sua senha.
2. Copie a chave (começa com `AGE-SECRET-KEY-1`) para um gerenciador de senhas.
3. Marque **Guardei a chave num lugar seguro**. O aviso amarelo some.

Quem tiver a chave e um arquivo de backup lê os dados. Não mande a chave por e-mail nem chat.

Para recuperar num servidor novo, instale o Masterbase, copie o arquivo para lá e rode:

```bash
masterbase recuperar backup-20261117-060000.dump.age
```

Ele pede a chave e restaura no banco configurado.

## Atividade

A página **Atividade** mostra quem fez o quê: entradas, linhas criadas, editadas e apagadas, mudanças de estrutura, pessoas e chaves criadas, backups. Use o seletor no canto para ver só uma tabela.

## Configurações

Em **Configurações** você muda o seu nome e a sua senha. O quadro **Integrar com outro sistema** traz o endereço da documentação para IA, com botão para copiar.

O administrador também vê como o Masterbase está rodando: versão, banco, endereço, quem pode acessar e onde ficam os backups. Para mudar isso, rode `masterbase configurar` no servidor e reinicie.

## Problemas comuns

| Mensagem | O que quer dizer |
|---|---|
| "Valor inválido para essa coluna." | O valor não combina com o tipo: texto numa coluna de número, data fora do formato ano-mês-dia. |
| "Um campo obrigatório ficou vazio." | Faltou preencher uma coluna marcada como obrigatória. |
| "Esse valor já existe e a coluna não aceita repetidos." | A coluna é "sem repetidos" e outra linha já tem esse valor. |
| "Ligação inválida: o registro ligado não existe ou ainda está em uso." | Você apontou para um `id` que não existe, ou tentou apagar uma linha que outra tabela usa. |
| "Outras tabelas dependem disso. Remova as ligações antes." | Apague primeiro a coluna de ligação na outra tabela. |
| "Seu acesso é só de leitura." | Seu papel é leitor. Peça a um administrador. |
| "Muitas tentativas. Espere 15 minutos." | Cinco senhas erradas seguidas. |
| "Tabela sem chave: só leitura." | A tabela foi criada fora do Masterbase sem chave primária. Dá para ver, não para editar. |
