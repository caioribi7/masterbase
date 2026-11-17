use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::{header, request::Parts, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

const COOKIE: &str = "mb_session";
const PAGE: i64 = 12;

#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL").expect("defina DATABASE_URL");
    // search_path com masterbase primeiro: a tabela _sqlx_migrations fica fora de public (e fora da tela).
    let db = sqlx::postgres::PgPoolOptions::new()
        .after_connect(|conn, _| Box::pin(async move {
            sqlx::Executor::execute(conn, "SET search_path = masterbase, public").await.map(|_| ())
        }))
        .connect(&url)
        .await
        .expect("sem conexão com o Postgres");
    sqlx::query("CREATE SCHEMA IF NOT EXISTS masterbase").execute(&db).await.expect("sem permissão para criar schema");
    sqlx::migrate!().run(&db).await.expect("migração falhou");

    let web = std::env::var("MB_WEB_DIR").unwrap_or_else(|_| "../web/dist".into());
    let api = Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/tables", get(list_tables))
        .route("/tables/{name}", get(read_table))
        .route("/tables/{name}/rows", post(insert_row).patch(update_cell));

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(&web).fallback(ServeFile::new(format!("{web}/index.html"))))
        .layer(header_layer(header::CONTENT_SECURITY_POLICY, "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'"))
        .layer(header_layer(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(header_layer(header::REFERRER_POLICY, "no-referrer"))
        .with_state(db);

    let addr = std::env::var("MB_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("porta ocupada");
    println!("Masterbase em http://{addr}");
    axum::serve(listener, app).await.expect("servidor caiu");
}

fn header_layer(name: header::HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(value))
}

// ---------- erros ----------

struct ApiError(StatusCode, &'static str);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        eprintln!("db: {e}");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Erro no banco de dados.")
    }
}

type ApiResult<T> = Result<T, ApiError>;

const UNAUTHORIZED: ApiError = ApiError(StatusCode::UNAUTHORIZED, "Entre na sua conta.");

/// Erro de valor digitado (tipo errado, restrição violada) vira 400, não 500.
fn value_error(e: sqlx::Error) -> ApiError {
    match e.as_database_error().and_then(|d| d.code()) {
        Some(code) if code.starts_with("22") || code.starts_with("23") => {
            ApiError(StatusCode::BAD_REQUEST, "Valor inválido para essa coluna.")
        }
        _ => e.into(),
    }
}

// ---------- autenticação ----------

struct User {
    id: i64,
}

impl FromRequestParts<PgPool> for User {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, db: &PgPool) -> Result<Self, ApiError> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(COOKIE).ok_or(UNAUTHORIZED)?;
        let raw = hex::decode(token.value()).map_err(|_| UNAUTHORIZED)?;
        let id: i64 = sqlx::query_scalar(
            "SELECT user_id FROM masterbase.sessions WHERE token_hash = $1 AND expires_at > now()",
        )
        .bind(Sha256::digest(&raw).to_vec())
        .fetch_optional(db)
        .await?
        .ok_or(UNAUTHORIZED)?;
        Ok(User { id })
    }
}

#[derive(Deserialize)]
struct SignupBody {
    name: String,
    email: String,
    password: String,
}

async fn signup(State(db): State<PgPool>, jar: CookieJar, Json(b): Json<SignupBody>) -> ApiResult<(CookieJar, Json<Value>)> {
    let email = b.email.trim().to_lowercase();
    if b.name.trim().is_empty() || !email.contains('@') || b.password.chars().count() < 10 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "Use nome, e-mail válido e senha com 10 caracteres ou mais."));
    }
    let hash = hash_password(&b.password)?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO masterbase.users (name, email, password_hash) VALUES ($1, $2, $3)
         ON CONFLICT (email) DO NOTHING RETURNING id",
    )
    .bind(b.name.trim())
    .bind(&email)
    .bind(hash)
    .fetch_optional(&db)
    .await?
    .ok_or(ApiError(StatusCode::CONFLICT, "Esse e-mail já tem conta."))?;
    log(&db, id, "signup", json!({})).await?;
    start_session(&db, jar, id).await
}

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

