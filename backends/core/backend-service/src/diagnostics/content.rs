//! Explicitly authorized diagnostic contents; never arbitrary request/error objects.
use super::*;
use float_view_application_runtime::coordinator as c;

impl Diagnostics {
    pub(super) fn content_event(
        &mut self,
        now: u64,
        epoch: i64,
        event: &'static str,
        facts: Value,
        content: (ContentKind, &[u8]),
        persist: bool,
    ) {
        let (category, bytes) = content;
        let capture = self.sequence.saturating_add(1);
        let retained = &bytes[..bytes.len().min(MAX_CONTENT_BYTES)];
        for (index, chunk) in retained.chunks(CONTENT_CHUNK_BYTES).enumerate() {
            let mut facts = facts.clone();
            facts["content_id"] = json!(capture);
            facts["offset"] = json!(index * CONTENT_CHUNK_BYTES);
            facts["total_bytes"] = json!(bytes.len());
            facts["truncated"] = json!(retained.len() != bytes.len());
            let hex = chunk.iter().map(|byte| format!("{byte:02x}")).collect();
            self.event_with_content(
                now,
                epoch,
                event,
                facts,
                persist,
                Some(Content { category, hex }),
            );
        }
    }
}

fn configuration_values(
    schema: &c::configuration_codec::ConfigurationSchema,
    values: &c::configuration_codec::ConfigurationValues,
) -> Value {
    use c::configuration_codec::{ConfigurationValue as V, FieldType as T};
    let fields = schema
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.field_type))
        .collect::<BTreeMap<_, _>>();
    let values = values
        .iter()
        .filter_map(|(name, value)| {
            let kind = fields.get(name.as_str())?;
            let value = match (kind, value) {
                (T::Decimal, V::Decimal(v)) if v.is_finite() => json!(v),
                (T::Integer | T::Enumeration | T::Bitfield, V::Integer(v)) => json!(v),
                (T::Boolean, V::Boolean(v)) => json!(v),
                (T::String, V::String(v)) if v.is_empty() => json!(""),
                (T::String, V::String(_)) => json!({"redacted":true,"type":"text"}),
                _ => return None,
            };
            Some((name.clone(), value))
        })
        .collect::<serde_json::Map<_, _>>();
    Value::Object(values)
}

impl BackendService {
    pub(crate) fn record_configuration_contents(&mut self, requested: Option<&c::Command>) {
        let state = self.runtime.snapshot().application;
        let Some(schema) = self.runtime.configuration_schema() else {
            if let Some(observed) = &state.configuration.observed_schema {
                self.diagnostics.event(self.policy.exchange.now, self.policy.exchange.epoch, "configuration_operation",
                    json!({"application_generation":state.generation.0,"schema_fingerprint":observed.fingerprint,"wire_signature":observed.wire_signature}), true);
            }
            return;
        };
        let fingerprint = schema.fingerprint.clone();
        let signature = schema.wire_signature;
        let current = state
            .configuration
            .current
            .as_ref()
            .map(|v| (v.revision, configuration_values(schema, &v.values)));
        let defaults = state
            .configuration
            .defaults
            .as_ref()
            .map(|v| (v.revision, configuration_values(schema, &v.values)));
        let requested_values = if let Some(c::Command::WriteConfiguration {
            values,
            base_revision,
            fingerprint,
            wire_signature,
            ..
        }) = requested
        {
            Some((
                *base_revision,
                fingerprint.clone(),
                *wire_signature,
                configuration_values(schema, values),
            ))
        } else {
            None
        };
        let now = self.policy.exchange.now;
        let epoch = self.policy.exchange.epoch;
        let base = json!({"application_generation":state.generation.0,"schema_fingerprint":fingerprint,"wire_signature":signature});
        let snapshot_key = (
            state.generation.0,
            current.as_ref().map(|v| v.0),
            defaults.as_ref().map(|v| v.0),
            now,
        );
        let record_snapshot = self.diagnostics.last_configuration_content != Some(snapshot_key);
        self.diagnostics.last_configuration_content = Some(snapshot_key);
        if record_snapshot {
            self.diagnostics
                .event(now, epoch, "configuration_operation", base.clone(), true);
        }
        for (category, loaded) in [
            (ContentKind::ConfigurationCurrent, current),
            (ContentKind::ConfigurationDefault, defaults),
        ] {
            if record_snapshot && let Some((revision, values)) = loaded {
                let mut facts = base.clone();
                facts["current_revision"] = json!(revision);
                if let Ok(bytes) = serde_json::to_vec(&values) {
                    self.diagnostics.content_event(
                        now,
                        epoch,
                        "diagnostic_content",
                        facts,
                        (category, &bytes),
                        true,
                    );
                }
            }
        }
        if let Some((revision, fingerprint, signature, values)) = requested_values {
            let mut facts = base;
            facts["base_revision"] = json!(revision);
            // A syntactically valid client string could still be a credential.
            // Only known schema digests are safe to retain as requested identity.
            if c::configuration::CATALOG
                .iter()
                .any(|entry| entry.fingerprint == fingerprint)
            {
                facts["requested_schema_fingerprint"] = json!(fingerprint);
            }
            facts["requested_wire_signature"] = json!(signature);
            if let Ok(bytes) = serde_json::to_vec(&values) {
                self.diagnostics.content_event(
                    now,
                    epoch,
                    "diagnostic_content",
                    facts,
                    (ContentKind::ConfigurationWrite, &bytes),
                    true,
                );
            }
        }
    }

