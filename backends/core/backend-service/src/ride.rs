//! Recording continuity policy is shared by every native host.
use crate::{BackendService, operations::Failure};
impl BackendService {
    pub(crate) fn sync_ride_timer(&mut self) -> Result<(), Failure> {
        let deadline = self.policy.ride.as_ref().and_then(|r| r.resume_until_ms);
        if let Some(deadline) = deadline {
            if !self
                .policy
                .timers
                .values()
                .any(|t| t.kind == crate::policy::Timer::RideResume)
            {
                self.arm_timer(
                    crate::policy::Timer::RideResume,
                    deadline.saturating_sub(self.epoch_ms()).max(1) as u64,
                    None,
                )?;
            }
        } else {
            self.cancel_timer(crate::policy::Timer::RideResume);
        }
        Ok(())
    }

    pub(crate) fn expire_paused_ride(&mut self) -> Result<(), Failure> {
        let expired = self
            .policy
            .ride
            .as_ref()
            .filter(|r| {
                r.paused_at_ms
                    .zip(r.resume_until_ms)
                    .is_some_and(|(start, end)| self.epoch_ms() < start || self.epoch_ms() >= end)
            })
            .map(|r| r.session_id.clone());
        if let Some(id) = expired {
            self.finish_paused_ride(&id, "resume_expired")?;
        }
        Ok(())
    }
    pub(crate) fn finish_paused_ride(&mut self, id: &str, reason: &str) -> Result<(), Failure> {
        if self
            .policy
            .active
            .as_ref()
            .and_then(|a| a.session.as_ref())
            .is_some_and(|s| s.session_id == id)
        {
            if reason != "resume_expired" {
                return self.disconnect(reason);
            }
            // The ride window never shortens the independent Bluetooth retry budget.
            if self.runtime.snapshot().session_attached {
                self.runtime
                    .apply(float_view_application_runtime::Command::DetachSession)
                    .map_err(|_| Failure::Internal)?;
            }
            if let Some(active) = self.policy.active.as_mut() {
                active.session = None;
                active.latest = None;
                active.session_start = None;
                active.session_elapsed_offset = 0;
                active.continuity = Default::default();
            }
            self.runtime.clear_recording();
        }
        let session = self.store.session(id).map_err(crate::store_failure)?;
        self.store
            .close_session(id, self.epoch_ms().max(session.started_at_ms), reason)
            .map_err(crate::store_failure)?;
        self.policy.ride = self.store.ride_checkpoint().map_err(crate::store_failure)?;
        self.sync_ride_timer()?;
        self.refresh_boards().map_err(crate::store_failure)?;
        self.changed()
    }
    pub(crate) fn extend_ride_pause(&mut self, id: &str) -> Result<(), Failure> {
        self.store
            .extend_ride_pause(id, self.epoch_ms())
            .map_err(crate::store_failure)?;
        self.policy.ride = self.store.ride_checkpoint().map_err(crate::store_failure)?;
        self.cancel_timer(crate::policy::Timer::RideResume);
        self.sync_ride_timer()?;
        self.changed()
    }
    pub(crate) fn end_ride(&mut self, id: &str) -> Result<(), Failure> {
        if self.policy.ride.as_ref().is_none_or(|r| r.session_id != id) {
            return Err(Failure::InvalidState);
        }
        self.cancel_manual_auto_connect()?;
        self.finish_paused_ride(id, "end_ride")
    }
}
