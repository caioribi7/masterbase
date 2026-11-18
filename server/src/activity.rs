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
    let email: Option<String> = sqlx::query_scalar(
        "WITH a AS (INSERT INTO masterbase.activity (user_id, action, detail) VALUES ($1, $2, $3))
         SELECT email FROM masterbase.users WHERE id = $1",
    )
    .bind(user_id)
    .bind(action)
    .bind(&detail)
    .fetch_optional(db)
    .await?;
    if logging() {
        println!("{} {} {action} {detail}", clock(), email.as_deref().unwrap_or("?"));
    }
    Ok(())
}

/// Registro feito pelo próprio Masterbase (backup automático): sem usuário.
pub async fn log_system(db: &PgPool, action: &str, detail: Value) -> ApiResult<()> {
    sqlx::query("INSERT INTO masterbase.activity (user_id, action, detail) VALUES (NULL, $1, $2)")
        .bind(action)
        .bind(&detail)
        .execute(db)
        .await?;
    if logging() {
        println!("{} sistema {action} {detail}", clock());
    }
    Ok(())
}

/// Log no terminal: liga por padrão, MB_LOG=off desliga.
pub fn logging() -> bool {
    std::env::var("MB_LOG").as_deref() != Ok("off")
}

/// Hora local do servidor não está disponível sem dependência extra: usa UTC, marcado com Z.
pub fn clock() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{:02}:{:02}:{:02}Z", s / 3600 % 24, s / 60 % 60, s % 60)
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

pub async fn overview(user: User, p: crate::projects::Db) -> ApiResult<Json<Value>> {
    let db = p.meta.clone();
    let (size, version): (String, String) =
        sqlx::query_as("SELECT pg_size_pretty(pg_database_size(current_database())), current_setting('server_version')")
            .fetch_one(&p.data)
            .await?;
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM masterbase.users").fetch_one(&db).await?;
    let tables = table_counts(&p.data, crate::rules::all_for(&p, &user).await?.as_ref()).await?;
    let rows: i64 = tables.iter().map(|t| t.1).sum();
    let tables: Vec<Value> = tables.into_iter().map(|(name, rows)| json!({ "name": name, "rows": rows })).collect();
    let recent = recent(&db, None, None, 6).await?;
    Ok(Json(json!({ "size": size, "version": version, "users": users, "rows": rows, "tables": tables, "recent": recent })))
}
