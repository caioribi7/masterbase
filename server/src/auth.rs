use crate::activity::log;
use crate::error::{bad, ApiError, ApiResult, UNAUTHORIZED};
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{ConnectInfo, FromRequestParts, Path, State};
use axum::http::{header, request::Parts, HeaderMap, StatusCode};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

const COOKIE: &str = "mb_session";

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Role {
    Leitor,
    Editor,
    Admin,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        Some(match s {
            "leitor" => Role::Leitor,
            "editor" => Role::Editor,
            "admin" => Role::Admin,
            _ => return None,
        })
    }
}

pub struct User {
    pub id: i64,
    pub role: Role,
    /// Entrou com chave de API (Authorization: Bearer). Chave nunca administra.
    pub via_key: bool,
    /// Hash do token da sessão atual (só para quem entrou pelo navegador).
    pub session: Option<Vec<u8>>,
}

impl User {
    pub fn editor(&self) -> ApiResult<()> {
        if self.role >= Role::Editor { Ok(()) } else { Err(ApiError(StatusCode::FORBIDDEN, "Seu acesso é só de leitura.")) }
    }

    pub fn admin(&self) -> ApiResult<()> {
        if self.role == Role::Admin && !self.via_key { Ok(()) } else { Err(ApiError(StatusCode::FORBIDDEN, "Só administradores podem fazer isso.")) }
    }
}

pub fn sha(raw: &[u8]) -> Vec<u8> {
    Sha256::digest(raw).to_vec()
}

pub fn random_token() -> [u8; 32] {
    let mut raw = [0u8; 32];
    OsRng.fill_bytes(&mut raw);
    raw
}

impl FromRequestParts<PgPool> for User {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, db: &PgPool) -> Result<Self, ApiError> {
        let bearer = parts.headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
        let mut session = None;
        let (row, via_key): (Option<(i64, String)>, bool) = if let Some(key) = bearer {
            let row = sqlx::query_as(
                "UPDATE masterbase.api_keys k SET last_used_at = now() FROM masterbase.users u
                 WHERE k.token_hash = $1 AND u.id = k.user_id AND u.active RETURNING u.id, k.role",
            )
            .bind(sha(key.as_bytes()))
            .fetch_optional(db)
            .await?;
            (row, true)
        } else {
            let jar = CookieJar::from_headers(&parts.headers);
            let token = jar.get(COOKIE).ok_or(UNAUTHORIZED)?;
            let raw = hex::decode(token.value()).map_err(|_| UNAUTHORIZED)?;
            let hash = sha(&raw);
            let row: Option<(i64, String, bool)> = sqlx::query_as(
                "SELECT u.id, u.role, s.last_seen_at < now() - interval '5 minutes'
                 FROM masterbase.sessions s JOIN masterbase.users u ON u.id = s.user_id
                 WHERE s.token_hash = $1 AND s.expires_at > now() AND u.active",
            )
            .bind(&hash)
            .fetch_optional(db)
            .await?;
            // "Visto por último" em Configurações. Grava no máximo a cada 5 minutos.
            if row.as_ref().is_some_and(|r| r.2) {
                sqlx::query("UPDATE masterbase.sessions SET last_seen_at = now() WHERE token_hash = $1").bind(&hash).execute(db).await?;
            }
            session = Some(hash);
            (row.map(|(id, role, _)| (id, role)), false)
        };
        let (id, role) = row.ok_or(UNAUTHORIZED)?;
        Ok(User { id, role: Role::parse(&role).ok_or(UNAUTHORIZED)?, via_key, session })
    }
}

/// Cadastro aberto só enquanto não existe conta: a primeira pessoa vira dona da instância.
pub async fn setup(State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let (open, name): (bool, String) = sqlx::query_as(
        "SELECT NOT EXISTS (SELECT 1 FROM masterbase.users), coalesce((SELECT name FROM masterbase.settings), '')",
    )
    .fetch_one(&db)
    .await?;
    Ok(Json(json!({ "signup": open, "name": name, "oauth": crate::oauth::enabled(), "mail": crate::mail::enabled() })))
}

#[derive(Deserialize)]
pub struct SignupBody {
    name: String,
    email: String,
    password: String,
}

pub async fn signup(State(db): State<PgPool>, headers: HeaderMap, jar: CookieJar, Json(b): Json<SignupBody>) -> ApiResult<(CookieJar, Json<Value>)> {
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
    .ok_or(ApiError(StatusCode::FORBIDDEN, "Cadastro fechado. Peça acesso a um administrador."))?;
    tx.commit().await?;
    log(&db, id, "signup", json!({})).await?;
    start_session(&db, jar, id, &headers).await
}