// Limite de tentativas por e-mail: 5 erros em 15 min bloqueiam até a janela acabar.
// Em memória: zera ao reiniciar e não cobre vários e-mails vindos do mesmo IP.
const MAX_FAILS: u32 = 5;
const FAIL_WINDOW: Duration = Duration::from_secs(15 * 60);

struct Attempts(HashMap<String, (u32, Instant)>);

impl Attempts {
    fn blocked(&self, key: &str, now: Instant) -> bool {
        matches!(self.0.get(key), Some(&(n, start)) if n >= MAX_FAILS && now - start < FAIL_WINDOW)
    }

    fn fail(&mut self, key: &str, now: Instant) {
        if self.0.len() > 10_000 {
            self.0.retain(|_, (_, start)| now - *start < FAIL_WINDOW);
        }
        let e = self.0.entry(key.to_owned()).or_insert((0, now));
        if now - e.1 >= FAIL_WINDOW {
            *e = (0, now);
        }
        e.0 += 1;
    }
}

fn attempts() -> MutexGuard<'static, Attempts> {
    static A: OnceLock<Mutex<Attempts>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(Attempts(HashMap::new()))).lock().unwrap_or_else(|p| p.into_inner())
}

async fn login(State(db): State<PgPool>, jar: CookieJar, Json(b): Json<LoginBody>) -> ApiResult<(CookieJar, Json<Value>)> {
    let email = b.email.trim().to_lowercase();
    if attempts().blocked(&email, Instant::now()) {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "Muitas tentativas. Espere 15 minutos."));
    }
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM masterbase.users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&db)
            .await?;
    // Sem usuário, verifica contra um hash fixo: o tempo de resposta não revela quais e-mails existem.
    let (id, hash) = row.unwrap_or((0, dummy_hash().to_owned()));
    let ok = PasswordHash::new(&hash)
        .map(|h| Argon2::default().verify_password(b.password.as_bytes(), &h).is_ok())
        .unwrap_or(false);
    if !ok || id == 0 {
        attempts().fail(&email, Instant::now());
        return Err(ApiError(StatusCode::UNAUTHORIZED, "E-mail ou senha incorretos."));
    }
    attempts().0.remove(&email);
    log(&db, id, "login", json!({})).await?;
    start_session(&db, jar, id).await
}

async fn logout(State(db): State<PgPool>, jar: CookieJar) -> ApiResult<CookieJar> {
    if let Some(raw) = jar.get(COOKIE).and_then(|c| hex::decode(c.value()).ok()) {
        sqlx::query("DELETE FROM masterbase.sessions WHERE token_hash = $1")
            .bind(Sha256::digest(&raw).to_vec())
            .execute(&db)
            .await?;
    }
    Ok(jar.remove(Cookie::build(COOKIE).path("/")))
}

async fn me(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let (name, email): (String, String) =
        sqlx::query_as("SELECT name, email FROM masterbase.users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&db)
            .await?;
    Ok(Json(json!({ "name": name, "email": email })))
}

fn hash_password(password: &str) -> ApiResult<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Falha ao proteger a senha."))
}

fn dummy_hash() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| hash_password("masterbase-dummy-password").unwrap_or_default())
}

async fn start_session(db: &PgPool, jar: CookieJar, user_id: i64) -> ApiResult<(CookieJar, Json<Value>)> {
    let mut raw = [0u8; 32];
    OsRng.fill_bytes(&mut raw);
    sqlx::query("INSERT INTO masterbase.sessions (token_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '7 days')")
        .bind(Sha256::digest(raw).to_vec())
        .bind(user_id)
        .execute(db)
        .await?;
    let cookie = Cookie::build((COOKIE, hex::encode(raw)))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/");
    Ok((jar.add(cookie), Json(json!({ "ok": true }))))
}

