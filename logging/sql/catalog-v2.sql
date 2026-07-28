ALTER TABLE session_catalog ADD COLUMN riding_elapsed_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_catalog ADD COLUMN total_elapsed_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_catalog ADD COLUMN distance_m REAL;
ALTER TABLE session_catalog ADD COLUMN max_speed_mps REAL;
ALTER TABLE session_catalog ADD COLUMN max_duty_cycle REAL;
ALTER TABLE session_catalog ADD COLUMN last_sample_elapsed_ms INTEGER;
ALTER TABLE session_catalog ADD COLUMN last_speed_mps REAL;

CREATE INDEX IF NOT EXISTS session_catalog_started
  ON session_catalog(started_at_ms DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS session_catalog_total_elapsed
  ON session_catalog(total_elapsed_ms DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS session_catalog_distance
  ON session_catalog(distance_m DESC, session_id DESC);

INSERT OR REPLACE INTO schema_metadata(key, value) VALUES ('catalog_version', '2');