#[derive(Deserialize)]
pub struct LoginBody {
    email: String,
    password: String,
}

// Limites: 5 erros por e-mail ou 30 por IP em 15 min bloqueiam até a janela acabar.
// Em memória: zera ao reiniciar. O do IP pega quem testa muitos e-mails do mesmo lugar.
const MAX_FAILS: u32 = 5;
const MAX_FAILS_IP: u32 = 30;
const FAIL_WINDOW: Duration = Duration::from_secs(15 * 60);

pub(crate) struct Attempts(HashMap<String, (u32, Instant)>);

impl Attempts {
    pub(crate) fn blocked(&self, key: &str, max: u32, now: Instant) -> bool {
        matches!(self.0.get(key), Some(&(n, start)) if n >= max && now - start < FAIL_WINDOW)
    }

    pub(crate) fn fail(&mut self, key: &str, now: Instant) {
        if self.0.len() > 10_000 {
            self.0.retain(|_, (_, start)| now - *start < FAIL_WINDOW);
        }
        let e = self.0.entry(key.to_owned()).or_insert((0, now));
        if now - e.1 >= FAIL_WINDOW {
            *e = (0, now);
        }
        e.0 += 1;
    }

    pub(crate) fn clear(&mut self, key: &str) {
        self.0.remove(key);
    }
}

pub(crate) fn attempts() -> MutexGuard<'static, Attempts> {
    static A: OnceLock<Mutex<Attempts>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(Attempts(HashMap::new()))).lock().unwrap_or_else(|p| p.into_inner())
}

/// IP de quem pediu. Atrás de um proxy na mesma máquina (Caddy, Nginx), vale o último X-Forwarded-For,
/// que é o que o proxy viu. Fora disso o cabeçalho é ignorado: qualquer um poderia inventar.
pub fn client_ip(peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    if peer.is_loopback() {
        let last = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()).and_then(|v| v.rsplit(',').next());
        if let Some(ip) = last.and_then(|s| s.trim().parse().ok()) {
            return ip;
        }
    }
    peer
}

pub async fn login(
    State(db): State<PgPool>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(b): Json<LoginBody>,
) -> ApiResult<(CookieJar, Json<Value>)> {
    let email = b.email.trim().to_lowercase();
    let ip = client_ip(peer.ip(), &headers);
    let ip_key = format!("ip:{ip}");
    {
        let a = attempts();
        let now = Instant::now();
        if a.blocked(&email, MAX_FAILS, now) || a.blocked(&ip_key, MAX_FAILS_IP, now) {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "Muitas tentativas. Espere 15 minutos."));
        }
    }
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM masterbase.users WHERE email = $1 AND active")
            .bind(&email)
            .fetch_optional(&db)
            .await?;
    // Sem usuário, verifica contra um hash fixo: o tempo de resposta não revela quais e-mails existem.
    let (id, hash) = row.unwrap_or((0, dummy_hash().to_owned()));
    let ok = PasswordHash::new(&hash)
        .map(|h| Argon2::default().verify_password(b.password.as_bytes(), &h).is_ok())
        .unwrap_or(false);
    if !ok || id == 0 {
        {
            let mut a = attempts();
            a.fail(&email, Instant::now());
            a.fail(&ip_key, Instant::now());
        }
        if crate::activity::logging() {
            println!("{} login recusado para {email} ({ip})", crate::activity::clock());
        }
        return Err(ApiError(StatusCode::UNAUTHORIZED, "E-mail ou senha incorretos."));
    }
    attempts().clear(&email);
    log(&db, id, "login", json!({})).await?;
    start_session(&db, jar, id, &headers).await
}

pub async fn logout(State(db): State<PgPool>, jar: CookieJar) -> ApiResult<CookieJar> {
    if let Some(raw) = jar.get(COOKIE).and_then(|c| hex::decode(c.value()).ok()) {
        sqlx::query("DELETE FROM masterbase.sessions WHERE token_hash = $1")
            .bind(sha(&raw))
            .execute(&db)
            .await?;
    }
    Ok(jar.remove(Cookie::build(COOKIE).path("/")))
}

pub async fn me(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let (name, email, role, project, pg): (String, String, String, String, String) = sqlx::query_as(
        "SELECT name, email, role,
                coalesce(nullif((SELECT s.name FROM masterbase.settings s), ''), current_database()::text),
                current_setting('server_version')
         FROM masterbase.users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_one(&db)
    .await?;
    let role = if user.via_key && role == "admin" { "editor".to_owned() } else { role };
    Ok(Json(json!({ "name": name, "email": email, "role": role, "project": project, "postgres": pg })))
}

pub fn hash_password(password: &str) -> ApiResult<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Falha ao proteger a senha."))
}

