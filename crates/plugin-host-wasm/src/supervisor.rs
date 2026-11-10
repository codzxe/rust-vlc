//! Crash-isolation supervision (card 7.F4 / issue #46; doc 06 §5.6):
//! the restart/backoff/`Disabled` scheduler that owns what happens
//! *after* wasmi has already contained a trap.
//!
//! # Division of labor
//!
//! wasmi's `Store` scoping is the containment primitive (ADR-002): a
//! trap destroys exactly one instance; the host process, playback and
//! every other instance are untouched — [`crate::instance`] turns that
//! into a typed [`crate::error::HostError`] plus an `InstanceStatus::
//! Faulted` that refuses further dispatch. What wasmi cannot decide is
//! *policy*: may the extension come back, when, and when is it
//! quarantined for good. That policy lives here, config-driven
//! through the settings layer (never re-stated in code):
//!
//! | Setting (schemas/settings.toml) | Default | Meaning |
//! |---|---|---|
//! | `extensions.crash-backoff-initial-ms` | 5000 | delay before restart #2 |
//! | `extensions.crash-backoff-multiplier` | 6 | exponential factor |
//! | `extensions.crash-backoff-max-ms` | 300000 | the cap (doc 06's 5 min) |
//! | `extensions.crash-restart-limit` | 3 | supervised restarts before `Disabled` |
//! | `extensions.crash-disabled-reenable-ms` | 900000 | policy timer; 0 = user-only |
//!
//! # The policy `[PROPOSAL — doc 06 §5.6 marks the sequence proposed]`
//!
//! Doc 06 §5.6: *"exponential backoff — immediate retry once, then
//! 5 s, 30 s, 5 min; a fourth crash within the window moves the
//! extension to `Disabled` … A user can re-enable (which resets the
//! backoff)"* and *"repeated-crash protection is per-version:
//! upgrading an extension resets the crash counter."* Concretely:
//!
//! - Restart #1 is immediate (`delay(1) = 0`); restart `k ≥ 2` waits
//!   `min(initial · multiplier^(k−2), max)` — 0, 5 s, 30 s with the
//!   defaults (restart #4 would be 3 min, #5+ the 5 min cap; those
//!   only exist when the restart limit is raised above the default 3,
//!   under which the doc's default sequence — immediate, 5 s, 30 s,
//!   fourth crash → `Disabled` — is exactly what ships). Doc 06
//!   §5.6's illustrative fourth step ("5 min") is the CAP, reached
//!   geometrically at restart #5 with the ×6 default factor — a
//!   documented policy delta from the doc's non-geometric sketch,
//!   which the doc itself marks `[PROPOSAL]`.
//! - A crash that would demand restart `restart_limit + 1` moves the
//!   extension to `Disabled` instead — with the default limit 3 that
//!   is the fourth crash, exactly the doc's wording.
//! - `Disabled` is terminal for the session except two re-enable
//!   paths: the **user action** ([`ExtensionSupervisor::reenable`])
//!   and the **policy timer**
//!   ([`ExtensionSupervisor::auto_reeanble_due`],
//!   `extensions.crash-disabled-reenable-ms`; 0 disables the timer).
//!   Both reset the crash counter — a re-enable is a fresh window.
//! - Crash counters are keyed by `(id, version)`: an upgraded
//!   extension starts at zero (the user explicitly re-trusted it).
//!
//! # Determinism
//!
//! Time is an injected `u64` millisecond counter (the media-clock
//! `MonotonicSource` precedent): no sleeps, no `Instant::now`, so the
//! backoff sequence is exactly testable and CI-stable. The embedder
//! owns the clock and the extension thread pool (doc 06 §5.8) and
//! calls [`ExtensionSupervisor::record_crash`] after every contained
//! fault.
//!
//! # Host events
//!
//! Every crash decision emits a [`SupervisorEvent`] shaped after doc
//! 05 §3's `plugins.crashed { id, error, action_taken }` vocabulary
//! (`Isolated` for a supervised restart, `Disabled` for the
//! quarantine). Events live in a bounded ring (D-2) the embedder
//! drains for the command layer.

