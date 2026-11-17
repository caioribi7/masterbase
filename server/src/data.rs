//! Tela Tabelas: ler, filtrar, ordenar, editar, inserir e apagar linhas.
use crate::activity::log;
use crate::auth::User;
use crate::catalog::{columns, ident, table_counts, Col};
use crate::error::{bad, ApiError, ApiResult};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::PgPool;

const PAGE: i64 = 12;

pub async fn list_tables(_user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let out: Vec<Value> = table_counts(&db).await?.into_iter().map(|(name, rows)| json!({ "name": name, "rows": rows })).collect();
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

pub async fn read_table(_user: User, State(db): State<PgPool>, Path(table): Path<String>, Query(q): Query<ReadQuery>) -> ApiResult<Json<Value>> {
    let cols = columns(&db, Some(&table)).await?;
    let mut where_sql = String::new();
    let mut val = String::new();
    if let (Some(c), Some(v)) = (&q.col, &q.val) {
        if !v.is_empty() {
            let col = ident(&find(&cols, c)?.name);
            where_sql = match q.op.as_deref() {
                Some("contém") => format!("WHERE position(lower($1) in lower(t.{col}::text)) > 0"),
                _ => format!("WHERE lower(t.{col}::text) = lower($1)"),
            };
            val = v.clone();
        }
    }
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

    let total: i64 = bind_opt(sqlx::query_scalar(&format!("SELECT count(*) {from}")), &val).fetch_one(&db).await?;
    let pages = ((total + PAGE - 1) / PAGE).max(1);
    let page = q.page.unwrap_or(1).clamp(1, pages);
    let rows: Vec<Value> = bind_opt(
        sqlx::query_scalar(&format!("SELECT row_to_json(t)::jsonb {from} ORDER BY {order} LIMIT {PAGE} OFFSET {}", (page - 1) * PAGE)),
        &val,
    )
    .fetch_all(&db)
    .await?;

    let columns: Vec<Value> = cols.iter().map(Col::json).collect();
    Ok(Json(json!({ "columns": columns, "rows": rows, "total": total, "page": page, "pages": pages })))
}

fn bind_opt<'q, O>(
    q: sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    val: &'q str,
) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    if val.is_empty() { q } else { q.bind(val) }
}

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

pub async fn update_cell(user: User, State(db): State<PgPool>, Path(table): Path<String>, Json(b): Json<UpdateBody>) -> ApiResult<Json<Value>> {
    user.editor()?;
    let cols = columns(&db, Some(&table)).await?;
    let pk = pk_of(&cols)?;
    let col = find(&cols, &b.column)?;
    if col.pk {
        return Err(bad("A chave da linha não pode ser editada."));
    }
    let pk_text = as_text(&b.pk).ok_or(bad("Linha inválida."))?;
    // col.ty vem de format_type() do catálogo, não do usuário.
    let sql = format!(
        "UPDATE public.{} AS t SET {} = CAST($1 AS {}) WHERE t.{}::text = $2 RETURNING row_to_json(t)::jsonb",
        ident(&table), ident(&col.name), col.ty, ident(&pk.name)
    );
    let row: Value = sqlx::query_scalar(&sql)
        .bind(&b.value)
        .bind(&pk_text)
        .fetch_optional(&db)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Linha não encontrada."))?;
    log(&db, user.id, "update", json!({ "table": table, "column": col.name, "pk": pk_text })).await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct InsertBody {
    values: Map<String, Value>,
}

pub async fn insert_row(user: User, State(db): State<PgPool>, Path(table): Path<String>, Json(b): Json<InsertBody>) -> ApiResult<Json<Value>> {
    user.editor()?;
    let cols = columns(&db, Some(&table)).await?;
    let mut names = Vec::new();
    let mut casts = Vec::new();
    let mut vals: Vec<Option<String>> = Vec::new();
    for (key, v) in &b.values {
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
    log(&db, user.id, "insert", json!({ "table": table })).await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct DeleteBody {
    pks: Vec<Value>,
}

pub async fn delete_rows(user: User, State(db): State<PgPool>, Path(table): Path<String>, Json(b): Json<DeleteBody>) -> ApiResult<Json<Value>> {
    user.editor()?;
    if b.pks.is_empty() || b.pks.len() > 500 {
        return Err(bad("Escolha de 1 a 500 linhas."));
    }
    let cols = columns(&db, Some(&table)).await?;
    let pk = pk_of(&cols)?;
    let keys: Vec<String> = b.pks.iter().filter_map(as_text).collect();
    let sql = format!("DELETE FROM public.{} AS t WHERE t.{}::text = ANY($1)", ident(&table), ident(&pk.name));
    let n = sqlx::query(&sql).bind(&keys).execute(&db).await?.rows_affected();
    log(&db, user.id, "delete", json!({ "table": table, "rows": n })).await?;
    Ok(Json(json!({ "deleted": n })))
}

#[derive(Deserialize)]
pub struct ImportBody {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
}

/// Importar CSV: a tela lê o arquivo e manda as linhas. Uma transação: ou entra tudo, ou nada.
pub async fn import_rows(user: User, State(db): State<PgPool>, Path(table): Path<String>, Json(b): Json<ImportBody>) -> ApiResult<Json<Value>> {
    user.editor()?;
    if b.columns.is_empty() || b.rows.is_empty() || b.rows.len() > 5000 {
        return Err(bad("Envie de 1 a 5.000 linhas por vez."));
    }
    let cols = columns(&db, Some(&table)).await?;
    let mut names = Vec::new();
    let mut casts = Vec::new();
    for (i, key) in b.columns.iter().enumerate() {
        let col = cols.iter().find(|c| &c.name == key).ok_or(bad("O arquivo tem uma coluna que a tabela não tem."))?;
        names.push(ident(&col.name));
        casts.push(format!("CAST(${} AS {})", i + 1, col.ty));
    }
    let sql = format!("INSERT INTO public.{} ({}) VALUES ({})", ident(&table), names.join(", "), casts.join(", "));
    let mut tx = db.begin().await?;
    for row in &b.rows {
        if row.len() != b.columns.len() {
            return Err(bad("Uma linha do arquivo tem número de campos diferente do cabeçalho."));
        }
        let mut q = sqlx::query(&sql);
        for v in row {
            q = q.bind(v.as_deref().filter(|s| !s.is_empty()));
        }
        q.execute(&mut *tx).await?;
    }
    tx.commit().await?;
    log(&db, user.id, "import", json!({ "table": table, "rows": b.rows.len() })).await?;
    Ok(Json(json!({ "imported": b.rows.len() })))
}
