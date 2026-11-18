//! Verificação em duas etapas: código de 6 dígitos de um app autenticador (TOTP, RFC 6238),
//! mais 10 códigos de recuperação de uso único para quem perder o celular.
//!
//! Quem tem o segundo fator ligado entra em dois passos: a senha (ou Google, GitHub, link por e-mail)
//! devolve um "ticket" de 5 minutos, e o código troca o ticket pela sessão. Passkey não pede o código:
//! ela já é o aparelho da pessoa mais a digital ou o PIN.

use crate::activity::log;
use crate::auth::{random_token, sha, User};
use crate::error::{bad, ApiError, ApiResult};
use crate::limits;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

const STEP: u64 = 30;

fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(secret).expect("hmac aceita qualquer tamanho");
    mac.update(&counter.to_be_bytes());
    let h = mac.finalize().into_bytes();
    let o = (h[19] & 0x0f) as usize;
    let bin = (u32::from(h[o] & 0x7f) << 24) | (u32::from(h[o + 1]) << 16) | (u32::from(h[o + 2]) << 8) | u32::from(h[o + 3]);
    bin % 1_000_000
}

/// Aceita o passo atual e um de cada lado (relógio do celular adiantado ou atrasado). Devolve o passo usado.
/// Passo igual ou anterior ao último aceito é recusado: o mesmo código não entra duas vezes.
fn check(secret: &[u8], code: &str, now: u64, last: i64) -> Option<i64> {
    let code: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    if code.len() != 6 {
        return None;
    }
    let step = now / STEP;
    [step.saturating_sub(1), step, step + 1]
        .into_iter()
        .find(|&s| s as i64 > last && format!("{:06}", hotp(secret, s)) == code)
        .map(|s| s as i64)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32(data: &[u8]) -> String {
    let mut out = String::new();
    let (mut buf, mut bits) = (0u32, 0);
    for &b in data {
        buf = (buf << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((buf >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn pct(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

// ---------- ligar e desligar ----------

/// Gera um segredo novo e devolve o QR para o app. Só vale depois de confirmar com um código.
pub async fn setup(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    let secret = &random_token()[..20];
    sqlx::query(
        "INSERT INTO masterbase.totp_pending (user_id, secret) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET secret = $2, created_at = now()",
    )
    .bind(user.id)
    .bind(secret)
    .execute(&db)
    .await?;
    let (email, instance): (String, String) = sqlx::query_as(
        "SELECT email, coalesce(nullif((SELECT name FROM masterbase.settings), ''), 'Masterbase') FROM masterbase.users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_one(&db)
    .await?;
    let key = base32(secret);
    let uri = format!("otpauth://totp/{}:{}?secret={key}&issuer={}&digits=6&period=30", pct(&instance), pct(&email), pct(&instance));
    let qr = qrcode::QrCode::new(uri.as_bytes())
        .map(|c| c.render::<qrcode::render::svg::Color>().min_dimensions(200, 200).quiet_zone(true).build())
        .unwrap_or_default();
    Ok(Json(json!({ "secret": key, "uri": uri, "qr": qr })))
}

#[derive(Deserialize)]
pub struct CodeBody {
    code: String,
}

/// Confere o primeiro código e liga. Devolve os códigos de recuperação (aparecem só agora).
pub async fn enable(user: User, State(db): State<PgPool>, Json(b): Json<CodeBody>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    let secret: Vec<u8> = sqlx::query_scalar("SELECT secret FROM masterbase.totp_pending WHERE user_id = $1 AND created_at > now() - interval '30 minutes'")
        .bind(user.id)
        .fetch_optional(&db)
        .await?
        .ok_or(bad("Comece de novo: o QR venceu."))?;
    let step = check(&secret, &b.code, now(), 0).ok_or(bad("Código errado. Confira a hora do celular e tente o código novo."))?;
    let codes: Vec<String> = (0..10)
        .map(|_| {
            let h = hex::encode(&random_token()[..5]);
            format!("{}-{}", &h[..5], &h[5..])
        })
        .collect();
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE masterbase.users SET totp_secret = $2, totp_last = $3 WHERE id = $1").bind(user.id).bind(&secret).bind(step).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM masterbase.totp_pending WHERE user_id = $1").bind(user.id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM masterbase.recovery_codes WHERE user_id = $1").bind(user.id).execute(&mut *tx).await?;
    for c in &codes {
        sqlx::query("INSERT INTO masterbase.recovery_codes (user_id, code_hash) VALUES ($1, $2)").bind(user.id).bind(sha(c.as_bytes())).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    log(&db, user.id, "2fa_on", json!({})).await?;
    Ok(Json(json!({ "codes": codes })))
}

/// Desliga. Pede um código (do app ou de recuperação): uma sessão esquecida aberta não tira a proteção.
pub async fn disable(user: User, State(db): State<PgPool>, Json(b): Json<CodeBody>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    verify(&db, user.id, &b.code).await?;
    sqlx::query("UPDATE masterbase.users SET totp_secret = NULL WHERE id = $1").bind(user.id).execute(&db).await?;
    sqlx::query("DELETE FROM masterbase.recovery_codes WHERE user_id = $1").bind(user.id).execute(&db).await?;
    log(&db, user.id, "2fa_off", json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Confere um código do app ou de recuperação (que é gasto). 5 erros em 15 minutos travam.
async fn verify(db: &PgPool, user_id: i64, code: &str) -> ApiResult<()> {
    let lock = format!("2fa:{user_id}");
    if limits::blocked(db, &lock, 5).await? {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "Muitas tentativas. Espere 15 minutos."));
    }
    let (secret, last): (Option<Vec<u8>>, i64) = sqlx::query_as("SELECT totp_secret, totp_last FROM masterbase.users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await?;
    let secret = secret.ok_or(bad("A verificação em duas etapas não está ligada."))?;
    if let Some(step) = check(&secret, code, now(), last) {
        // WHERE totp_last < step: dois pedidos com o mesmo código ao mesmo tempo, só um passa.
        let ok = sqlx::query("UPDATE masterbase.users SET totp_last = $2 WHERE id = $1 AND totp_last < $2").bind(user_id).bind(step).execute(db).await?.rows_affected();
        if ok == 1 {
            limits::clear(db, &lock).await?;
            return Ok(());
        }
    }
    let recovery = code.trim().to_lowercase();
    let used = sqlx::query("DELETE FROM masterbase.recovery_codes WHERE user_id = $1 AND code_hash = $2")
        .bind(user_id)
        .bind(sha(recovery.as_bytes()))
        .execute(db)
        .await?
        .rows_affected();
    if used == 1 {
        limits::clear(db, &lock).await?;
        log(db, user_id, "2fa_recovery", json!({})).await?;
        return Ok(());
    }
    limits::fail(db, &lock).await?;
    Err(bad("Código errado."))
}

// ---------- login em dois passos ----------

/// Se a pessoa usa segundo fator, devolve o ticket que espera o código. Senão, None (entra direto).
pub async fn ticket(db: &PgPool, user_id: i64, via: &str) -> ApiResult<Option<String>> {
    let on: bool = sqlx::query_scalar("SELECT totp_secret IS NOT NULL FROM masterbase.users WHERE id = $1").bind(user_id).fetch_one(db).await?;
    if !on {
        return Ok(None);
    }
    let raw = random_token();
    sqlx::query("DELETE FROM masterbase.login_tickets WHERE expires_at < now()").execute(db).await?;
    sqlx::query("INSERT INTO masterbase.login_tickets (ticket_hash, user_id, via, expires_at) VALUES ($1, $2, $3, now() + interval '5 minutes')")
        .bind(sha(&raw))
        .bind(user_id)
        .bind(via)
        .execute(db)
        .await?;
    Ok(Some(hex::encode(raw)))
}

#[derive(Deserialize)]
pub struct SecondStep {
    ticket: String,
    code: String,
}

pub async fn login(State(db): State<PgPool>, headers: HeaderMap, jar: CookieJar, Json(b): Json<SecondStep>) -> ApiResult<(CookieJar, Json<Value>)> {
    const GONE: ApiError = ApiError(StatusCode::UNAUTHORIZED, "O tempo para o código acabou. Entre de novo.");
    let raw = hex::decode(b.ticket.trim()).map_err(|_| GONE)?;
    let (user_id, via): (i64, String) = sqlx::query_as(
        "SELECT t.user_id, t.via FROM masterbase.login_tickets t JOIN masterbase.users u ON u.id = t.user_id
         WHERE t.ticket_hash = $1 AND t.expires_at > now() AND u.active",
    )
    .bind(sha(&raw))
    .fetch_optional(&db)
    .await?
    .ok_or(GONE)?;
    verify(&db, user_id, &b.code).await?;
    sqlx::query("DELETE FROM masterbase.login_tickets WHERE ticket_hash = $1").bind(sha(&raw)).execute(&db).await?;
    log(&db, user_id, "login", json!({ "via": via, "2fa": true })).await?;
    Ok((crate::auth::new_session(&db, jar, user_id, &headers).await?, Json(json!({ "ok": true }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_vector() {
        // RFC 6238, apêndice B (SHA1): T = 59 s, passo 1, código de 8 dígitos 94287082.
        let secret = b"12345678901234567890";
        assert_eq!(hotp(secret, 1), 287082);
        assert_eq!(check(secret, "287 082", 59, 0), Some(1));
        assert_eq!(check(secret, "287082", 59, 1), None, "mesmo passo não vale duas vezes");
        assert_eq!(check(secret, "000000", 59, 0), None);
    }

    #[test]
    fn base32_like_authenticator_apps() {
        assert_eq!(base32(b"12345678901234567890"), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(base32(b"f"), "MY");
    }
}