fn dummy_hash() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| hash_password("masterbase-dummy-password").unwrap_or_default())
}

/// Secure por padrão. Só sai na rede local sem HTTPS (escolha explícita em `masterbase configurar`).
pub fn secure_cookie() -> bool {
    std::env::var("MB_INSECURE_COOKIE").as_deref() != Ok("1")
}

async fn start_session(db: &PgPool, jar: CookieJar, user_id: i64, headers: &HeaderMap) -> ApiResult<(CookieJar, Json<Value>)> {
    Ok((new_session(db, jar, user_id, headers).await?, Json(json!({ "ok": true }))))
}

pub async fn new_session(db: &PgPool, jar: CookieJar, user_id: i64, headers: &HeaderMap) -> ApiResult<CookieJar> {
    let raw = random_token();
    let agent: String = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("").chars().take(300).collect();
    sqlx::query("DELETE FROM masterbase.sessions WHERE expires_at < now()").execute(db).await?;
    // Duração vem de Configurações (padrão 7 dias). O cookie dura o mesmo tanto.
    let days: i32 = sqlx::query_scalar("SELECT coalesce((SELECT session_days FROM masterbase.settings), 7)").fetch_one(db).await?;
    sqlx::query(
        "INSERT INTO masterbase.sessions (token_hash, user_id, expires_at, user_agent)
         VALUES ($1, $2, now() + make_interval(days => $3), $4)",
    )
    .bind(sha(&raw))
    .bind(user_id)
    .bind(days)
    .bind(agent)
    .execute(db)
    .await?;
    let cookie = Cookie::build((COOKIE, hex::encode(raw)))
        .max_age(time::Duration::days(days.into()))
        .http_only(true)
        .secure(secure_cookie())
        .same_site(SameSite::Strict)
        .path("/");
    Ok(jar.add(cookie))
}

// ---------- sessões ----------

fn browser_only(user: &User) -> ApiResult<&[u8]> {
    user.session.as_deref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))
}

/// Aparelhos com sessão aberta da própria pessoa. A atual vem primeiro.
pub async fn list_sessions(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let current = browser_only(&user)?;
    let rows: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', id, 'created_at', created_at, 'last_seen_at', last_seen_at,
                'device', user_agent, 'current', token_hash = $2)
         FROM masterbase.sessions WHERE user_id = $1 AND expires_at > now()
         ORDER BY token_hash = $2 DESC, last_seen_at DESC",
    )
    .bind(user.id)
    .bind(current)
    .fetch_all(&db)
    .await?;
    Ok(Json(Value::Array(rows)))
}

pub async fn end_session(user: User, State(db): State<PgPool>, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    browser_only(&user)?;
    sqlx::query("DELETE FROM masterbase.sessions WHERE id = $1 AND user_id = $2").bind(id).bind(user.id).execute(&db).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Sair de todos os outros aparelhos. A sessão atual continua.
pub async fn end_other_sessions(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let current = browser_only(&user)?;
    let n = sqlx::query("DELETE FROM masterbase.sessions WHERE user_id = $1 AND token_hash <> $2")
        .bind(user.id)
        .bind(current)
        .execute(&db)
        .await?
        .rows_affected();
    log(&db, user.id, "end_sessions", json!({ "count": n })).await?;
    Ok(Json(json!({ "ended": n })))
}

// ---------- link para criar ou trocar a senha ----------

/// Link de uso único, válido por 24 horas. Guarda só o hash e apaga links antigos da pessoa.
pub async fn new_password_link(db: &PgPool, user_id: i64) -> ApiResult<String> {
    let raw = random_token();
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM masterbase.password_tokens WHERE user_id = $1 OR expires_at < now()").bind(user_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO masterbase.password_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '24 hours')")
        .bind(sha(&raw))
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(hex::encode(raw))
}

#[derive(Deserialize)]
pub struct ForgotBody {
    email: String,
}

