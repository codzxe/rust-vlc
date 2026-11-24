# AUDIT: crates/plugin-host-wasm/src/supervisor.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Lines / functions | 693 / 22 (15 prod items, 7 unit tests — `rg -c '#\[test\]'` = 7) |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point loads it) |
| Reachable from shipped entry point? | NO — `plugin-host-wasm` is a dependency of no other crate (rg over all Cargo.toml); lumen-cli release build compiles `plugin-api` but never this crate (baseline cargo-build-release-lumen-cli.txt:140); only callers are this crate's own unit/integration tests |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | `sed -n` quote verification; `rg -n 'ExtensionSupervisor|BackoffPolicy'` (self+tests only); `rg -n 'extensions\.crash' schemas/settings.toml` (5 keys, defaults 5000/6/300000/3/900000); `rg -c '#[test]'` (= 7); baseline cite cargo-test-workspace.txt:4935-4941 (7/7 supervisor unit tests ok); python3 truncation semantics demo |

## 1. One-paragraph verdict
A well-built, honestly-tested pure state machine that nothing ships. The backoff math is correct and schema-driven (defaults verified against schemas/settings.toml), the unit tests are exact (sequence, timer boundary, per-version reset, bounds), and the bounded-ring/map discipline is real. But the supervisor never enforces its own policy: it tracks no restart deadline (`next_restart_ms` is a dead always-`None` field), so the "5 s / 30 s" backoff is advisory data an embedder must choose to obey — and no embedder exists anywhere in the repo. The `plugins.crashed` events it emits are admitted unwired by the repo's own ignored contract test. Add a reason-truncation bug that wipes multi-byte error text and an eviction comment that describes a different algorithm than the code runs.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/cargo-test-workspace.txt:4935-4941` -> 7/7 `supervisor::tests` ok (apply_setting_validates_through_the_schema, backoff_sequence_then_disabled_then_reeanble, event_ring_and_reasons_are_bounded, policy_timer_reeanables_only_after_the_window, schema_defaults_are_doc06_sequence, version_upgrade_resets_the_counter, zero_timer_is_user_action_only). Schema defaults cross-checked: `sed -n '1409,1469p' schemas/settings.toml` -> 5000 / 6 / 300000 / 3 / 900000, matching `from_schema_defaults` expectations and every test assertion.

