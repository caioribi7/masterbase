mod access;
mod activity;
mod auth;
mod backup;
mod catalog;
mod data;
mod error;
mod query;
mod schema;

use axum::http::{header, HeaderValue};
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

#[tokio::main]
async fn main() {
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

    let web = std::env::var("MB_WEB_DIR").unwrap_or_else(|_| "../web/dist".into());
    let api = Router::new()
        .route("/setup", get(auth::setup))
        .route("/signup", post(auth::signup))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/me", get(auth::me))
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
        .route("/password", post(access::change_password))
        .route("/keys", get(access::list_keys).post(access::create_key))
        .route("/keys/{id}", delete(access::delete_key))
        .route("/backups", get(backup::list).post(backup::create))
        .route("/backups/{name}", get(backup::download).delete(backup::delete))
        .route("/backups/{name}/restore", post(backup::restore))
        .route("/schema", get(schema::get_schema).post(schema::create_table))
        .route("/schema/{table}", patch(schema::rename_table).delete(schema::drop_table))
        .route("/schema/{table}/columns", post(schema::add_column))
        .route("/schema/{table}/columns/{column}", patch(schema::change_column).delete(schema::drop_column));

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(&web).fallback(ServeFile::new(format!("{web}/index.html"))))
        .layer(header_layer(header::CONTENT_SECURITY_POLICY, "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'"))
        .layer(header_layer(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(header_layer(header::REFERRER_POLICY, "no-referrer"))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(db);

    let addr = std::env::var("MB_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("porta ocupada");
    println!("Masterbase em http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await
        .expect("servidor caiu");
}

fn header_layer(name: header::HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(value))
}
