//! Página Consultas: o construtor visual manda uma descrição (Spec) e o servidor monta o SELECT.
//! Não existe SQL livre: nomes vêm do catálogo, operadores de uma lista fixa, valores como parâmetro.
use crate::auth::User;
use crate::projects::Db;
use crate::catalog::{columns, ident, Col};
use crate::error::{bad, ApiError, ApiResult};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize)]
pub struct Filter {
    col: String,
    op: String,
    val: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Spec {
    table: String,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    filters: Vec<Filter>,
    sort: Option<String>,
    #[serde(default)]
    desc: bool,
    limit: Option<i64>,
}

fn find<'a>(cols: &'a [Col], name: &str) -> ApiResult<&'a Col> {
    cols.iter().find(|c| c.name == name).ok_or(bad("Coluna não existe."))
}

/// Monta o SELECT. Devolve o SQL com $1, $2... e os valores na ordem.
/// `mine`: regra por linha de quem pede (só as linhas com o e-mail da pessoa).
fn build(cols: &[Col], s: &Spec, mine: Option<&crate::rules::Owner>) -> ApiResult<(String, Vec<String>)> {
    if s.filters.len() > 20 {
        return Err(bad("Máximo de 20 condições."));
    }
    let picked: Vec<&Col> = if s.columns.is_empty() {
        cols.iter().collect()
    } else {
        s.columns.iter().map(|c| find(cols, c)).collect::<ApiResult<_>>()?
    };
    let list: Vec<String> = picked.iter().map(|c| format!("t.{}", ident(&c.name))).collect();
    let mut sql = format!("SELECT {} FROM public.{} t", list.join(", "), ident(&s.table));
    let mut binds = Vec::new();
    let mut conds = Vec::new();
    if let Some(o) = mine {
        binds.push(o.email.clone());
        conds.push(o.cond(1));
    }
    for f in &s.filters {
        let col = find(cols, &f.col)?;
        let c = format!("t.{}", ident(&col.name));
        if f.op == "vazio" || f.op == "não vazio" {
            conds.push(format!("{c} IS {}NULL", if f.op == "vazio" { "" } else { "NOT " }));
            continue;
        }
        binds.push(f.val.clone().filter(|v| !v.is_empty()).ok_or(bad("Falta o valor de uma condição."))?);
        let n = binds.len();
        conds.push(match f.op.as_str() {
            "é" => format!("lower({c}::text) = lower(${n})"),
            "não é" => format!("lower({c}::text) IS DISTINCT FROM lower(${n})"),
            "contém" => format!("position(lower(${n}) in lower({c}::text)) > 0"),
            // col.ty vem de format_type() do catálogo.
            "maior que" => format!("{c} > CAST(${n} AS {})", col.ty),
            "menor que" => format!("{c} < CAST(${n} AS {})", col.ty),
            _ => return Err(bad("Condição desconhecida.")),
        });
    }
    if !conds.is_empty() {
        sql += &format!(" WHERE {}", conds.join(" AND "));
    }
    if let Some(sort) = &s.sort {
        sql += &format!(" ORDER BY t.{} {} NULLS LAST", ident(&find(cols, sort)?.name), if s.desc { "DESC" } else { "ASC" });
    }
    sql += &format!(" LIMIT {}", s.limit.unwrap_or(100).clamp(1, 500));
    Ok((sql, binds))
}

/// SQL para mostrar na tela (valores no lugar dos $n). Só leitura: nunca é executado.
fn readable(sql: &str, binds: &[String]) -> String {
    let mut out = sql.to_owned();
    for (i, b) in binds.iter().enumerate().rev() {
        out = out.replace(&format!("${}", i + 1), &format!("'{}'", b.replace('\'', "''")));
    }
    out
}

pub async fn run(user: User, p: Db, Json(s): Json<Spec>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    let cols = columns(&db, Some(&s.table)).await?;
    let mine = crate::rules::owner(&p, &user, &s.table).await?;
    let (sql, binds) = build(&cols, &s, mine.as_ref())?;
    let wrapped = format!("SELECT row_to_json(q)::jsonb FROM ({sql}) q");
    let mut q = sqlx::query_scalar(&wrapped);
    for b in &binds {
        q = q.bind(b);
    }
    let rows: Vec<Value> = q.fetch_all(&db).await?;
    let shown: Vec<Value> = cols.iter().filter(|c| s.columns.is_empty() || s.columns.contains(&c.name)).map(Col::json).collect();
    Ok(Json(json!({ "columns": shown, "rows": rows, "sql": readable(&sql, &binds) })))
}

