//! Páginas Acesso (pessoas e papéis) e API e chaves. Tudo aqui é só para quem administra.
use crate::activity::log;
use crate::auth::{hash_password, sha, Role, User};
use crate::error::{bad, ApiError, ApiResult};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

fn check_password(p: &str) -> ApiResult<()> {
    if p.chars().count() < 10 { Err(bad("A senha precisa de 10 caracteres ou mais.")) } else { Ok(()) }
}

pub async fn list_users(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let rows: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', id, 'name', name, 'email', email, 'role', role, 'active', active, 'created_at', created_at, 'totp', totp_secret IS NOT NULL)
         FROM masterbase.users ORDER BY id",
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(json!({ "me": user.id, "users": rows })))
}

#[derive(Deserialize)]
pub struct NewUser {
    name: String,
    email: String,
    #[serde(default)]
    password: String,
    role: String,
}

pub async fn create_user(user: User, State(db): State<PgPool>, Json(b): Json<NewUser>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let email = b.email.trim().to_lowercase();
    if b.name.trim().is_empty() || !email.contains('@') {
        return Err(bad("Informe nome e um e-mail válido."));
    }
    // Sem senha: a pessoa recebe um link de convite e cria a própria senha.
    let invite = b.password.is_empty();
    let password = if invite {
        hex::encode(crate::auth::random_token())
    } else {
        check_password(&b.password)?;
        b.password.clone()
    };
    Role::parse(&b.role).ok_or(bad("Papel desconhecido."))?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO masterbase.users (name, email, password_hash, role) VALUES ($1, $2, $3, $4)
         ON CONFLICT (email) DO NOTHING RETURNING id",
    )
    .bind(b.name.trim())
    .bind(&email)
    .bind(hash_password(&password)?)
    .bind(&b.role)
    .fetch_optional(&db)
    .await?
    .ok_or(ApiError(StatusCode::CONFLICT, "Esse e-mail já tem conta."))?;
    log(&db, user.id, "add_user", json!({ "email": email, "role": b.role })).await?;
    let token = if invite { Some(crate::auth::new_password_link(&db, id).await?) } else { None };
    let mailed = match &token {
        Some(t) => crate::mail::password_link(&db, id, t, true).await,
        None => false,
    };
    Ok(Json(json!({ "id": id, "token": token, "mailed": mailed })))
}

/// Link para a pessoa criar uma senha nova (esqueceu a dela). Vale 24 horas, uma vez só.
/// Com e-mail ligado, o link também vai para a caixa da pessoa. Sem, quem administra copia e envia.
pub async fn password_link(user: User, State(db): State<PgPool>, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let email: String = sqlx::query_scalar("SELECT email FROM masterbase.users WHERE id = $1 AND active")
        .bind(id)
        .fetch_optional(&db)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Pessoa não encontrada ou desativada."))?;
    let token = crate::auth::new_password_link(&db, id).await?;
    log(&db, user.id, "password_link", json!({ "email": email })).await?;
    let mailed = crate::mail::password_link(&db, id, &token, false).await;
    Ok(Json(json!({ "token": token, "mailed": mailed })))
}

#[derive(Deserialize)]
pub struct UserChange {
    role: Option<String>,
    active: Option<bool>,
    password: Option<String>,
    /// true desliga o segundo fator da pessoa (perdeu o celular e os códigos).
    reset_2fa: Option<bool>,
}