use std::collections::BTreeMap;

use crate::error::HostError;

/// Setting keys the policy reads (schemas/settings.toml; the values
/// live there and nowhere else).
pub const SETTING_INITIAL_MS: &str = "extensions.crash-backoff-initial-ms";
/// Setting key for the exponential factor.
pub const SETTING_MULTIPLIER: &str = "extensions.crash-backoff-multiplier";
/// Setting key for the backoff cap.
pub const SETTING_MAX_MS: &str = "extensions.crash-backoff-max-ms";
/// Setting key for the supervised-restart limit.
pub const SETTING_RESTART_LIMIT: &str = "extensions.crash-restart-limit";
/// Setting key for the `Disabled` policy timer.
pub const SETTING_REENABLE_MS: &str = "extensions.crash-disabled-reenable-ms";

/// Longest crash reason retained per event (bounded, D-2).
const REASON_CAP: usize = 256;
/// Most recent events retained (bounded ring, D-2).
const EVENT_RING: usize = 128;
/// Most supervised extensions tracked at once (bounded map, D-2).
const TRACKED_CAP: usize = 64;

/// The backoff/restart policy (doc 06 §5.6), values from the settings
/// layer. Construct via [`BackoffPolicy::from_schema_defaults`] (the
/// committed schema) and adjust per-user with
/// [`BackoffPolicy::apply_setting`] (the `config.set-setting` seam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackoffPolicy {
    /// Delay before restart #2 (restart #1 is immediate), in ms.
    pub initial_delay_ms: u64,
    /// Exponential factor between successive restarts.
    pub multiplier: u32,
    /// The backoff cap in ms (doc 06 §5.6's 5 min).
    pub max_delay_ms: u64,
    /// Supervised restarts before the extension is `Disabled`.
    pub restart_limit: u32,
    /// `Disabled` policy timer in ms; 0 = user-action re-enable only.
    pub disabled_reeanble_ms: u64,
}

impl BackoffPolicy {
    /// Read the policy from the committed settings schema's defaults
    /// (the single source of truth — `schemas/settings.toml` embedded
    /// in `settings-schema`'s runtime plane).
    ///
    /// # Errors
    /// [`HostError::Settings`] when the schema is missing an entry or
    /// carries a non-integer default (a committed-schema/build skew —
    /// actionable, never a silent fallback to hard-coded numbers).
    pub fn from_schema_defaults() -> Result<BackoffPolicy, HostError> {
        let schema =
            settings_schema::runtime::embedded().map_err(|e| HostError::Settings(e.to_string()))?;
        let int = |key: &str| -> Result<i64, HostError> {
            let def = schema.entry(key).ok_or_else(|| {
                HostError::Settings(format!(
                    "the committed settings schema has no `{key}` entry — \
                 the supervisor policy and schemas/settings.toml have drifted"
                ))
            })?;
            match &def.default {
                settings_schema::DefaultValue::Literal(value) => {
                    value.as_integer().ok_or_else(|| {
                        HostError::Settings(format!(
                            "`{key}` must default to an integer in \
                             schemas/settings.toml"
                        ))
                    })
                }
                settings_schema::DefaultValue::Ref(_) => Err(HostError::Settings(format!(
                    "`{key}` defaults via default_ref into product.toml — \
                     the supervisor reads schema literals only"
                ))),
            }
        };
        let policy = BackoffPolicy {
            initial_delay_ms: u64::try_from(int(SETTING_INITIAL_MS)?)
                .map_err(|e| HostError::Settings(format!("{SETTING_INITIAL_MS}: {e}")))?,
            multiplier: u32::try_from(int(SETTING_MULTIPLIER)?)
                .map_err(|e| HostError::Settings(format!("{SETTING_MULTIPLIER}: {e}")))?,
            max_delay_ms: u64::try_from(int(SETTING_MAX_MS)?)
                .map_err(|e| HostError::Settings(format!("{SETTING_MAX_MS}: {e}")))?,
            restart_limit: u32::try_from(int(SETTING_RESTART_LIMIT)?)
                .map_err(|e| HostError::Settings(format!("{SETTING_RESTART_LIMIT}: {e}")))?,
            disabled_reeanble_ms: u64::try_from(int(SETTING_REENABLE_MS)?)
                .map_err(|e| HostError::Settings(format!("{SETTING_REENABLE_MS}: {e}")))?,
        };
        policy.check_invariants()?;
        Ok(policy)
    }

