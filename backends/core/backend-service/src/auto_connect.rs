//! One foreground opportunity over the existing normal discovery transport.
use crate::{
    BackendService,
    operations::Failure,
    os::{Effect, Permission, Radio},
    policy::Timer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opportunity {
    Waiting,
    Searching { until: u64 },
    Closed,
}

impl BackendService {
    pub(crate) fn cancel_auto_connect(&mut self) {
        self.policy.auto_connect = Opportunity::Closed;
    }

    pub(crate) fn cancel_manual_auto_connect(&mut self) -> Result<(), Failure> {
        let scanning = self.policy.scan.is_some();
        self.cancel_auto_connect();
        self.step_auto_connect()?;
        if scanning != self.policy.scan.is_some() {
            self.changed()?;
        }
        Ok(())
    }

    fn auto_connect_ready(&self) -> bool {
        let granted = |p| matches!(p, Permission::Granted | Permission::NotRequired);
        self.policy.foreground_observed
            && self.policy.foreground
            && self.policy.settings.safety_disclaimer_accepted
            && self.policy.radio == Radio::PoweredOn
            && granted(self.policy.scan_permission)
            && granted(self.policy.connect_permission)
            && self
                .policy
                .permissions
                .as_ref()
                .is_none_or(|p| p.scan_location_ready())
    }

    pub(crate) fn step_auto_connect(&mut self) -> Result<(), Failure> {
        let now = self.policy.exchange.now;
        if self.policy.active.is_some() {
            self.cancel_auto_connect();
        }
        let target = self.policy.settings.auto_connect_saved_device_id.is_some();
        match self.policy.auto_connect {
            Opportunity::Waiting if target && self.auto_connect_ready() => {
                self.policy.auto_connect = Opportunity::Searching {
                    until: now + 15_000,
                };
            }
            Opportunity::Searching { until } if !target || now >= until => {
                self.cancel_auto_connect()
            }
            _ => {}
        }
        let automatic = match self.policy.auto_connect {
            Opportunity::Searching { until } if self.auto_connect_ready() => Some(until),
            _ => None,
        };
        if self
            .policy
            .normal_scan_until
            .is_some_and(|until| now >= until)
        {
            self.policy.normal_scan_until = None;
        }
        let until = self
            .policy
            .normal_scan_until
            .into_iter()
            .chain(automatic)
            .max();
        if let Some(until) = until {
            if let Some(scan) = self.policy.scan.as_mut() {
                scan.1 = until;
            } else {
                self.policy.exchange.admit()?;
                let id = self.policy.exchange.id()?;
                self.policy.scan = Some((id, until));
                self.policy.exchange.push(Effect::Scan { scan_id: id })?;
                self.arm_timer(Timer::Discovery, 250, Some(250))?;
            }
        } else if self.policy.scan.take().is_some() {
            self.policy.exchange.request_stop_scan();
        }
        Ok(())
    }

    pub(crate) fn auto_connect_discovered(&mut self, id: &str) -> Result<(), Failure> {
        if !matches!(self.policy.auto_connect, Opportunity::Searching { until } if self.policy.exchange.now < until)
            || !self.auto_connect_ready()
            || self.policy.active.is_some()
        {
            return Ok(());
        }
        let matched = self.policy.boards.iter().any(|b| {
            Some(&b.saved_device_id) == self.policy.settings.auto_connect_saved_device_id.as_ref()
                && b.bluetooth_identifier.as_deref() == Some(id)
        });
        if matched {
            self.cancel_auto_connect();
            self.connect(id, true)?;
        }
        Ok(())
    }
}
