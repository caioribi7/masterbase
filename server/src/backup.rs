//! Página Backups: pg_dump/pg_restore do schema public para arquivos em MB_BACKUP_DIR. Só para quem administra.
//! Os metadados do Masterbase (contas, sessões, auditoria) ficam fora do backup de propósito.
//!
//! Com MB_BACKUP_KEY (gerada na primeira vez que o Masterbase sobe), o backup sai criptografado no formato age:
//! a saída do pg_dump passa direto pela criptografia e nunca existe cópia aberta no disco.
//! A mesma chave abre o arquivo em outra máquina (`masterbase recuperar` ou `age -d`).
//!
//! Chave fora do servidor (`masterbase chave-backup tirar`): o arquivo de configuração fica só com a parte pública
//! (MB_BACKUP_RECIPIENT). O servidor continua criptografando, mas não consegue mais abrir os backups: quem invadir
//! a máquina leva os arquivos e não lê. Para restaurar, a pessoa cola a chave na tela ou usa `masterbase recuperar`.
//!
//! Backup automático: um por dia no horário escolhido. Guarda os últimos N (padrão 7) e um por semana de mais M (padrão 4).
//!
//! Cópia fora do servidor (só no arquivo de configuração, nunca pela tela):
//! MB_BACKUP_COPY_DIR copia cada backup novo para outra pasta (um disco montado, por exemplo);
//! MB_BACKUP_HOOK roda um comando com o caminho do arquivo em $MB_BACKUP_FILE (ex.: rclone copy "$MB_BACKUP_FILE" remoto:backups).
use crate::activity::{log, log_system};
use crate::auth::User;
use crate::error::{bad, ApiError, ApiResult};
use age::secrecy::ExposeSecret;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::io::{Read, Write};
use std::path::{Path as FsPath, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

const DAY: u64 = 86_400;

fn dir() -> PathBuf {
    PathBuf::from(std::env::var("MB_BACKUP_DIR").unwrap_or_else(|_| "backups".into()))
}

// ---------- chave ----------

pub fn new_key() -> String {
    age::x25519::Identity::generate().to_string().expose_secret().to_string()
}

pub fn parse_key(s: &str) -> Option<age::x25519::Identity> {
    s.trim().parse().ok()
}

/// A chave dos backups, se estiver no servidor. main.rs recusa subir com MB_BACKUP_KEY inválida, então aqui é Some ou ausente.
fn key() -> Option<age::x25519::Identity> {
    std::env::var("MB_BACKUP_KEY").ok().and_then(|k| parse_key(&k))
}

pub fn parse_recipient(s: &str) -> Option<age::x25519::Recipient> {
    s.trim().parse().ok()
}

/// Para quem o backup é criptografado: a parte pública da chave, esteja a chave aqui ou não.
fn recipient() -> Option<age::x25519::Recipient> {
    key().map(|k| k.to_public()).or_else(|| std::env::var("MB_BACKUP_RECIPIENT").ok().and_then(|r| parse_recipient(&r)))
}

// ---------- nomes ----------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Manual,
    Auto,
    /// Guardado sozinho antes de uma restauração.
    Before,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Kind::Manual => "backup",
            Kind::Auto => "auto",
            Kind::Before => "antes",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Kind::Manual => "manual",
            Kind::Auto => "auto",
            Kind::Before => "antes",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Name {
    pub kind: Kind,
    /// Segundos desde 1970, UTC.
    pub at: u64,
    pub encrypted: bool,
}

/// Nome = {backup|auto|antes}-AAAAMMDD-HHMMSS.dump[.age], sempre em UTC (a tela converte para o horário local).
/// Só aceita o formato que nós mesmos geramos: nada de "../" nem nome inventado.
pub fn parse(name: &str) -> Option<Name> {
    let (prefix, rest) = name.split_once('-')?;
    let kind = [Kind::Manual, Kind::Auto, Kind::Before].into_iter().find(|k| k.prefix() == prefix)?;
    let (stamp, ext) = rest.split_at_checked(15)?;
    let encrypted = match ext {
        ".dump" => false,
        ".dump.age" => true,
        _ => return None,
    };
    let b = stamp.as_bytes();
    if b[8] != b'-' || !b[..8].iter().chain(&b[9..]).all(u8::is_ascii_digit) {
        return None;
    }
    let n = |r: std::ops::Range<usize>| stamp[r].parse::<u32>().ok();
    let (y, mo, d, h, mi, s) = (n(0..4)?, n(4..6)?, n(6..8)?, n(9..11)?, n(11..13)?, n(13..15)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 59 {
        return None;
    }
    let days = days_from_civil(y as i64, mo, d);
    let at = u64::try_from(days).ok()? * DAY + (h * 3600 + mi * 60 + s) as u64;
    Some(Name { kind, at, encrypted })
}

fn stamp(at: u64) -> String {
    let (y, m, d) = civil_from_days((at / DAY) as i64);
    let s = at % DAY;
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", s / 3600, s / 60 % 60, s % 60)
}

// Calendário sem dependência (algoritmo de Howard Hinnant).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn path(name: &str) -> ApiResult<PathBuf> {
    match parse(name) {
        Some(_) => Ok(dir().join(name)),
        None => Err(ApiError(StatusCode::NOT_FOUND, "Backup não encontrado.")),
    }
}

/// (nome, informações, tamanho) dos backups da pasta, mais novo primeiro.
fn scan() -> Vec<(String, Name, u64)> {
    let mut items: Vec<_> = std::fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let info = parse(&name)?;
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            Some((name, info, size))
        })
        .collect();
    items.sort_by(|a, b| b.1.at.cmp(&a.1.at).then_with(|| b.0.cmp(&a.0)));
    items
}

