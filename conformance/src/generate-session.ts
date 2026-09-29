import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, resolve } from 'node:path';

const SAMPLE_COUNT = 150_000;
const PERSISTENCE_MS = 1_000;
const SEGMENT_LENGTH = 20_000;
const GENERATED_SESSION_ID = 'generated-150k-session';
const GENERATED_DEVICE_ID = 'generated-performance-device';
const STARTED_AT_MS = Date.UTC(2026, 0, 1);
const outputArgument = process.argv[2] ?? 'generated/float-view-150000.floatview';
const output = isAbsolute(outputArgument) ? outputArgument : resolve(import.meta.dirname, '..', outputArgument);

const sampleColumns = [
  'sequence',
  'observed_at_ms',
  'slow_observed_at_ms',
  'monotonic_elapsed_ms',
  'ride_state',
  'package_state',
  'package_mode',
  'stop_condition',
  'setpoint_adjustment',
  'wheelslip',
  'darkride',
  'charging',
  'alert_reason',
  'firmware_fault',
  'raw_pack_voltage_v',
  'duty_cycle',
  'controller_temp_c',
  'motor_temp_c',
  'footpad_state',
  'motor_current_a',
  'battery_current_a',
  'erpm',
  'speed_kph',
  'pitch_deg',
  'balance_pitch_deg',
  'roll_deg',
  'directional_motor_current_a',
  'filtered_motor_current_a',
  'requested_balance_current_a',
  'booster_current_a',
  'field_weakening_current_a',
  'footpad_adc_1_v',
  'footpad_adc_2_v',
  'remote_input',
  'setpoint_deg',
  'atr_setpoint_deg',
  'brake_tilt_setpoint_deg',
  'torque_tilt_setpoint_deg',
  'turn_tilt_setpoint_deg',
  'remote_setpoint_deg',
  'atr_acceleration_difference_erpm_per_second',
  'atr_speed_boost',
  'distance_m',
  'odometer_m',
  'amp_hours_drawn_ah',
  'amp_hours_charged_ah',
  'watt_hours_drawn_wh',
  'watt_hours_charged_wh',
  'battery_level',
  'charging_current_a',
  'charging_voltage_v',
  'extension_json',
] as const;