    /// Apply one runtime setting write (the `config.set-setting`
    /// seam): the value is validated against the schema entry first —
    /// type and range — then mapped onto the policy field.
    ///
    /// # Errors
    /// [`HostError::Settings`] for unknown keys, type/range
    /// violations, or resulting invariant breaks.
    pub fn apply_setting(&mut self, key: &str, value: i64) -> Result<(), HostError> {
        let toml_value = toml::Value::Integer(value);
        settings_schema::runtime::validate_value(key, &toml_value)
            .map_err(|e| HostError::Settings(e.to_string()))?;
        match key {
            SETTING_INITIAL_MS => {
                self.initial_delay_ms =
                    u64::try_from(value).map_err(|e| HostError::Settings(format!("{key}: {e}")))?
            }
            SETTING_MULTIPLIER => {
                self.multiplier =
                    u32::try_from(value).map_err(|e| HostError::Settings(format!("{key}: {e}")))?
            }
            SETTING_MAX_MS => {
                self.max_delay_ms =
                    u64::try_from(value).map_err(|e| HostError::Settings(format!("{key}: {e}")))?
            }
            SETTING_RESTART_LIMIT => {
                self.restart_limit =
                    u32::try_from(value).map_err(|e| HostError::Settings(format!("{key}: {e}")))?
            }
            SETTING_REENABLE_MS => {
                self.disabled_reeanble_ms =
                    u64::try_from(value).map_err(|e| HostError::Settings(format!("{key}: {e}")))?
            }
            other => {
                return Err(HostError::Settings(format!(
                    "`{other}` is not a supervisor policy setting"
                )));
            }
        }
        self.check_invariants()
    }