// ---------- pg_dump / pg_restore ----------

pub enum Fail {
    Url,
    Tool(String),
    Key,
    Failed(String),
}

impl Fail {
    fn api(self) -> ApiError {
        let msg = match self {
            Fail::Url => "DATABASE_URL em formato inesperado.",
            Fail::Tool(e) => {
                eprintln!("backup: {e}");
                "Ferramenta de backup do Postgres não encontrada no servidor."
            }
            Fail::Key => return bad("A chave dos backups deste servidor não abre esse arquivo."),
            Fail::Failed(e) => {
                eprintln!("backup: {e}");
                "O Postgres recusou a operação de backup. Veja o log do servidor."
            }
        };
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }

    pub fn text(&self) -> String {
        match self {
            Fail::Url => "DATABASE_URL em formato inesperado".into(),
            Fail::Tool(e) => format!("ferramenta do Postgres não encontrada: {e}"),
            Fail::Key => "a chave não abre esse arquivo".into(),
            Fail::Failed(e) => e.trim().to_string(),
        }
    }
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

/// Comando do Postgres com a conexão em variáveis de ambiente: a senha não aparece na lista de processos.
fn pg(tool: &str, url: &str) -> Result<(Command, String), Fail> {
    let [host, port, user, pass, db] = conn_parts(url).ok_or(Fail::Url)?;
    let mut cmd = Command::new(tool);
    cmd.env("PGHOST", host).env("PGPORT", port).env("PGUSER", user).env("PGPASSWORD", pass).env("PGDATABASE", &db);
    Ok((cmd, db))
}

/// Lê o stderr numa thread à parte: um processo que escreve muito erro não trava esperando a gente.
fn collect_stderr(child: &mut std::process::Child) -> std::thread::JoinHandle<String> {
    let mut err = child.stderr.take();
    std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = err.as_mut() {
            e.read_to_string(&mut s).ok();
        }
        s
    })
}

fn finish(child: std::process::Child, err: std::thread::JoinHandle<String>, io: std::io::Result<()>) -> Result<(), Fail> {
    let out = child.wait_with_output().map_err(|e| Fail::Failed(e.to_string()))?;
    let err = err.join().unwrap_or_default();
    if !out.status.success() {
        return Err(Fail::Failed(err));
    }
    io.map_err(|e| Fail::Failed(format!("{e} {err}")))
}