function csv(value: string | number | null): string {
  if (value === null) return '';
  const text = String(value);
  return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function sampleRow(index: number): string {
  const sequence = index + 1;
  const elapsed = index * PERSISTENCE_MS;
  const riding = index % SEGMENT_LENGTH < SEGMENT_LENGTH * 0.9;
  const speed = riding ? 12 + ((index * 17) % 240) / 10 : 0;
  const distance = index * 5.4;
  const values: Array<string | number | null> = [
    sequence,
    STARTED_AT_MS + elapsed,
    null,
    elapsed,
    riding ? 'riding' : 'idle',
    'running',
    'normal',
    'none',
    'none',
    null,
    null,
    0,
    'none',
    null,
    58 - (index % 2_000) / 200,
    riding ? 0.25 + ((index * 13) % 600) / 1_000 : 0,
    30 + (index % 600) / 10,
    35 + (index % 700) / 10,
    riding ? 'both' : 'none',
    riding ? (((index * 7) % 1_800) - 900) / 10 : 0,
    riding ? (((index * 5) % 800) - 400) / 10 : 0,
    riding ? speed * 120 : 0,
    speed,
    (((index * 11) % 80) - 40) / 10,
    (((index * 7) % 60) - 30) / 10,
    (((index * 13) % 50) - 25) / 10,
    riding ? speed / 2 : 0,
    riding ? speed / 2.1 : 0,
    riding ? speed / 1.8 : 0,
    riding ? ((index * 3) % 40) / 10 : 0,
    riding ? ((index * 2) % 30) / 10 : 0,
    2.7,
    2.8,
    0,
    riding ? 1 : 0,
    riding ? 0.4 : 0,
    0,
    riding ? 0.6 : 0,
    0,
    0,
    riding ? 180 : 0,
    riding ? 0.05 : 0,
    distance,
    12_345 + distance,
    index / 10_000,
    index / 100_000,
    index * 0.8,
    index * 0.1,
    riding ? 0.65 : 0.64,
    0,
    0,
    JSON.stringify({ generatedWave: Math.sin(index / 100) }),
  ];
  return values.map(csv).join(',');
}

function sqlString(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}

function buildImportSQL(csvPath: string): string {
  const schema = readFileSync(resolve(import.meta.dirname, '../../backends/sql/session.sql'), 'utf8');
  const importColumns = sampleColumns.map((column) => `${column} TEXT`).join(', ');
  const selectedColumns = sampleColumns
    .map((column) => {
      if (column === 'extension_json') return `COALESCE(NULLIF(${column}, ''), '{}')`;
      const type = ['sequence', 'observed_at_ms', 'slow_observed_at_ms', 'monotonic_elapsed_ms'].includes(column)
        ? 'INTEGER'
        : ['wheelslip', 'darkride', 'charging'].includes(column)
          ? 'INTEGER'
          : [
                'ride_state',
                'package_state',
                'package_mode',
                'stop_condition',
                'setpoint_adjustment',
                'alert_reason',
                'firmware_fault',
                'footpad_state',
              ].includes(column)
            ? 'TEXT'
            : 'REAL';
      return type === 'TEXT' ? `NULLIF(${column}, '')` : `CAST(NULLIF(${column}, '') AS ${type})`;
    })
    .join(', ');
  const totalElapsed = (SAMPLE_COUNT - 1) * PERSISTENCE_MS;
  const ridingElapsed = Math.round(SAMPLE_COUNT * 0.9) * PERSISTENCE_MS;
  const manifest = {
    sessionId: GENERATED_SESSION_ID,
    deviceHistoryId: GENERATED_DEVICE_ID,
    deviceSnapshot: {
      savedDeviceId: GENERATED_DEVICE_ID,
      deviceHistoryId: GENERATED_DEVICE_ID,
      displayName: 'Generated 150k benchmark',
    },
    boardProfileSnapshot: {},
    boardProfileRevision: 0,
    startedAtMs: STARTED_AT_MS,
    endedAtMs: STARTED_AT_MS + totalElapsed,
    state: 'completed',
    integrity: 'complete',
    committedSequence: SAMPLE_COUNT,
    sampleCount: SAMPLE_COUNT,
    eventCount: 2,
    requestedIdleMs: PERSISTENCE_MS,
    effectiveIdleMs: PERSISTENCE_MS,
    requestedRidingMs: PERSISTENCE_MS,
    effectiveRidingMs: PERSISTENCE_MS,
    closeReason: 'generated_fixture',
    ridingElapsedMs: ridingElapsed,
    totalElapsedMs: totalElapsed,
    distanceM: SAMPLE_COUNT * 5.4,
    maxSpeedMps: 36 / 3.6,
    maxDutyCycle: 0.849,
  };
  const archive = JSON.stringify(manifest);
  const escapedCSVPath = csvPath.replaceAll("'", "''");
  return `${schema}
CREATE TABLE sample_import (${importColumns});
.mode csv
.import --skip 1 '${escapedCSVPath}' sample_import
INSERT INTO sample (${sampleColumns.join(', ')}) SELECT ${selectedColumns} FROM sample_import;
DROP TABLE sample_import;
INSERT INTO session_metadata(singleton, expected_riding_persistence_ms, expected_idle_persistence_ms)
  VALUES (1, ${PERSISTENCE_MS}, ${PERSISTENCE_MS});
INSERT INTO session_numeric_field(field_name, source, first_sequence) VALUES
  ('speedKph', 'stable', 1),
  ('packVoltageV', 'stable', 1),
  ('motorCurrentA', 'stable', 1),
  ('controllerTempC', 'stable', 1),
  ('generatedWave', 'extension', 1);
INSERT INTO session_event(sequence, source_sample_sequence, observed_at_ms, monotonic_elapsed_ms, kind, details_json)
  VALUES
    (1, 1, ${STARTED_AT_MS}, 0, 'rideState', '{"value":"riding"}'),
    (2, 18001, ${STARTED_AT_MS + 18_000_000}, 18000000, 'rideState', '{"value":"idle"}');
INSERT INTO commit_marker(
  singleton, committed_sample_sequence, committed_event_sequence,
  recorded_sample_count, stored_sample_count, committed_event_count,
  committed_elapsed_ms, history_revision, committed_at_ms, riding_elapsed_ms,
  distance_m, maximum_speed_kph, maximum_speed_source_sequence,
  maximum_duty_cycle, maximum_duty_source_sequence, last_speed_kph, last_ride_state
) VALUES (
  1, ${SAMPLE_COUNT}, 2, ${SAMPLE_COUNT}, ${SAMPLE_COUNT}, 2,
  ${totalElapsed}, 0, ${STARTED_AT_MS + totalElapsed}, ${ridingElapsed},
  ${SAMPLE_COUNT * 5.4}, 36, 240, 0.849, 1, 0, 'idle'
);
CREATE TABLE float_view_archive_metadata(key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
INSERT INTO float_view_archive_metadata(key, value) VALUES ('format_version', '2');
INSERT INTO float_view_archive_metadata(key, value) VALUES ('session', ${sqlString(archive)});
PRAGMA wal_checkpoint(TRUNCATE);
`;
}

async function main() {
  mkdirSync(dirname(output), { recursive: true });
  for (const suffix of ['', '-wal', '-shm']) rmSync(`${output}${suffix}`, { force: true });
  const temporary = mkdtempSync(join(tmpdir(), 'float-view-generated-session-'));
  const csvPath = join(temporary, 'samples.csv');
  try {
    const header = `${sampleColumns.join(',')}\n`;
    const rows = new Array<string>(SAMPLE_COUNT + 1);
    rows[0] = header;
    for (let index = 0; index < SAMPLE_COUNT; index += 1) rows[index + 1] = `${sampleRow(index)}\n`;
    writeFileSync(csvPath, rows.join(''));
    execFileSync('sqlite3', [output], {
      input: buildImportSQL(csvPath),
      stdio: ['pipe', 'ignore', 'inherit'],
    });
    for (const suffix of ['-wal', '-shm']) rmSync(`${output}${suffix}`, { force: true });
    console.log(`Generated ${SAMPLE_COUNT.toLocaleString()} samples at ${output}`);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

await main();