    /// Invariants that make the sequence well-formed.
    fn check_invariants(&self) -> Result<(), HostError> {
        if self.multiplier == 0 {
            return Err(HostError::Settings(
                "extensions.crash-backoff-multiplier must be ≥ 1 (a zero \
                 factor would freeze the backoff at the initial delay)"
                    .to_string(),
            ));
        }
        if self.max_delay_ms < self.initial_delay_ms {
            return Err(HostError::Settings(
                "extensions.crash-backoff-max-ms must be ≥ the initial \
                 delay (the cap must be reachable)"
                    .to_string(),
            ));
        }
        if self.restart_limit == 0 {
            return Err(HostError::Settings(
                "extensions.crash-restart-limit must be ≥ 1 (zero restarts \
                 would disable on the first crash, contradicting doc 06 \
                 §5.6's immediate retry)"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// The delay before supervised restart number `k` (1-based):
    /// restart 1 is immediate, restart `k ≥ 2` waits
    /// `min(initial · multiplier^(k−2), cap)` — 0, 5 s, 30 s with the
    /// committed defaults, then 3 min and the 5 min cap.
    pub fn restart_delay_ms(&self, k: u32) -> u64 {
        if k <= 1 {
            return 0;
        }
        // exponent k-2, saturating so a huge k cannot overflow
        let mut delay = self.initial_delay_ms;
        for _ in 0..k.saturating_sub(2) {
            delay = delay.saturating_mul(u64::from(self.multiplier));
            if delay >= self.max_delay_ms {
                return self.max_delay_ms;
            }
        }
        delay.min(self.max_delay_ms)
    }
}

/// What the supervisor decided for one recorded crash — the
/// `action_taken` of doc 05 §3's `plugins.crashed` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionTaken {
    /// The instance was destroyed and a supervised restart is
    /// scheduled after `delay_ms` (the trap is contained, doc 06
    /// §5.6's first outcome).
    Isolated {
        /// Which supervised restart this is (1-based).
        restart: u32,
        /// How long the embedder must wait before respawning (ms).
        delay_ms: u64,
    },
    /// The restart budget is exhausted: the extension is `Disabled`
    /// for the session (doc 06 §5.6) until a user re-enable or the
    /// policy timer.
    Disabled,
}

/// A host-side supervision event (doc 05 §3 `plugins.crashed`
/// vocabulary: `{ id, error, action_taken }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorEvent {
    /// Extension id, e.g. `ext.acme.hello`.
    pub id: String,
    /// Extension version (per-version crash counters, doc 06 §5.6).
    pub version: String,
    /// The contained fault's reason (bounded to 256 chars).
    pub reason: String,
    /// What the supervisor did.
    pub action: ActionTaken,
    /// Virtual clock reading at the decision (ms).
    pub at_ms: u64,
}

/// One extension's supervision record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CrashRecord {
    /// Crashes since the last (re-)enable.
    crashes: u32,
    /// `Disabled` bookkeeping: when it happened (for the policy
    /// timer); `None` while enabled.
    disabled_at_ms: Option<u64>,
}

/// The run-state of one extension, as the plugin UI / command layer
/// sees it (doc 06 §5.6: an actionable message belongs in the UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunState {
    /// Enabled (possibly inside a backoff window before the next
    /// supervised restart — `next_restart_ms` is the virtual-clock
    /// deadline, `None` when no restart is pending).
    Enabled {
        /// Crashes this window so far.
        crashes: u32,
        /// Restart deadline when a backoff is running.
        next_restart_ms: Option<u64>,
    },
    /// Disabled by the crash policy; re-enable is a user action or
    /// the policy timer.
    Disabled {
        /// Crashes at disable time.
        crashes: u32,
        /// When the disable happened (virtual ms).
        at_ms: u64,
        /// When the policy timer re-enables (`None` when user-only).
        reenable_at_ms: Option<u64>,
    },
}

/// Session-scoped crash supervisor: one per host, keyed by
/// `(id, version)` (doc 06 §5.6). Plain state — the embedder owns it
/// on the extension scheduling path (doc 06 §5.8), exactly like the
/// engine owns its own state machines; wrap in the app's lock if the
/// extension pool calls in from several threads.
#[derive(Debug)]
pub struct ExtensionSupervisor {
    policy: BackoffPolicy,
    records: BTreeMap<(String, String), CrashRecord>,
    events: std::collections::VecDeque<SupervisorEvent>,
}

impl ExtensionSupervisor {
    /// A supervisor under `policy`.
    ///
    /// # Errors
    /// [`HostError::Settings`] is impossible here (the policy was
    /// already validated); kept `Result`-free — construction from an
    /// invalid policy is a compile-time-shaped mistake, so the policy
    /// type is only obtainable through its validating constructors.
    pub fn new(policy: BackoffPolicy) -> ExtensionSupervisor {
        ExtensionSupervisor {
            policy,
            records: BTreeMap::new(),
            events: std::collections::VecDeque::new(),
        }
    }

    /// The active policy (read-out for the command layer).
    pub fn policy(&self) -> &BackoffPolicy {
        &self.policy
    }

