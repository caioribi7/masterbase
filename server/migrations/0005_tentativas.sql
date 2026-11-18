-- Tentativas erradas (login, "esqueci a senha", chave dos backups). No banco: reiniciar não zera o bloqueio.
CREATE TABLE masterbase.attempts (
    key        TEXT PRIMARY KEY,
    fails      INT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
