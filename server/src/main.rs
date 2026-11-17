mod access;
mod activity;
mod auth;
mod backup;
mod catalog;
mod config;
mod data;
mod error;
mod mail;
mod oauth;
mod query;
mod schema;
mod sql;

use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

/// A tela compilada (web/dist) vai dentro do binário. Sem a pasta, compila igual e avisa ao abrir.
#[derive(rust_embed::Embed)]
#[folder = "../web/dist"]
#[allow_missing = true]
struct Web;

/// Uma linha por pedido à API: hora, método, caminho (sem a parte depois do ?, que pode ter dados), status, tempo.
async fn log_request(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = std::time::Instant::now();
    let res = next.run(req).await;
    if path.starts_with("/api/") && activity::logging() {
        println!("{} {method} {path} {} {}ms", activity::clock(), res.status().as_u16(), started.elapsed().as_millis());
    }
    res
}

async fn web_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let file = Web::get(path).filter(|_| !path.is_empty()).or_else(|| Web::get("index.html"));
    match file {
        Some(f) => ([(header::CONTENT_TYPE, f.metadata.mimetype().to_string())], f.data).into_response(),
        None => (StatusCode::NOT_FOUND, "tela não compilada: rode npm run build em web/").into_response(),
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        None | Some("iniciar") => {}
        Some("configurar") => return config::configure(),
        Some("servico" | "serviço") => return config::service(),
        Some("versao" | "versão" | "--version" | "-V") => return println!("masterbase {}", env!("CARGO_PKG_VERSION")),
        Some("ajuda" | "--help" | "-h") => return config::help(),
        Some("chave-backup") => return config::show_backup_key(),
        Some("recuperar") => return config::recover(std::env::args().nth(2)),
        Some("senha") => return config::password_link(std::env::args().nth(2)),
        Some(other) => {
            eprintln!("comando desconhecido: {other}
");
            config::help();
            std::process::exit(2);
        }
    }
    config::load();
    if std::env::var_os("DATABASE_URL").is_none() {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            eprintln!("sem configuração: rode `masterbase configurar` ou defina DATABASE_URL");
            std::process::exit(1);
        }
        println!("Primeira vez por aqui.
");
        config::configure();
        config::load();
    }
    config::ensure_backup_key();
    tokio::runtime::Runtime::new().expect("runtime").block_on(serve());
}

async fn serve() {
    let url = std::env::var("DATABASE_URL").expect("defina DATABASE_URL");
    // search_path com masterbase primeiro: a tabela _sqlx_migrations fica fora de public (e fora da tela).
    // statement_timeout: consulta pesada não segura conexão para sempre.
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .after_connect(|conn, _| Box::pin(async move {
            sqlx::Executor::execute(conn, "SET search_path = masterbase, public; SET statement_timeout = '15s'").await.map(|_| ())
        }))
        .connect(&url)
        .await
        .expect("sem conexão com o Postgres");
    sqlx::query("CREATE SCHEMA IF NOT EXISTS masterbase").execute(&db).await.expect("sem permissão para criar schema");
    sqlx::migrate!().run(&db).await.expect("migração falhou");
    sql::connect(&db).await;
    tokio::spawn(backup::scheduler(db.clone()));

    let api = Router::new()
        .route("/setup", get(auth::setup))
        .route("/signup", post(auth::signup))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/me", get(auth::me).patch(sql::update_me))
        .route("/settings", get(sql::settings).patch(sql::change_settings))
        .route("/sessions", get(auth::list_sessions).delete(auth::end_other_sessions))
        .route("/sessions/{id}", delete(auth::end_session))
        .route("/reset/check", post(auth::reset_check))
        .route("/reset", post(auth::reset_password))
        .route("/forgot", post(auth::forgot))
        .route("/oauth/{provider}", get(oauth::start))
        .route("/oauth/{provider}/callback", get(oauth::callback))
        .route("/sql", post(sql::run))
        .route("/overview", get(activity::overview))
        .route("/activity", get(activity::list))
        .route("/tables", get(data::list_tables))
        .route("/tables/{name}", get(data::read_table))
        .route("/tables/{name}/rows", post(data::insert_row).patch(data::update_cell).delete(data::delete_rows))
        .route("/tables/{name}/import", post(data::import_rows).layer(DefaultBodyLimit::max(8 * 1024 * 1024)))
        .route("/query", post(query::run))
        .route("/queries", get(query::list_saved).post(query::save))
        .route("/queries/{id}", delete(query::delete_saved))
        .route("/users", get(access::list_users).post(access::create_user))
        .route("/users/{id}", patch(access::change_user))
        .route("/users/{id}/link", post(access::password_link))
        .route("/password", post(access::change_password))
        .route("/keys", get(access::list_keys).post(access::create_key))
        .route("/keys/{id}", delete(access::delete_key))
        .route("/backups", get(backup::list).post(backup::create))
        .route("/backups/settings", get(backup::get_settings).patch(backup::change_settings))
        .route("/backups/key", post(backup::show_key))
        .route("/backups/{name}", get(backup::download).delete(backup::delete))
        .route("/backups/{name}/restore", post(backup::restore))
        .route("/schema", get(schema::get_schema).post(schema::create_table))
        .route("/schema/{table}", patch(schema::rename_table).delete(schema::drop_table))
        .route("/schema/{table}/columns", post(schema::add_column))
        .route("/schema/{table}/columns/{column}", patch(schema::change_column).delete(schema::drop_column))
        // Respostas da API têm dados da conta: nada de cache no navegador nem em proxy.
        .layer(header_layer(header::CACHE_CONTROL, "no-store"));

    // MB_WEB_DIR serve a tela do disco (desenvolvimento). Sem ela, usa a que está dentro do binário.
    let app = Router::new()
        .nest("/api", api)
        .route("/llms.txt", get(|| async { ([(header::CONTENT_TYPE, "text/markdown; charset=utf-8")], include_str!("../../docs/integracao-ia.md")) }));
    let app = match std::env::var("MB_WEB_DIR") {
        Ok(web) => app.fallback_service(ServeDir::new(&web).fallback(ServeFile::new(format!("{web}/index.html")))),
        Err(_) => app.fallback(web_file),
    };
    let app = app
        .layer(header_layer(header::CONTENT_SECURITY_POLICY, "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'"))
        .layer(header_layer(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(header_layer(header::REFERRER_POLICY, "no-referrer"))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(axum::middleware::from_fn(log_request))
        .with_state(db);

    let addr = std::env::var("MB_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        eprintln!("não deu para usar {addr}: {e}. Mude a porta com `masterbase configurar`.");
        std::process::exit(1);
    });
    let local = addr.replace("0.0.0.0", "127.0.0.1");
    println!("Masterbase em http://{local}");
    if addr.starts_with("0.0.0.0") {
        println!("Nas outras máquinas da rede, use o IP deste computador no lugar de 127.0.0.1.");
    }
    println!("Ctrl+C para parar. Cada pedido e cada mudança aparecem aqui embaixo (MB_LOG=off desliga).");
    config::open_browser(&format!("http://{local}"));
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await
        .expect("servidor caiu");
}

fn header_layer(name: header::HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(value))
}
