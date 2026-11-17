//! Auditoria (página Atividade) e resumo da página Início.
use crate::auth::User;
use crate::catalog::table_counts;
use crate::error::ApiResult;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

pub async fn log(db: &PgPool, user_id: i64, action: &str, detail: Value) -> ApiResult<()> {
    sqlx::query("INSERT INTO masterbase.activity (user_id, action, detail) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(action)
        .bind(detail)
        .execute(db)
        .await?;
    Ok(())
}

async fn recent(db: &PgPool, table: Option<&str>, before: Option<i64>, limit: i64) -> ApiResult<Vec<Value>> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('id', a.id, 'action', a.action, 'detail', a.detail, 'at', a.at, 'user', u.name)
         FROM masterbase.activity a LEFT JOIN masterbase.users u ON u.id = a.user_id
         WHERE ($1::text IS NULL OR a.detail->>'table' = $1 OR a.detail->>'to' = $1)
           AND ($2::bigint IS NULL OR a.id < $2)
         ORDER BY a.id DESC LIMIT $3",
    )
    .bind(table)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    table: Option<String>,
    before: Option<i64>,
}

const ACTIVITY_PAGE: i64 = 30;

pub async fn list(_user: User, State(db): State<PgPool>, Query(q): Query<ActivityQuery>) -> ApiResult<Json<Value>> {
    let table = q.table.as_deref().filter(|t| !t.is_empty());
    let items = recent(&db, table, q.before, ACTIVITY_PAGE + 1).await?;
    let more = items.len() as i64 > ACTIVITY_PAGE;
    let items: Vec<Value> = items.into_iter().take(ACTIVITY_PAGE as usize).collect();
    Ok(Json(json!({ "items": items, "more": more })))
}

pub async fn overview(_user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let (size, version, users): (String, String, i64) = sqlx::query_as(
        "SELECT pg_size_pretty(pg_database_size(current_database())), current_setting('server_version'),
                (SELECT count(*) FROM masterbase.users)",
    )
    .fetch_one(&db)
    .await?;
    let tables = table_counts(&db).await?;
    let rows: i64 = tables.iter().map(|t| t.1).sum();
    let tables: Vec<Value> = tables.into_iter().map(|(name, rows)| json!({ "name": name, "rows": rows })).collect();
    let recent = recent(&db, None, None, 6).await?;
    Ok(Json(json!({ "size": size, "version": version, "users": users, "rows": rows, "tables": tables, "recent": recent })))
}
