//! Entrar com Google ou GitHub. Só para quem já tem conta: o e-mail verificado no provedor
//! precisa ser o mesmo da conta no Masterbase. Ninguém se cadastra sozinho por aqui.
//! Liga com MB_URL mais MB_GOOGLE_ID/MB_GOOGLE_SECRET ou MB_GITHUB_ID/MB_GITHUB_SECRET.

use crate::activity::log;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

const STATE_COOKIE: &str = "mb_oauth";

struct Provider {
    name: &'static str,
    authorize: &'static str,
    token: &'static str,
    scope: &'static str,
    id: String,
    secret: String,
}

fn provider(name: &str) -> Option<Provider> {
    let (env, authorize, token, scope) = match name {
        "google" => ("GOOGLE", "https://accounts.google.com/o/oauth2/v2/auth", "https://oauth2.googleapis.com/token", "openid email"),
        "github" => ("GITHUB", "https://github.com/login/oauth/authorize", "https://github.com/login/oauth/access_token", "user:email"),
        _ => return None,
    };
    crate::mail::public_url()?;
    let get = |k: &str| std::env::var(format!("MB_{env}_{k}")).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    Some(Provider { name: if name == "google" { "google" } else { "github" }, authorize, token, scope, id: get("ID")?, secret: get("SECRET")? })
}

/// Provedores ligados, para a tela mostrar os botões.
pub fn enabled() -> Vec<&'static str> {
    ["google", "github"].into_iter().filter(|p| provider(p).is_some()).collect()
}

fn callback_url(p: &Provider) -> String {
    format!("{}/api/oauth/{}/callback", crate::mail::public_url().unwrap_or_default(), p.name)
}

fn back(error: &str) -> Response {
    Redirect::to(&format!("/#/entrar?erro={error}")).into_response()
}

pub async fn start(Path(name): Path<String>, jar: CookieJar) -> Response {
    let Some(p) = provider(&name) else { return back("desligado") };
    let state = hex::encode(crate::auth::random_token());
    let mut params = vec![
        ("client_id", p.id.clone()),
        ("redirect_uri", callback_url(&p)),
        ("response_type", "code".into()),
        ("scope", p.scope.into()),
        ("state", state.clone()),
    ];
    if p.name == "google" {
        params.push(("prompt", "select_account".into()));
    }
    let Ok(url) = reqwest::Url::parse_with_params(p.authorize, &params) else { return back("falhou") };
    // Lax: o cookie precisa voltar quando o provedor redireciona de volta para cá.
    let cookie = Cookie::build((STATE_COOKIE, state))
        .http_only(true)
        .secure(crate::auth::secure_cookie())
        .same_site(SameSite::Lax)
        .path("/api/oauth")
        .max_age(time::Duration::minutes(10));
    (jar.add(cookie), Redirect::to(url.as_str())).into_response()
}

#[derive(Deserialize)]
pub struct Callback {
    code: Option<String>,
    state: Option<String>,
}

pub async fn callback(State(db): State<PgPool>, Path(name): Path<String>, Query(q): Query<Callback>, headers: HeaderMap, jar: CookieJar) -> Response {
    let Some(p) = provider(&name) else { return back("desligado") };
    let expected = jar.get(STATE_COOKIE).map(|c| c.value().to_string());
    let jar = jar.remove(Cookie::build(STATE_COOKIE).path("/api/oauth"));
    // O state amarra a volta a este navegador: sem ele, alguém faria você entrar na conta dele.
    let (Some(code), Some(state), Some(expected)) = (q.code, q.state, expected) else { return (jar, back("cancelado")).into_response() };
    if state != expected {
        return (jar, back("falhou")).into_response();
    }
    let emails = match verified_emails(&p, &code).await {
        Ok(e) if !e.is_empty() => e,
        Ok(_) => return (jar, back("sem-email")).into_response(),
        Err(e) => {
            eprintln!("{} login com {} falhou: {e}", crate::activity::clock(), p.name);
            return (jar, back("falhou")).into_response();
        }
    };
    let id: Result<Option<i64>, _> = sqlx::query_scalar("SELECT id FROM masterbase.users WHERE email = ANY($1) AND active ORDER BY id LIMIT 1")
        .bind(&emails)
        .fetch_optional(&db)
        .await;
    let Ok(Some(id)) = id else { return (jar, back("sem-conta")).into_response() };
    match crate::mfa::ticket(&db, id, p.name).await {
        Ok(Some(t)) => return (jar, Redirect::to(&format!("/#/entrar?fator={t}"))).into_response(),
        Ok(None) => {}
        Err(_) => return (jar, back("falhou")).into_response(),
    }
    if log(&db, id, "login", json!({ "via": p.name })).await.is_err() {
        return (jar, back("falhou")).into_response();
    }
    match crate::auth::new_session(&db, jar, id, &headers).await {
        Ok(jar) => (jar, Redirect::to("/#/inicio")).into_response(),
        Err(_) => back("falhou"),
    }
}

/// Troca o código pelo token e pergunta ao provedor quais e-mails verificados a pessoa tem.
async fn verified_emails(p: &Provider, code: &str) -> Result<Vec<String>, String> {
    let http = reqwest::Client::builder()
        .user_agent("masterbase")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let redirect = callback_url(p);
    let token: Value = http
        .post(p.token)
        .header("accept", "application/json")
        .form(&[
            ("client_id", p.id.as_str()),
            ("client_secret", p.secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let access = token["access_token"].as_str().ok_or_else(|| format!("sem token: {}", token["error"]))?;
    let url = if p.name == "google" { "https://openidconnect.googleapis.com/v1/userinfo" } else { "https://api.github.com/user/emails" };
    let info: Value = http.get(url).bearer_auth(access).send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    Ok(emails_from(p.name, &info))
}

fn emails_from(provider: &str, info: &Value) -> Vec<String> {
    let list: Vec<&Value> = if provider == "google" { vec![info] } else { info.as_array().map(|a| a.iter().collect()).unwrap_or_default() };
    list.into_iter()
        .filter(|e| e["email_verified"].as_bool().or(e["verified"].as_bool()) == Some(true))
        .filter_map(|e| e["email"].as_str())
        .map(|e| e.trim().to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_verified_emails() {
        let gh = json!([
            { "email": "Ana@Exemplo.com", "verified": true, "primary": true },
            { "email": "velho@exemplo.com", "verified": false, "primary": false }
        ]);
        assert_eq!(emails_from("github", &gh), vec!["ana@exemplo.com"]);
        assert_eq!(emails_from("google", &json!({ "email": "a@b.com", "email_verified": true })), vec!["a@b.com"]);
        assert!(emails_from("google", &json!({ "email": "a@b.com", "email_verified": false })).is_empty());
        assert!(emails_from("github", &json!({ "message": "Bad credentials" })).is_empty());
    }
}