    pub(crate) fn record_request_diagnostic(&mut self, request: &c::RequestTiming, persist: bool) {
        let mut facts = json!({"application_generation":request.generation,"sequence":request.sequence,"kind":request.kind,
            "stage":request.stage,"configuration_operation":request.configuration_operation,"outcome":request.outcome,
            "queue_ms":request.queue_ms,"response_ms":request.response_ms});
        if let Some(packet) = &request.packet {
            facts["routing_removed"] = json!(packet.routing_removed);
            facts["packet_omitted"] = json!(packet.omission);
            if let Some(bytes) = &packet.bytes {
                let category = if request.outcome == "sent" {
                    ContentKind::PacketTx
                } else {
                    ContentKind::PacketRx
                };
                self.diagnostics.content_event(
                    self.policy.exchange.now,
                    self.policy.exchange.epoch,
                    "request",
                    facts,
                    (category, bytes),
                    persist,
                );
                return;
            }
        }
        self.diagnostics.event(
            self.policy.exchange.now,
            self.policy.exchange.epoch,
            "request",
            facts,
            persist,
        );
    }

    pub(crate) fn record_telemetry_contents(&mut self, sample: &c::telemetry::NormalizedSample) {
        let now = self.policy.exchange.now;
        if self
            .diagnostics
            .last_telemetry_content
            .is_some_and(|at| now.saturating_sub(at) < 1000)
        {
            return;
        }
        self.diagnostics.last_telemetry_content = Some(now);
        // Explicit field selection keeps future location/identity/extension fields out.
        macro_rules! numeric { ($($field:ident),* $(,)?) => {{
            let mut values = serde_json::Map::new();
            $(values.insert(stringify!($field).to_owned(), json!(sample.$field));)*
            Value::Object(values)
        }}; }
        let values = numeric!(
            pack_voltage_v,
            duty_cycle,
            controller_temp_c,
            motor_temp_c,
            bms_circuitry_temp_c,
            bms_highest_cell_temp_c,
            motor_current_a,
            battery_current_a,
            erpm,
            speed_kph,
            pitch_deg,
            balance_pitch_deg,
            roll_deg,
            directional_motor_current_a,
            filtered_motor_current_a,
            requested_balance_current_a,
            booster_current_a,
            field_weakening_current_a,
            footpad_adc1_v,
            footpad_adc2_v,
            remote_input,
            setpoint_deg,
            atr_setpoint_deg,
            brake_tilt_setpoint_deg,
            torque_tilt_setpoint_deg,
            turn_tilt_setpoint_deg,
            remote_setpoint_deg,
            atr_acceleration_difference_erpm_per_second,
            atr_speed_boost,
            distance_m,
            odometer_m,
            amp_hours_drawn_ah,
            amp_hours_charged_ah,
            watt_hours_drawn_wh,
            watt_hours_charged_wh,
            battery_level,
            charging_current_a,
            charging_voltage_v
        );
        if let Ok(bytes) = serde_json::to_vec(&values) {
            self.diagnostics.content_event(now, self.policy.exchange.epoch, "diagnostic_content",
                json!({"application_generation":self.runtime.snapshot().application.generation.0,"sequence":sample.sequence}),
                (ContentKind::Telemetry, &bytes), false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_text_cannot_smuggle_excluded_paths_or_credentials() {
        let mut schema = c::configuration_codec::decode_qt_compressed(include_bytes!(
            "../../../../fixtures/refloat-configuration/schemas/1.3.0.qcompress"
        ))
        .unwrap();
        schema.fields[0].field_type = c::configuration_codec::FieldType::String;
        let name = schema.fields[0].name.clone();
        let values = vec![(
            name.clone(),
            c::configuration_codec::ConfigurationValue::String(
                "PRIVATE-token-/private/path".into(),
            ),
        )];
        let captured = configuration_values(&schema, &values);
        assert_eq!(captured[&name], json!({"redacted":true,"type":"text"}));
        assert!(!captured.to_string().contains("PRIVATE"));
    }
}
