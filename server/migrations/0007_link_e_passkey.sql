-- Entrar com um link por e-mail (vale 15 minutos, uma vez).
CREATE TABLE masterbase.magic_tokens (
    token_hash BYTEA PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

-- Passkeys (WebAuthn). Só ES256: a chave pública é o ponto P-256 sem compressão (65 bytes).
CREATE TABLE masterbase.passkeys (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    cred_id      BYTEA NOT NULL UNIQUE,
    public_key   BYTEA NOT NULL,
    sign_count   BIGINT NOT NULL DEFAULT 0,
    name         TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 60),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

-- Desafio de cada cerimônia, de uso único. user_id vazio = login (ainda não se sabe quem é).
CREATE TABLE masterbase.webauthn_challenges (
    challenge  BYTEA PRIMARY KEY,
    user_id    BIGINT REFERENCES masterbase.users(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('registro', 'login')),
    expires_at TIMESTAMPTZ NOT NULL
);
