ALTER TABLE sample ADD COLUMN motor_current_a REAL;
ALTER TABLE sample ADD COLUMN battery_current_a REAL;
ALTER TABLE sample ADD COLUMN erpm REAL;
ALTER TABLE sample ADD COLUMN speed_kph REAL;

INSERT OR REPLACE INTO schema_metadata(key, value) VALUES ('session_version', '2');
