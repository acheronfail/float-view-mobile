PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS saved_device (
  saved_device_id TEXT PRIMARY KEY NOT NULL,
  device_history_id TEXT UNIQUE NOT NULL,
  opaque_platform_id TEXT UNIQUE NOT NULL,
  advertised_name TEXT,
  custom_display_name TEXT,
  revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
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
  close_reason TEXT,
  riding_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  total_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  distance_m REAL,
  max_speed_mps REAL,
  max_duty_cycle REAL,
  last_sample_elapsed_ms INTEGER,
  last_speed_mps REAL,
  pending_delete INTEGER NOT NULL DEFAULT 0 CHECK (pending_delete IN (0, 1))
);
CREATE INDEX IF NOT EXISTS session_catalog_device_started
  ON session_catalog(device_history_id, started_at_ms DESC);
CREATE INDEX IF NOT EXISTS session_catalog_started
  ON session_catalog(started_at_ms DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS session_catalog_total_elapsed
  ON session_catalog(total_elapsed_ms DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS session_catalog_distance
  ON session_catalog(distance_m DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS session_catalog_pending_delete
  ON session_catalog(pending_delete, started_at_ms DESC);

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