/// pg_dump do schema public para `file`. Com chave, criptografa enquanto escreve.
fn write_dump(file: &FsPath, url: &str, to: Option<&age::x25519::Recipient>) -> Result<(), Fail> {
    let (mut cmd, _) = pg("pg_dump", url)?;
    let mut child = cmd
        .args(["-Fc", "-n", "public"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Fail::Tool(e.to_string()))?;
    let err = collect_stderr(&mut child);
    let mut stdout = child.stdout.take().expect("stdout");
    let io = (|| -> std::io::Result<()> {
        let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(file)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            out.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        match to {
            Some(r) => {
                let enc = age::Encryptor::with_recipients(std::iter::once(r as &dyn age::Recipient)).map_err(std::io::Error::other)?;
                let mut w = enc.wrap_output(&mut out)?;
                std::io::copy(&mut stdout, &mut w)?;
                w.finish()?;
            }
            None => {
                std::io::copy(&mut stdout, &mut out)?;
            }
        }
        out.sync_all()
    })();
    drop(stdout);
    finish(child, err, io)
}

/// Volta o schema public ao estado do arquivo. Uma transação só: se falhar no meio, nada muda.
pub fn restore_file(file: &FsPath, url: &str, key: Option<&age::x25519::Identity>) -> Result<(), Fail> {
    let (mut cmd, db) = pg("pg_restore", url)?;
    cmd.args(["-d", &db, "--clean", "--if-exists", "--no-owner", "--single-transaction"]);
    if !is_encrypted(file) {
        let mut child = cmd.arg(file).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| Fail::Tool(e.to_string()))?;
        let err = collect_stderr(&mut child);
        return finish(child, err, Ok(()));
    }
    // Abre e confere a chave antes de chamar o pg_restore: chave errada não toca no banco.
    let key = key.ok_or(Fail::Key)?;
    let reader = std::fs::File::open(file).map_err(|e| Fail::Failed(e.to_string()))?;
    let dec = age::Decryptor::new(std::io::BufReader::new(reader)).map_err(|_| Fail::Key)?;
    let mut plain = dec.decrypt(std::iter::once(key as &dyn age::Identity)).map_err(|_| Fail::Key)?;
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| Fail::Tool(e.to_string()))?;
    let err = collect_stderr(&mut child);
    let mut stdin = child.stdin.take().expect("stdin");
    let io = std::io::copy(&mut plain, &mut stdin).map(|_| ()).and_then(|_| stdin.flush());
    drop(stdin);
    finish(child, err, io)
}

pub fn is_encrypted(file: &FsPath) -> bool {
    file.extension().is_some_and(|e| e == "age")
}

/// Faz um backup e devolve o nome. Escreve em .part e renomeia no fim: a lista nunca mostra arquivo pela metade.
async fn dump(kind: Kind) -> ApiResult<String> {
    let dir = dir();
    tokio::fs::create_dir_all(&dir).await.map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Não consegui criar a pasta de backups."))?;
    let key = recipient();
    let ext = if key.is_some() { ".dump.age" } else { ".dump" };
    // O nome tem precisão de segundo. Se já existe um desse segundo, espera o próximo: backup nunca sobrescreve backup.
    let (name, file, part) = loop {
        let name = format!("{}-{}{ext}", kind.prefix(), stamp(now()));
        let file = dir.join(&name);
        let part = dir.join(format!("{name}.part"));
        if !file.exists() && !part.exists() {
            break (name, file, part);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    };
    let url = std::env::var("DATABASE_URL").unwrap_or_default();
    let tmp = part.clone();
    let res = tokio::task::spawn_blocking(move || write_dump(&tmp, &url, key.as_ref()))
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "O backup parou no meio."))?;
    if let Err(e) = res {
        tokio::fs::remove_file(&part).await.ok();
        return Err(e.api());
    }
    tokio::fs::rename(&part, &file).await.map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Não consegui salvar o backup."))?;
    if kind != Kind::Before {
        let copy_name = name.clone();
        tokio::task::spawn_blocking(move || offsite(&file, &copy_name));
    }
    Ok(name)
}

