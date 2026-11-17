# Masterbase API — reference for AI assistants and integrators

> Give this file to an AI (ChatGPT, Claude, Copilot...) so it can write code that reads and writes data in a Masterbase instance.
> Every running Masterbase also serves this file at `GET /llms.txt`.
> Written in English so any model reads it well; data, table names and error messages in a real instance are usually in Portuguese.

## What Masterbase is

Masterbase is a self-hosted web panel on top of a normal PostgreSQL database. People create tables and edit rows in the panel; other systems read and write the same rows through this HTTP API.

- User tables live in the Postgres schema `public`. The API only sees `public`.
- Every table you write to should have a primary key (usually a column `id`). Tables without one are read-only.
- All requests and responses are JSON (UTF-8). Base path: `{BASE_URL}/api`.

## Authentication

Use an API key. An administrator creates it in the panel: **API e chaves → Nova chave**. Send it on every request:

```
Authorization: Bearer mb_<64 hex chars>
```

| Key role | Can do |
|---|---|
| `leitor` (read only) | List tables, read schema, read rows, run queries. |
| `editor` (read and edit) | All of the above, plus insert, update, delete and import rows. |

A key can never change table structure, users, keys or backups, and cannot use the SQL editor. Keep the key on the server side (backend, serverless function, env var). Never ship it to a browser or mobile app.

## Errors

Non-2xx responses have this body:

```json
{ "error": "Human-readable message in Portuguese." }
```

| Status | Meaning | What to do |
|---|---|---|
| 400 | Invalid input: unknown column, value that does not fit the column type, required field empty, duplicate in a unique column, broken link to another table. | Show or log `error`; fix the input. Do not retry unchanged. |
| 401 | Missing, invalid or revoked key. | Check the key. |
| 403 | The key's role is not allowed to do this (e.g. `leitor` trying to write). | Use an `editor` key. |
| 404 | Table or row not found. | Check names. Table and column names are case-sensitive. |
| 413 | Body too large (256 KB normally, 8 MB on import). This one has a plain-text body, not JSON. | Split the request. |
| 500 | Server or database error. | Retry later with backoff. |

Queries running longer than 15 seconds are cancelled (400).

## Data types

Columns report a `label` (Portuguese type name) and the raw Postgres `type`.

| `label` | Postgres | JSON you get back | What to send |
|---|---|---|---|
| `texto` | text, varchar | string | string |
| `número` | integer, bigint | number | number or numeric string |
| `decimal` | numeric, real | number | number or string like `"12.50"` |
| `sim/não` | boolean | true / false | `true`, `false`, `"true"`, `"false"` |
| `data` | date | `"2026-11-17"` | `"YYYY-MM-DD"` |
| `data e hora` | timestamptz | `"2026-11-17T14:03:00+00:00"` | ISO 8601 with time zone |
| `dados` | jsonb | object/array | JSON value as a string, e.g. `"{\"a\":1}"` |
| `id único` | uuid | string | uuid string |
| `opção` | enum | string | one of the allowed values |

When writing, every value is converted by Postgres with `CAST(value AS column_type)`. Send `null` to empty a cell. A column with `link` set is a foreign key: send the primary key value of a row in the linked table.

## Endpoints

### `GET /api/tables` — list tables

```json
[{ "name": "clientes", "rows": 17 }, { "name": "pedidos", "rows": 120 }]
```

### `GET /api/schema` — all tables with their columns

Call this first to learn the structure.

```json
[
  {
    "name": "pedidos",
    "columns": [
      { "name": "id", "type": "bigint", "label": "número", "pk": true, "required": true, "unique": true, "link": null },
      { "name": "cliente", "type": "bigint", "label": "número", "pk": false, "required": true, "unique": false, "link": "clientes" },
      { "name": "total", "type": "numeric", "label": "decimal", "pk": false, "required": false, "unique": false, "link": null }
    ]
  }
]
```

- `pk`: primary key. Use its value to update or delete a row.
- `required`: NOT NULL with no default. Must be sent on insert.
- `unique`: no duplicates allowed.
- `link`: name of the table this column points to (foreign key), or `null`.

### `GET /api/tables/{table}` — read rows, 12 per page

Query parameters (all optional):

| Param | Meaning |
|---|---|
| `page` | Page number, starts at 1. |
| `col` + `val` | Filter on one column. Case-insensitive. |
| `op` | With `col`/`val`: `é` (equals, default) or `contém` (contains). URL-encode it: `%C3%A9`, `cont%C3%A9m`. |
| `sort` | Column to sort by. |
| `desc` | `true` for descending. |

```json
{
  "columns": [{ "name": "id", "label": "número", "type": "bigint", "pk": true, "required": true, "unique": true, "link": null }],
  "rows": [{ "id": 1, "nome": "Café Bom Dia", "cidade": "Curitiba" }],
  "total": 17,
  "page": 1,
  "pages": 2
}
```

