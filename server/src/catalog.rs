//! Leitura do catálogo do Postgres. Todo nome de tabela/coluna passa por aqui antes de entrar num SQL.
use crate::error::{bad, ApiError, ApiResult};
use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;

/// Aspas de identificador do Postgres. Nomes de coisas existentes só chegam aqui depois de conferidos no catálogo.
pub fn ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Nome novo (tabela/coluna) digitado pelo usuário. Vai sempre entre aspas, então só barramos o que confunde.
pub fn valid_name(s: &str) -> ApiResult<String> {
    let s = s.trim();
    if s.is_empty() || s.len() > 63 || s.chars().any(char::is_control) || s.starts_with("pg_") {
        return Err(bad("Nome inválido. Use até 63 caracteres, sem começar com pg_."));
    }
    Ok(s.to_owned())
}

/// Tipo do Postgres em palavras para quem não sabe SQL.
pub fn label(ty: &str, typtype: &str) -> &'static str {
    match ty {
        _ if typtype == "e" => "opção",
        "email" => "e-mail",
        "boolean" => "sim/não",
        "uuid" => "id único",
        "json" | "jsonb" => "dados",
        "date" => "data",
        t if t.starts_with("timestamp") => "data e hora",
        "smallint" | "integer" | "bigint" => "número",
        "real" | "double precision" => "decimal",
        t if t.starts_with("numeric") => "decimal",
        t if t.starts_with("text") || t.starts_with("character") => "texto",
        _ => "outro",
    }
}

/// Tipos que a tela Estrutura oferece. Ligação é tratada à parte (copia o tipo da chave da outra tabela).
pub fn kind_sql(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "texto" => "text",
        "número" => "bigint",
        "decimal" => "numeric",
        "sim/não" => "boolean",
        "data" => "date",
        "data e hora" => "timestamptz",
        "dados" => "jsonb",
        "id único" => "uuid",
        _ => return None,
    })
}

pub struct Col {
    pub table: String,
    pub name: String,
    pub ty: String,
    pub typtype: String,
    pub pk: bool,
    pub required: bool,
    pub unique: bool,
    pub link: Option<String>,
    /// Expressão do valor padrão, como o Postgres guarda (pg_get_expr). "auto" para coluna identity.
    pub default: Option<String>,
}

impl Col {
    pub fn json(&self) -> Value {
        json!({
            "name": self.name, "type": self.ty, "label": label(&self.ty, &self.typtype),
            "pk": self.pk, "required": self.required, "unique": self.unique, "link": self.link,
            "default": self.default.as_deref().map(default_json),
        })
    }
}

/// Valor padrão em palavras: {mode: agora | hoje | id novo | auto | fixo | outro, value}.
pub fn default_json(expr: &str) -> Value {
    let e = expr.trim();
    let mode = match e.to_lowercase().as_str() {
        "auto" => "auto",
        "now()" | "current_timestamp" | "clock_timestamp()" | "statement_timestamp()" => "agora",
        "current_date" => "hoje",
        "gen_random_uuid()" | "uuid_generate_v4()" => "id novo",
        _ => "",
    };
    if !mode.is_empty() {
        return json!({ "mode": mode });
    }
    if let Some(v) = literal(e) {
        return json!({ "mode": "fixo", "value": v });
    }
    json!({ "mode": "outro", "value": e })
}

/// 'texto'::tipo, número ou true/false. Outra coisa (função, expressão) devolve None.
fn literal(e: &str) -> Option<String> {
    if let Some(rest) = e.strip_prefix('\'') {
        let end = rest.rfind('\'')?;
        let (inner, tail) = (&rest[..end], &rest[end + 1..]);
        if !(tail.is_empty() || tail.starts_with("::")) || inner.replace("''", "").contains('\'') {
            return None;
        }
        return Some(inner.replace("''", "'"));
    }
    let bare = e.split("::").next()?.trim().trim_start_matches('(').trim_end_matches(')').trim();
    if bare == "true" || bare == "false" || bare.parse::<f64>().is_ok() {
        return Some(bare.to_owned());
    }
    None
}

type ColRow = (String, String, String, String, bool, bool, bool, Option<String>, Option<String>);