// ---------- cópia fora do servidor ----------

/// Resultado da última cópia externa: (quando, nome, erro). A tela mostra.
static LAST_COPY: Mutex<Option<(u64, String, Option<String>)>> = Mutex::new(None);

fn offsite(file: &FsPath, name: &str) {
    let dir = std::env::var("MB_BACKUP_COPY_DIR").ok().filter(|d| !d.trim().is_empty());
    let hook = std::env::var("MB_BACKUP_HOOK").ok().filter(|h| !h.trim().is_empty());
    if dir.is_none() && hook.is_none() {
        return;
    }
    let mut error = None;
    if let Some(d) = dir {
        let target = PathBuf::from(d);
        let part = target.join(format!("{name}.part"));
        let res = std::fs::create_dir_all(&target)
            .and_then(|_| std::fs::copy(file, &part))
            .and_then(|_| std::fs::rename(&part, target.join(name)));
        if let Err(e) = res {
            std::fs::remove_file(&part).ok();
            error = Some(format!("cópia para a pasta falhou: {e}"));
        }
    }
    if let Some(h) = hook {
        match Command::new("sh").arg("-c").arg(&h).env("MB_BACKUP_FILE", file).stdout(Stdio::null()).stderr(Stdio::piped()).output() {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                let msg: String = String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("").chars().take(200).collect();
                error.get_or_insert(format!("comando de cópia falhou: {msg}"));
            }
            Err(e) => {
                error.get_or_insert(format!("comando de cópia não rodou: {e}"));
            }
        }
    }
    if let Some(e) = &error {
        eprintln!("{} {e}", crate::activity::clock());
    } else if crate::activity::logging() {
        println!("{} sistema copiou {name} para fora do servidor", crate::activity::clock());
    }
    *LAST_COPY.lock().unwrap_or_else(|p| p.into_inner()) = Some((now(), name.to_string(), error));
}

// ---------- rotação ----------

/// Dos backups automáticos (horários, mais novo primeiro), quais apagar:
/// ficam os `daily` mais novos e, dos mais velhos, o mais novo de cada uma das `weekly` semanas seguintes.
fn to_prune(autos: &[u64], daily: usize, weekly: usize) -> Vec<u64> {
    let mut weeks: Vec<u64> = Vec::new();
    let mut out = Vec::new();
    for (i, &at) in autos.iter().enumerate() {
        if i < daily {
            continue;
        }
        let week = at / DAY / 7;
        if weeks.contains(&week) || weeks.len() >= weekly {
            out.push(at);
        } else {
            weeks.push(week);
        }
    }
    out
}

fn prune(daily: usize, weekly: usize) {
    let all = scan();
    let autos: Vec<u64> = all.iter().filter(|(_, n, _)| n.kind == Kind::Auto).map(|(_, n, _)| n.at).collect();
    let old = to_prune(&autos, daily, weekly);
    for (name, n, _) in &all {
        if n.kind == Kind::Auto && old.contains(&n.at) {
            if std::fs::remove_file(dir().join(name)).is_ok() && crate::activity::logging() {
                println!("{} sistema apagou o backup antigo {name}", crate::activity::clock());
            }
        }
    }
    // Restos de backup interrompido (servidor desligado no meio) com mais de um dia.
    for e in std::fs::read_dir(dir()).into_iter().flatten().flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d.as_secs() > DAY);
        if old && e.file_name().to_string_lossy().ends_with(".part") {
            std::fs::remove_file(e.path()).ok();
        }
    }
}

// ---------- agenda ----------