/// "Esqueci a senha" na tela de login: manda o link por e-mail, se a conta existir.
/// A resposta é sempre a mesma, para não revelar quais e-mails têm conta. O envio roda à parte pelo mesmo motivo (tempo).
pub async fn forgot(State(db): State<PgPool>, ConnectInfo(peer): ConnectInfo<SocketAddr>, headers: HeaderMap, Json(b): Json<ForgotBody>) -> ApiResult<Json<Value>> {
    if !crate::mail::enabled() {
        return Err(bad("Este servidor não manda e-mail. Peça o link a um administrador."));
    }
    let email = b.email.trim().to_lowercase();
    let (by_email, by_ip) = (format!("forgot:{email}"), format!("forgot-ip:{}", client_ip(peer.ip(), &headers)));
    {
        let mut a = attempts();
        let now = Instant::now();
        if a.blocked(&by_email, 3, now) || a.blocked(&by_ip, 20, now) {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "Muitos pedidos. Espere 15 minutos."));
        }
        a.fail(&by_email, now);
        a.fail(&by_ip, now);
    }
    tokio::spawn(async move {
        let id: Option<i64> = sqlx::query_scalar("SELECT id FROM masterbase.users WHERE email = $1 AND active")
            .bind(&email)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten();
        if let Some(id) = id {
            if let Ok(token) = new_password_link(&db, id).await {
                crate::mail::password_link(&db, id, &token, false).await;
            }
        }
    });
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ResetBody {
    token: String,
    password: Option<String>,
}

async fn token_user(db: &PgPool, token: &str) -> ApiResult<(i64, String, String)> {
    const BAD: ApiError = ApiError(StatusCode::NOT_FOUND, "Link inválido ou vencido. Peça outro.");
    let raw = hex::decode(token.trim()).map_err(|_| BAD)?;
    sqlx::query_as(
        "SELECT u.id, u.name, u.email FROM masterbase.password_tokens t JOIN masterbase.users u ON u.id = t.user_id
         WHERE t.token_hash = $1 AND t.expires_at > now() AND u.active",
    )
    .bind(sha(&raw))
    .fetch_optional(db)
    .await?
    .ok_or(BAD)
}

/// O link ainda vale? Devolve para quem é (a tela mostra o nome e o e-mail).
/// POST e não GET: o token não vai parar no log de pedidos.
pub async fn reset_check(State(db): State<PgPool>, Json(b): Json<ResetBody>) -> ApiResult<Json<Value>> {
    let (_, name, email) = token_user(&db, &b.token).await?;
    Ok(Json(json!({ "name": name, "email": email })))
}

/// Grava a senha nova, gasta o link, encerra as sessões antigas e já entra.
pub async fn reset_password(State(db): State<PgPool>, headers: HeaderMap, jar: CookieJar, Json(b): Json<ResetBody>) -> ApiResult<(CookieJar, Json<Value>)> {
    let (id, _, email) = token_user(&db, &b.token).await?;
    let password = b.password.unwrap_or_default();
    if password.chars().count() < 10 {
        return Err(bad("A senha precisa de 10 caracteres ou mais."));
    }
    let hash = hash_password(&password)?;
    let mut tx = db.begin().await?;
    // DELETE ... RETURNING: dois envios do mesmo link ao mesmo tempo, só um passa.
    let used: Option<i64> = sqlx::query_scalar("DELETE FROM masterbase.password_tokens WHERE user_id = $1 RETURNING user_id")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if used.is_none() {
        return Err(ApiError(StatusCode::NOT_FOUND, "Esse link já foi usado."));
    }
    sqlx::query("UPDATE masterbase.users SET password_hash = $2 WHERE id = $1").bind(id).bind(hash).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM masterbase.sessions WHERE user_id = $1").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    attempts().clear(&email);
    log(&db, id, "reset_password", json!({})).await?;
    start_session(&db, jar, id, &headers).await
}

/// `masterbase senha EMAIL`: link para quem perdeu a senha e não tem outro administrador.
pub async fn cli_link(db: &PgPool, email: &str) -> Result<String, String> {
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM masterbase.users WHERE email = $1 AND active")
        .bind(email.trim().to_lowercase())
        .fetch_optional(db)
        .await
        .map_err(|e| e.to_string())?;
    let id = id.ok_or_else(|| format!("nenhuma conta ativa com o e-mail {email}"))?;
    new_password_link(db, id).await.map_err(|e| e.1.to_string())
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
        assert!(!a.blocked("x@y", MAX_FAILS, t));
        a.fail("x@y", t);
        assert!(a.blocked("x@y", MAX_FAILS, t));
        assert!(!a.blocked("x@y", MAX_FAILS_IP, t));
        assert!(!a.blocked("outro@y", MAX_FAILS, t));
        assert!(!a.blocked("x@y", MAX_FAILS, t + FAIL_WINDOW));
    }

    #[test]
    fn forwarded_ip_only_from_local_proxy() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        let local: IpAddr = "127.0.0.1".parse().unwrap();
        let remote: IpAddr = "9.9.9.9".parse().unwrap();
        assert_eq!(client_ip(local, &h), "5.6.7.8".parse::<IpAddr>().unwrap());
        assert_eq!(client_ip(remote, &h), remote);
        h.insert("x-forwarded-for", "lixo".parse().unwrap());
        assert_eq!(client_ip(local, &h), local);
    }
}
