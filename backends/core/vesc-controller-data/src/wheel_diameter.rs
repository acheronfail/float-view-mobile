//! Audited opaque motor configuration patching. Offsets include command + signature.
//! Layouts derive from vendor/vesc_tool/res/config/{6.02,6.05,6.06,7.00}/parameters_mcconf.xml
//! SerOrder and vTx widths; signatures are the existing controller-data inventory.
//! Never deserialize/re-encode unrelated fields or accept an unrecognized extent.
//! Offline bldc signature-introduction commits audited for SET_MCCONF processing:
//! 6.02: 4ed731479c1a, 6.05: 04f13d6e458f, 6.06: c249d0affe7d, 7.00: 49854b6a53a9.
//! Each applies hardware limits, stores/reapplies the whole config, then acknowledges.
//! Exact readback deliberately rejects unexpected firmware normalization.
use super::{ByteReader, ControllerDataError};

pub const SET: u8 = 13;
pub const GET: u8 = 14;
const PROFILES: [(u32, usize, usize); 4] = [
    (0x2E43_A161, 482, 457),
    (0x3F82_9CF7, 478, 447),
    (0x2EFD_0142, 484, 453),
    (0x57AD_8F53, 489, 458),
];

fn offset(payload: &[u8]) -> Option<usize> {
    if payload.first() != Some(&GET) {
        return None;
    }
    let signature = u32::from_be_bytes(payload.get(1..5)?.try_into().ok()?);
    PROFILES
        .iter()
        .find_map(|&(s, len, offset)| (s == signature && payload.len() == len).then_some(offset))
}

pub fn supported_for(payload: &[u8], major: u8, minor: u8) -> bool {
    let index = match (major, minor) {
        (6, 2) => 0,
        (6, 5) => 1,
        (6, 6) => 2,
        (7, 0) => 3,
        _ => return false,
    };
    let signature = payload
        .get(1..5)
        .and_then(|v| <[u8; 4]>::try_from(v).ok())
        .map(u32::from_be_bytes);
    signature == Some(PROFILES[index].0) && diameter_mm(payload).is_some()
}

pub fn diameter_mm(payload: &[u8]) -> Option<f64> {
    let offset = offset(payload)?;
    let mut reader = ByteReader::new(payload.get(offset..offset + 4)?);
    let meters = reader.float32_auto().ok()?;
    // Validate the representable wire endpoints: 0.6f32 is slightly above 0.6.
    // Clamp only this encoding error; neighboring values outside the range fail.
    (f64::from(0.1_f32)..=f64::from(0.6_f32))
        .contains(&meters)
        .then(|| (meters * 1000.).clamp(100., 600.))
}

pub fn valid(mm: f64) -> bool {
    mm.is_finite() && (100.0..=600.0).contains(&mm)
}

pub fn patch(payload: &[u8], expected_mm: f64, mm: f64) -> Result<Vec<u8>, ControllerDataError> {
    let fail = ControllerDataError::TruncatedPayload;
    if !valid(mm) || !valid(expected_mm) {
        return Err(fail);
    }
    let current = diameter_mm(payload).ok_or(fail)?;
    if (current - expected_mm).abs() > 0.01 {
        return Err(fail);
    }
    let offset = offset(payload).ok_or(fail)?;
    let mut result = payload.to_vec();
    result[0] = SET;
    // All allowed diameters are positive normal f32 values. VESC float32_auto
    // has IEEE754 encoding for this range (no denormals/NaN/Inf).
    result[offset..offset + 4].copy_from_slice(&((mm / 1000.) as f32).to_bits().to_be_bytes());
    Ok(result)
}

pub fn verified(written: &[u8], readback: &[u8]) -> bool {
    written.first() == Some(&SET)
        && diameter_mm(readback).is_some()
        && written.len() == readback.len()
        && written.get(1..) == readback.get(1..)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_every_other_byte_for_each_audited_layout() {
        // Independent expected extents/positions from each upstream SerOrder.
        for (signature, len, pos) in [
            (0x2e43a161_u32, 482, 457),
            (0x3f829cf7, 478, 447),
            (0x2efd0142, 484, 453),
            (0x57ad8f53, 489, 458),
        ] {
            let mut raw: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            raw[0] = GET;
            raw[1..5].copy_from_slice(&signature.to_be_bytes());
            raw[pos..pos + 4].copy_from_slice(&0.28_f32.to_be_bytes());
            let patched = patch(&raw, 280., 268.8).unwrap();
            assert_eq!(&patched[pos..pos + 4], &0.2688_f32.to_be_bytes());
            for i in 1..len {
                if !(pos..pos + 4).contains(&i) {
                    assert_eq!(raw[i], patched[i]);
                }
            }
            let mut readback = patched.clone();
            readback[0] = GET;
            assert!(verified(&patched, &readback));
            readback[50] ^= 1;
            assert!(!verified(&patched, &readback));
            assert!(patch(&raw, 281., 268.8).is_err());
            assert!(!supported_for(&raw, 255, 255));
            raw.push(0);
            assert!(diameter_mm(&raw).is_none());
            raw.pop();
            raw[1] ^= 1;
            assert!(diameter_mm(&raw).is_none());
        }
    }
    #[test]
    fn inclusive_bounds_round_trip_through_the_wire_encoding() {
        for (signature, len, pos) in PROFILES {
            let mut raw = vec![0; len];
            raw[0] = GET;
            raw[1..5].copy_from_slice(&signature.to_be_bytes());
            raw[pos..pos + 4].copy_from_slice(&0.28_f32.to_be_bytes());
            for mm in [100., 600.] {
                let mut written = patch(&raw, 280., mm).unwrap();
                let mut readback = written.clone();
                readback[0] = GET;
                assert!(verified(&written, &readback));
                assert!((diameter_mm(&readback).unwrap() - mm).abs() < 0.001);
                assert!(patch(&readback, mm, 280.).is_ok());
                // The next representable value outside each endpoint is invalid.
                written[0] = GET;
                let bits = ((mm / 1000.) as f32).to_bits();
                let outside = if mm == 100. { bits - 1 } else { bits + 1 };
                written[pos..pos + 4].copy_from_slice(&outside.to_be_bytes());
                assert!(diameter_mm(&written).is_none());
            }
        }
    }

    #[test]
    fn rejects_unsafe_and_malformed_values() {
        for v in [0., -1., 99., 601., f64::NAN, f64::INFINITY] {
            assert!(!valid(v));
        }
        assert!(diameter_mm(&[]).is_none());
    }
}
