-- Regra por linha: em table_name, editor e leitor só veem e mexem nas linhas em que column_name tem o e-mail deles.
-- Administrador vê tudo. project: 0 = banco principal (veja 0009). Nomes acompanham renomear tabela e coluna (schema.rs).
CREATE TABLE masterbase.row_rules (
    project     BIGINT NOT NULL DEFAULT 0,
    table_name  TEXT NOT NULL,
    column_name TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project, table_name)
);
