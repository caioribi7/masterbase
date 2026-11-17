//! E-mail por SMTP: convite e "esqueci a senha". Desligado sem MB_SMTP_HOST e MB_URL.
//! Serve qualquer SMTP: Gmail com senha de app, Brevo, Resend, o do provedor do domínio.

use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use sqlx::PgPool;

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Endereço público do painel (MB_URL), sem barra no fim. Links de e-mail e de login usam este.
/// Nunca o Host do pedido: quem pede "esqueci a senha" poderia trocar o endereço do link.
pub fn public_url() -> Option<String> {
    env("MB_URL").map(|u| u.trim_end_matches('/').to_string()).filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

pub fn enabled() -> bool {
    env("MB_SMTP_HOST").is_some() && public_url().is_some()
}

fn transport() -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let host = env("MB_SMTP_HOST").ok_or("sem MB_SMTP_HOST")?;
    let port: u16 = env("MB_SMTP_PORT").and_then(|p| p.parse().ok()).unwrap_or(465);
    // Sem TLS só para um servidor de e-mail na própria máquina. Fora dela, 465 é TLS direto e o resto STARTTLS obrigatório.
    let mut b = if host == "localhost" || host == "127.0.0.1" {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
    } else if port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&host).map_err(|e| e.to_string())?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host).map_err(|e| e.to_string())?
    };
    b = b.port(port);
    if let Some(user) = env("MB_SMTP_USER") {
        b = b.credentials(Credentials::new(user, env("MB_SMTP_PASS").unwrap_or_default()));
    }
    Ok(b.build())
}

pub async fn send(to: &str, subject: &str, body: String) -> Result<(), String> {
    let from = env("MB_MAIL_FROM").or_else(|| env("MB_SMTP_USER")).ok_or("sem MB_MAIL_FROM")?;
    let from: Mailbox = if from.contains('<') { from.parse() } else { format!("Masterbase <{from}>").parse() }.map_err(|_| "MB_MAIL_FROM inválido")?;
    let to: Mailbox = to.parse().map_err(|_| "e-mail de destino inválido")?;
    let msg = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| e.to_string())?;
    transport()?.send(msg).await.map(|_| ()).map_err(|e| e.to_string())
}

/// Manda o link de criar senha. `invite`: conta nova. Devolve se saiu.
pub async fn password_link(db: &PgPool, user_id: i64, token: &str, invite: bool) -> bool {
    let (Some(url), true) = (public_url(), enabled()) else { return false };
    let row: Result<(String, String, String), _> = sqlx::query_as(
        "SELECT u.name, u.email, coalesce(nullif((SELECT s.name FROM masterbase.settings s), ''), 'Masterbase')
         FROM masterbase.users u WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await;
    let Ok((name, email, instance)) = row else { return false };
    let link = format!("{url}/#/senha/{token}");
    let (subject, body) = if invite {
        (
            format!("Seu acesso ao {instance}"),
            format!(
                "Oi, {name}.\n\nVocê ganhou acesso ao {instance}. Crie sua senha neste link:\n\n{link}\n\n\
                 O link vale 24 horas e funciona uma vez só.\n\nSe não esperava este e-mail, pode ignorar."
            ),
        )
    } else {
        (
            format!("Senha nova no {instance}"),
            format!(
                "Oi, {name}.\n\nPediram uma senha nova para a sua conta no {instance}. Crie a senha neste link:\n\n{link}\n\n\
                 O link vale 24 horas e funciona uma vez só. Ao salvar, os aparelhos conectados saem da conta.\n\n\
                 Se não foi você, ignore este e-mail: sua senha continua a mesma."
            ),
        )
    };
    match send(&email, &subject, body).await {
        Ok(()) => true,
        Err(e) => {
            eprintln!("{} e-mail para {email} não saiu: {e}", crate::activity::clock());
            false
        }
    }
}