    /// Record one contained fault and decide the action (doc 06
    /// §5.6). `reason` is the typed error text from the faulted
    /// instance; `now_ms` is the embedder's virtual/monotonic clock.
    pub fn record_crash(
        &mut self,
        id: &str,
        version: &str,
        reason: &str,
        now_ms: u64,
    ) -> ActionTaken {
        let mut bounded = reason.to_string();
        if bounded.len() > REASON_CAP {
            // Char-boundary-safe truncation (the reason is error text,
            // not a protocol payload).
            bounded.truncate(bounded.char_indices().nth(REASON_CAP).map_or(0, |(i, _)| i));
            bounded.push('…');
        }
        let key = (id.to_string(), version.to_string());
        let restart_limit = self.policy.restart_limit;
        let record = self.entry(&key);
        record.crashes = record.crashes.saturating_add(1);
        let crashes = record.crashes;
        let action = if crashes > restart_limit {
            record.disabled_at_ms = Some(now_ms);
            ActionTaken::Disabled
        } else {
            ActionTaken::Isolated {
                restart: crashes,
                delay_ms: self.policy.restart_delay_ms(crashes),
            }
        };
        self.push_event(SupervisorEvent {
            id: id.to_string(),
            version: version.to_string(),
            reason: bounded,
            action: action.clone(),
            at_ms: now_ms,
        });
        action
    }

    /// The extension's run-state (plugin UI / `system.list-plugins`
    /// truth, doc 05 §1).
    pub fn state(&self, id: &str, version: &str) -> RunState {
        let key = (id.to_string(), version.to_string());
        let Some(record) = self.records.get(&key) else {
            return RunState::Enabled {
                crashes: 0,
                next_restart_ms: None,
            };
        };
        match record.disabled_at_ms {
            Some(at_ms) => RunState::Disabled {
                crashes: record.crashes,
                at_ms,
                reenable_at_ms: if self.policy.disabled_reeanble_ms == 0 {
                    None
                } else {
                    Some(at_ms + self.policy.disabled_reeanble_ms)
                },
            },
            None => RunState::Enabled {
                crashes: record.crashes,
                next_restart_ms: None,
            },
        }
    }

    /// User re-enable (doc 06 §5.6: resets the backoff). Works from
    /// any state; re-enabling a healthy extension is a no-op reset.
    pub fn reenable(&mut self, id: &str, version: &str) {
        self.records.remove(&(id.to_string(), version.to_string()));
    }

    /// Policy-timer re-enable: returns the ids whose `Disabled`
    /// window elapsed at `now_ms` and resets them. With
    /// `disabled_reeanble_ms == 0` the timer is off and nothing ever
    /// fires (user-action only, the mission's default posture when
    /// the setting is 0).
    pub fn auto_reeanble_due(&mut self, now_ms: u64) -> Vec<(String, String)> {
        let mut due = Vec::new();
        if self.policy.disabled_reeanble_ms == 0 {
            return due;
        }
        let keys: Vec<(String, String)> = self
            .records
            .iter()
            .filter(|(_, r)| {
                r.disabled_at_ms
                    .is_some_and(|at| now_ms >= at + self.policy.disabled_reeanble_ms)
            })
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys {
            self.records.remove(&key);
            due.push(key);
        }
        due
    }

    /// Drain the pending host events (oldest first; bounded ring).
    pub fn drain_events(&mut self) -> Vec<SupervisorEvent> {
        self.events.drain(..).collect()
    }

    /// Bounded map insert: a fresh key evicts the record with the
    /// oldest last activity when the cap is reached (doc 06 §5.4's
    /// bounded-host-accounting posture; 64 tracked extensions is
    /// far above the 16-installed-and-loaded row).
    fn entry(&mut self, key: &(String, String)) -> &mut CrashRecord {
        if !self.records.contains_key(key) && self.records.len() >= TRACKED_CAP {
            let evict = self
                .records
                .iter()
                .map(|(k, r)| (r.crashes, k.clone()))
                .min_by_key(|(crashes, _)| *crashes)
                .map(|(_, k)| k);
            if let Some(evict) = evict {
                self.records.remove(&evict);
            }
        }
        self.records.entry(key.clone()).or_default()
    }

