# API

Para ler e gravar nas tabelas a partir de um site ou sistema. A tela do Masterbase usa esta mesma API.

## Autenticação

Crie uma chave em **API e chaves** (veja o [guia de uso](guia-de-uso.md#chaves-de-api)) e mande-a em toda requisição:

```
Authorization: Bearer mb_...
```

Uma chave **só lê** ou **lê e edita linhas**. Chave nenhuma mexe em estrutura, pessoas, outras chaves ou backups. Guarde a chave no servidor do seu sistema, nunca em código que roda no navegador do visitante.

Nos exemplos, `$URL` é o endereço do seu Masterbase e `$CHAVE` é a chave.

## Respostas e erros

Tudo é JSON. Erro vem com o status HTTP e uma frase:

```json
{ "error": "Valor inválido para essa coluna." }
```

| Status | Quando |
|---|---|
| 400 | O pedido está errado: valor inválido, coluna que não existe, campo obrigatório vazio. |
| 401 | Chave ausente, inválida ou revogada. |
| 403 | A chave não tem permissão para isso. |
| 404 | Tabela ou linha não encontrada. |
| 429 | Tentativas demais de login. |
| 500 | Erro no servidor. |

## Listar tabelas

```bash
curl -H "Authorization: Bearer $CHAVE" "$URL/api/tables"
```

```json
[{ "name": "clientes", "rows": 17 }, { "name": "pedidos", "rows": 120 }]
```

## Ler linhas

```bash
curl -H "Authorization: Bearer $CHAVE" "$URL/api/tables/clientes?page=1"
```

Parâmetros, todos opcionais:

| Parâmetro | O que faz |
|---|---|
| `page` | Página (12 linhas por página). Começa em 1. |
| `col`, `val` | Filtra: só linhas em que a coluna `col` é igual a `val` (sem diferenciar maiúsculas). |
| `op` | Com `col` e `val`: `é` (padrão) ou `contém`. |
| `sort` | Nome da coluna para ordenar. |
| `desc` | `true` para ordem decrescente. |

Resposta:

```json
{
  "columns": [{ "name": "id", "label": "número", "type": "integer", "pk": true, "required": true, "unique": false, "link": null }],
  "rows": [{ "id": 1, "nome": "Café Bom Dia", "cidade": "Curitiba" }],
  "total": 17,
  "page": 1,
  "pages": 2
}
```

Em `columns`, `label` é o tipo em português, `pk` marca a chave e `link` traz o nome da tabela ligada, quando a coluna é uma ligação.

## Consultar com várias condições

Para mais de um filtro ou para escolher colunas, use a consulta (até 500 linhas):

```bash
curl -X POST -H "Authorization: Bearer $CHAVE" -H "Content-Type: application/json" \
  -d '{
    "table": "clientes",
    "columns": ["nome", "cidade"],
    "filters": [
      { "col": "plano", "op": "é", "val": "pro" },
      { "col": "id", "op": "menor que", "val": "50" }
    ],
    "sort": "nome",
    "desc": false,
    "limit": 100
  }' \
  "$URL/api/query"
```

Operadores: `é`, `não é`, `contém`, `maior que`, `menor que`, `vazio`, `não vazio` (os dois últimos não usam `val`). `columns` vazio ou ausente traz todas. A resposta tem `columns`, `rows` e `sql` (o comando gerado, para leitura).

## Adicionar uma linha

Precisa de chave que edita. Os valores vão como texto; o Masterbase converte para o tipo da coluna.

```bash
curl -X POST -H "Authorization: Bearer $CHAVE" -H "Content-Type: application/json" \
  -d '{ "values": { "nome": "Doceria Lua", "cidade": "Maringá" } }' \
  "$URL/api/tables/clientes/rows"
```

Devolve a linha criada, já com o `id`. Coluna que você não mandar usa o valor padrão.

## Editar uma célula

```bash
curl -X PATCH -H "Authorization: Bearer $CHAVE" -H "Content-Type: application/json" \
  -d '{ "pk": 1, "column": "cidade", "value": "Londrina" }' \
  "$URL/api/tables/clientes/rows"
```

`pk` é o valor da chave da linha (o `id`). `value` nulo esvazia a célula. Devolve a linha atualizada.

## Apagar linhas

```bash
curl -X DELETE -H "Authorization: Bearer $CHAVE" -H "Content-Type: application/json" \
  -d '{ "pks": [3, 4] }' \
  "$URL/api/tables/clientes/rows"
```

De 1 a 500 chaves por vez. Devolve `{ "deleted": 2 }`.

## Importar várias linhas

```bash
curl -X POST -H "Authorization: Bearer $CHAVE" -H "Content-Type: application/json" \
  -d '{ "columns": ["nome", "cidade"], "rows": [["Moinho Sul", "Pelotas"], ["Laticínios Serra", "Gramado"]] }' \
  "$URL/api/tables/fornecedores/import"
```

Até 5.000 linhas e 8 MB. Uma transação: se uma linha falhar, nenhuma entra.

## Em JavaScript

```js
const r = await fetch(`${URL}/api/tables/clientes?sort=nome`, {
  headers: { Authorization: `Bearer ${CHAVE}` },
});
if (!r.ok) throw new Error((await r.json()).error);
const { rows, total } = await r.json();
```

## O que a API não faz (ainda)

- Não cria nem muda tabelas por chave. Isso é só pela tela, por quem administra.
- Não tem limite de requisições por chave.
- Não avisa em tempo real quando um dado muda.