/// Último horário marcado que já passou (minuto do dia em UTC).
fn last_slot(now: u64, minute: u64) -> u64 {
    let t = now / DAY * DAY + minute * 60;
    if t <= now { t } else { t - DAY }
}

fn newest_auto() -> u64 {
    scan().iter().find(|(_, n, _)| n.kind == Kind::Auto).map_or(0, |(_, n, _)| n.at)
}

/// Erro do último backup automático, para a tela avisar. Some(texto) até o próximo dar certo.
static AUTO_FAILED: Mutex<Option<String>> = Mutex::new(None);

/// Roda para sempre: a cada minuto vê se o backup do dia já foi feito. Se o servidor estava desligado
/// no horário, faz assim que subir. Se falhar, tenta de novo em uma hora.
pub async fn scheduler(db: PgPool) {
    let mut retry_at = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let Ok((enabled, minute, daily, weekly)) =
            sqlx::query_as::<_, (bool, i32, i32, i32)>("SELECT enabled, minute, keep_daily, keep_weekly FROM masterbase.backup_settings").fetch_one(&db).await
        else {
            continue;
        };
        let now = now();
        if !enabled || now < retry_at || newest_auto() >= last_slot(now, minute as u64) {
            continue;
        }
        match dump(Kind::Auto).await {
            Ok(name) => {
                *AUTO_FAILED.lock().unwrap_or_else(|p| p.into_inner()) = None;
                tokio::task::spawn_blocking(move || prune(daily as usize, weekly as usize)).await.ok();
                log_system(&db, "auto_backup", json!({ "name": name })).await.ok();
            }
            Err(e) => {
                retry_at = now + 3600;
                eprintln!("{} backup automático falhou: {}", crate::activity::clock(), e.1);
                *AUTO_FAILED.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.1.to_string());
            }
        }
    }
}

// ---------- rotas ----------

pub async fn list(user: User) -> ApiResult<Json<Value>> {
    user.admin()?;
    let items: Vec<Value> = scan()
        .into_iter()
        .map(|(name, n, size)| json!({ "name": name, "size": size, "kind": n.kind.label(), "encrypted": n.encrypted, "at": n.at * 1000 }))
        .collect();
    Ok(Json(Value::Array(items)))
}

pub async fn create(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let name = dump(Kind::Manual).await?;
    log(&db, user.id, "backup", json!({ "name": name })).await?;
    Ok(Json(json!({ "name": name })))
}

/// Manda o arquivo aos pedaços: backup de vários GB não passa pela memória.
pub async fn download(user: User, Path(name): Path<String>) -> ApiResult<impl IntoResponse> {
    user.admin()?;
    let file = tokio::fs::File::open(path(&name)?).await.map_err(|_| ApiError(StatusCode::NOT_FOUND, "Backup não encontrado."))?;
    let size = file.metadata().await.map(|m| m.len()).unwrap_or(0);
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\"")),
            (header::CONTENT_LENGTH, size.to_string()),
        ],
        axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(file)),
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
    /// Chave colada na tela, quando ela não fica no servidor.
    key: Option<String>,
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
    let key = match b.key.as_deref().filter(|k| !k.trim().is_empty()) {
        Some(k) => Some(parse_key(k).ok_or(bad("Essa não é uma chave válida. Ela começa com AGE-SECRET-KEY-1."))?),
        None => key(),
    };
    if is_encrypted(&file) && key.is_none() {
        return Err(bad("A chave não fica neste servidor. Cole a chave de recuperação para restaurar."));
    }
    let safety = dump(Kind::Before).await?;
    let url = std::env::var("DATABASE_URL").unwrap_or_default();
    tokio::task::spawn_blocking(move || restore_file(&file, &url, key.as_ref()))
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "A restauração parou no meio."))?
        .map_err(Fail::api)?;
    log(&db, user.id, "restore", json!({ "name": name, "safety": safety })).await?;
    Ok(Json(json!({ "ok": true, "safety": safety })))
}

