//! One bounded speech slot and monotonic cooldown, independent of views.
use crate::{
    BackendService, Dispatch,
    facts::Completion,
    operations::Failure,
    os::{Effect, Fact},
    policy::Timer,
};
use float_view_application_runtime::{
    load::{self, Source, Winner},
    telemetry::{FootpadState, NormalizedSample},
};
use serde::{Deserialize, Serialize};

mod battery;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Battery {
    Low,
    OverVoltage,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TestKind {
    Speed,
    Load,
    Combined,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Finished,
    Interrupted,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Readiness {
    Initializing,
    Ready,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    NoOfflineVoice,
    EngineUnavailable,
    AudioUnavailable,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lowering {
    Adjustable,
    Automatic,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioState {
    pub status: Readiness,
    pub reason: Option<Reason>,
    pub lowering_control: Lowering,
    pub voice_settings_available: bool,
}
impl Default for AudioState {
    fn default() -> Self {
        Self {
            status: Readiness::Unavailable,
            reason: Some(Reason::EngineUnavailable),
            lowering_control: Lowering::Automatic,
            voice_settings_available: false,
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Footpad {
    Left,
    Right,
    Both,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Speech {
    pub id: u64,
    pub expires_at_ms: u64,
    pub footpad: Option<Footpad>,
    pub battery: Option<Battery>,
    pub load: Option<SpokenLoad>,
    pub speed: Option<u64>,
    pub imperial: bool,
    pub volume: u64,
    pub lower_other_audio: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpokenLoad {
    pub source: Source,
    pub percent: u64,
}
struct Active {
    id: u64,
    deadline: u64,
    generation: Option<u64>,
    test: bool,
    battery: bool,
}
#[derive(Default)]
pub(crate) struct State {
    pub audio: AudioState,
    pub last_published_battery_warning: Option<Battery>,
    last_attempt: Option<u64>,
    active: Option<Active>,
    observation: Option<(u64, u64, load::Load, Option<f64>)>,
    released_at: [Option<u64>; 2],
    battery: battery::State,
}
impl State {
    pub(crate) fn reset_footpads(&mut self) {
        self.released_at = [None; 2];
    }
    pub(crate) fn invalidate(&mut self) {
        self.observation = None;
        self.reset_footpads();
        self.battery.reset();
    }
}
impl BackendService {
    pub(crate) fn observe_announcement(&mut self, sample: &NormalizedSample) {
        self.refresh_battery_context();
        // Check the previous sample before replacing its freshness evidence.
        if self.speech_values().2.is_none() {
            self.policy.announcements.reset_footpads();
            if self
                .policy
                .announcements
                .active
                .as_ref()
                .is_some_and(|a| !a.test)
            {
                self.cancel_speech(Outcome::Interrupted);
            }
        }
        if let Some(a) = &self.policy.active {
            self.policy.announcements.observation = Some((
                a.generation,
                self.policy.exchange.now,
                load::calculate(
                    sample,
                    self.runtime.snapshot().application.current_limits.as_ref(),
                ),
                sample.speed_kph,
            ));
            if sample
                .pack_voltage_v
                .is_none_or(|v| !v.is_finite() || v <= 0.)
                || matches!(
                    sample.package_state,
                    None | Some(float_view_application_runtime::telemetry::PackageState::Unknown)
                )
            {
                self.reset_battery_announcements();
            }
            self.policy.announcements.battery.observe(
                self.policy.exchange.now,
                sample.package_state
                    == Some(float_view_application_runtime::telemetry::PackageState::Running),
                sample.pack_voltage_v,
                sample.battery_current_a,
            );
            let released = match sample.footpad_state {
                Some(FootpadState::Left) => Some([false, true]),
                Some(FootpadState::Right) => Some([true, false]),
                Some(FootpadState::None) => Some([true, true]),
                Some(FootpadState::Both) => Some([false, false]),
                _ => None,
            };
            if self.policy.settings.announcements.footpad_enabled
                && self.speech_values().2.is_some()
                && sample
                    .speed_kph
                    .is_some_and(|v| v.is_finite() && v.abs() > 5. && v.abs() <= 200.)
                && let Some(released) = released
            {
                for (start, released) in self
                    .policy
                    .announcements
                    .released_at
                    .iter_mut()
                    .zip(released)
                {
                    if released {
                        start.get_or_insert(self.policy.exchange.now);
                    } else {
                        *start = None;
                    }
                }
            } else {
                self.policy.announcements.reset_footpads();
            }
        }
    }
    pub(crate) fn cancel_speech(&mut self, outcome: Outcome) {
        if let Some(a) = self.policy.announcements.active.take() {
            self.policy.exchange.cancel_speech(a.id);
            if a.test {
                self.finish_pending(a.id, Ok(Completion::Announcement { outcome }));
            }
        }
    }
    pub(crate) fn complete_speech(&mut self, id: u64, outcome: Outcome) {
        if self
            .policy
            .announcements
            .active
            .as_ref()
            .is_some_and(|a| a.id == id)
        {
            if self
                .policy
                .announcements
                .active
                .as_ref()
                .is_some_and(|a| self.policy.exchange.now >= a.deadline)
            {
                self.cancel_speech(Outcome::Unavailable);
            } else {
                self.policy.announcements.active = None;
                self.finish_pending(id, Ok(Completion::Announcement { outcome }));
            }
        }
    }
    pub(crate) fn speech_client_disconnected(&mut self, owner: u64) {
        if self
            .policy
            .announcements
            .active
            .as_ref()
            .is_some_and(|a| self.pending.get(&a.id).is_some_and(|p| p.owner == owner))
        {
            self.cancel_speech(Outcome::Interrupted);
        }
    }
    fn speech_ready(&self) -> bool {
        self.policy.announcements.last_attempt.is_none_or(|last| {
            self.policy.exchange.now.saturating_sub(last)
                >= self.policy.settings.announcements.repeat_interval_seconds * 1000
        })
    }
    fn speech_values(&self) -> (Option<Winner>, Option<f64>, Option<u64>) {
        let Some((generation, at, load, speed)) = &self.policy.announcements.observation else {
            return (None, None, None);
        };
        let runtime = self.runtime.snapshot().application;
        if self.policy.active.as_ref().is_none_or(|a| {
            a.generation != *generation
                || a.state != crate::os::LinkState::Ready
                || a.reconnect_started.is_some()
        }) || runtime.phase != float_view_application_runtime::coordinator::Phase::Ready
            || runtime.standalone_bms
            || self.policy.exchange.now.saturating_sub(*at) >= 2000
        {
            return (None, None, None);
        }
        let settings = &self.policy.settings.announcements;
        let winner = load.winner.filter(|w| {
            settings.load_enabled && w.fraction * 100. > settings.load_threshold_percent
        });
        let speed = speed
            .filter(|v| {
                v.is_finite()
                    && v.abs() <= 200.
                    && settings.speed_enabled
                    && v.abs() > settings.speed_threshold_kph
            })
            .map(f64::abs);
        (winner, speed, Some(*generation))
    }
    fn emit_speech(
        &mut self,
        id: u64,
        load: Option<Winner>,
        speed: Option<f64>,
        generation: Option<u64>,
        test: bool,
        battery: Option<Battery>,
    ) -> Result<(), Failure> {
        let settings = &self.policy.settings.announcements;
        let imperial = self.policy.settings.unit_preference == "imperial";
        self.policy.announcements.last_attempt = Some(self.policy.exchange.now);
        self.policy.exchange.push(Effect::Speak {
            speech: Speech {
                id,
                expires_at_ms: self.policy.exchange.now.saturating_add(10_000),
                footpad: if test || battery.is_some() {
                    None
                } else {
                    self.qualified_footpad()
                },
                battery,
                load: load.map(|w| SpokenLoad {
                    source: w.source,
                    percent: (w.fraction * 100.).round() as u64,
                }),
                speed: speed
                    .map(|v| (v * if imperial { 0.621371192237334 } else { 1. }).round() as u64),
                imperial,
                volume: settings.volume_percent,
                lower_other_audio: settings.lower_other_audio,
            },
        })?;
        self.policy.announcements.active = Some(Active {
            id,
            deadline: self.policy.exchange.now.saturating_add(10_000),
            generation,
            test,
            battery: battery.is_some(),
        });
        Ok(())
    }
    pub(crate) fn test_announcement(
        &mut self,
        owner: u64,
        kind: TestKind,
    ) -> Result<Dispatch, Failure> {
        self.step_announcements()?;
        // Manual previews wait only for the speech slot, not the automatic repeat interval.
        if self.policy.announcements.active.is_some() {
            return Err(Failure::InvalidState);
        }
        self.policy.announcements.last_attempt = Some(self.policy.exchange.now);
        if self.policy.settings.announcements.volume_percent == 0 {
            return Ok(Dispatch::Complete(Box::new(Completion::Announcement {
                outcome: Outcome::Finished,
            })));
        }
        if self.policy.announcements.audio.status != Readiness::Ready {
            self.policy.exchange.push(Effect::RefreshSpeech)?;
            return Ok(Dispatch::Complete(Box::new(Completion::Announcement {
                outcome: Outcome::Unavailable,
            })));
        }
        let id = self.policy.exchange.id()?;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.pending.insert(
            id,
            crate::dispatch::Pending {
                owner,
                kind: crate::dispatch::PendingKind::Speech,
                deadline: self.policy.exchange.now.saturating_add(10_000),
                sender,
            },
        );
        let admitted = self
            .emit_speech(
                id,
                (!matches!(kind, TestKind::Speed)).then_some(Winner {
                    source: Source::Motor,
                    fraction: 0.72,
                }),
                (!matches!(kind, TestKind::Load)).then_some(25.),
                None,
                true,
                None,
            )
            .and_then(|()| self.arm_timer(Timer::Speech, 100, None));
        if let Err(error) = admitted {
            self.cancel_timer(Timer::Speech);
            self.cancel_speech(Outcome::Unavailable);
            self.finish_pending(id, Err(error.clone()));
            return Err(error);
        }
        Ok(Dispatch::Pending(receiver))
    }
    fn qualified_footpad(&self) -> Option<Footpad> {
        let settings = &self.policy.settings.announcements;
        if !settings.footpad_enabled {
            return None;
        }
        let qualified = self.policy.announcements.released_at.map(|start| {
            start.is_some_and(|at| {
                self.policy.exchange.now.saturating_sub(at) >= settings.footpad_delay_seconds * 1000
            })
        });
        match qualified {
            [true, true] => Some(Footpad::Both),
            [true, false] => Some(Footpad::Left),
            [false, true] => Some(Footpad::Right),
            _ => None,
        }
    }
    pub(crate) fn refresh_battery_context(&mut self) {
        let snapshot = self.runtime.snapshot().application;
        let context = battery::resolve(&snapshot)
            .map(|(revision, limits)| (snapshot.generation.0, revision, limits));
        if self.policy.announcements.battery.context(context)
            && self
                .policy
                .announcements
                .active
                .as_ref()
                .is_some_and(|a| a.battery)
        {
            self.cancel_speech(Outcome::Interrupted);
        }
    }
    pub(crate) fn battery_warning(&self) -> Option<Battery> {
        self.speech_values().2?;
        self.policy
            .announcements
            .battery
            .warning(self.policy.exchange.now)
    }
    pub(crate) fn reset_battery_announcements(&mut self) {
        self.policy.announcements.battery.reset();
        if self
            .policy
            .announcements
            .active
            .as_ref()
            .is_some_and(|a| a.battery)
        {
            self.cancel_speech(Outcome::Interrupted);
        }
    }
    pub(crate) fn step_announcements(&mut self) -> Result<(), Failure> {
        self.refresh_battery_context();
        let (winner, speed, generation) = self.speech_values();
        let now = self.policy.exchange.now;
        if generation.is_none() {
            self.policy.announcements.reset_footpads();
            self.policy.announcements.battery.reset();
        }
        let footpad = self.qualified_footpad();
        // Threshold crossings stop future attempts, not an admitted phrase.
        // Invalid or stale board context still interrupts automatic speech.
        if let Some(a) = &self.policy.announcements.active {
            if now >= a.deadline {
                self.cancel_speech(Outcome::Unavailable);
            } else if !a.test && a.generation != generation {
                self.cancel_speech(Outcome::Interrupted);
            }
        }
        let battery = if self.policy.settings.announcements.battery_charge_enabled {
            self.policy.announcements.battery.due(now)
        } else {
            None
        };
        if let Some(battery) = battery.filter(|_| self.policy.announcements.active.is_none()) {
            self.policy.announcements.battery.attempted(battery, now);
            self.policy.announcements.last_attempt = Some(now);
            if self.policy.settings.announcements.volume_percent > 0
                && self.policy.announcements.audio.status == Readiness::Ready
                && let Ok(id) = self.policy.exchange.id()
            {
                let _ = self.emit_speech(id, None, None, generation, false, Some(battery));
            }
        }
        if (footpad.is_some() || winner.is_some() || speed.is_some()) && self.speech_ready() {
            self.policy.announcements.last_attempt = Some(now);
            if self.policy.announcements.active.is_none()
                && self.policy.settings.announcements.volume_percent > 0
                && self.policy.announcements.audio.status == Readiness::Ready
                && let Ok(id) = self.policy.exchange.id()
            {
                let _ = self.emit_speech(id, winner, speed, generation, false, None);
            }
        }
        let needed = self.policy.announcements.battery.pending()
            || self.policy.announcements.active.is_some()
            || winner.is_some()
            || speed.is_some()
            || self
                .policy
                .announcements
                .released_at
                .iter()
                .any(Option::is_some);
        if needed
            && !self.policy.timers.values().any(|t| t.kind == Timer::Speech)
            && self.arm_timer(Timer::Speech, 100, None).is_err()
        {
            self.cancel_timer(Timer::Speech);
            self.cancel_speech(Outcome::Unavailable);
        }
        Ok(())
    }
    pub(crate) fn apply_speech_fact(&mut self, fact: Fact) {
        match fact {
            Fact::SpeechReady { state } => self.policy.announcements.audio = state,
            Fact::SpeechFinished { id, outcome } => self.complete_speech(id, outcome),
            _ => {}
        }
    }
}
