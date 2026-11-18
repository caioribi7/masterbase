-- Vários projetos: outros bancos Postgres que o painel abre. url tem usuário e senha do banco do projeto
-- (fica só aqui, nunca volta para a tela). O projeto 0 é o banco principal, o deste schema.
CREATE TABLE masterbase.projects (
    id         BIGSERIAL PRIMARY KEY,
    name       TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 60),
    url        TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE masterbase.queries ADD COLUMN project BIGINT NOT NULL DEFAULT 0;
