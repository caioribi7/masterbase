-- Metadados do Masterbase ficam no schema próprio (criado em main.rs); dados do usuário ficam em public.

CREATE TABLE masterbase.users (
    id            BIGSERIAL PRIMARY KEY,
    name          TEXT NOT NULL,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Guarda só o hash SHA-256 do token: vazamento do banco não vira sessão válida.
CREATE TABLE masterbase.sessions (
    token_hash BYTEA PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

-- Auditoria (página Atividade). Só INSERT pela aplicação.
CREATE TABLE masterbase.activity (
    id      BIGSERIAL PRIMARY KEY,
    user_id BIGINT REFERENCES masterbase.users(id),
    action  TEXT NOT NULL,
    detail  JSONB NOT NULL DEFAULT '{}',
    at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
