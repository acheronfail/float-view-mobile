-- This schema is created only for new disposable session databases. Incremental
-- auto-vacuum must be selected before the first tables are created.
PRAGMA auto_vacuum = INCREMENTAL;
VACUUM;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS session_metadata (
  singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
  expected_riding_persistence_ms INTEGER NOT NULL CHECK (expected_riding_persistence_ms > 0),
  expected_idle_persistence_ms INTEGER NOT NULL CHECK (expected_idle_persistence_ms > 0),
  pruned_at_ms INTEGER,
  retained_resolution_seconds INTEGER CHECK (retained_resolution_seconds BETWEEN 1 AND 300),
  prune_policy TEXT
);

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
  bms_circuitry_temp_c REAL,
  bms_highest_cell_temp_c REAL,
  footpad_state TEXT,
  motor_current_a REAL,
  battery_current_a REAL,
  erpm REAL,
  speed_kph REAL,
  pitch_deg REAL,
  balance_pitch_deg REAL,
  roll_deg REAL,
  directional_motor_current_a REAL,
  filtered_motor_current_a REAL,
  requested_balance_current_a REAL,
  booster_current_a REAL,
  field_weakening_current_a REAL,
  footpad_adc_1_v REAL,
  footpad_adc_2_v REAL,
  remote_input REAL,
  setpoint_deg REAL,
  atr_setpoint_deg REAL,
  brake_tilt_setpoint_deg REAL,
  torque_tilt_setpoint_deg REAL,
  turn_tilt_setpoint_deg REAL,
  remote_setpoint_deg REAL,
  atr_acceleration_difference_erpm_per_second REAL,
  atr_speed_boost REAL,
  distance_m REAL,
  odometer_m REAL,
  amp_hours_drawn_ah REAL,
  amp_hours_charged_ah REAL,
  watt_hours_drawn_wh REAL,
  watt_hours_charged_wh REAL,
  battery_level REAL,
  charging_current_a REAL,
  charging_voltage_v REAL,
  extension_json TEXT NOT NULL DEFAULT '{}' CHECK (length(CAST(extension_json AS BLOB)) <= 16384)
);
CREATE INDEX IF NOT EXISTS sample_elapsed ON sample(monotonic_elapsed_ms, sequence);

CREATE TABLE IF NOT EXISTS location (
  location_sequence INTEGER PRIMARY KEY NOT NULL,
  observed_at_ms INTEGER NOT NULL,
  monotonic_elapsed_ms INTEGER NOT NULL CHECK (monotonic_elapsed_ms >= 0),
  latitude REAL NOT NULL CHECK (latitude BETWEEN -90.0 AND 90.0),
  longitude REAL NOT NULL CHECK (longitude BETWEEN -180.0 AND 180.0),
  accuracy_m REAL CHECK (accuracy_m IS NULL OR accuracy_m >= 0.0),
  UNIQUE(monotonic_elapsed_ms, latitude, longitude)
);
CREATE INDEX IF NOT EXISTS location_elapsed ON location(monotonic_elapsed_ms, location_sequence);

CREATE TABLE IF NOT EXISTS session_numeric_field (
  field_name TEXT PRIMARY KEY NOT NULL CHECK (length(CAST(field_name AS BLOB)) BETWEEN 1 AND 64),
  source TEXT NOT NULL CHECK (source IN ('stable', 'extension')),
  first_sequence INTEGER NOT NULL REFERENCES sample(sequence)
);

CREATE TABLE IF NOT EXISTS session_event (
  sequence INTEGER PRIMARY KEY NOT NULL,
  source_sample_sequence INTEGER REFERENCES sample(sequence),
  observed_at_ms INTEGER NOT NULL,
  monotonic_elapsed_ms INTEGER NOT NULL,
  kind TEXT NOT NULL,
  details_json TEXT NOT NULL
);
-- Sample deletion must seek referencing events rather than scan the event
-- table for each removed sample during pruning.
CREATE INDEX IF NOT EXISTS session_event_source_sample ON session_event(source_sample_sequence);
CREATE INDEX IF NOT EXISTS session_event_elapsed ON session_event(monotonic_elapsed_ms, sequence);

CREATE INDEX IF NOT EXISTS session_ride_markers ON session_event(monotonic_elapsed_ms, sequence) WHERE kind = 'session_marker';

CREATE TABLE IF NOT EXISTS riding_interval (
  interval_id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_elapsed_ms INTEGER NOT NULL,
  ended_elapsed_ms INTEGER,
  source TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commit_marker (
  singleton INTEGER PRIMARY KEY NOT NULL CHECK (singleton = 1),
  committed_sample_sequence INTEGER NOT NULL DEFAULT 0,
  committed_event_sequence INTEGER NOT NULL DEFAULT 0,
  recorded_sample_count INTEGER NOT NULL DEFAULT 0,
  stored_sample_count INTEGER NOT NULL DEFAULT 0,
  committed_event_count INTEGER NOT NULL DEFAULT 0,
  committed_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  history_revision INTEGER NOT NULL DEFAULT 0,
  committed_at_ms INTEGER NOT NULL,
  riding_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  distance_m REAL,
  maximum_speed_kph REAL,
  maximum_speed_source_sequence INTEGER,
  maximum_duty_cycle REAL,
  maximum_duty_source_sequence INTEGER,
  last_speed_kph REAL,
  last_ride_state TEXT,
  committed_location_sequence INTEGER NOT NULL DEFAULT 0,
  committed_location_elapsed_ms INTEGER NOT NULL DEFAULT 0,
  recorded_location_count INTEGER NOT NULL DEFAULT 0,
  stored_location_count INTEGER NOT NULL DEFAULT 0,
  last_location_status TEXT NOT NULL DEFAULT 'disabled'
);

-- Payload-free timing diagnostics; the asynchronous writer retains the newest 5000 rows.
CREATE TABLE IF NOT EXISTS session_diagnostic (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  record_json TEXT NOT NULL CHECK (length(CAST(record_json AS BLOB)) <= 1024)
);
