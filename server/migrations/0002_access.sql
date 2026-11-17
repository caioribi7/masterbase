-- Papéis: admin (tudo), editor (mexe nas linhas), leitor (só vê). Conta desativada não entra, mas fica no histórico.
ALTER TABLE masterbase.users
    ADD COLUMN role   TEXT NOT NULL DEFAULT 'admin' CHECK (role IN ('admin', 'editor', 'leitor')),
    ADD COLUMN active BOOLEAN NOT NULL DEFAULT true;

-- Chave de API: guardamos só o hash. O prefixo ajuda a pessoa a reconhecer a chave na lista.
CREATE TABLE masterbase.api_keys (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    role         TEXT NOT NULL CHECK (role IN ('editor', 'leitor')),
    token_hash   BYTEA NOT NULL UNIQUE,
    prefix       TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

-- Consultas salvas (página Consultas). spec = o que o construtor visual montou.
CREATE TABLE masterbase.queries (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT REFERENCES masterbase.users(id) ON DELETE SET NULL,
    name       TEXT NOT NULL,
    spec       JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