pub async fn change_user(user: User, State(db): State<PgPool>, Path(id): Path<i64>, Json(b): Json<UserChange>) -> ApiResult<Json<Value>> {
    user.admin()?;
    if id == user.id && (b.role.is_some() || b.active.is_some()) {
        // Sem isso, a única conta que administra poderia se trancar para fora.
        return Err(bad("Você não pode mudar o próprio papel nem se desativar."));
    }
    if let Some(r) = &b.role {
        Role::parse(r).ok_or(bad("Papel desconhecido."))?;
    }
    let hash = match &b.password {
        Some(p) => {
            check_password(p)?;
            Some(hash_password(p)?)
        }
        None => None,
    };
    let mut tx = db.begin().await?;
    let email: String = sqlx::query_scalar(
        "UPDATE masterbase.users SET role = coalesce($2, role), active = coalesce($3, active), password_hash = coalesce($4, password_hash)
         WHERE id = $1 RETURNING email",
    )
    .bind(id)
    .bind(&b.role)
    .bind(b.active)
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError(StatusCode::NOT_FOUND, "Pessoa não encontrada."))?;
    if b.active == Some(false) || hash.is_some() {
        sqlx::query("DELETE FROM masterbase.sessions WHERE user_id = $1").bind(id).execute(&mut *tx).await?;
    }
    if b.reset_2fa == Some(true) {
        sqlx::query("UPDATE masterbase.users SET totp_secret = NULL WHERE id = $1").bind(id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM masterbase.recovery_codes WHERE user_id = $1").bind(id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    log(&db, user.id, "change_user", json!({ "email": email, "role": b.role, "active": b.active, "password": hash.is_some(), "reset_2fa": b.reset_2fa })).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct PasswordChange {
    current: String,
    new: String,
}

/// Trocar a própria senha. Encerra as outras sessões.
pub async fn change_password(user: User, State(db): State<PgPool>, Json(b): Json<PasswordChange>) -> ApiResult<Json<Value>> {
    if user.via_key {
        return Err(ApiError(StatusCode::FORBIDDEN, "Chave de API não troca senha."));
    }
    check_password(&b.new)?;
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM masterbase.users WHERE id = $1").bind(user.id).fetch_one(&db).await?;
    let ok = PasswordHash::new(&hash).map(|h| Argon2::default().verify_password(b.current.as_bytes(), &h).is_ok()).unwrap_or(false);
    if !ok {
        return Err(bad("A senha atual não confere."));
    }
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE masterbase.users SET password_hash = $2 WHERE id = $1")
        .bind(user.id)
        .bind(hash_password(&b.new)?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM masterbase.sessions WHERE user_id = $1 AND token_hash IS DISTINCT FROM $2")
        .bind(user.id)
        .bind(&user.session)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM masterbase.password_tokens WHERE user_id = $1").bind(user.id).execute(&mut *tx).await?;
    tx.commit().await?;
    log(&db, user.id, "change_password", json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- chaves de API ----------

pub async fn list_keys(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let rows: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', k.id, 'name', k.name, 'role', k.role, 'prefix', k.prefix,
                'created_at', k.created_at, 'last_used_at', k.last_used_at, 'user', u.name)
         FROM masterbase.api_keys k JOIN masterbase.users u ON u.id = k.user_id ORDER BY k.id DESC",
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(Value::Array(rows)))
}

#[derive(Deserialize)]
pub struct NewKey {
    name: String,
    role: String,
}

pub async fn create_key(user: User, State(db): State<PgPool>, Json(b): Json<NewKey>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name = b.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(bad("Dê um nome para a chave (até 80 caracteres)."));
    }
    if b.role != "leitor" && b.role != "editor" {
        return Err(bad("Chave só pode ler ou editar linhas."));
    }
    let mut raw = [0u8; 24];
    OsRng.fill_bytes(&mut raw);
    let token = format!("mb_{}", hex::encode(raw));
    let prefix = &token[..11];
    sqlx::query("INSERT INTO masterbase.api_keys (user_id, name, role, token_hash, prefix) VALUES ($1, $2, $3, $4, $5)")
        .bind(user.id)
        .bind(name)
        .bind(&b.role)
        .bind(sha(token.as_bytes()))
        .bind(prefix)
        .execute(&db)
        .await?;
    log(&db, user.id, "add_key", json!({ "name": name, "role": b.role })).await?;
    // Única vez em que a chave inteira sai do servidor.
    Ok(Json(json!({ "token": token })))
}

pub async fn delete_key(user: User, State(db): State<PgPool>, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name: String = sqlx::query_scalar("DELETE FROM masterbase.api_keys WHERE id = $1 RETURNING name")
        .bind(id)
        .fetch_optional(&db)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Chave não encontrada."))?;
    log(&db, user.id, "drop_key", json!({ "name": name })).await?;
    Ok(Json(json!({ "ok": true })))
}