async fn log(db: &PgPool, user_id: i64, action: &str, detail: Value) -> ApiResult<()> {
    sqlx::query("INSERT INTO masterbase.activity (user_id, action, detail) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(action)
        .bind(detail)
        .execute(db)
        .await?;
    Ok(())
}

// ---------- tabelas ----------

/// Aspas de identificador do Postgres. Nomes só chegam aqui depois de conferidos no catálogo.
fn ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Tipo do Postgres em palavras para quem não sabe SQL.
fn label(ty: &str, typtype: &str) -> &'static str {
    match ty {
        _ if typtype == "e" => "opção",
        "email" => "e-mail",
        "boolean" => "sim/não",
        "uuid" => "id único",
        "json" | "jsonb" => "dados",
        "date" => "data",
        t if t.starts_with("timestamp") => "data e hora",
        "smallint" | "integer" | "bigint" | "real" | "double precision" => "número",
        t if t.starts_with("numeric") => "número",
        t if t.starts_with("text") || t.starts_with("character") => "texto",
        _ => "outro",
    }
}

struct Col {
    name: String,
    ty: String,
    typtype: String,
    pk: bool,
}

async fn columns(db: &PgPool, table: &str) -> ApiResult<Vec<Col>> {
    let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
        "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), t.typtype::text,
                coalesce(a.attnum = ANY(i.indkey), false)
         FROM pg_attribute a
         JOIN pg_class c ON c.oid = a.attrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         JOIN pg_type t ON t.oid = a.atttypid
         LEFT JOIN pg_index i ON i.indrelid = c.oid AND i.indisprimary
         WHERE n.nspname = 'public' AND c.relname = $1 AND c.relkind = 'r'
           AND a.attnum > 0 AND NOT a.attisdropped
         ORDER BY a.attnum",
    )
    .bind(table)
    .fetch_all(db)
    .await?;
    if rows.is_empty() {
        return Err(ApiError(StatusCode::NOT_FOUND, "Tabela não encontrada."));
    }
    Ok(rows.into_iter().map(|(name, ty, typtype, pk)| Col { name, ty, typtype, pk }).collect())
}

async fn list_tables(_user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let names: Vec<String> =
        sqlx::query_scalar("SELECT tablename::text FROM pg_tables WHERE schemaname = 'public' ORDER BY 1")
            .fetch_all(&db)
            .await?;
    let mut out = Vec::new();
    for name in names {
        // shortcut: count(*) por tabela fica lento acima de ~1M linhas; trocar por pg_class.reltuples aí.
        let rows: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM public.{}", ident(&name)))
            .fetch_one(&db)
            .await?;
        out.push(json!({ "name": name, "rows": rows }));
    }
    Ok(Json(Value::Array(out)))
}

#[derive(Deserialize)]
struct ReadQuery {
    page: Option<i64>,
    col: Option<String>,
    val: Option<String>,
}

async fn read_table(_user: User, State(db): State<PgPool>, Path(table): Path<String>, Query(q): Query<ReadQuery>) -> ApiResult<Json<Value>> {
    let cols = columns(&db, &table).await?;
    let filter = match (&q.col, &q.val) {
        (Some(c), Some(v)) if !v.is_empty() => {
            if !cols.iter().any(|x| &x.name == c) {
                return Err(ApiError(StatusCode::BAD_REQUEST, "Coluna não existe."));
            }
            Some((format!("WHERE lower(t.{}::text) = lower($1)", ident(c)), v.clone()))
        }
        _ => None,
    };
    let (where_sql, val) = filter.unwrap_or_default();
    let from = format!("FROM public.{} t {where_sql}", ident(&table));
    let order = cols.iter().find(|c| c.pk).map(|c| format!("t.{}", ident(&c.name))).unwrap_or_else(|| "t.ctid".into());

    let total: i64 = bind_opt(sqlx::query_scalar(&format!("SELECT count(*) {from}")), &val).fetch_one(&db).await?;
    let page = q.page.unwrap_or(1).max(1);
    let rows: Vec<Value> = bind_opt(
        sqlx::query_scalar(&format!("SELECT row_to_json(t)::jsonb {from} ORDER BY {order} LIMIT {PAGE} OFFSET {}", (page - 1) * PAGE)),
        &val,
    )
    .fetch_all(&db)
    .await?;

    let columns: Vec<Value> = cols
        .iter()
        .map(|c| json!({ "name": c.name, "type": c.ty, "label": label(&c.ty, &c.typtype), "pk": c.pk }))
        .collect();
    Ok(Json(json!({ "columns": columns, "rows": rows, "total": total, "page": page, "pages": ((total + PAGE - 1) / PAGE).max(1) })))
}

