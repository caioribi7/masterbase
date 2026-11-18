//! Passkeys (WebAuthn): entrar com a digital, o rosto ou o PIN do aparelho, sem senha.
//! Implementação enxuta: só ES256 (P-256), atestação "none", chave guardada no aparelho (residente)
//! e verificação do usuário obrigatória. É o que celulares, Windows Hello, Mac e gerenciadores de senha usam.
//!
//! Precisa de MB_URL com um nome (não IP): o navegador amarra a passkey ao domínio.
//! Em desenvolvimento, http://localhost:PORTA serve.

use crate::activity::log;
use crate::auth::{random_token, sha, User};
use crate::error::{bad, ApiError, ApiResult};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use ciborium::Value as Cbor;
use p256::ecdsa::signature::Verifier;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

/// (origem, rp id). Origem é o MB_URL; rp id é o nome do servidor nele. IP não serve.
fn relying_party() -> Option<(String, String)> {
    let url = crate::mail::public_url()?;
    let host = reqwest::Url::parse(&url).ok()?.host_str()?.to_string();
    if host.parse::<std::net::IpAddr>().is_ok() || host.starts_with('[') {
        return None;
    }
    let origin = reqwest::Url::parse(&url).ok()?.origin().ascii_serialization();
    Some((origin, host))
}

pub fn enabled() -> bool {
    relying_party().is_some()
}

fn rp() -> ApiResult<(String, String)> {
    relying_party().ok_or(bad("Passkey precisa do endereço do painel com nome (MB_URL), não IP."))
}

fn unb64(s: &str) -> ApiResult<Vec<u8>> {
    B64.decode(s.trim().trim_end_matches('=')).map_err(|_| bad("Resposta do aparelho inválida."))
}

async fn new_challenge(db: &PgPool, user_id: Option<i64>, kind: &str) -> ApiResult<String> {
    let raw = random_token();
    sqlx::query("DELETE FROM masterbase.webauthn_challenges WHERE expires_at < now()").execute(db).await?;
    sqlx::query("INSERT INTO masterbase.webauthn_challenges (challenge, user_id, kind, expires_at) VALUES ($1, $2, $3, now() + interval '5 minutes')")
        .bind(&raw[..])
        .bind(user_id)
        .bind(kind)
        .execute(db)
        .await?;
    Ok(B64.encode(raw))
}

/// Confere o clientDataJSON e gasta o desafio. Devolve o user_id gravado no desafio (registro).
async fn client_data(db: &PgPool, raw: &[u8], kind: &str, origin: &str) -> ApiResult<Option<i64>> {
    let cd: Value = serde_json::from_slice(raw).map_err(|_| bad("Resposta do aparelho inválida."))?;
    let want = if kind == "registro" { "webauthn.create" } else { "webauthn.get" };
    if cd["type"] != want || cd["origin"] != origin {
        return Err(bad("A passkey não é deste endereço."));
    }
    let challenge = unb64(cd["challenge"].as_str().unwrap_or(""))?;
    let row: Option<(Option<i64>,)> = sqlx::query_as(
        "DELETE FROM masterbase.webauthn_challenges WHERE challenge = $1 AND kind = $2 AND expires_at > now() RETURNING user_id",
    )
    .bind(&challenge)
    .bind(kind)
    .fetch_optional(db)
    .await?;
    row.map(|r| r.0).ok_or(bad("O pedido venceu. Tente de novo."))
}

struct AuthData {
    flags: u8,
    count: u32,
    /// (credential id, chave pública SEC1) quando é registro.
    cred: Option<(Vec<u8>, Vec<u8>)>,
}

fn parse_auth_data(d: &[u8], rp_id: &str) -> Result<AuthData, &'static str> {
    if d.len() < 37 {
        return Err("authData curto");
    }
    if d[..32] != *sha(rp_id.as_bytes()) {
        return Err("rp id diferente");
    }
    let flags = d[32];
    if flags & 0x01 == 0 || flags & 0x04 == 0 {
        return Err("sem presença ou verificação do usuário");
    }
    let count = u32::from_be_bytes([d[33], d[34], d[35], d[36]]);
    let mut cred = None;
    if flags & 0x40 != 0 {
        if d.len() < 55 {
            return Err("credencial curta");
        }
        let len = u16::from_be_bytes([d[53], d[54]]) as usize;
        let id = d.get(55..55 + len).ok_or("id da credencial")?.to_vec();
        let key: Cbor = ciborium::de::from_reader(&d[55 + len..]).map_err(|_| "chave em CBOR")?;
        cred = Some((id, cose_p256(&key)?));
    }
    Ok(AuthData { flags, count, cred })
}