To read everything, loop `page` from 1 to `pages`. For more than one filter or for up to 500 rows at once, use `/api/query`.

### `POST /api/query` — filtered query, up to 500 rows

```json
{
  "table": "clientes",
  "columns": ["nome", "cidade"],
  "filters": [
    { "col": "plano", "op": "é", "val": "pro" },
    { "col": "id", "op": "menor que", "val": "50" }
  ],
  "sort": "nome",
  "desc": false,
  "limit": 100
}
```

- `op`: `é` (equals), `não é` (not equals), `contém` (contains, case-insensitive), `maior que` (>), `menor que` (<), `vazio` (is null/empty), `não vazio` (not null). `vazio` and `não vazio` take no `val`.
- Filters are combined with AND. `val` is always a string.
- `columns` empty or missing = all columns. `limit` 1–500, default 100.

Response: `{ "columns": [...], "rows": [{...}], "sql": "SELECT ... (for display only)" }`.

### `POST /api/tables/{table}/rows` — insert one row (editor)

```json
{ "values": { "nome": "Doceria Lua", "cidade": "Maringá" } }
```

Returns the created row as an object, including generated `id` and defaults. Columns you omit get their default value.

### `PATCH /api/tables/{table}/rows` — update one cell (editor)

```json
{ "pk": 1, "column": "cidade", "value": "Londrina" }
```

`pk` is the primary key value of the row. `value` is a string or `null`. Returns the updated row. To change several columns, send one request per column.

### `DELETE /api/tables/{table}/rows` — delete rows (editor)

```json
{ "pks": [3, 4] }
```

1 to 500 keys. Returns `{ "deleted": 2 }`. Fails with 400 if another table still links to the row.

### `POST /api/tables/{table}/import` — insert many rows (editor)

```json
{ "columns": ["nome", "cidade"], "rows": [["Moinho Sul", "Pelotas"], ["Laticínios Serra", null]] }
```

Up to 5,000 rows and 8 MB. One transaction: if one row fails, none are inserted. Returns `{ "imported": 2 }`.

### `GET /api/me` — who is this key

Returns `{ "name", "email", "role", "project", "postgres" }`. Useful to test that the key works.

## Examples

### JavaScript (Node 18+, server side)

```js
const BASE_URL = process.env.MASTERBASE_URL;   // e.g. https://dados.empresa.com.br
const KEY = process.env.MASTERBASE_KEY;        // mb_...

async function mb(path, { method = 'GET', body } = {}) {
  const res = await fetch(`${BASE_URL}/api${path}`, {
    method,
    headers: { Authorization: `Bearer ${KEY}`, ...(body ? { 'Content-Type': 'application/json' } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new Error(`Masterbase ${res.status}: ${data?.error ?? res.statusText}`);
  return data;
}

const pro = await mb('/query', { method: 'POST', body: { table: 'clientes', filters: [{ col: 'plano', op: 'é', val: 'pro' }] } });
const novo = await mb('/tables/clientes/rows', { method: 'POST', body: { values: { nome: 'Doceria Lua', plano: 'pro' } } });
await mb('/tables/clientes/rows', { method: 'PATCH', body: { pk: novo.id, column: 'cidade', value: 'Maringá' } });
```

### Python

```python
import os, requests

BASE = os.environ["MASTERBASE_URL"] + "/api"
H = {"Authorization": f"Bearer {os.environ['MASTERBASE_KEY']}"}

def mb(method, path, body=None):
    r = requests.request(method, BASE + path, headers=H, json=body, timeout=30)
    if not r.ok:
        raise RuntimeError(f"Masterbase {r.status_code}: {r.json().get('error')}")
    return r.json()

rows = mb("POST", "/query", {"table": "pedidos", "filters": [{"col": "status", "op": "é", "val": "aberto"}], "limit": 500})["rows"]
mb("DELETE", "/tables/pedidos/rows", {"pks": [r["id"] for r in rows if r["total"] is None]})
```

### curl

```bash
curl -H "Authorization: Bearer $MASTERBASE_KEY" "$MASTERBASE_URL/api/schema"
```

## Rules for code you generate

1. Call `GET /api/schema` (or ask the user for it) before writing queries. Never guess table or column names.
2. Read the key from an environment variable. Never hard-code it, never send it to the browser.
3. Always check `res.ok` and surface the `error` message.
4. For many inserts use `/import` (one transaction) instead of a loop of single inserts.
5. Identify rows by the primary key (`pk: true` column), not by position.
6. The API has no raw SQL, no joins, no aggregates and no realtime. Fetch rows and join or aggregate in your code.
7. There is no rate limit yet, but be polite: batch writes and avoid tight polling loops.

## What the API does not do

- Create or change tables (only administrators in the panel).
- Run raw SQL (the SQL editor is panel-only, for administrators).
- Push changes in real time (poll instead).
- Upload files.
