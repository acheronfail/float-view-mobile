PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS saved_device (
  saved_device_id TEXT PRIMARY KEY NOT NULL,
  device_history_id TEXT UNIQUE NOT NULL,
  opaque_platform_id TEXT UNIQUE,
  advertised_name TEXT,
  custom_display_name TEXT,
  revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
  created_at_ms INTEGER NOT NULL,
  last_seen_at_ms INTEGER,
  last_connected_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS board_profile (
  saved_device_id TEXT PRIMARY KEY NOT NULL
    REFERENCES saved_device(saved_device_id) ON DELETE CASCADE,
  revision INTEGER NOT NULL CHECK (revision >= 0),
  json TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

-- Private connection hint. controller_uuid is a 12-byte firmware UUID encoded
-- as 24 lowercase hex characters and is never exposed through the API.
CREATE TABLE IF NOT EXISTS controller_route_hint (
  saved_device_id TEXT PRIMARY KEY NOT NULL
    REFERENCES saved_device(saved_device_id) ON DELETE CASCADE,
  can_id INTEGER NOT NULL CHECK (can_id BETWEEN 0 AND 253),
  controller_uuid TEXT CHECK (
    controller_uuid IS NULL OR length(controller_uuid) = 24
  ),
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
  committed_event_sequence INTEGER NOT NULL DEFAULT 0,
  sample_count INTEGER NOT NULL DEFAULT 0,
  stored_sample_count INTEGER NOT NULL DEFAULT 0,
  event_count INTEGER NOT NULL DEFAULT 0,
  committed_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  history_revision INTEGER NOT NULL DEFAULT 0,
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
  max_speed_source_sequence INTEGER,
  max_duty_source_sequence INTEGER,
  last_sample_elapsed_ms INTEGER,
  last_speed_mps REAL,
  committed_location_sequence INTEGER NOT NULL DEFAULT 0,
  committed_location_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  location_count INTEGER NOT NULL DEFAULT 0,
  recorded_location_count INTEGER NOT NULL DEFAULT 0,
  stored_location_count INTEGER NOT NULL DEFAULT 0,
  location_status TEXT NOT NULL DEFAULT 'disabled',
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

CREATE TABLE IF NOT EXISTS settings (
  singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
  revision INTEGER NOT NULL,
  json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session_maintenance_job (
  singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
  operation_id TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('prune_preview', 'prune')),
  scope TEXT NOT NULL CHECK (scope IN ('one', 'all')),
  session_id TEXT,
  state TEXT NOT NULL CHECK (state IN ('pending', 'running', 'cancel_requested', 'completed', 'failed', 'cancelled')),
  target_resolution_seconds INTEGER NOT NULL CHECK (target_resolution_seconds BETWEEN 1 AND 300),
  upper_session_id TEXT,
  last_session_id TEXT,
  completed_sessions INTEGER NOT NULL DEFAULT 0,
  skipped_sessions INTEGER NOT NULL DEFAULT 0,
  retained_samples INTEGER NOT NULL DEFAULT 0,
  reclaimable_bytes INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  updated_at_ms INTEGER NOT NULL
);
