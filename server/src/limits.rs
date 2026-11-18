//! Tentativas erradas, guardadas no banco: reiniciar o serviço não zera um bloqueio.
//! Janela de 15 minutos a partir do primeiro erro. Chaves: e-mail, "ip:...", "forgot:...", "key:ID".

use crate::error::ApiResult;
use sqlx::PgPool;

pub async fn blocked(db: &PgPool, key: &str, max: i32) -> ApiResult<bool> {
    let n: Option<bool> = sqlx::query_scalar(
        "SELECT fails >= $2 FROM masterbase.attempts WHERE key = $1 AND started_at > now() - interval '15 minutes'",
    )
    .bind(key)
    .bind(max)
    .fetch_optional(db)
    .await?;
    Ok(n.unwrap_or(false))
}

pub async fn fail(db: &PgPool, key: &str) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO masterbase.attempts AS a (key, fails) VALUES ($1, 1)
         ON CONFLICT (key) DO UPDATE SET
           fails = CASE WHEN a.started_at <= now() - interval '15 minutes' THEN 1 ELSE a.fails + 1 END,
           started_at = CASE WHEN a.started_at <= now() - interval '15 minutes' THEN now() ELSE a.started_at END",
    )
    .bind(key)
    .execute(db)
    .await?;
    sqlx::query("DELETE FROM masterbase.attempts WHERE started_at < now() - interval '1 day'").execute(db).await?;
    Ok(())
}

pub async fn clear(db: &PgPool, key: &str) -> ApiResult<()> {
    sqlx::query("DELETE FROM masterbase.attempts WHERE key = $1").bind(key).execute(db).await?;
    Ok(())
}