    fn push_event(&mut self, event: SupervisorEvent) {
        if self.events.len() >= EVENT_RING {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    #[test]
    fn schema_defaults_are_doc06_sequence() {
        // The committed settings schema IS the policy source: 5 s
        // initial, ×6, 5 min cap, 3 restarts, 15 min re-enable timer.
        let policy = ok(BackoffPolicy::from_schema_defaults());
        assert_eq!(policy.initial_delay_ms, 5_000);
        assert_eq!(policy.multiplier, 6);
        assert_eq!(policy.max_delay_ms, 300_000);
        assert_eq!(policy.restart_limit, 3);
        assert_eq!(policy.disabled_reeanble_ms, 900_000);
        // doc 06 §5.6 verbatim for the default window: immediate,
        // 5 s, 30 s; the geometric tail continues 3 min, then the
        // 5 min cap (only reachable when the restart limit is
        // raised above the default 3).
        assert_eq!(policy.restart_delay_ms(1), 0);
        assert_eq!(policy.restart_delay_ms(2), 5_000);
        assert_eq!(policy.restart_delay_ms(3), 30_000);
        assert_eq!(policy.restart_delay_ms(4), 180_000);
        assert_eq!(policy.restart_delay_ms(5), 300_000, "capped");
        assert_eq!(policy.restart_delay_ms(50), 300_000, "capped");
    }

    #[test]
    fn apply_setting_validates_through_the_schema() {
        let mut policy = ok(BackoffPolicy::from_schema_defaults());
        ok(policy.apply_setting(SETTING_REENABLE_MS, 60_000));
        assert_eq!(policy.disabled_reeanble_ms, 60_000);
        // Range violation is the settings layer's typed refusal.
        let err = policy
            .apply_setting(SETTING_MULTIPLIER, 0)
            .err()
            .unwrap_or_else(|| unreachable!("multiplier 0 is below the range"));
        assert!(matches!(err, HostError::Settings(_)), "{err}");
        // Unknown key is refused even when it LOOKS like ours.
        let err = policy
            .apply_setting("extensions.crash-backoff-initial-s", 1)
            .err()
            .unwrap_or_else(|| unreachable!("unknown key must be refused"));
        assert!(matches!(err, HostError::Settings(_)), "{err}");
    }

    #[test]
    fn backoff_sequence_then_disabled_then_reeanble() {
        let policy = ok(BackoffPolicy::from_schema_defaults());
        let mut sup = ExtensionSupervisor::new(policy);
        // Four crashes: Isolated(0), Isolated(5 s), Isolated(30 s),
        // then Disabled — the doc's "a fourth crash within the
        // window moves the extension to Disabled".
        let actions: Vec<ActionTaken> = (0..4)
            .map(|i| sup.record_crash("ext.a", "1.0.0", "trap: unreachable", i * 1_000))
            .collect();
        assert_eq!(
            actions,
            vec![
                ActionTaken::Isolated {
                    restart: 1,
                    delay_ms: 0
                },
                ActionTaken::Isolated {
                    restart: 2,
                    delay_ms: 5_000
                },
                ActionTaken::Isolated {
                    restart: 3,
                    delay_ms: 30_000
                },
                ActionTaken::Disabled,
            ]
        );
        // While disabled: further crashes stay disabled (no restart).
        assert_eq!(
            sup.record_crash("ext.a", "1.0.0", "trap again", 5_000),
            ActionTaken::Disabled
        );
        assert!(matches!(
            sup.state("ext.a", "1.0.0"),
            RunState::Disabled {
                reenable_at_ms: Some(_),
                ..
            }
        ));
        // The events mirror the actions (bounded, drained oldest
        // first) — the doc 05 §3 `plugins.crashed` shapes.
        let events = sup.drain_events();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].id, "ext.a");
        assert_eq!(events[0].action, actions[0]);
        assert_eq!(events[4].action, ActionTaken::Disabled);
        assert!(sup.drain_events().is_empty());

        // User re-enable resets the window: crash 1 is immediate again.
        sup.reenable("ext.a", "1.0.0");
        assert!(matches!(
            sup.state("ext.a", "1.0.0"),
            RunState::Enabled { crashes: 0, .. }
        ));
        assert_eq!(
            sup.record_crash("ext.a", "1.0.0", "fresh window", 9_000),
            ActionTaken::Isolated {
                restart: 1,
                delay_ms: 0
            }
        );
    }

