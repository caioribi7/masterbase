use crate::activity::log;
use crate::error::{bad, ApiError, ApiResult, UNAUTHORIZED};
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRequestParts, State};
use axum::http::{request::Parts, StatusCode};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

const COOKIE: &str = "mb_session";

pub struct User {
    pub id: i64,
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

/// Cadastro aberto só enquanto não existe conta: a primeira pessoa vira dona da instância.
pub async fn setup(State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let open: bool = sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM masterbase.users)").fetch_one(&db).await?;
    Ok(Json(json!({ "signup": open })))
}

#[derive(Deserialize)]
pub struct SignupBody {
    name: String,
    email: String,
    password: String,
}

pub async fn signup(State(db): State<PgPool>, jar: CookieJar, Json(b): Json<SignupBody>) -> ApiResult<(CookieJar, Json<Value>)> {
    let email = b.email.trim().to_lowercase();
    if b.name.trim().is_empty() || !email.contains('@') || b.password.chars().count() < 10 {
        return Err(bad("Use nome, e-mail válido e senha com 10 caracteres ou mais."));
    }
    let hash = hash_password(&b.password)?;
    // Trava a tabela: dois cadastros simultâneos não viram dois donos.
    let mut tx = db.begin().await?;
    sqlx::query("LOCK TABLE masterbase.users IN EXCLUSIVE MODE").execute(&mut *tx).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO masterbase.users (name, email, password_hash)
         SELECT $1, $2, $3 WHERE NOT EXISTS (SELECT 1 FROM masterbase.users) RETURNING id",
    )
    .bind(b.name.trim())
    .bind(&email)
    .bind(hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError(StatusCode::FORBIDDEN, "Cadastro fechado. Peça acesso a quem administra o projeto."))?;
    tx.commit().await?;
    log(&db, id, "signup", json!({})).await?;
    start_session(&db, jar, id).await
}

#[derive(Deserialize)]
pub struct LoginBody {
    email: String,
    password: String,
}

// Limite por e-mail: 5 erros em 15 min bloqueiam até a janela acabar.
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

pub async fn login(State(db): State<PgPool>, jar: CookieJar, Json(b): Json<LoginBody>) -> ApiResult<(CookieJar, Json<Value>)> {
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

pub async fn logout(State(db): State<PgPool>, jar: CookieJar) -> ApiResult<CookieJar> {
    if let Some(raw) = jar.get(COOKIE).and_then(|c| hex::decode(c.value()).ok()) {
        sqlx::query("DELETE FROM masterbase.sessions WHERE token_hash = $1")
            .bind(Sha256::digest(&raw).to_vec())
            .execute(&db)
            .await?;
    }
    Ok(jar.remove(Cookie::build(COOKIE).path("/")))
}

pub async fn me(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
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
    sqlx::query("DELETE FROM masterbase.sessions WHERE expires_at < now()").execute(db).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
