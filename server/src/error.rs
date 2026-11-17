use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub struct ApiError(pub StatusCode, pub &'static str);

pub type ApiResult<T> = Result<T, ApiError>;

pub const UNAUTHORIZED: ApiError = ApiError(StatusCode::UNAUTHORIZED, "Entre na sua conta.");

pub fn bad(msg: &'static str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg)
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

/// Erro do Postgres vira frase que quem não sabe SQL entende. O que não é culpa do usuário vira 500 e vai pro log.
impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        let code = e.as_database_error().and_then(|d| d.code()).map(|c| c.into_owned());
        let msg = match code.as_deref() {
            Some("42P07") => "Já existe uma tabela com esse nome.",
            Some("42701") => "Já existe uma coluna com esse nome.",
            Some("2BP01") => "Outras tabelas dependem disso. Remova as ligações antes.",
            Some("23503") => "Ligação inválida: o registro ligado não existe ou ainda está em uso.",
            Some("23505") => "Esse valor já existe e a coluna não aceita repetidos.",
            Some("23502") => "Um campo obrigatório ficou vazio.",
            Some("57014") => "A operação demorou demais e foi cancelada.",
            Some(c) if c.starts_with("22") || c.starts_with("23") => "Valor inválido para essa coluna.",
            _ => {
                eprintln!("db: {e}");
                return ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Erro no banco de dados.");
            }
        };
        bad(msg)
    }
}