fn bind_opt<'q, O>(
    q: sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    val: &'q str,
) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    if val.is_empty() { q } else { q.bind(val) }
}

#[derive(Deserialize)]
struct UpdateBody {
    pk: Value,
    column: String,
    value: Option<String>,
}

async fn update_cell(user: User, State(db): State<PgPool>, Path(table): Path<String>, Json(b): Json<UpdateBody>) -> ApiResult<Json<Value>> {
    let cols = columns(&db, &table).await?;
    let pk = cols.iter().find(|c| c.pk).ok_or(ApiError(StatusCode::BAD_REQUEST, "Tabela sem chave primária: edição bloqueada."))?;
    let col = cols
        .iter()
        .find(|c| c.name == b.column && !c.pk)
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "Coluna não pode ser editada."))?;
    let pk_text = match &b.pk {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    // col.ty vem de format_type() do catálogo, não do usuário.
    let sql = format!(
        "UPDATE public.{} AS t SET {} = CAST($1 AS {}) WHERE t.{}::text = $2 RETURNING row_to_json(t)::jsonb",
        ident(&table), ident(&col.name), col.ty, ident(&pk.name)
    );
    let row: Value = sqlx::query_scalar(&sql)
        .bind(&b.value)
        .bind(&pk_text)
        .fetch_optional(&db)
        .await
        .map_err(value_error)?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Linha não encontrada."))?;
    log(&db, user.id, "update", json!({ "table": table, "column": col.name, "pk": pk_text })).await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
struct InsertBody {
    values: Map<String, Value>,
}

async fn insert_row(user: User, State(db): State<PgPool>, Path(table): Path<String>, Json(b): Json<InsertBody>) -> ApiResult<Json<Value>> {
    let cols = columns(&db, &table).await?;
    let mut names = Vec::new();
    let mut casts = Vec::new();
    let mut vals: Vec<Option<String>> = Vec::new();
    for (key, v) in &b.values {
        let col = cols.iter().find(|c| &c.name == key).ok_or(ApiError(StatusCode::BAD_REQUEST, "Coluna não existe."))?;
        names.push(ident(&col.name));
        casts.push(format!("CAST(${} AS {})", vals.len() + 1, col.ty));
        vals.push(match v {
            Value::Null => None,
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        });
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
    let row: Value = q.fetch_one(&db).await.map_err(value_error)?;
    log(&db, user.id, "insert", json!({ "table": table })).await?;
    Ok(Json(row))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_escapes_quotes() {
        assert_eq!(ident("clientes"), "\"clientes\"");
        assert_eq!(ident("a\"; DROP TABLE x; --"), "\"a\"\"; DROP TABLE x; --\"");
    }

    #[test]
    fn login_blocks_after_five_fails() {
        let mut a = Attempts(HashMap::new());
        let t = Instant::now();
        for _ in 0..MAX_FAILS - 1 {
            a.fail("x@y", t);
        }
        assert!(!a.blocked("x@y", t));
        a.fail("x@y", t);
        assert!(a.blocked("x@y", t));
        assert!(!a.blocked("outro@y", t));
        assert!(!a.blocked("x@y", t + FAIL_WINDOW));
    }

    #[test]
    fn labels_are_human() {
        assert_eq!(label("integer", "b"), "número");
        assert_eq!(label("character varying(120)", "b"), "texto");
        assert_eq!(label("plano", "e"), "opção");
        assert_eq!(label("email", "d"), "e-mail");
        assert_eq!(label("timestamp with time zone", "b"), "data e hora");
    }
}
