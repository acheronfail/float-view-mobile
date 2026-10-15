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
    telemetry::NormalizedSample,
};
use serde::{Deserialize, Serialize};

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
}
impl Default for AudioState {
    fn default() -> Self {
        Self {
            status: Readiness::Unavailable,
            reason: Some(Reason::EngineUnavailable),
            lowering_control: Lowering::Automatic,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Speech {
    pub id: u64,
    pub expires_at_ms: u64,
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
}
#[derive(Default)]
pub(crate) struct State {
    pub audio: AudioState,
    last_attempt: Option<u64>,
    active: Option<Active>,
    observation: Option<(u64, u64, load::Load, Option<f64>)>,
}
impl State {
    pub(crate) fn invalidate(&mut self) {
        self.observation = None;
    }
}
impl BackendService {
    pub(crate) fn observe_announcement(&mut self, sample: &NormalizedSample) {
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
    ) -> Result<(), Failure> {
        let settings = &self.policy.settings.announcements;
        let imperial = self.policy.settings.unit_preference == "imperial";
        self.policy.announcements.last_attempt = Some(self.policy.exchange.now);
        self.policy.exchange.push(Effect::Speak {
            speech: Speech {
                id,
                expires_at_ms: self.policy.exchange.now.saturating_add(10_000),
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
        });
        Ok(())
    }
    pub(crate) fn test_announcement(
        &mut self,
        owner: u64,
        kind: TestKind,
    ) -> Result<Dispatch, Failure> {
        self.step_announcements()?;
        if !self.speech_ready() || self.policy.announcements.active.is_some() {
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
    pub(crate) fn step_announcements(&mut self) -> Result<(), Failure> {
        let (winner, speed, generation) = self.speech_values();
        let now = self.policy.exchange.now;
        if let Some(a) = &self.policy.announcements.active {
            if now >= a.deadline {
                self.cancel_speech(Outcome::Unavailable);
            } else if !a.test
                && (a.generation != generation || (winner.is_none() && speed.is_none()))
            {
                self.cancel_speech(Outcome::Interrupted);
            }
        }
        if (winner.is_some() || speed.is_some()) && self.speech_ready() {
            self.policy.announcements.last_attempt = Some(now);
            if self.policy.announcements.active.is_none()
                && self.policy.settings.announcements.volume_percent > 0
                && self.policy.announcements.audio.status == Readiness::Ready
                && let Ok(id) = self.policy.exchange.id()
            {
                let _ = self.emit_speech(id, winner, speed, generation, false);
            }
        }
        let needed =
            self.policy.announcements.active.is_some() || winner.is_some() || speed.is_some();
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