pub async fn get_settings(user: User, State(db): State<PgPool>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let (enabled, minute, key_saved, keep_daily, keep_weekly): (bool, i32, bool, i32, i32) = sqlx::query_as(
        "SELECT enabled, minute, key_saved_at IS NOT NULL, keep_daily, keep_weekly FROM masterbase.backup_settings",
    )
    .fetch_one(&db)
    .await?;
    let now = now();
    let slot = last_slot(now, minute as u64);
    let next = if newest_auto() < slot { now + 60 } else { slot + DAY };
    let failed = AUTO_FAILED.lock().unwrap_or_else(|p| p.into_inner()).clone();
    Ok(Json(json!({
        "enabled": enabled,
        "minute": minute,
        "next": enabled.then_some(next * 1000),
        "encrypted": recipient().is_some(),
        "key_on_server": key().is_some(),
        "key_saved": key_saved,
        "keep_daily": keep_daily,
        "keep_weekly": keep_weekly,
        "failed": failed,
        "copy": copy_info(),
    })))
}

/// Para onde vai a cópia externa e como foi a última. Não mostra o comando: pode ter senha de serviço.
fn copy_info() -> Value {
    let dir = std::env::var("MB_BACKUP_COPY_DIR").ok().filter(|d| !d.trim().is_empty());
    let hook = std::env::var("MB_BACKUP_HOOK").ok().filter(|h| !h.trim().is_empty());
    let last = LAST_COPY.lock().unwrap_or_else(|p| p.into_inner()).clone();
    json!({
        "dir": dir,
        "hook": hook.is_some(),
        "last": last.map(|(at, name, error)| json!({ "at": at * 1000, "name": name, "error": error })),
    })
}

#[derive(Deserialize)]
pub struct SettingsChange {
    enabled: Option<bool>,
    /// Minuto do dia em UTC (0 a 1439). A tela converte do horário de quem vê.
    minute: Option<i32>,
    key_saved: Option<bool>,
    keep_daily: Option<i32>,
    keep_weekly: Option<i32>,
}

pub async fn change_settings(user: User, State(db): State<PgPool>, Json(b): Json<SettingsChange>) -> ApiResult<Json<Value>> {
    user.admin()?;
    if b.minute.is_some_and(|m| !(0..1440).contains(&m)) {
        return Err(bad("Horário inválido."));
    }
    if b.keep_daily.is_some_and(|n| !(1..=60).contains(&n)) || b.keep_weekly.is_some_and(|n| !(0..=52).contains(&n)) {
        return Err(bad("Guarde de 1 a 60 diários e de 0 a 52 semanais."));
    }
    sqlx::query(
        "UPDATE masterbase.backup_settings SET enabled = coalesce($1, enabled), minute = coalesce($2, minute),
         key_saved_at = CASE WHEN $3 THEN now() WHEN $3 = false THEN NULL ELSE key_saved_at END,
         keep_daily = coalesce($4, keep_daily), keep_weekly = coalesce($5, keep_weekly)",
    )
    .bind(b.enabled)
    .bind(b.minute)
    .bind(b.key_saved)
    .bind(b.keep_daily)
    .bind(b.keep_weekly)
    .execute(&db)
    .await?;
    if b.enabled.is_some() || b.minute.is_some() || b.keep_daily.is_some() || b.keep_weekly.is_some() {
        let detail = json!({ "enabled": b.enabled, "minute": b.minute, "keep_daily": b.keep_daily, "keep_weekly": b.keep_weekly });
        log(&db, user.id, "backup_settings", detail).await?;
    }
    get_settings(user, State(db)).await
}

#[derive(Deserialize)]
pub struct ShowKey {
    password: String,
}

