//! Classify diagnostic payloads before they leave the protocol owner.
//! No firmware identities, CAN routing bytes, arbitrary layout names or GNSS.
use super::*;

#[derive(Clone)]
pub struct PacketCapture {
    pub bytes: Option<Vec<u8>>,
    pub omission: Option<&'static str>,
    pub routing_removed: bool,
}
impl std::fmt::Debug for PacketCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PacketCapture")
            .field("bytes", &self.bytes.as_ref().map(Vec::len))
            .field("omission", &self.omission)
            .field("routing_removed", &self.routing_removed)
            .finish()
    }
}

pub(super) fn capture(
    request: &scheduler::ScheduledRequest,
    response: Option<&[u8]>,
    schema: Option<&configuration_codec::ConfigurationSchema>,
) -> PacketCapture {
    use scheduler::RequestContext as C;
    let tx = response.is_none();
    let routed = matches!(request.context.route(), Some(scheduler::Route::Can(_)));
    let sent = if routed && request.payload.first() == Some(&34) {
        request.payload.get(2..).unwrap_or_default()
    } else {
        &request.payload
    };
    let body = response.unwrap_or(sent);
    let allowed = match &request.context {
        C::ControllerFirmware(_)
        | C::ControllerRouteVerification(_)
        | C::AuxiliaryFirmware(_)
        | C::RefloatInfo(_)
        | C::CanPing
        | C::AuxiliaryCanPing => Err("identity_packet"),
        C::RefloatConfigurationSchema { .. } => {
            if tx {
                Ok(())
            } else {
                Err("unclassified_packet")
            }
        }
        C::RefloatConfigurationWrite { .. } if !tx => {
            if matches!(
                custom_configuration::decode_response(body),
                Ok(custom_configuration::CustomConfigurationResponse::WriteAcknowledgement)
            ) {
                Ok(())
            } else {
                Err("malformed_packet")
            }
        }
        C::RefloatConfigurationValues { .. }
        | C::RefloatConfigurationVerification { .. }
        | C::RefloatConfigurationWrite { .. } => {
            if tx && !matches!(request.context, C::RefloatConfigurationWrite { .. }) {
                Ok(())
            } else if let Some(schema) = schema {
                match body.get(2..).and_then(|bytes| configuration_codec::decode_values(bytes, schema).ok()) {
                    Some(values) if values.iter().any(|(_, v)| matches!(v, configuration_codec::ConfigurationValue::String(text) if !text.is_empty())) => Err("unclassified_packet"),
                    Some(_) => Ok(()),
                    None => Err("malformed_packet"),
                }
            } else {
                Err("unclassified_packet")
            }
        }
        C::RefloatTelemetry(_) if tx => Ok(()),
        C::RefloatTelemetry(_) if sent.get(..3) == Some(&[36, 101, 33]) && sent.len() == 12 => {
            let mask1 = u32::from_be_bytes(sent[4..8].try_into().expect("extent"));
            let mask2 = u32::from_be_bytes(sent[8..12].try_into().expect("extent"));
            if mask2 & dynamic::SELECTABLE_GNSS_MASK2 != 0 {
                Err("location_packet")
            } else if dynamic::decode_selectable(mask1, mask2, body).is_ok() {
                Ok(())
            } else {
                Err("malformed_packet")
            }
        }
        C::RefloatTelemetry(_) if sent.get(..3) == Some(&[36, 101, 10]) => {
            if legacy::decode(body).is_ok() {
                Ok(())
            } else {
                Err("malformed_packet")
            }
        }
        C::RefloatControl(_) | C::RefloatLighting(_) | C::ControllerShutdown(_) if tx => Ok(()),
        _ => Err("unclassified_packet"),
    };
    match allowed {
        Ok(()) if body.len() <= 4096 => PacketCapture {
            bytes: Some(body.to_vec()),
            omission: None,
            routing_removed: routed,
        },
        result => PacketCapture {
            bytes: None,
            omission: Some(result.err().unwrap_or("malformed_packet")),
            routing_removed: routed,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(
        context: scheduler::RequestContext,
        payload: Vec<u8>,
    ) -> scheduler::ScheduledRequest {
        scheduler::ScheduledRequest {
            enqueued_at_ms: 0,
            dispatched_at_ms: Some(0),
            sequence: 1,
            payload,
            expected_response: scheduler::ExpectedResponse {
                command: 36,
                refloat_command: None,
            },
            priority: scheduler::Priority::ControllerWork,
            context,
            deadline_monotonic_ms: 1000,
            generation: scheduler::Generation(1),
        }
    }
    fn fixture(name: &str) -> Vec<u8> {
        let text = include_str!("../../../fixtures/refloat/detailed-source-provenance.json");
        let start = text.split_once(&format!("\"name\": \"{name}\"")).unwrap().1;
        let hex = start
            .split_once("\"payloadHex\": \"")
            .unwrap()
            .1
            .split('"')
            .next()
            .unwrap();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }
    #[test]
    fn direct_and_can_capture_payloads_without_routing_identifiers() {
        let body = dynamic::selectable_fast_request().to_vec();
        for route in [scheduler::Route::Direct, scheduler::Route::Can(42)] {
            let mut payload = if route == scheduler::Route::Direct {
                vec![]
            } else {
                vec![34, 42]
            };
            payload.extend_from_slice(&body);
            let request = request(scheduler::RequestContext::RefloatTelemetry(route), payload);
            let sent = capture(&request, None, None);
            assert_eq!(sent.bytes.as_ref().unwrap(), &body);
            assert_eq!(sent.routing_removed, route != scheduler::Route::Direct);
            let response = fixture("1.3.0-command-33-fast-float32");
            let received = capture(&request, Some(&response), None);
            assert_eq!(received.bytes.unwrap(), response);
            assert!(!format!("{sent:?}").contains("[36"));
        }
    }
    #[test]
    fn identity_location_unknown_and_malformed_bytes_are_not_dumped() {
        let cases = [
            (
                scheduler::RequestContext::ControllerFirmware(scheduler::Route::Direct),
                vec![0],
                "identity_packet",
            ),
            (
                scheduler::RequestContext::RefloatInfo(scheduler::Route::Direct),
                vec![36, 101, 0, 2, 0],
                "identity_packet",
            ),
            (
                scheduler::RequestContext::RefloatTelemetry(scheduler::Route::Direct),
                dynamic::selectable_slow_request(true).to_vec(),
                "location_packet",
            ),
            (
                scheduler::RequestContext::RefloatLayout(scheduler::Route::Direct),
                vec![36, 101, 32],
                "unclassified_packet",
            ),
            (
                scheduler::RequestContext::RefloatTelemetry(scheduler::Route::Direct),
                dynamic::selectable_fast_request().to_vec(),
                "malformed_packet",
            ),
        ];
        for (context, payload, reason) in cases {
            let captured = capture(
                &request(context, payload),
                Some(b"PRIVATE-address-location-token"),
                None,
            );
            assert!(captured.bytes.is_none());
            assert_eq!(captured.omission, Some(reason));
            assert!(!format!("{captured:?}").contains("PRIVATE"));
        }
    }
    #[test]
    fn configuration_packets_require_the_selected_validated_schema() {
        let schema = configuration_codec::decode_qt_compressed(include_bytes!(
            "../../../fixtures/refloat-configuration/schemas/1.3.0.qcompress"
        ))
        .unwrap();
        let request = request(
            scheduler::RequestContext::RefloatConfigurationValues {
                route: scheduler::Route::Direct,
                index: 0,
                defaults: false,
                operation: 1,
            },
            vec![93, 0],
        );
        let mut response = vec![93, 0];
        response.extend_from_slice(include_bytes!(
            "../../../fixtures/refloat-configuration/values/1.3.0-current.bin"
        ));
        assert_eq!(
            capture(&request, Some(&response), Some(&schema)).bytes,
            Some(response)
        );
        assert_eq!(
            capture(&request, Some(b"PRIVATE"), Some(&schema)).omission,
            Some("malformed_packet")
        );
        assert_eq!(
            capture(&request, Some(b"PRIVATE"), None).omission,
            Some("unclassified_packet")
        );
    }
    #[test]
    fn valid_configuration_text_is_omitted_from_raw_packets() {
        let mut schema = configuration_codec::decode_qt_compressed(include_bytes!(
            "../../../fixtures/refloat-configuration/schemas/1.3.0.qcompress"
        ))
        .unwrap();
        schema.fields.truncate(1);
        schema.fields[0].field_type = configuration_codec::FieldType::String;
        schema.fields[0].wire_type = None;
        let values = vec![(
            schema.fields[0].name.clone(),
            configuration_codec::ConfigurationValue::String("PRIVATE-token-/private/path".into()),
        )];
        let encoded = configuration_codec::encode_values(&values, &schema).unwrap();
        let mut response = vec![93, 0];
        response.extend_from_slice(&encoded);
        let request = request(
            scheduler::RequestContext::RefloatConfigurationValues {
                route: scheduler::Route::Direct,
                index: 0,
                defaults: false,
                operation: 1,
            },
            vec![93, 0],
        );
        assert_eq!(
            capture(&request, Some(&response), Some(&schema)).omission,
            Some("unclassified_packet")
        );
    }
}
