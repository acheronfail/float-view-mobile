PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS sample (
  sequence INTEGER PRIMARY KEY NOT NULL,
  observed_at_ms INTEGER NOT NULL,
  slow_observed_at_ms INTEGER,
  monotonic_elapsed_ms INTEGER NOT NULL,
  ride_state TEXT NOT NULL,
  package_state TEXT,
  package_mode TEXT,
  stop_condition TEXT,
  setpoint_adjustment TEXT,
  wheelslip INTEGER,
  darkride INTEGER,
  charging INTEGER,
  alert_reason TEXT,
  firmware_fault TEXT,
  raw_pack_voltage_v REAL,
  duty_cycle REAL,
  controller_temp_c REAL,
  motor_temp_c REAL,
  footpad_state TEXT,
  detail_json TEXT NOT NULL DEFAULT '{}',
  extension_json TEXT NOT NULL DEFAULT '{}',
  motor_current_a REAL,
  battery_current_a REAL,
  erpm REAL,
  speed_kph REAL
);
CREATE INDEX IF NOT EXISTS sample_elapsed ON sample(monotonic_elapsed_ms);

CREATE TABLE IF NOT EXISTS session_event (
  sequence INTEGER PRIMARY KEY NOT NULL,
  observed_at_ms INTEGER NOT NULL,
  monotonic_elapsed_ms INTEGER NOT NULL,
  kind TEXT NOT NULL,
  details_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS session_event_elapsed ON session_event(monotonic_elapsed_ms);

CREATE TABLE IF NOT EXISTS riding_interval (
  interval_id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_elapsed_ms INTEGER NOT NULL,
  ended_elapsed_ms INTEGER,
  source TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commit_marker (
  singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
  committed_sequence INTEGER NOT NULL,
  committed_at_ms INTEGER NOT NULL
);