/// Mostra a chave de recuperação. Pede a senha de novo: uma sessão esquecida aberta não entrega a chave.
pub async fn show_key(user: User, State(db): State<PgPool>, Json(b): Json<ShowKey>) -> ApiResult<Json<Value>> {
    user.admin()?;
    // Mesmo limite do login: 5 senhas erradas em 15 minutos travam.
    let lock = format!("key:{}", user.id);
    if crate::limits::blocked(&db, &lock, 5).await? {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "Muitas tentativas. Espere 15 minutos."));
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM masterbase.users WHERE id = $1").bind(user.id).fetch_one(&db).await?;
    let ok = PasswordHash::new(&hash).map(|h| Argon2::default().verify_password(b.password.as_bytes(), &h).is_ok()).unwrap_or(false);
    if !ok {
        crate::limits::fail(&db, &lock).await?;
        return Err(bad("Senha incorreta."));
    }
    crate::limits::clear(&db, &lock).await?;
    let key = std::env::var("MB_BACKUP_KEY").map_err(|_| bad("A chave não fica neste servidor. Ela está com quem a tirou daqui."))?;
    log(&db, user.id, "backup_key", json!({})).await?;
    Ok(Json(json!({ "key": key.trim() })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_file_names() {
        assert_eq!(parse("backup-20261117-023015.dump").map(|n| (n.kind, n.encrypted)), Some((Kind::Manual, false)));
        assert_eq!(parse("auto-20261117-023015.dump.age").map(|n| (n.kind, n.encrypted)), Some((Kind::Auto, true)));
        assert_eq!(parse("antes-20261117-023015.dump.age").map(|n| n.kind), Some(Kind::Before));
        assert!(parse("../../etc/passwd").is_none());
        assert!(parse("backup-20261117-02301x.dump").is_none());
        assert!(parse("backup-20261117-023015.dump.sh").is_none());
        assert!(parse("backup-20261117-023015.dump.age.part").is_none());
        assert!(parse("outro-20261117-023015.dump").is_none());
        assert!(parse("backup-20261317-023015.dump").is_none());
        assert!(parse("auto-ção").is_none());
    }

    #[test]
    fn stamp_round_trip() {
        let n = parse("auto-20261117-023015.dump").unwrap();
        assert_eq!(n.at, 1_794_882_615);
        assert_eq!(stamp(n.at), "20261117-023015");
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(951_782_400), "20000229-000000");
    }

    #[test]
    fn schedule_slot() {
        let midnight = 1_794_873_600; // 17/11/2026 00:00 UTC
        assert_eq!(last_slot(midnight + 7 * 3600, 6 * 60), midnight + 6 * 3600);
        assert_eq!(last_slot(midnight + 5 * 3600, 6 * 60), midnight + 6 * 3600 - DAY);
    }

    #[test]
    fn prune_keeps_seven_days_and_four_weeks() {
        let base = 1_794_882_615;
        let autos: Vec<u64> = (0..60).map(|i| base - i * DAY).collect();
        let old = to_prune(&autos, 7, 4);
        let kept: Vec<u64> = autos.iter().copied().filter(|a| !old.contains(a)).collect();
        assert_eq!(kept.len(), 7 + 4);
        assert_eq!(&kept[..7], &autos[..7]);
        let weeks: Vec<u64> = kept[7..].iter().map(|a| a / DAY / 7).collect();
        assert!(weeks.windows(2).all(|w| w[0] > w[1]));
        assert!(to_prune(&autos[..5], 7, 4).is_empty());
        assert_eq!(to_prune(&autos, 3, 0).len(), 57);
    }

    #[test]
    fn key_round_trip() {
        let k = new_key();
        assert!(k.starts_with("AGE-SECRET-KEY-1"));
        assert!(parse_key(&k).is_some());
        assert!(parse_key("lixo").is_none());
    }

    #[test]
    fn parses_database_url() {
        let p = conn_parts("postgresql://mb:p%40ss%3Aword@127.0.0.1:5432/base?sslmode=disable").unwrap();
        assert_eq!(p, ["127.0.0.1", "5432", "mb", "p@ss:word", "base"].map(String::from));
        assert!(conn_parts("lixo").is_none());
    }
}
