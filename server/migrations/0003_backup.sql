-- Backup automático (uma linha só). minute = minuto do dia em UTC; 360 = 06:00 UTC = 03:00 em Brasília.
-- key_saved_at: quando quem administra confirmou que guardou a chave de recuperação.
CREATE TABLE masterbase.backup_settings (
    id           BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    enabled      BOOLEAN NOT NULL DEFAULT true,
    minute       INTEGER NOT NULL DEFAULT 360 CHECK (minute BETWEEN 0 AND 1439),
    key_saved_at TIMESTAMPTZ
);
INSERT INTO masterbase.backup_settings DEFAULT VALUES;
