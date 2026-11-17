//! Página Backups: pg_dump/pg_restore do schema public para arquivos em MB_BACKUP_DIR. Só para quem administra.
//! Os metadados do Masterbase (contas, sessões, auditoria) ficam fora do backup de propósito.
use crate::activity::log;
use crate::auth::User;
use crate::error::{bad, ApiError, ApiResult};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::process::Command;

fn dir() -> PathBuf {
    PathBuf::from(std::env::var("MB_BACKUP_DIR").unwrap_or_else(|_| "backups".into()))
}

/// Nome = backup-AAAAMMDD-HHMMSS.dump, sempre em UTC (a tela converte para o horário local).
/// Só aceita o formato que nós mesmos geramos: nada de "../" nem nome inventado.
fn valid(name: &str) -> bool {
    let b = name.as_bytes();
    name.len() == 27
        && name.starts_with("backup-")
        && name.ends_with(".dump")
        && b[7..15].iter().all(u8::is_ascii_digit)
        && b[15] == b'-'
        && b[16..22].iter().all(u8::is_ascii_digit)
}

fn path(name: &str) -> ApiResult<PathBuf> {
    if !valid(name) {
        return Err(ApiError(StatusCode::NOT_FOUND, "Backup não encontrado."));
    }
    Ok(dir().join(name))
}

fn pct(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && s.is_char_boundary(i + 1) && s.is_char_boundary(i + 3) {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// (host, porta, usuário, senha, banco) a partir de DATABASE_URL.
fn conn_parts(url: &str) -> Option<[String; 5]> {
    let rest = url.split_once("://")?.1.split('?').next()?;
    let (auth, hostdb) = rest.rsplit_once('@')?;
    let (user, pass) = auth.split_once(':').unwrap_or((auth, ""));
    let (hostport, db) = hostdb.split_once('/')?;
    let (host, port) = hostport.rsplit_once(':').unwrap_or((hostport, "5432"));
    Some([pct(host), port.to_owned(), pct(user), pct(pass), pct(db)])
}

/// pg_dump/pg_restore com a conexão em variáveis de ambiente: a senha não aparece na lista de processos.
async fn pg(tool: &str, args: &[&str]) -> ApiResult<()> {
    let url = std::env::var("DATABASE_URL").unwrap_or_default();
    let [host, port, user, pass, db] = conn_parts(&url).ok_or(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_URL em formato inesperado."))?;
    let mut cmd = Command::new(tool);
    if tool == "pg_restore" {
        cmd.args(["-d", &db]);
    }
    let out = cmd
        .args(args)
        .env("PGHOST", host)
        .env("PGPORT", port)
        .env("PGUSER", user)
        .env("PGPASSWORD", pass)
        .env("PGDATABASE", db)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| {
            eprintln!("{tool}: {e}");
            ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Ferramenta de backup do Postgres não encontrada no servidor.")
        })?;
    if !out.status.success() {
        eprintln!("{tool}: {}", String::from_utf8_lossy(&out.stderr));
        return Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, "O Postgres recusou a operação de backup. Veja o log do servidor."));
    }
    Ok(())
}

pub async fn list(user: User) -> ApiResult<Json<Value>> {
    user.admin()?;
    let mut items = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(dir()).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            let name = e.file_name().to_string_lossy().into_owned();
            if valid(&name) {
                let size = e.metadata().await.map(|m| m.len()).unwrap_or(0);
                items.push(json!({ "name": name, "size": size }));
            }
        }
    }
    // O nome carrega data e hora: ordem alfabética invertida = mais novo primeiro.
    items.sort_by(|a, b| b["name"].as_str().cmp(&a["name"].as_str()));
    Ok(Json(Value::Array(items)))
}

async fn dump(db: &PgPool) -> ApiResult<String> {
    tokio::fs::create_dir_all(dir()).await.map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Não consegui criar a pasta de backups."))?;
    // O nome tem precisão de segundo. Se já existe um desse segundo, espera o próximo: backup nunca sobrescreve backup.
    let (name, file) = loop {
        let stamp: String = sqlx::query_scalar("SELECT to_char(now() AT TIME ZONE 'UTC', 'YYYYMMDD-HH24MISS')").fetch_one(db).await?;
        let name = format!("backup-{stamp}.dump");
        let file = dir().join(&name);
        if !file.exists() {
            break (name, file);
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    };
    pg("pg_dump", &["-Fc", "-n", "public", "-f", &file.to_string_lossy()]).await?;
    Ok(name)
}

pub async fn create(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name = dump(&db).await?;
    log(&db, user.id, "backup", json!({ "name": name })).await?;
    Ok(Json(json!({ "name": name })))
}

pub async fn download(user: User, Path(name): Path<String>) -> ApiResult<impl IntoResponse> {
    user.admin()?;
    // shortcut: arquivo inteiro na memória. Trocar por stream quando os bancos passarem de centenas de MB.
    let bytes = tokio::fs::read(path(&name)?).await.map_err(|_| ApiError(StatusCode::NOT_FOUND, "Backup não encontrado."))?;
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream".to_owned()), (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\""))],
        bytes,
    ))
}

pub async fn delete(user: User, State(db): State<PgPool>, Path(name): Path<String>) -> ApiResult<Json<Value>> {
    user.admin()?;
    tokio::fs::remove_file(path(&name)?).await.map_err(|_| ApiError(StatusCode::NOT_FOUND, "Backup não encontrado."))?;
    log(&db, user.id, "drop_backup", json!({ "name": name })).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct Confirm {
    confirm: String,
}

/// Volta as tabelas do backup ao estado em que estavam. Uma transação só: se falhar no meio, nada muda.
/// Antes, guarda um backup do estado atual. Tabelas criadas depois do backup continuam existindo.
pub async fn restore(user: User, State(db): State<PgPool>, Path(name): Path<String>, Json(b): Json<Confirm>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let file = path(&name)?;
    if b.confirm != "restaurar" {
        return Err(bad("Digite restaurar para confirmar."));
    }
    if !file.exists() {
        return Err(ApiError(StatusCode::NOT_FOUND, "Backup não encontrado."));
    }
    let safety = dump(&db).await?;
    pg("pg_restore", &["--clean", "--if-exists", "--no-owner", "--single-transaction", &file.to_string_lossy()]).await?;
    log(&db, user.id, "restore", json!({ "name": name, "safety": safety })).await?;
    Ok(Json(json!({ "ok": true, "safety": safety })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_file_names() {
        assert!(valid("backup-20261117-023015.dump"));
        assert!(!valid("../../etc/passwd"));
        assert!(!valid("backup-20261117-02301x.dump"));
        assert!(!valid("backup-20261117-023015.dump.sh"));
    }

    #[test]
    fn parses_database_url() {
        let p = conn_parts("postgresql://mb:p%40ss%3Aword@127.0.0.1:5432/base?sslmode=disable").unwrap();
        assert_eq!(p, ["127.0.0.1", "5432", "mb", "p@ss:word", "base"].map(String::from));
        assert!(conn_parts("lixo").is_none());
    }
}
