use crate::{Error, FileCancellation, Result, StagedFile, limits};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    fs::File,
    io::{BufWriter, Write},
};
const HEADER: &str = "sequence,observed_at_ms,slow_observed_at_ms,elapsed_ms,ride_state,package_state,package_mode,footpad_state,stop_condition,setpoint_adjustment,wheelslip,darkride,charging,alert_reason,firmware_fault,pack_voltage_v,duty_cycle,controller_temp_c,motor_temp_c,bms_circuitry_temp_c,bms_highest_cell_temp_c,motor_current_a,battery_current_a,erpm,speed_kph,pitchDeg,balancePitchDeg,rollDeg,directionalMotorCurrentA,filteredMotorCurrentA,requestedBalanceCurrentA,boosterCurrentA,fieldWeakeningCurrentA,footpadAdc1V,footpadAdc2V,remoteInput,setpointDeg,atrSetpointDeg,brakeTiltSetpointDeg,torqueTiltSetpointDeg,turnTiltSetpointDeg,remoteSetpointDeg,atrAccelerationDifferenceErpmPerSecond,atrSpeedBoost,distanceM,odometerM,ampHoursDrawnAh,ampHoursChargedAh,wattHoursDrawnWh,wattHoursChargedWh,batteryLevel,chargingCurrentA,chargingVoltageV,extension_json";
const SELECT: &str = "SELECT CAST(sequence AS TEXT), CAST(observed_at_ms AS TEXT), CAST(slow_observed_at_ms AS TEXT), CAST(monotonic_elapsed_ms AS TEXT), CAST(ride_state AS TEXT), CAST(package_state AS TEXT), CAST(package_mode AS TEXT), CAST(footpad_state AS TEXT), CAST(stop_condition AS TEXT), CAST(setpoint_adjustment AS TEXT), CAST(wheelslip AS TEXT), CAST(darkride AS TEXT), CAST(charging AS TEXT), CAST(alert_reason AS TEXT), CAST(firmware_fault AS TEXT), CAST(raw_pack_voltage_v AS TEXT), CAST(duty_cycle AS TEXT), CAST(controller_temp_c AS TEXT), CAST(motor_temp_c AS TEXT), CAST(bms_circuitry_temp_c AS TEXT), CAST(bms_highest_cell_temp_c AS TEXT), CAST(motor_current_a AS TEXT), CAST(battery_current_a AS TEXT), CAST(erpm AS TEXT), CAST(speed_kph AS TEXT), CAST(pitch_deg AS TEXT), CAST(balance_pitch_deg AS TEXT), CAST(roll_deg AS TEXT), CAST(directional_motor_current_a AS TEXT), CAST(filtered_motor_current_a AS TEXT), CAST(requested_balance_current_a AS TEXT), CAST(booster_current_a AS TEXT), CAST(field_weakening_current_a AS TEXT), CAST(footpad_adc_1_v AS TEXT), CAST(footpad_adc_2_v AS TEXT), CAST(remote_input AS TEXT), CAST(setpoint_deg AS TEXT), CAST(atr_setpoint_deg AS TEXT), CAST(brake_tilt_setpoint_deg AS TEXT), CAST(torque_tilt_setpoint_deg AS TEXT), CAST(turn_tilt_setpoint_deg AS TEXT), CAST(remote_setpoint_deg AS TEXT), CAST(atr_acceleration_difference_erpm_per_second AS TEXT), CAST(atr_speed_boost AS TEXT), CAST(distance_m AS TEXT), CAST(odometer_m AS TEXT), CAST(amp_hours_drawn_ah AS TEXT), CAST(amp_hours_charged_ah AS TEXT), CAST(watt_hours_drawn_wh AS TEXT), CAST(watt_hours_charged_wh AS TEXT), CAST(battery_level AS TEXT), CAST(charging_current_a AS TEXT), CAST(charging_voltage_v AS TEXT), CAST(extension_json AS TEXT) FROM sample ORDER BY sequence";
const COLUMN_COUNT: usize = 54;
pub(crate) fn write(
    db: &Connection,
    stage: &StagedFile,
    gps: bool,
    cancel: &FileCancellation,
) -> Result<()> {
    let mut output = BufWriter::with_capacity(65536, File::create(stage.path())?);
    let mut written = 0_u64;
    let header = format!(
        "{HEADER}{}\n",
        if gps {
            ",gpsAccuracy,gpsLongitude,gpsLatitude"
        } else {
            ""
        }
    );
    emit(&mut output, &mut written, header.as_bytes())?;
    let mut statement = db.prepare(SELECT)?;
    let mut rows = statement.query([])?;
    let mut before = db.prepare(LOCATION_BEFORE)?;
    let mut after = db.prepare(LOCATION_AFTER)?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let mut line = String::new();
        for i in 0..COLUMN_COUNT {
            if i > 0 {
                line.push(',');
            }
            if let Some(text) = row
                .get_ref(i)?
                .as_str_or_null()
                .map_err(|_| Error::Corrupt)?
            {
                if text.len() > 16_384 {
                    return Err(Error::Corrupt);
                }
                escape(text, &mut line);
            }
        }
        if gps {
            let elapsed = row
                .get::<_, String>(3)?
                .parse::<i64>()
                .map_err(|_| Error::Corrupt)?;
            // Only the nearest predecessor and successor can win. Each query
            // seeks an elapsed-time boundary and then the earliest tied sequence.
            let before = location_candidate(&mut before, elapsed.saturating_sub(10000), elapsed)?;
            let after = location_candidate(&mut after, elapsed, elapsed.saturating_add(10000))?;
            let values = [before, after]
                .into_iter()
                .flatten()
                .min_by_key(|candidate| (candidate.elapsed.abs_diff(elapsed), candidate.sequence))
                .map(|candidate| candidate.values);
            for value in values.unwrap_or([None; 3]) {
                line.push(',');
                if let Some(v) = value {
                    if !v.is_finite() {
                        return Err(Error::Corrupt);
                    }
                    if v.fract() == 0. {
                        line.push_str(&format!("{v:.1}"));
                    } else {
                        line.push_str(&v.to_string());
                    }
                }
            }
        }
        line.push('\n');
        emit(&mut output, &mut written, line.as_bytes())?;
    }
    output.flush()?;
    Ok(())
}
fn emit(out: &mut impl Write, written: &mut u64, data: &[u8]) -> Result<()> {
    let next = written
        .checked_add(data.len() as u64)
        .filter(|v| *v <= limits::MAX_EXPORT_BYTES)
        .ok_or(Error::Capacity)?;
    out.write_all(data)?;
    *written = next;
    Ok(())
}
fn escape(text: &str, out: &mut String) {
    if text.contains([',', '"', '\n', '\r']) {
        out.push('"');
        out.push_str(&text.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(text);
    }
}

// MAX/MIN seek the existing elapsed index. The outer equality resolves ties
// by sequence without sorting an entire ten-second neighborhood per sample.
const LOCATION_BEFORE: &str = "SELECT accuracy_m,longitude,latitude,monotonic_elapsed_ms,location_sequence FROM location WHERE monotonic_elapsed_ms=(SELECT MAX(monotonic_elapsed_ms) FROM location WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2) ORDER BY location_sequence LIMIT 1";
const LOCATION_AFTER: &str = "SELECT accuracy_m,longitude,latitude,monotonic_elapsed_ms,location_sequence FROM location WHERE monotonic_elapsed_ms=(SELECT MIN(monotonic_elapsed_ms) FROM location WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2) ORDER BY location_sequence LIMIT 1";
struct LocationCandidate {
    values: [Option<f64>; 3],
    elapsed: i64,
    sequence: i64,
}
fn location_candidate(
    statement: &mut rusqlite::Statement<'_>,
    start: i64,
    end: i64,
) -> Result<Option<LocationCandidate>> {
    Ok(statement
        .query_row(params![start, end], |row| {
            Ok(LocationCandidate {
                values: [row.get(0)?, row.get(1)?, row.get(2)?],
                elapsed: row.get(3)?,
                sequence: row.get(4)?,
            })
        })
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexed_gps_candidates_match_neighborhood_sort_with_ties_gaps_and_limits() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(float_view_session_sqlite::SESSION_SCHEMA)
            .unwrap();
        let tx = db.unchecked_transaction().unwrap();
        for n in 1..=3000 {
            let elapsed = if n < 2000 { n / 3 * 100 } else { n * 1000 };
            tx.execute("INSERT INTO location(location_sequence,observed_at_ms,monotonic_elapsed_ms,latitude,longitude,accuracy_m) VALUES (?1,?2,?2,0,?3,3)",params![n,elapsed,(n%3) as f64]).unwrap();
        }
        tx.commit().unwrap();
        let mut before = db.prepare(LOCATION_BEFORE).unwrap();
        let mut after = db.prepare(LOCATION_AFTER).unwrap();
        for elapsed in [
            0,
            50,
            100,
            150,
            10000,
            50000,
            200000,
            1990000,
            2000000,
            2000500,
            i64::MAX - 100,
            i64::MAX,
        ] {
            let left =
                location_candidate(&mut before, elapsed.saturating_sub(10000), elapsed).unwrap();
            let right =
                location_candidate(&mut after, elapsed, elapsed.saturating_add(10000)).unwrap();
            let winner = [left, right]
                .into_iter()
                .flatten()
                .min_by_key(|c| (c.elapsed.abs_diff(elapsed), c.sequence))
                .map(|c| c.sequence);
            let expected=db.query_row("SELECT location_sequence FROM location WHERE monotonic_elapsed_ms BETWEEN ?1 AND ?2 ORDER BY ABS(monotonic_elapsed_ms-?3),location_sequence LIMIT 1",params![elapsed.saturating_sub(10000),elapsed.saturating_add(10000),elapsed],|r|r.get::<_,i64>(0)).optional().unwrap();
            assert_eq!(winner, expected, "elapsed={elapsed}");
        }
        for sql in [LOCATION_BEFORE, LOCATION_AFTER] {
            let mut statement = db.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
            let plan = statement
                .query_map([0, 10000], |r| r.get::<_, String>(3))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
                .join("; ");
            assert!(!plan.contains("TEMP B-TREE"), "{plan}");
            assert!(plan.contains("SEARCH location"), "{plan}");
        }
    }
}
