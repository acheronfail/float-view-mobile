ALTER TABLE session_catalog
  ADD COLUMN pending_delete INTEGER NOT NULL DEFAULT 0 CHECK (pending_delete IN (0, 1));

CREATE INDEX IF NOT EXISTS session_catalog_pending_delete
  ON session_catalog(pending_delete, started_at_ms DESC);

INSERT OR REPLACE INTO schema_metadata(key, value) VALUES ('catalog_version', '3');