/// Chave COSE EC2 P-256 (kty 2, alg -7, crv 1) para o ponto sem compressão 04||x||y.
fn cose_p256(key: &Cbor) -> Result<Vec<u8>, &'static str> {
    let map = key.as_map().ok_or("chave não é mapa")?;
    let get = |k: i64| map.iter().find(|(a, _)| a.as_integer() == Some(k.into())).map(|(_, v)| v);
    let int = |k: i64| get(k).and_then(|v| v.as_integer()).map(i128::from);
    if int(1) != Some(2) || int(3) != Some(-7) || int(-1) != Some(1) {
        return Err("só ES256 (P-256)");
    }
    let x = get(-2).and_then(|v| v.as_bytes()).ok_or("x")?;
    let y = get(-3).and_then(|v| v.as_bytes()).ok_or("y")?;
    if x.len() != 32 || y.len() != 32 {
        return Err("ponto inválido");
    }
    Ok([&[4u8][..], x, y].concat())
}

fn verify_sig(public_key: &[u8], auth_data: &[u8], client_data: &[u8], sig_der: &[u8]) -> bool {
    let Ok(key) = p256::ecdsa::VerifyingKey::from_sec1_bytes(public_key) else { return false };
    let Ok(sig) = p256::ecdsa::Signature::from_der(sig_der) else { return false };
    let msg = [auth_data, &sha(client_data)].concat();
    key.verify(&msg, &sig).is_ok()
}

// ---------- registrar (Configurações) ----------

pub async fn register_options(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    let (_, rp_id) = rp()?;
    let (name, email, instance): (String, String, String) = sqlx::query_as(
        "SELECT name, email, coalesce(nullif((SELECT name FROM masterbase.settings), ''), 'Masterbase') FROM masterbase.users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_one(&db)
    .await?;
    let existing: Vec<Vec<u8>> = sqlx::query_scalar("SELECT cred_id FROM masterbase.passkeys WHERE user_id = $1").bind(user.id).fetch_all(&db).await?;
    let challenge = new_challenge(&db, Some(user.id), "registro").await?;
    Ok(Json(json!({
        "challenge": challenge,
        "rp": { "id": rp_id, "name": instance },
        "user": { "id": B64.encode(user.id.to_be_bytes()), "name": email, "displayName": name },
        "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
        "authenticatorSelection": { "residentKey": "required", "userVerification": "required" },
        "excludeCredentials": existing.iter().map(|id| json!({ "type": "public-key", "id": B64.encode(id) })).collect::<Vec<_>>(),
        "attestation": "none",
        "timeout": 120000,
    })))
}

#[derive(Deserialize)]
pub struct Registration {
    name: String,
    #[serde(rename = "clientDataJSON")]
    client_data: String,
    #[serde(rename = "attestationObject")]
    attestation: String,
}

pub async fn register(user: User, State(db): State<PgPool>, Json(b): Json<Registration>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    let (origin, rp_id) = rp()?;
    let name = b.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(bad("Dê um nome de até 60 letras (ex.: celular, notebook)."));
    }
    let cd = unb64(&b.client_data)?;
    if client_data(&db, &cd, "registro", &origin).await? != Some(user.id) {
        return Err(bad("O pedido venceu. Tente de novo."));
    }
    let att: Cbor = ciborium::de::from_reader(&unb64(&b.attestation)?[..]).map_err(|_| bad("Resposta do aparelho inválida."))?;
    let auth_data = att
        .as_map()
        .and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("authData")))
        .and_then(|(_, v)| v.as_bytes())
        .ok_or(bad("Resposta do aparelho inválida."))?;
    let ad = parse_auth_data(auth_data, &rp_id).map_err(|_| bad("Este aparelho não serve: use um que peça digital, rosto ou PIN."))?;
    let (cred_id, public_key) = ad.cred.ok_or(bad("Resposta do aparelho inválida."))?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO masterbase.passkeys (user_id, cred_id, public_key, sign_count, name) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (cred_id) DO NOTHING RETURNING id",
    )
    .bind(user.id)
    .bind(&cred_id)
    .bind(&public_key)
    .bind(i64::from(ad.count))
    .bind(name)
    .fetch_optional(&db)
    .await?
    .ok_or(bad("Essa passkey já está cadastrada."))?;
    log(&db, user.id, "passkey_add", json!({ "name": name })).await?;
    Ok(Json(json!({ "id": id })))
}

pub async fn list(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    let rows: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', id, 'name', name, 'created_at', created_at, 'last_used_at', last_used_at)
         FROM masterbase.passkeys WHERE user_id = $1 ORDER BY id",
    )
    .bind(user.id)
    .fetch_all(&db)
    .await?;
    Ok(Json(json!({ "enabled": enabled(), "passkeys": rows })))
}

