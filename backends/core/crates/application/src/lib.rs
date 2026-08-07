//! Process-owned application coordination boundary.

mod actor;
mod configuration_service;
mod connection;
mod controller_discovery;
mod model;
mod policy;

pub use actor::*;
pub use configuration_service::*;
pub use connection::*;
pub use controller_discovery::*;
pub use model::*;
pub use policy::*;

pub use float_view_domain::{OperationId, RuntimeGeneration};
pub use float_view_platform_ports::{PlatformCommand, PlatformEvent};

/// Marker proving that protocol and storage owners are linked below the app.
#[must_use]
pub fn fixture_inventory() -> (usize, usize, usize) {
    (
        float_view_vesc::fixtures::FRAMING.len(),
        float_view_refloat::fixtures::PROTOCOL_123.len(),
        float_view_bms::fixtures::SOURCE_DERIVED.len(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn application_sees_all_domain_fixture_families() {
        let (vesc, refloat, bms) = super::fixture_inventory();
        assert!(vesc > 0 && refloat > 0 && bms > 0);
    }
}
