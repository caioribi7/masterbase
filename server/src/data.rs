//! Tela Tabelas: ler, filtrar, ordenar, editar, inserir e apagar linhas.
use crate::auth::User;
use crate::projects::Db;
use crate::catalog::{columns, ident, table_counts, Col};
use crate::error::{bad, ApiError, ApiResult};
use crate::rules::owner;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};

const PAGE: i64 = 12;
const BIG: i64 = 1_000_000;

pub async fn list_tables(user: User, p: Db) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    let out: Vec<Value> = table_counts(&db, crate::rules::all_for(&p, &user).await?.as_ref()).await?.into_iter().map(|(name, rows)| json!({ "name": name, "rows": rows })).collect();
    Ok(Json(Value::Array(out)))
}

#[derive(Deserialize)]
pub struct ReadQuery {
    page: Option<i64>,
    col: Option<String>,
    val: Option<String>,
    op: Option<String>,
    sort: Option<String>,
    desc: Option<bool>,
}

fn find<'a>(cols: &'a [Col], name: &str) -> ApiResult<&'a Col> {
    cols.iter().find(|c| c.name == name).ok_or(bad("Coluna não existe."))
}

fn pk_of(cols: &[Col]) -> ApiResult<&Col> {
    cols.iter().find(|c| c.pk).ok_or(bad("Tabela sem chave primária: só leitura."))
}

pub async fn read_table(user: User, p: Db, Path(table): Path<String>, Query(q): Query<ReadQuery>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    let cols = columns(&db, Some(&table)).await?;
    let mine = owner(&p, &user, &table).await?;
    let mut conds = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    if let Some(o) = &mine {
        binds.push(o.email.clone());
        conds.push(o.cond(1));
    }
    if let (Some(c), Some(v)) = (&q.col, &q.val) {
        if !v.is_empty() {
            let col = ident(&find(&cols, c)?.name);
            binds.push(v.clone());
            let n = binds.len();
            conds.push(match q.op.as_deref() {
                Some("contém") => format!("position(lower(${n}) in lower(t.{col}::text)) > 0"),
                _ => format!("lower(t.{col}::text) = lower(${n})"),
            });
        }
    }
    let where_sql = if conds.is_empty() { String::new() } else { format!("WHERE {}", conds.join(" AND ")) };
    let from = format!("FROM public.{} t {where_sql}", ident(&table));
    let pk_order = cols.iter().find(|c| c.pk).map(|c| format!("t.{}", ident(&c.name))).unwrap_or_else(|| "t.ctid".into());
    let order = match &q.sort {
        Some(s) => format!(
            "t.{} {} NULLS LAST, {pk_order}",
            ident(&find(&cols, s)?.name),
            if q.desc.unwrap_or(false) { "DESC" } else { "ASC" }
        ),
        None => pk_order,
    };

    // count(*) fica lento em tabela enorme. Sem filtro e acima de 1 milhão, vale a estimativa do Postgres;
    // com filtro, conta até 1 milhão e para. A tela mostra "cerca de".
    let estimate: i64 = sqlx::query_scalar("SELECT coalesce((SELECT reltuples::bigint FROM pg_class WHERE oid = to_regclass($1)), 0)")
        .bind(format!("public.{}", ident(&table)))
        .fetch_one(&db)
        .await?;
    let (total, approx) = if binds.is_empty() && estimate > BIG {
        (estimate, true)
    } else {
        let count_sql = format!("SELECT count(*) FROM (SELECT 1 {from} LIMIT {}) s", BIG + 1);
        let mut cq = sqlx::query_scalar(&count_sql);
        for b in &binds {
            cq = cq.bind(b);
        }
        let n: i64 = cq.fetch_one(&db).await?;
        (n.min(BIG), n > BIG)
    };
    let pages = ((total + PAGE - 1) / PAGE).max(1);
    let page = q.page.unwrap_or(1).clamp(1, pages);
    let rows_sql = format!("SELECT row_to_json(t)::jsonb {from} ORDER BY {order} LIMIT {PAGE} OFFSET {}", (page - 1) * PAGE);
    let mut rq = sqlx::query_scalar(&rows_sql);
    for b in &binds {
        rq = rq.bind(b);
    }
    let rows: Vec<Value> = rq.fetch_all(&db).await?;

    let columns: Vec<Value> = cols.iter().map(Col::json).collect();
    // owner: coluna que a pessoa não escolhe (vem do e-mail dela). A tela trava essa coluna.
    let owner = mine.map(|o| o.column);
    Ok(Json(json!({ "columns": columns, "rows": rows, "total": total, "approx": approx, "page": page, "pages": pages, "owner": owner })))
}

const OWNER_COL: &str = "Essa coluna diz de quem é a linha e só o administrador muda.";