pub async fn list_saved(_user: User, p: Db) -> ApiResult<Json<Value>> {
    let rows: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', id, 'name', name, 'spec', spec) FROM masterbase.queries WHERE project = $1 ORDER BY name",
    )
    .bind(p.project)
    .fetch_all(&p.meta)
    .await?;
    Ok(Json(Value::Array(rows)))
}

#[derive(Deserialize)]
pub struct Save {
    name: String,
    spec: Spec,
}

pub async fn save(user: User, p: Db, Json(b): Json<Save>) -> ApiResult<Json<Value>> {
    let db = p.data.clone();
    user.editor()?;
    let name = b.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(bad("Dê um nome para a consulta (até 80 caracteres)."));
    }
    // Só salva o que roda: valida contra o catálogo antes.
    build(&columns(&db, Some(&b.spec.table)).await?, &b.spec, None)?;
    let id: i64 = sqlx::query_scalar("INSERT INTO masterbase.queries (user_id, name, spec, project) VALUES ($1, $2, $3, $4) RETURNING id")
        .bind(user.id)
        .bind(name)
        .bind(sqlx::types::Json(&b.spec))
        .bind(p.project)
        .fetch_one(&p.meta)
        .await?;
    Ok(Json(json!({ "id": id })))
}

pub async fn delete_saved(user: User, p: Db, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    user.editor()?;
    let n = sqlx::query("DELETE FROM masterbase.queries WHERE id = $1 AND project = $2").bind(id).bind(p.project).execute(&p.meta).await?.rows_affected();
    if n == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "Consulta não encontrada."));
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, ty: &str) -> Col {
        Col { table: "t".into(), name: name.into(), ty: ty.into(), typtype: "b".into(), pk: false, required: false, unique: false, link: None, default: None }
    }

    fn spec(filters: Vec<Filter>) -> Spec {
        Spec { table: "t".into(), columns: vec![], filters, sort: Some("total".into()), desc: true, limit: Some(9999) }
    }

    #[test]
    fn builds_parameterized_select() {
        let cols = [col("nome", "text"), col("total", "numeric(10,2)")];
        let f = vec![
            Filter { col: "nome".into(), op: "contém".into(), val: Some("o'x".into()) },
            Filter { col: "total".into(), op: "maior que".into(), val: Some("10".into()) },
            Filter { col: "nome".into(), op: "não vazio".into(), val: None },
        ];
        let (sql, binds) = build(&cols, &spec(f), None).ok().unwrap();
        assert_eq!(
            sql,
            "SELECT t.\"nome\", t.\"total\" FROM public.\"t\" t WHERE position(lower($1) in lower(t.\"nome\"::text)) > 0 \
             AND t.\"total\" > CAST($2 AS numeric(10,2)) AND t.\"nome\" IS NOT NULL ORDER BY t.\"total\" DESC NULLS LAST LIMIT 500"
        );
        assert_eq!(binds, ["o'x", "10"]);
        assert!(readable(&sql, &binds).contains("'o''x'"));
    }

    #[test]
    fn rejects_unknown_column_and_operator() {
        let cols = [col("nome", "text"), col("total", "numeric")];
        assert!(build(&cols, &spec(vec![Filter { col: "x\"; drop".into(), op: "é".into(), val: Some("1".into()) }]), None).is_err());
        assert!(build(&cols, &spec(vec![Filter { col: "nome".into(), op: "OR 1=1".into(), val: Some("1".into()) }]), None).is_err());
    }

    #[test]
    fn owner_rule_comes_first() {
        let cols = [col("nome", "text"), col("vendedor", "text"), col("total", "numeric")];
        let o = crate::rules::Owner { column: "vendedor".into(), email: "a@b.c".into() };
        let f = vec![Filter { col: "nome".into(), op: "é".into(), val: Some("x".into()) }];
        let (sql, binds) = build(&cols, &spec(f), Some(&o)).ok().unwrap();
        assert!(sql.contains("WHERE lower(t.\"vendedor\"::text) = lower($1) AND lower(t.\"nome\"::text) = lower($2)"));
        assert_eq!(binds, ["a@b.c", "x"]);
    }
}