pub async fn remove(user: User, State(db): State<PgPool>, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    user.session.as_ref().ok_or(ApiError(StatusCode::FORBIDDEN, "Só pelo navegador."))?;
    let n = sqlx::query("DELETE FROM masterbase.passkeys WHERE id = $1 AND user_id = $2").bind(id).bind(user.id).execute(&db).await?.rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "Passkey não encontrada."));
    }
    log(&db, user.id, "passkey_remove", json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- entrar ----------

pub async fn login_options(State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    let (_, rp_id) = rp()?;
    let challenge = new_challenge(&db, None, "login").await?;
    Ok(Json(json!({ "challenge": challenge, "rpId": rp_id, "userVerification": "required", "timeout": 120000 })))
}

#[derive(Deserialize)]
pub struct Assertion {
    id: String,
    #[serde(rename = "clientDataJSON")]
    client_data: String,
    #[serde(rename = "authenticatorData")]
    auth_data: String,
    signature: String,
}

pub async fn login(State(db): State<PgPool>, headers: HeaderMap, jar: CookieJar, Json(b): Json<Assertion>) -> ApiResult<(CookieJar, Json<Value>)> {
    const NO: ApiError = ApiError(StatusCode::UNAUTHORIZED, "Essa passkey não entra aqui. Ela pode ter sido apagada.");
    let (origin, rp_id) = rp()?;
    let cred_id = unb64(&b.id)?;
    let (pk_id, user_id, public_key, stored): (i64, i64, Vec<u8>, i64) = sqlx::query_as(
        "SELECT p.id, p.user_id, p.public_key, p.sign_count FROM masterbase.passkeys p JOIN masterbase.users u ON u.id = p.user_id
         WHERE p.cred_id = $1 AND u.active",
    )
    .bind(&cred_id)
    .fetch_optional(&db)
    .await?
    .ok_or(NO)?;
    let cd = unb64(&b.client_data)?;
    client_data(&db, &cd, "login", &origin).await?;
    let ad_raw = unb64(&b.auth_data)?;
    let ad = parse_auth_data(&ad_raw, &rp_id).map_err(|_| NO)?;
    if !verify_sig(&public_key, &ad_raw, &cd, &unb64(&b.signature)?) {
        return Err(NO);
    }
    // Contador que volta para trás indica passkey clonada. Zero sempre (sincronizadas) é normal.
    let count = i64::from(ad.count);
    if count != 0 && count <= stored {
        return Err(NO);
    }
    let _ = ad.flags;
    sqlx::query("UPDATE masterbase.passkeys SET sign_count = $2, last_used_at = now() WHERE id = $1").bind(pk_id).bind(count).execute(&db).await?;
    log(&db, user_id, "login", json!({ "via": "passkey" })).await?;
    Ok((crate::auth::new_session(&db, jar, user_id, &headers).await?, Json(json!({ "ok": true }))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{signature::Signer, SigningKey};

    fn cose(key: &SigningKey) -> Vec<u8> {
        let point = key.verifying_key().to_encoded_point(false);
        let map = Cbor::Map(vec![
            (Cbor::Integer(1.into()), Cbor::Integer(2.into())),
            (Cbor::Integer(3.into()), Cbor::Integer((-7).into())),
            (Cbor::Integer((-1).into()), Cbor::Integer(1.into())),
            (Cbor::Integer((-2).into()), Cbor::Bytes(point.x().unwrap().to_vec())),
            (Cbor::Integer((-3).into()), Cbor::Bytes(point.y().unwrap().to_vec())),
        ]);
        let mut out = Vec::new();
        ciborium::ser::into_writer(&map, &mut out).unwrap();
        out
    }

    #[test]
    fn register_then_sign() {
        let key = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let cred = b"credencial-1".to_vec();
        let mut reg = sha(b"painel.exemplo.com");
        reg.push(0x45); // UP + UV + AT
        reg.extend(0u32.to_be_bytes());
        reg.extend([0u8; 16]);
        reg.extend((cred.len() as u16).to_be_bytes());
        reg.extend(&cred);
        reg.extend(cose(&key));
        let ad = parse_auth_data(&reg, "painel.exemplo.com").unwrap();
        let (id, public_key) = ad.cred.unwrap();
        assert_eq!(id, cred);
        assert!(parse_auth_data(&reg, "outro.com").is_err(), "rp id errado");

        let mut auth = sha(b"painel.exemplo.com");
        auth.push(0x05);
        auth.extend(1u32.to_be_bytes());
        let cd = br#"{"type":"webauthn.get"}"#;
        let sig: p256::ecdsa::Signature = key.sign(&[&auth[..], &sha(cd)].concat());
        let der = sig.to_der();
        assert!(verify_sig(&public_key, &auth, cd, der.as_bytes()));
        assert!(!verify_sig(&public_key, &auth, br#"{"type":"outro"}"#, der.as_bytes()));

        let mut no_uv = auth.clone();
        no_uv[32] = 0x01;
        assert!(parse_auth_data(&no_uv, "painel.exemplo.com").is_err(), "sem verificação do usuário");
    }
}