## 3. Findings
F-001 [S2]
Where: crates/plugin-host-wasm/src/supervisor.rs:412-419
Code:
```
            None => RunState::Enabled {
                crashes: record.crashes,
                next_restart_ms: None,
            },
```
What is wrong: `RunState::Enabled { next_restart_ms }` is documented as "Restart deadline when a backoff is running" but the supervisor NEVER sets it to `Some` — `CrashRecord` stores no crash timestamp, so the supervisor cannot know when the last crash happened or when a restart is legal. The backoff delays exist only as numbers on an already-emitted event. Why it matters: the entire timing half of doc 06 §5.6 ("5 s, 30 s") is unenforceable; any embedder that polls `state()` (the documented pattern — the crash-isolation test's `supervised_dispatch` does exactly this) will respawn immediately every time. The disable counter half works; the backoff half is decoration. What correct looks like: store `last_crash_ms` in `CrashRecord`, compute `next_restart_ms = last_crash_ms + restart_delay_ms(crashes)`, and have `state()` report it (and `record_crash` reset the window when the deadline passed). Confidence: HIGH — Verification: read.
F-002 [S2]
Where: crates/plugin-host-wasm/src/supervisor.rs:326-341 (the type nobody constructs)
Code:
```
pub struct ExtensionSupervisor {
    policy: BackoffPolicy,
    records: BTreeMap<(String, String), CrashRecord>,
    events: std::collections::VecDeque<SupervisorEvent>,
}
```
What is wrong: UNREACHABLE from any shipped entry point. No crate's Cargo.toml depends on plugin-host-wasm; the release lane for lumen-cli compiles plugin-api only; `ExtensionSupervisor`/`BackoffPolicy` constructors appear nowhere outside this crate and its tests. The five `extensions.crash-*` settings are read by this code alone — the generated UI schema (ui/src/screens/settings/generated/schema.ts:2147-2242) and CLI flag metadata advertise them, but no shipped binary contains a reader. The supervisor's `plugins.crashed` events are consumed by no command layer (crates/playback-engine/tests/events_contract.rs:1727, baseline line 4234: "gap: plugins.crashed not wired — handoff H-10"). Why it matters: settings that appear configurable but affect nothing in shipped builds; a supervisor that supervises nothing. What correct looks like: wire the host into an app or mark the settings `ui=false`+hidden until then. Confidence: HIGH — Verification: read + rg + baseline.
F-003 [S3]
Where: crates/plugin-host-wasm/src/supervisor.rs:357-364
Code:
```
        let mut bounded = reason.to_string();
        if bounded.len() > REASON_CAP {
            // Char-boundary-safe truncation (the reason is error text,
            // not a protocol payload).
            bounded.truncate(bounded.char_indices().nth(REASON_CAP).map_or(0, |(i, _)| i));
            bounded.push('…');
```
What is wrong: The truncation wipes multi-byte reasons entirely. If `reason.len() > 256` bytes but the string has ≤ 256 chars (e.g. 250 Cyrillic chars = 750 bytes), `char_indices().nth(256)` is `None`, `map_or(0, …)` yields 0, and `truncate(0)` empties the string — the event's reason becomes just `"…"`, losing the fault text completely instead of keeping 256 chars. Verified semantics by python3 emulation (250×3-byte chars -> `truncate at byte 0 -> reason becomes '…'`). The unit test only exercises ASCII (`"x".repeat(10_000)`), so the bug is invisible to it. Currently latent (HostError Display strings in src/error.rs contain no non-ASCII — rg found none), but the API accepts arbitrary `&str`. What correct looks like: `bounded.char_indices().nth(REASON_CAP).map_or(bounded.len(), |(i,_)| i)`. Confidence: HIGH — Verification: read + python3 emulation (not executed against the Rust code: UNVERIFIED at runtime).
F-004 [S3]
Where: crates/plugin-host-wasm/src/supervisor.rs:456-474
Code:
```
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
```
What is wrong: The comment says eviction picks "the record with the oldest last activity"; no last-activity timestamp exists anywhere in `CrashRecord` — the code evicts the record with the FEWEST crashes. The comment describes an algorithm that was never written. Behavior consequence: evicting a live record silently forgets its crash history — an extension with 1-3 recorded crashes gets its backoff window and restart budget reset by unrelated churn (65th extension registers). Disabled records (crashes ≥ 4) are the last to be evicted, which is the accidental saving grace. Why it matters: an inaccurate invariant comment on a policy-eviction path is exactly where future edits go wrong. What correct looks like: either implement last-activity (add `last_crash_ms`, evict max) or fix the comment to "fewest crashes (closest to clean)". Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/plugin-host-wasm/src/supervisor.rs:433 (and 108)
Code:
```
    pub fn auto_reeanble_due(&mut self, now_ms: u64) -> Vec<(String, String)> {
```
What is wrong: Public API misspells "reenable" as "reeanble" (`auto_reeanble_due`, field `disabled_reeanble_ms`, test `policy_timer_reeanables_only_after_the_window`) while the setting key `extensions.crash-disabled-reenable-ms` and `SETTING_REENABLE_MS` are spelled correctly — two spellings of the same concept in one file. Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/plugin-host-wasm/src/supervisor.rs:372-391
Code:
```
        record.crashes = record.crashes.saturating_add(1);
        let crashes = record.crashes;
        let action = if crashes > restart_limit {
            record.disabled_at_ms = Some(now_ms);
            ActionTaken::Disabled
```
What is wrong: Recording a crash for an already-`Disabled` extension keeps incrementing `crashes` and emits ANOTHER `Disabled` event each time (asserted by the unit test: 5 crashes -> 5 events, 2 of them `Disabled`). An embedder that keeps dispatching a dead extension spams the 128-slot ring, displacing other extensions' events. Bounded (D-2 holds) but redundant. Confidence: HIGH — Verification: read (unit test documents the behavior).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SETTING_* consts | 88-94 | REAL | — | match schema keys verbatim |
| BackoffPolicy (struct) | 99-110 | REAL | S4 | `disabled_reeanble_ms` typo (F-005) |
| BackoffPolicy::from_schema_defaults | 121-160 | REAL | — | embedded schema, typed errors on drift; no silent fallback — good |
| BackoffPolicy::apply_setting | 168-201 | REAL | — | validates through schema first; unknown key refused |
| BackoffPolicy::check_invariants | 204-224 | REAL | — | multiplier≥1, max≥initial, limit≥1 |
| BackoffPolicy::restart_delay_ms | 237-249 | REAL | — | 0/5k/30k/180k/300k-cap verified by unit test; saturating mul |
| ActionTaken / SupervisorEvent / CrashRecord / RunState | 254-329 | REAL | S2 | `next_restart_ms` dead (F-001) |
| ExtensionSupervisor::new / policy | 340-351 | REAL | — | trivial |
| ExtensionSupervisor::record_crash | 355-392 | REAL | S3/S4 | truncation bug (F-003); repeat-Disabled events (F-006) |
| ExtensionSupervisor::state | 395-420 | REAL | S2 | never Some(next_restart_ms) (F-001) |
| ExtensionSupervisor::reenable | 422-425 | REAL | — | resets by removal — correct |
| ExtensionSupervisor::auto_reeanble_due | 427-449 | REAL | — | boundary exact (600_299 vs 600_300 tested) |
| ExtensionSupervisor::drain_events | 452-455 | REAL | — | drain(..) oldest-first |
| ExtensionSupervisor::entry | 460-474 | REAL | S3 | comment lies; min-crashes eviction (F-004) |
| ExtensionSupervisor::push_event | 476-481 | REAL | — | bounded ring 128 |
| 7 unit tests | 491-693 | REAL | — | exact-value assertions; would fail on any policy regression |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| schema_defaults_are_doc06_sequence | YES | asserts all 5 schema values + 6 delay points incl. caps |
| apply_setting_validates_through_the_schema | YES | range violation and unknown-key refusals asserted |
| backoff_sequence_then_disabled_then_reeanble | YES | exact 4-action vector; post-disable crash stays Disabled; re-enable resets |
| policy_timer_reeanables_only_after_the_window | YES | 1 ms before vs at deadline |
| zero_timer_is_user_action_only | YES | u64::MAX never re-enables |
| version_upgrade_resets_the_counter | YES | (id,version) keying asserted both directions |
| event_ring_and_reasons_are_bounded | PARTIAL | ASCII-only reason — misses F-003 multi-byte wipe |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "immediate retry once, then 5 s, 30 s, 5 min; a fourth crash → Disabled" | doc 06 §5.6 via module doc | REAL (with disclosed delta) | code gives 0/5s/30s then Disabled at limit 3; "5 min" is the cap reached at restart #5 — delta disclosed in module doc lines 24-40 |
| "Events … shaped after doc 05 §3's plugins.crashed vocabulary" | module doc | REAL-UNPROVEN | shapes exist; no consumer (events_contract.rs:1727 gap) |
| "values live there and nowhere else" (settings) | module doc line 16 | REAL | defaults only in schemas/settings.toml; code reads embedded schema |
| eviction picks "oldest last activity" | entry() doc | FALSE | code picks min-crashes; no activity timestamp exists (F-004) |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped builds (F-002). In-file: `RunState::Enabled::next_restart_ms` is written nowhere (always `None`) — dead field. `ExtensionSupervisor::policy()` getter has no caller outside tests.

## 8. Dependencies & flags
Deps: settings-schema (embedded runtime plane), toml (for apply_setting validation). No feature gates. No unsafe. `#[allow]`: none.

## 9. Open questions for the next agent
- If/when an embedder lands, F-001 (deadline tracking) must be fixed first or the backoff remains fictional end to end.
- Should `record_crash` on a Disabled extension be a no-op event-wise (F-006)?
