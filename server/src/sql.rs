//! Editor SQL e Configurações. Editor só para administradores.
//! O SQL do editor roda num papel à parte (MB_SQL_URL): lê e muda linhas das tabelas de public,
//! mas não cria nem apaga tabelas, não vê o schema masterbase (senhas, sessões, chaves) e não cria gatilhos.
use crate::activity::log;
use crate::auth::User;
use crate::error::{bad, ApiResult};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Column, Either, PgPool, Row};
use std::sync::OnceLock;
use std::time::Instant;

static SQL_POOL: OnceLock<PgPool> = OnceLock::new();

/// Liga o editor SQL se MB_SQL_URL existir. Antes, como dono, dá ao papel do editor só o que ele pode fazer.
/// Sem MB_SQL_URL o editor fica desligado: nunca roda SQL livre como dono do banco.
pub async fn connect(owner: &PgPool) {
    let Ok(url) = std::env::var("MB_SQL_URL") else { return };
    let Ok(opts) = url.parse::<sqlx::postgres::PgConnectOptions>() else {
        eprintln!("MB_SQL_URL inválida: editor SQL desligado");
        return;
    };
    let role = crate::catalog::ident(opts.get_username());
    let grants = [
        format!("GRANT USAGE ON SCHEMA public TO {role}"),
        format!("REVOKE CREATE ON SCHEMA public FROM {role}"),
        format!("REVOKE ALL ON SCHEMA masterbase FROM {role}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES ON ALL TABLES IN SCHEMA public TO {role}"),
        format!("GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO {role}"),
        format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES ON TABLES TO {role}"),
        format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO {role}"),
    ];
    for g in &grants {
        if let Err(e) = sqlx::query(g).execute(owner).await {
            eprintln!("editor SQL desligado: não deu para ajustar permissões ({e})");
            return;
        }
    }
    // Postgres antigo (antes do 15) deixa qualquer papel criar objetos em public. Tira, se der.
    let _ = sqlx::query("REVOKE CREATE ON SCHEMA public FROM PUBLIC").execute(owner).await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .after_connect(|conn, _| Box::pin(async move {
            sqlx::Executor::execute(conn, "SET search_path = public; SET statement_timeout = '15s'").await.map(|_| ())
        }))
        .connect_lazy_with(opts);
    match sqlx::query_scalar::<_, bool>("SELECT NOT rolsuper AND NOT rolcreaterole AND NOT rolcreatedb FROM pg_roles WHERE rolname = current_user")
        .fetch_one(&pool)
        .await
    {
        Ok(true) => {
            let _ = SQL_POOL.set(pool);
        }
        Ok(false) => eprintln!("editor SQL desligado: o papel de MB_SQL_URL tem poderes demais (superusuário ou cria papéis/bancos)"),
        Err(e) => eprintln!("editor SQL desligado: sem conexão por MB_SQL_URL ({e})"),
    }
}

pub fn sql_enabled() -> bool {
    SQL_POOL.get().is_some()
}

const MAX_ROWS: usize = 1000;

#[derive(Deserialize)]
pub struct SqlBody {
    sql: String,
    #[serde(default)]
    read_only: bool,
}

/// Roda o texto como está, numa transação. Devolve o resultado do último comando.
/// Protocolo simples do Postgres: todo valor chega como texto, então qualquer tipo aparece.
pub async fn run(user: User, p: crate::projects::Db, Json(b): Json<SqlBody>) -> Result<Json<Value>, Response> {
    user.admin().map_err(IntoResponse::into_response)?;
    // Projeto principal: papel restrito (MB_SQL_URL). Outro projeto: o usuário do endereço que o administrador conectou.
    let pool = if p.project != 0 {
        &p.data
    } else {
        match SQL_POOL.get() {
            Some(pool) => pool,
            None => return Err(bad("O editor SQL está desligado neste servidor. Veja Configurações.").into_response()),
        }
    };
    let sql = b.sql.trim();
    if sql.is_empty() {
        return Err(bad("Escreva um comando SQL.").into_response());
    }
    if sql.len() > 100_000 {
        return Err(bad("Comando grande demais (até 100 mil caracteres).").into_response());
    }
    let started = Instant::now();
    let result = execute(pool, sql, b.read_only).await.map_err(db_error)?;
    if !b.read_only {
        let short: String = sql.chars().take(500).collect();
        p.log(user.id, "sql", json!({ "sql": short })).await.map_err(IntoResponse::into_response)?;
    }
    let (columns, rows, affected, truncated, statements) = result;
    Ok(Json(json!({
        "columns": columns,
        "rows": rows,
        "affected": affected,
        "truncated": truncated,
        "statements": statements,
        "ms": started.elapsed().as_millis() as u64,
    })))
}

type Outcome = (Vec<String>, Vec<Vec<Value>>, u64, bool, u32);