/// Colunas de uma tabela do schema public (ou de todas, com `None`). Tabela inexistente = 404.
pub async fn columns(db: &PgPool, table: Option<&str>) -> ApiResult<Vec<Col>> {
    let rows: Vec<ColRow> = sqlx::query_as(
        "SELECT c.relname::text, a.attname::text, format_type(a.atttypid, a.atttypmod), t.typtype::text,
                coalesce(a.attnum = ANY(p.conkey), false),
                a.attnotnull,
                EXISTS (SELECT 1 FROM pg_constraint u WHERE u.conrelid = c.oid AND u.contype = 'u' AND u.conkey = ARRAY[a.attnum]),
                (SELECT r.relname::text FROM pg_constraint f JOIN pg_class r ON r.oid = f.confrelid
                  WHERE f.conrelid = c.oid AND f.contype = 'f' AND f.conkey = ARRAY[a.attnum] LIMIT 1),
                CASE WHEN a.attidentity <> '' THEN 'auto' ELSE pg_get_expr(d.adbin, d.adrelid) END
         FROM pg_attribute a
         JOIN pg_class c ON c.oid = a.attrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         JOIN pg_type t ON t.oid = a.atttypid
         LEFT JOIN pg_constraint p ON p.conrelid = c.oid AND p.contype = 'p'
         LEFT JOIN pg_attrdef d ON d.adrelid = c.oid AND d.adnum = a.attnum
         WHERE n.nspname = 'public' AND c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped
           AND ($1::text IS NULL OR c.relname = $1)
         ORDER BY c.relname, a.attnum",
    )
    .bind(table)
    .fetch_all(db)
    .await?;
    if table.is_some() && rows.is_empty() {
        return Err(ApiError(StatusCode::NOT_FOUND, "Tabela não encontrada."));
    }
    Ok(rows
        .into_iter()
        .map(|(table, name, ty, typtype, pk, required, unique, link, default)| Col { table, name, ty, typtype, pk, required, unique, link, default })
        .collect())
}

/// Nomes e contagem de linhas das tabelas de public.
pub async fn table_counts(db: &PgPool) -> ApiResult<Vec<(String, i64)>> {
    let names: Vec<String> =
        sqlx::query_scalar("SELECT tablename::text FROM pg_tables WHERE schemaname = 'public' ORDER BY 1")
            .fetch_all(db)
            .await?;
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        // shortcut: count(*) por tabela fica lento acima de ~1M linhas; trocar por pg_class.reltuples aí.
        let rows: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM public.{}", ident(&name)))
            .fetch_one(db)
            .await?;
        out.push((name, rows));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_escapes_quotes() {
        assert_eq!(ident("clientes"), "\"clientes\"");
        assert_eq!(ident("a\"; DROP TABLE x; --"), "\"a\"\"; DROP TABLE x; --\"");
    }

    #[test]
    fn labels_are_human() {
        assert_eq!(label("integer", "b"), "número");
        assert_eq!(label("numeric(10,2)", "b"), "decimal");
        assert_eq!(label("character varying(120)", "b"), "texto");
        assert_eq!(label("plano", "e"), "opção");
        assert_eq!(label("email", "d"), "e-mail");
        assert_eq!(label("timestamp with time zone", "b"), "data e hora");
    }

    #[test]
    fn every_kind_round_trips_to_its_label() {
        for k in ["texto", "número", "decimal", "sim/não", "data", "data e hora", "dados", "id único"] {
            let sql = kind_sql(k).unwrap();
            let pg = if sql == "timestamptz" { "timestamp with time zone" } else { sql };
            assert_eq!(label(pg, "b"), k);
        }
        assert!(kind_sql("text; DROP TABLE x").is_none());
    }

    #[test]
    fn defaults_in_words() {
        assert_eq!(default_json("now()")["mode"], "agora");
        assert_eq!(default_json("CURRENT_DATE")["mode"], "hoje");
        assert_eq!(default_json("gen_random_uuid()")["mode"], "id novo");
        assert_eq!(default_json("'ativo'::text"), json!({ "mode": "fixo", "value": "ativo" }));
        assert_eq!(default_json("'d''água'::text")["value"], "d'água");
        assert_eq!(default_json("0"), json!({ "mode": "fixo", "value": "0" }));
        assert_eq!(default_json("(-1.5)::numeric")["value"], "-1.5");
        assert_eq!(default_json("true")["value"], "true");
        assert_eq!(default_json("lower('A'::text)")["mode"], "outro");
        assert_eq!(default_json("'a'::text || 'b'::text")["mode"], "outro");
    }

    #[test]
    fn names_are_checked() {
        assert_eq!(valid_name("  pedidos ").ok().as_deref(), Some("pedidos"));
        assert!(valid_name("").is_err());
        assert!(valid_name("pg_shadow").is_err());
        assert!(valid_name("a\nb").is_err());
        assert!(valid_name(&"x".repeat(64)).is_err());
    }
}