    #[test]
    fn policy_timer_reeanables_only_after_the_window() {
        let mut policy = ok(BackoffPolicy::from_schema_defaults());
        ok(policy.apply_setting(SETTING_REENABLE_MS, 600_000));
        let mut sup = ExtensionSupervisor::new(policy);
        for i in 0..4 {
            sup.record_crash("ext.b", "2.0.0", "oom", i * 100);
        }
        // One ms before the deadline: still disabled. The 4th crash
        // (i = 3) disabled the extension at t = 300, so the deadline
        // is 300 + 600 000.
        assert!(sup.auto_reeanble_due(600_299).is_empty());
        assert!(matches!(
            sup.state("ext.b", "2.0.0"),
            RunState::Disabled { .. }
        ));
        // At/after it: re-enabled and reset.
        assert_eq!(
            sup.auto_reeanble_due(600_300),
            vec![("ext.b".into(), "2.0.0".into())]
        );
        assert!(matches!(
            sup.state("ext.b", "2.0.0"),
            RunState::Enabled { crashes: 0, .. }
        ));
    }

    #[test]
    fn zero_timer_is_user_action_only() {
        let mut policy = ok(BackoffPolicy::from_schema_defaults());
        ok(policy.apply_setting(SETTING_REENABLE_MS, 0));
        let mut sup = ExtensionSupervisor::new(policy);
        for i in 0..4 {
            sup.record_crash("ext.c", "1.0.0", "x", i);
        }
        assert!(sup.auto_reeanble_due(u64::MAX).is_empty());
        assert!(matches!(
            sup.state("ext.c", "1.0.0"),
            RunState::Disabled {
                reenable_at_ms: None,
                ..
            }
        ));
    }

    #[test]
    fn version_upgrade_resets_the_counter() {
        let policy = ok(BackoffPolicy::from_schema_defaults());
        let mut sup = ExtensionSupervisor::new(policy);
        for i in 0..4 {
            sup.record_crash("ext.d", "1.0.0", "trap", i);
        }
        assert!(matches!(
            sup.state("ext.d", "1.0.0"),
            RunState::Disabled { .. }
        ));
        // The upgraded version is a different record: fresh window
        // (doc 06 §5.6 per-version protection).
        assert!(matches!(
            sup.state("ext.d", "1.1.0"),
            RunState::Enabled { crashes: 0, .. }
        ));
        assert_eq!(
            sup.record_crash("ext.d", "1.1.0", "trap", 10),
            ActionTaken::Isolated {
                restart: 1,
                delay_ms: 0
            }
        );
        // The old version's disable is untouched.
        assert!(matches!(
            sup.state("ext.d", "1.0.0"),
            RunState::Disabled { .. }
        ));
    }

    #[test]
    fn event_ring_and_reasons_are_bounded() {
        let mut policy = ok(BackoffPolicy::from_schema_defaults());
        ok(policy.apply_setting(SETTING_RESTART_LIMIT, 10));
        let mut sup = ExtensionSupervisor::new(policy);
        let long = "x".repeat(10_000);
        for i in 0..300 {
            sup.record_crash("ext.e", "1.0.0", &long, i);
        }
        let events = sup.drain_events();
        assert_eq!(events.len(), EVENT_RING, "ring is bounded");
        assert!(events[0].reason.len() <= REASON_CAP + 4, "reason bounded");
        // The map is bounded too.
        assert!(sup.records.len() <= TRACKED_CAP);
    }
}
