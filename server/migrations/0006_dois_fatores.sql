-- Segundo fator: código de 6 dígitos de um app autenticador (TOTP), com códigos de recuperação.
ALTER TABLE masterbase.users
    ADD COLUMN totp_secret BYTEA,
    ADD COLUMN totp_last   BIGINT NOT NULL DEFAULT 0; -- último passo de 30 s aceito: o mesmo código não vale duas vezes

-- Segredo gerado e ainda não confirmado (a pessoa está escaneando o QR).
CREATE TABLE masterbase.totp_pending (
    user_id    BIGINT PRIMARY KEY REFERENCES masterbase.users(id) ON DELETE CASCADE,
    secret     BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE masterbase.recovery_codes (
    user_id   BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    code_hash BYTEA NOT NULL,
    PRIMARY KEY (user_id, code_hash)
);

-- Senha (ou Google, GitHub, link) já conferida, falta o código. Vale 5 minutos.
CREATE TABLE masterbase.login_tickets (
    ticket_hash BYTEA PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES masterbase.users(id) ON DELETE CASCADE,
    via         TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL
);