async fn execute(db: &PgPool, sql: &str, read_only: bool) -> Result<Outcome, sqlx::Error> {
    let mut tx = db.begin().await?;
    if read_only {
        sqlx::query("SET TRANSACTION READ ONLY").execute(&mut *tx).await?;
    }
    let (mut columns, mut rows, mut truncated) = (Vec::new(), Vec::new(), false);
    let mut last: Outcome = (Vec::new(), Vec::new(), 0, false, 0);
    let mut statements = 0;
    {
        let mut stream = sqlx::raw_sql(sql).fetch_many(&mut *tx);
        while let Some(item) = stream.try_next().await? {
            match item {
                Either::Right(row) => {
                    if columns.is_empty() {
                        columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                    }
                    if rows.len() >= MAX_ROWS {
                        truncated = true;
                        continue;
                    }
                    let cells = (0..row.len())
                        .map(|i| match row.try_get_unchecked::<Option<String>, _>(i) {
                            Ok(Some(s)) => Value::String(s),
                            _ => Value::Null,
                        })
                        .collect();
                    rows.push(cells);
                }
                Either::Left(done) => {
                    statements += 1;
                    last = (std::mem::take(&mut columns), std::mem::take(&mut rows), done.rows_affected(), truncated, 0);
                    truncated = false;
                }
            }
        }
    }
    tx.commit().await?;
    last.4 = statements;
    Ok(last)
}

/// No editor, quem escreve SQL quer a mensagem do Postgres como ela é.
fn db_error(e: sqlx::Error) -> Response {
    match e.as_database_error() {
        Some(d) => {
            let msg = if d.code().as_deref() == Some("25006") {
                "Modo só leitura: desmarque a opção para mudar dados.".to_string()
            } else if d.code().as_deref() == Some("57014") {
                "Demorou mais de 15 segundos e foi cancelado.".to_string()
            } else if d.code().as_deref() == Some("42501") {
                "O editor não tem permissão para isso: ele lê e muda as linhas das suas tabelas. Para criar ou mudar tabelas e colunas, use Estrutura.".to_string()
            } else {
                d.message().to_string()
            };
            (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
        }
        None => crate::error::ApiError::from(e).into_response(),
    }
}

/// Página Configurações: como este Masterbase está rodando. Sem senha do banco.
pub async fn settings(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let (database, pg, size, name, session_days): (String, String, String, String, i32) = sqlx::query_as(
        "SELECT current_database()::text, current_setting('server_version'), pg_size_pretty(pg_database_size(current_database())),
                s.name, s.session_days FROM masterbase.settings s",
    )
    .fetch_one(&db)
    .await?;
    let host = std::env::var("DATABASE_URL")
        .ok()
        .and_then(|u| u.rsplit_once('@').map(|(_, h)| h.split('/').next().unwrap_or("").to_string()))
        .unwrap_or_default();
    let addr = std::env::var("MB_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let access = if addr.starts_with("127.") || addr.starts_with("localhost") { "local" } else { "rede" };
    Ok(Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "database": database,
        "host": host,
        "postgres": pg,
        "size": size,
        "addr": addr,
        "access": access,
        "https_cookie": std::env::var("MB_INSECURE_COOKIE").as_deref() != Ok("1"),
        "backup_dir": std::env::var("MB_BACKUP_DIR").unwrap_or_else(|_| "backups".into()),
        "config": crate::config::path().to_string_lossy(),
        "sql_editor": sql_enabled(),
        "name": name,
        "session_days": session_days,
        "url": crate::mail::public_url(),
        "mail": crate::mail::enabled(),
        "oauth": crate::oauth::enabled(),
        "key_rate": crate::auth::key_limit(),
    })))
}

#[derive(Deserialize)]
pub struct InstanceChange {
    name: Option<String>,
    session_days: Option<i32>,
}

/// Nome que aparece na tela e quanto tempo dura uma sessão.
pub async fn change_settings(user: User, State(db): State<PgPool>, Json(b): Json<InstanceChange>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name = b.name.as_deref().map(str::trim);
    if name.is_some_and(|n| n.chars().count() > 60) {
        return Err(bad("Nome com até 60 caracteres."));
    }
    if b.session_days.is_some_and(|d| !(1..=90).contains(&d)) {
        return Err(bad("A sessão dura de 1 a 90 dias."));
    }
    sqlx::query("UPDATE masterbase.settings SET name = coalesce($1, name), session_days = coalesce($2, session_days)")
        .bind(name)
        .bind(b.session_days)
        .execute(&db)
        .await?;
    log(&db, user.id, "settings", json!({ "name": name, "session_days": b.session_days })).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct Profile {
    name: String,
}

/// Mudar o próprio nome.
pub async fn update_me(user: User, State(db): State<PgPool>, Json(b): Json<Profile>) -> ApiResult<Json<Value>> {
    if user.via_key {
        return Err(bad("Chave de API não tem perfil."));
    }
    let name = b.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(bad("Nome precisa ter de 1 a 80 caracteres."));
    }
    sqlx::query("UPDATE masterbase.users SET name = $1 WHERE id = $2").bind(name).bind(user.id).execute(&db).await?;
    Ok(Json(json!({ "ok": true })))
}
