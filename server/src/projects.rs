//! Vários projetos: além do banco principal, outros bancos Postgres conectados pelo endereço.
//! A tela manda o cabeçalho X-MB-Project com o id do projeto aberto (0 ou nada = principal).
//! Contas, sessões, chaves, atividade e regras ficam sempre no banco principal (meta); as tabelas, no do projeto (data).
use crate::activity::log;
use crate::auth::User;
use crate::error::{bad, ApiError, ApiResult};
use axum::extract::{FromRef, FromRequestParts, Path, State};
use axum::http::{request::Parts, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

pub const HEADER: &str = "x-mb-project";
const GONE: &str = "Projeto não existe.";

fn pools() -> &'static Mutex<HashMap<i64, PgPool>> {
    static P: OnceLock<Mutex<HashMap<i64, PgPool>>> = OnceLock::new();
    P.get_or_init(Default::default)
}

/// Banco do pedido: `data` tem as tabelas do projeto aberto, `meta` é sempre o principal.
pub struct Db {
    pub data: PgPool,
    pub meta: PgPool,
    pub project: i64,
}

fn open(opts: PgConnectOptions) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(|conn, _| Box::pin(async move {
            sqlx::Executor::execute(conn, "SET search_path = public; SET statement_timeout = '15s'").await.map(|_| ())
        }))
        .connect_lazy_with(opts)
}

async fn pool(meta: &PgPool, id: i64) -> ApiResult<PgPool> {
    if let Some(p) = pools().lock().unwrap_or_else(|e| e.into_inner()).get(&id) {
        return Ok(p.clone());
    }
    let url: String = sqlx::query_scalar("SELECT url FROM masterbase.projects WHERE id = $1")
        .bind(id)
        .fetch_optional(meta)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, GONE))?;
    let opts: PgConnectOptions = url.parse().map_err(|_| bad("Endereço do projeto inválido."))?;
    let p = open(opts);
    pools().lock().unwrap_or_else(|e| e.into_inner()).insert(id, p.clone());
    Ok(p)
}

impl<S: Send + Sync> FromRequestParts<S> for Db
where
    PgPool: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        let meta = PgPool::from_ref(state);
        let project: i64 = parts.headers.get(HEADER).and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
        let data = if project == 0 { meta.clone() } else { pool(&meta, project).await? };
        Ok(Db { data, meta, project })
    }
}

/// "host/banco" para mostrar na tela. Nunca a senha.
fn place(url: &str) -> String {
    url.parse::<PgConnectOptions>()
        .map(|o| format!("{}/{}", o.get_host(), o.get_database().unwrap_or("")))
        .unwrap_or_default()
}

pub async fn list(_user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let main: String = sqlx::query_scalar("SELECT coalesce(nullif(name, ''), current_database()::text) FROM masterbase.settings")
        .fetch_one(&db)
        .await?;
    let rows: Vec<(i64, String, String)> = sqlx::query_as("SELECT id, name, url FROM masterbase.projects ORDER BY name").fetch_all(&db).await?;
    let mut out = vec![json!({ "id": 0, "name": main, "main": true })];
    out.extend(rows.into_iter().map(|(id, name, url)| json!({ "id": id, "name": name, "place": place(&url), "main": false })));
    Ok(Json(Value::Array(out)))
}

#[derive(Deserialize)]
pub struct NewProject {
    name: String,
    url: String,
}

pub async fn create(user: User, State(db): State<PgPool>, Json(b): Json<NewProject>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name = b.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(bad("Dê um nome ao projeto (até 60 caracteres)."));
    }
    let url = b.url.trim();
    if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        return Err(bad("O endereço começa com postgres:// (ex.: postgres://usuario:senha@host:5432/banco)."));
    }
    let opts: PgConnectOptions = url.parse().map_err(|_| bad("Endereço inválido."))?;
    // Testa antes de guardar: conecta, e recusa o próprio banco do Masterbase (lá ficam senhas e sessões).
    let test = open(opts);
    let own: Result<bool, _> =
        tokio::time::timeout(Duration::from_secs(8), sqlx::query_scalar("SELECT to_regnamespace('masterbase') IS NOT NULL").fetch_one(&test))
            .await
            .unwrap_or(Err(sqlx::Error::PoolTimedOut));
    test.close().await;
    match own {
        Ok(false) => {}
        Ok(true) => return Err(bad("Esse é o banco do próprio Masterbase. Conecte outro banco.")),
        Err(e) => {
            eprintln!("{} projeto novo sem conexão: {e}", crate::activity::clock());
            return Err(bad("Não deu para conectar nesse banco. Confira endereço, usuário, senha e se o servidor aceita conexão daqui."));
        }
    }
    let id: i64 = sqlx::query_scalar("INSERT INTO masterbase.projects (name, url) VALUES ($1, $2) RETURNING id")
        .bind(name)
        .bind(url)
        .fetch_one(&db)
        .await?;
    log(&db, user.id, "project_add", json!({ "project": name })).await?;
    Ok(Json(json!({ "id": id, "name": name })))
}

/// Tira o projeto do painel. O banco em si continua lá, intacto.
pub async fn remove(user: User, State(db): State<PgPool>, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name: String = sqlx::query_scalar("DELETE FROM masterbase.projects WHERE id = $1 RETURNING name")
        .bind(id)
        .fetch_optional(&db)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, GONE))?;
    sqlx::query("DELETE FROM masterbase.row_rules WHERE project = $1").bind(id).execute(&db).await?;
    sqlx::query("DELETE FROM masterbase.queries WHERE project = $1").bind(id).execute(&db).await?;
    if let Some(p) = pools().lock().unwrap_or_else(|e| e.into_inner()).remove(&id) {
        tokio::spawn(async move { p.close().await });
    }
    log(&db, user.id, "project_remove", json!({ "project": name })).await?;
    Ok(Json(json!({ "ok": true })))
}

impl Db {
    /// Registra na atividade (sempre no banco principal), com o projeto quando não é o principal.
    pub async fn log(&self, user_id: i64, action: &str, mut detail: Value) -> ApiResult<()> {
        if self.project != 0 {
            if let Value::Object(m) = &mut detail {
                m.insert("project".into(), json!(self.project));
            }
        }
        log(&self.meta, user_id, action, detail).await
    }
}
