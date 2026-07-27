PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS schema_metadata (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);
INSERT OR REPLACE INTO schema_metadata(key, value) VALUES ('catalog_version', '1');

CREATE TABLE IF NOT EXISTS saved_device (
  saved_device_id TEXT PRIMARY KEY NOT NULL,
  device_history_id TEXT UNIQUE NOT NULL,
  opaque_platform_id TEXT UNIQUE NOT NULL,
  advertised_name TEXT,
  display_name TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  last_seen_at_ms INTEGER,
  last_connected_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS board_profile (
  saved_device_id TEXT PRIMARY KEY NOT NULL REFERENCES saved_device(saved_device_id),
  revision INTEGER NOT NULL CHECK (revision >= 0),
  json TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS session_catalog (
  session_id TEXT PRIMARY KEY NOT NULL,
  job_id TEXT UNIQUE NOT NULL,
  device_history_id TEXT NOT NULL,
  device_snapshot_json TEXT NOT NULL,
  profile_snapshot_json TEXT NOT NULL,
  profile_revision INTEGER NOT NULL,
  file_name TEXT UNIQUE NOT NULL,
  started_at_ms INTEGER NOT NULL,
  ended_at_ms INTEGER,
  state TEXT NOT NULL,
  integrity TEXT NOT NULL,
  committed_sequence INTEGER NOT NULL DEFAULT 0,
  sample_count INTEGER NOT NULL DEFAULT 0,
  event_count INTEGER NOT NULL DEFAULT 0,
  requested_idle_ms INTEGER NOT NULL,
  effective_idle_ms INTEGER NOT NULL,
  requested_riding_ms INTEGER NOT NULL,
  effective_riding_ms INTEGER NOT NULL,
  close_reason TEXT
);
CREATE INDEX IF NOT EXISTS session_catalog_device_started
  ON session_catalog(device_history_id, started_at_ms DESC);

CREATE TABLE IF NOT EXISTS session_group (
  session_group_id TEXT PRIMARY KEY NOT NULL,
  device_history_id TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS session_group_member (
  session_group_id TEXT NOT NULL REFERENCES session_group(session_group_id) ON DELETE CASCADE,
  session_id TEXT UNIQUE NOT NULL REFERENCES session_catalog(session_id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL,
  PRIMARY KEY(session_group_id, session_id)
);

CREATE TABLE IF NOT EXISTS settings (
  singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
  revision INTEGER NOT NULL,
  json TEXT NOT NULL
);
