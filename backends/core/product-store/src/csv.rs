use crate::{Error, FileCancellation, Result, StagedFile, limits};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    fs::File,
    io::{BufWriter, Write},
};
const HEADER: &str = "sequence,observed_at_ms,slow_observed_at_ms,elapsed_ms,ride_state,package_state,package_mode,footpad_state,stop_condition,setpoint_adjustment,wheelslip,darkride,charging,alert_reason,firmware_fault,pack_voltage_v,duty_cycle,controller_temp_c,motor_temp_c,motor_current_a,battery_current_a,erpm,speed_kph,pitchDeg,balancePitchDeg,rollDeg,directionalMotorCurrentA,filteredMotorCurrentA,requestedBalanceCurrentA,boosterCurrentA,fieldWeakeningCurrentA,footpadAdc1V,footpadAdc2V,remoteInput,setpointDeg,atrSetpointDeg,brakeTiltSetpointDeg,torqueTiltSetpointDeg,turnTiltSetpointDeg,remoteSetpointDeg,atrAccelerationDifferenceErpmPerSecond,atrSpeedBoost,distanceM,odometerM,ampHoursDrawnAh,ampHoursChargedAh,wattHoursDrawnWh,wattHoursChargedWh,batteryLevel,chargingCurrentA,chargingVoltageV,extension_json";
const SELECT: &str = "SELECT CAST(sequence AS TEXT), CAST(observed_at_ms AS TEXT), CAST(slow_observed_at_ms AS TEXT), CAST(monotonic_elapsed_ms AS TEXT), CAST(ride_state AS TEXT), CAST(package_state AS TEXT), CAST(package_mode AS TEXT), CAST(footpad_state AS TEXT), CAST(stop_condition AS TEXT), CAST(setpoint_adjustment AS TEXT), CAST(wheelslip AS TEXT), CAST(darkride AS TEXT), CAST(charging AS TEXT), CAST(alert_reason AS TEXT), CAST(firmware_fault AS TEXT), CAST(raw_pack_voltage_v AS TEXT), CAST(duty_cycle AS TEXT), CAST(controller_temp_c AS TEXT), CAST(motor_temp_c AS TEXT), CAST(motor_current_a AS TEXT), CAST(battery_current_a AS TEXT), CAST(erpm AS TEXT), CAST(speed_kph AS TEXT), CAST(pitch_deg AS TEXT), CAST(balance_pitch_deg AS TEXT), CAST(roll_deg AS TEXT), CAST(directional_motor_current_a AS TEXT), CAST(filtered_motor_current_a AS TEXT), CAST(requested_balance_current_a AS TEXT), CAST(booster_current_a AS TEXT), CAST(field_weakening_current_a AS TEXT), CAST(footpad_adc_1_v AS TEXT), CAST(footpad_adc_2_v AS TEXT), CAST(remote_input AS TEXT), CAST(setpoint_deg AS TEXT), CAST(atr_setpoint_deg AS TEXT), CAST(brake_tilt_setpoint_deg AS TEXT), CAST(torque_tilt_setpoint_deg AS TEXT), CAST(turn_tilt_setpoint_deg AS TEXT), CAST(remote_setpoint_deg AS TEXT), CAST(atr_acceleration_difference_erpm_per_second AS TEXT), CAST(atr_speed_boost AS TEXT), CAST(distance_m AS TEXT), CAST(odometer_m AS TEXT), CAST(amp_hours_drawn_ah AS TEXT), CAST(amp_hours_charged_ah AS TEXT), CAST(watt_hours_drawn_wh AS TEXT), CAST(watt_hours_charged_wh AS TEXT), CAST(battery_level AS TEXT), CAST(charging_current_a AS TEXT), CAST(charging_voltage_v AS TEXT), CAST(extension_json AS TEXT) FROM sample ORDER BY sequence";
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
    let mut location=db.prepare("SELECT accuracy_m,longitude,latitude FROM location WHERE monotonic_elapsed_ms BETWEEN ? AND ? ORDER BY ABS(monotonic_elapsed_ms-?),location_sequence LIMIT 1")?;
    while let Some(row) = rows.next()? {
        cancel.check()?;
        let mut line = String::new();
        for i in 0..52 {
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
            let values = location
                .query_row(
                    params![
                        elapsed.saturating_sub(10000),
                        elapsed.saturating_add(10000),
                        elapsed
                    ],
                    |r| Ok([r.get::<_, Option<f64>>(0)?, r.get(1)?, r.get(2)?]),
                )
                .optional()?;
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
