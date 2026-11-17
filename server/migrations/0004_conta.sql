-- Sessões com cara: id para encerrar uma, quando começou, quando foi vista e de qual navegador.
ALTER TABLE masterbase.sessions
    ADD COLUMN id           BIGSERIAL UNIQUE,
    ADD COLUMN created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN user_agent   TEXT NOT NULL DEFAULT '';

-- Link de uso único para criar ou trocar a senha. Só o hash fica no banco.
CREATE TABLE masterbase.password_tokens (
    token_hash BYTEA PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

-- Configurações da instância (uma linha só).
CREATE TABLE masterbase.settings (
    id           BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    name         TEXT NOT NULL DEFAULT '' CHECK (length(name) <= 60),
    session_days INT NOT NULL DEFAULT 7 CHECK (session_days BETWEEN 1 AND 90)
);
INSERT INTO masterbase.settings DEFAULT VALUES;

-- Quantos backups automáticos guardar.
ALTER TABLE masterbase.backup_settings
    ADD COLUMN keep_daily  INT NOT NULL DEFAULT 7 CHECK (keep_daily BETWEEN 1 AND 60),
    ADD COLUMN keep_weekly INT NOT NULL DEFAULT 4 CHECK (keep_weekly BETWEEN 0 AND 52);