fn as_text(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

#[derive(Deserialize)]
pub struct UpdateBody {
    pk: Value,
    column: String,
    value: Option<String>,
}

pub async fn update_cell(user: User, p: Db, Path(table): Path<String>, Json(b): Json<UpdateBody>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    user.editor()?;
    let cols = columns(&db, Some(&table)).await?;
    let pk = pk_of(&cols)?;
    let col = find(&cols, &b.column)?;
    if col.pk {
        return Err(bad("A chave da linha não pode ser editada."));
    }
    let pk_text = as_text(&b.pk).ok_or(bad("Linha inválida."))?;
    let mine = owner(&p, &user, &table).await?;
    if mine.as_ref().is_some_and(|o| o.column == col.name) {
        return Err(bad(OWNER_COL));
    }
    let only_mine = mine.as_ref().map(|o| format!(" AND {}", o.cond(3))).unwrap_or_default();
    // col.ty vem de format_type() do catálogo, não do usuário.
    let sql = format!(
        "UPDATE public.{} AS t SET {} = CAST($1 AS {}) WHERE t.{}::text = $2{only_mine} RETURNING row_to_json(t)::jsonb",
        ident(&table), ident(&col.name), col.ty, ident(&pk.name)
    );
    let mut q = sqlx::query_scalar(&sql).bind(&b.value).bind(&pk_text);
    if let Some(o) = &mine {
        q = q.bind(&o.email);
    }
    let row: Value = q
        .fetch_optional(&db)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Linha não encontrada."))?;
    p.log(user.id, "update", json!({ "table": table, "column": col.name, "pk": pk_text })).await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct InsertBody {
    values: Map<String, Value>,
}

pub async fn insert_row(user: User, p: Db, Path(table): Path<String>, Json(b): Json<InsertBody>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    user.editor()?;
    let cols = columns(&db, Some(&table)).await?;
    let mine = owner(&p, &user, &table).await?;
    let mut values = b.values;
    // Com regra por linha, a linha nova é sempre de quem cria.
    if let Some(o) = &mine {
        values.insert(o.column.clone(), Value::String(o.email.clone()));
    }
    let mut names = Vec::new();
    let mut casts = Vec::new();
    let mut vals: Vec<Option<String>> = Vec::new();
    for (key, v) in &values {
        let col = find(&cols, key)?;
        names.push(ident(&col.name));
        casts.push(format!("CAST(${} AS {})", vals.len() + 1, col.ty));
        vals.push(as_text(v));
    }
    let sql = if names.is_empty() {
        format!("INSERT INTO public.{} AS t DEFAULT VALUES RETURNING row_to_json(t)::jsonb", ident(&table))
    } else {
        format!(
            "INSERT INTO public.{} AS t ({}) VALUES ({}) RETURNING row_to_json(t)::jsonb",
            ident(&table), names.join(", "), casts.join(", ")
        )
    };
    let mut q = sqlx::query_scalar(&sql);
    for v in &vals {
        q = q.bind(v);
    }
    let row: Value = q.fetch_one(&db).await?;
    p.log(user.id, "insert", json!({ "table": table })).await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct DeleteBody {
    pks: Vec<Value>,
}

pub async fn delete_rows(user: User, p: Db, Path(table): Path<String>, Json(b): Json<DeleteBody>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    user.editor()?;
    if b.pks.is_empty() || b.pks.len() > 500 {
        return Err(bad("Escolha de 1 a 500 linhas."));
    }
    let cols = columns(&db, Some(&table)).await?;
    let pk = pk_of(&cols)?;
    let keys: Vec<String> = b.pks.iter().filter_map(as_text).collect();
    let mine = owner(&p, &user, &table).await?;
    let only_mine = mine.as_ref().map(|o| format!(" AND {}", o.cond(2))).unwrap_or_default();
    let sql = format!("DELETE FROM public.{} AS t WHERE t.{}::text = ANY($1){only_mine}", ident(&table), ident(&pk.name));
    let mut q = sqlx::query(&sql).bind(&keys);
    if let Some(o) = &mine {
        q = q.bind(&o.email);
    }
    let n = q.execute(&db).await?.rows_affected();
    p.log(user.id, "delete", json!({ "table": table, "rows": n })).await?;
    Ok(Json(json!({ "deleted": n })))
}

#[derive(Deserialize)]
pub struct ImportBody {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
}

/// Importar CSV: a tela lê o arquivo e manda as linhas. Uma transação: ou entra tudo, ou nada.
pub async fn import_rows(user: User, p: Db, Path(table): Path<String>, Json(b): Json<ImportBody>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    user.editor()?;
    if b.columns.is_empty() || b.rows.is_empty() || b.rows.len() > 5000 {
        return Err(bad("Envie de 1 a 5.000 linhas por vez."));
    }
    let cols = columns(&db, Some(&table)).await?;
    let mine = owner(&p, &user, &table).await?;
    let mut names = Vec::new();
    let mut casts = Vec::new();
    for (i, key) in b.columns.iter().enumerate() {
        let col = cols.iter().find(|c| &c.name == key).ok_or(bad("O arquivo tem uma coluna que a tabela não tem."))?;
        names.push(ident(&col.name));
        casts.push(format!("CAST(${} AS {})", i + 1, col.ty));
    }
    // Com regra por linha, toda linha importada é de quem importa: a coluna do dono leva o e-mail da pessoa.
    let owner_at = match &mine {
        Some(o) => match b.columns.iter().position(|c| c == &o.column) {
            Some(i) => Some(i),
            None => {
                let col = find(&cols, &o.column)?;
                names.push(ident(&col.name));
                casts.push(format!("CAST(${} AS {})", b.columns.len() + 1, col.ty));
                Some(b.columns.len())
            }
        },
        None => None,
    };
    let sql = format!("INSERT INTO public.{} ({}) VALUES ({})", ident(&table), names.join(", "), casts.join(", "));
    let mut tx = db.begin().await?;
    for row in &b.rows {
        if row.len() != b.columns.len() {
            return Err(bad("Uma linha do arquivo tem número de campos diferente do cabeçalho."));
        }
        let mut q = sqlx::query(&sql);
        for (i, v) in row.iter().enumerate() {
            if owner_at == Some(i) {
                q = q.bind(mine.as_ref().map(|o| o.email.as_str()));
            } else {
                q = q.bind(v.as_deref().filter(|s| !s.is_empty()));
            }
        }
        if owner_at == Some(row.len()) {
            q = q.bind(mine.as_ref().map(|o| o.email.as_str()));
        }
        q.execute(&mut *tx).await?;
    }
    tx.commit().await?;
    p.log(user.id, "import", json!({ "table": table, "rows": b.rows.len() })).await?;
    Ok(Json(json!({ "imported": b.rows.len() })))
}
