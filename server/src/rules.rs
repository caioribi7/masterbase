//! Regra por linha: "vendedor só vê os próprios pedidos". Uma coluna da tabela guarda o e-mail do dono da linha;
//! editor, leitor e chave de API só alcançam as linhas com o próprio e-mail. Administrador vê tudo.
use crate::activity::log;
use crate::auth::{Role, User};
use crate::catalog::{columns, ident};
use crate::error::{bad, ApiResult};
use crate::projects::Db;
use axum::extract::Path;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Dono da linha para este usuário nesta tabela: (coluna, e-mail). None = vê tudo.
pub struct Owner {
    pub column: String,
    pub email: String,
}

impl Owner {
    /// Condição SQL para a tabela com apelido `t`, com o e-mail no parâmetro $n.
    pub fn cond(&self, n: usize) -> String {
        format!("lower(t.{}::text) = lower(${n})", ident(&self.column))
    }
}

pub async fn owner(db: &Db, user: &User, table: &str) -> ApiResult<Option<Owner>> {
    if user.role == Role::Admin {
        return Ok(None);
    }
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT r.column_name, u.email FROM masterbase.row_rules r, masterbase.users u WHERE r.project = $3 AND r.table_name = $1 AND u.id = $2",
    )
    .bind(table)
    .bind(user.id)
    .bind(db.project)
    .fetch_optional(&db.meta)
    .await?;
    Ok(row.map(|(column, email)| Owner { column, email }))
}

/// Todas as regras que valem para este usuário (tabela → coluna) e o e-mail dele. None = vê tudo.
pub async fn all_for(db: &Db, user: &User) -> ApiResult<Option<(String, HashMap<String, String>)>> {
    if user.role == Role::Admin {
        return Ok(None);
    }
    let email: String = sqlx::query_scalar("SELECT email FROM masterbase.users WHERE id = $1").bind(user.id).fetch_one(&db.meta).await?;
    Ok(Some((email, list(db).await?)))
}

/// Regras de todas as tabelas, para a tela Estrutura.
pub async fn list(db: &Db) -> ApiResult<HashMap<String, String>> {
    let rules: Vec<(String, String)> = sqlx::query_as("SELECT table_name, column_name FROM masterbase.row_rules WHERE project = $1")
        .bind(db.project)
        .fetch_all(&db.meta)
        .await?;
    Ok(rules.into_iter().collect())
}

#[derive(Deserialize)]
pub struct SetRule {
    /// Coluna com o e-mail do dono. None = todo mundo vê todas as linhas.
    column: Option<String>,
}

pub async fn set(user: User, db: Db, Path(table): Path<String>, Json(b): Json<SetRule>) -> ApiResult<Json<Value>> {
    user.admin()?;
    let cols = columns(&db.data, Some(&table)).await?;
    match &b.column {
        Some(name) => {
            let col = cols.iter().find(|c| &c.name == name).ok_or(bad("Coluna não existe."))?;
            let text = col.ty == "text" || col.ty == "citext" || col.ty.starts_with("character varying");
            if col.pk || col.link.is_some() || !text {
                return Err(bad("Escolha uma coluna de texto com o e-mail de quem é dono da linha."));
            }
            sqlx::query(
                "INSERT INTO masterbase.row_rules (table_name, column_name, project) VALUES ($1, $2, $3)
                 ON CONFLICT (project, table_name) DO UPDATE SET column_name = EXCLUDED.column_name, created_at = now()",
            )
            .bind(&table)
            .bind(&col.name)
            .bind(db.project)
            .execute(&db.meta)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM masterbase.row_rules WHERE project = $2 AND table_name = $1").bind(&table).bind(db.project).execute(&db.meta).await?;
        }
    }
    log(&db.meta, user.id, "row_rule", json!({ "table": table, "column": b.column, "project": db.project })).await?;
    Ok(Json(json!({ "owner": b.column })))
}
