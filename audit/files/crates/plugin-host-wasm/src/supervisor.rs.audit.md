# AUDIT: crates/plugin-host-wasm/src/supervisor.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-21 |
| Lines / functions | 693 / 29 (21 impl/type rows + 8 test rows) |
| File verdict | UNREACHABLE (real, executed in tests; no shipped entry point links it) |
| Reachable from shipped entry point? | NO — proof in F-001 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | rg `ExtensionSupervisor\|record_crash\|BackoffPolicy` repo-wide → only lib.rs re-export + supervisor.rs + tests/plugin_crash_isolation.rs; rg `plugin-host-wasm\|plugin_host_wasm` over **/Cargo.toml → only its own manifest; rg `plugin_host_wasm` (rust, apps/ crates/ tests/ tools/, crate excluded) → no matches; grep -ln `Supervisor` crates/plugin-host-wasm/src/*.rs bin/*.rs → lib.rs, supervisor.rs only; sed -n verified every quote; baseline orchrun-test-workspace-nofailfast.log.txt:4534-4541 (7/7 supervisor tests ok), :4557-4564 (integration suite ok) |

## 1. One-paragraph verdict
The crash-backoff/`Disabled` policy is genuinely implemented, deterministic (injected `u64` ms clock, no `Instant`/sleep anywhere in the file), bounded (ring 128, map 64, reason 256), and unit+integration-tested green in the baseline no-fail-fast run — but it is a policy engine for an embedder that does not exist: no crate, no app, not even the crate's own `lumenext`/`lumenext-examples` binaries or `install.rs`/`instance.rs` reference `ExtensionSupervisor`. The only caller in the repository is `tests/plugin_crash_isolation.rs`. Nothing shipped (`lumen` CLI, `lumen-tauri`) links `plugin-host-wasm` at all, and the command layer's `plugins.crashed` event this module feeds is an `#[ignore]`d gap (baseline orchrun-test-workspace-nofailfast.log.txt:3880). Code quality inside the file is high with three real defects (dead `next_restart_ms` deadline, a UTF-8 truncation bug that can wipe an entire crash reason, and a doc-comment that misdescribes the eviction policy).

## 2. What actually works (verified)
— Unit tests executed green: `orchrun-test-workspace-nofailfast.log.txt:4534-4541` shows all 7 `supervisor::tests::*` passing; `:4557-4564` shows `plugin_crash_isolation` integration suite (real wasmi host + real supervisor, trap fixtures) passing. Schema-driven defaults verified against `schemas/settings.toml:1408-1471` (5000/6/300000/3/900000 — the test at supervisor.rs:502-506 asserts exactly these).
— Determinism claim verified: `rg -n "Instant\|sleep" supervisor.rs` → no matches; time is the injected `now_ms` parameter throughout.

## 3. Findings

### F-001 [S3] The entire crash-isolation supervisor is unreachable — no shipped or even in-crate production caller exists
Where: crates/plugin-host-wasm/src/supervisor.rs:356-361 (and the whole file)
Code:
```rust
    pub fn record_crash(
        &mut self,
        id: &str,
        version: &str,
        reason: &str,
        now_ms: u64,
    ) -> ActionTaken {
```
What is wrong: The doc comment at supervisor.rs:59-61 says "The embedder owns the clock and the extension thread pool (doc 06 §5.8) and calls [`ExtensionSupervisor::record_crash`] after every contained fault" — that embedder does not exist. Repo-wide search for `ExtensionSupervisor`/`record_crash` matches only `lib.rs:101` (re-export), `supervisor.rs` itself, and `tests/plugin_crash_isolation.rs`. `grep -ln Supervisor` over `crates/plugin-host-wasm/src/*.rs` and `bin/*.rs` returns only `lib.rs` + `supervisor.rs` — the crate's own `instance.rs` (trap containment), `install.rs`, and both binaries never consult the policy.
Why it matters: Doc 06 §5.6's crash-restart/`Disabled` machinery and the doc 05 §3 `plugins.crashed` events this module produces never run in any shipped binary. The `lumen` CLI and `lumen-tauri` do not link `plugin-host-wasm` (rg over `**/Cargo.toml`: only `crates/plugin-host-wasm/Cargo.toml` mentions it; `apps/cli/Cargo.toml:59` and `apps/web/Cargo.toml:61` depend on `plugin-api` only). The command layer's `plugins.crashed` consumer is `#[ignore]`d: baseline `orchrun-test-workspace-nofailfast.log.txt:3880` — `test gap_plugins_crashed ... ignored, gap: plugins.crashed not wired — handoff H-10`. Five shipped settings (`schemas/settings.toml:1408-1471`, `extensions.crash-*`, CLI flags) configure a policy that no shipped code executes.
What correct looks like: An embedding app (CLI or tauri) constructs `ExtensionSupervisor`, feeds faults from `WasmInstance` invocations, drains events into `plugins.crashed` — or the dead crate is documented as Phase-7-not-wired and the settings rows are marked unwired.
Confidence: HIGH — Verification: read + rg callers + baseline no-fail-fast log.

### F-002 [S3] `RunState::Enabled { next_restart_ms }` is dead — the documented restart deadline can never be `Some`
Where: crates/plugin-host-wasm/src/supervisor.rs:396-418 (and 297-307)
Code:
```rust
            None => RunState::Enabled {
                crashes: record.crashes,
                next_restart_ms: None,
            },
```
What is wrong: The `RunState::Enabled` doc (supervisor.rs:299-307) promises "possibly inside a backoff window before the next supervised restart — `next_restart_ms` is the virtual-clock deadline, `None` when no restart is pending", but `state()` returns `next_restart_ms: None` on every path. `CrashRecord` (supervisor.rs:286-293) stores only `crashes` and `disabled_at_ms` — there is no last-crash timestamp, so no deadline is computable.
Why it matters: An embedder consulting `state()` during a backoff window gets no deadline and cannot know when the restart is due without re-deriving policy itself; the field and its doc are a lie about the API's capability.
What correct looks like: `CrashRecord` stores `last_crash_at_ms` and `state()` computes `Some(last_crash_at_ms + restart_delay_ms(crashes))` while enabled, or the field is removed.
Confidence: HIGH — Verification: read (both construct sites of the `Enabled` variant, 399-403 and 414-417, pass `None`).

### F-003 [S3] Crash-reason truncation wipes the ENTIRE reason for multi-byte strings over 256 bytes
Where: crates/plugin-host-wasm/src/supervisor.rs:363-369
Code:
```rust
        let mut bounded = reason.to_string();
        if bounded.len() > REASON_CAP {
            // Char-boundary-safe truncation (the reason is error text,
            // not a protocol payload).
            bounded.truncate(bounded.char_indices().nth(REASON_CAP).map_or(0, |(i, _)| i));
            bounded.push('…');
        }
```
What is wrong: `bounded.len()` is BYTES; `char_indices().nth(256)` is the 257th CHAR. For a reason of e.g. 129 two-byte characters (258 bytes > 256, but only 129 chars), `nth(256)` is `None`, `map_or(0, …)` yields 0, and `truncate(0)` destroys the whole message — the event reason becomes just `"…"`. The doc on `SupervisorEvent.reason` (supervisor.rs:277) says "bounded to 256 chars"; such a string is within the char cap yet gets annihilated.
Why it matters: Crash diagnostics are the entire point of the reason field; realistic `HostError` text containing non-ASCII paths/messages >256 bytes loses all diagnostic content precisely when an extension is crash-looping toward `Disabled`.
What correct looks like: `if let Some((i, _)) = bounded.char_indices().nth(REASON_CAP) { bounded.truncate(i); bounded.push('…'); }` — only truncate when there really are more than 256 chars.
Confidence: HIGH — Verification: read (logic traced by hand; byte/char divergence is mechanical). No test covers it — see test table.
### F-004 [S4] `entry()` eviction doc-comment describes a policy the code does not implement ("oldest last activity")
Where: crates/plugin-host-wasm/src/supervisor.rs:458-475
Code:
```rust
    /// Bounded map insert: a fresh key evicts the record with the
    /// oldest last activity when the cap is reached (doc 06 §5.4's
    /// bounded-host-accounting posture; 64 tracked extensions is
    /// far above the 16-installed-and-loaded row).
    fn entry(&mut self, key: &(String, String)) -> &mut CrashRecord {
```
What is wrong: `CrashRecord` has no last-activity timestamp; eviction is `min_by_key` on `crashes` (fewest-crashes record is evicted, ties broken by BTreeMap key order). The comment describes an LRU-style policy that cannot be computed with the stored data. Side effect: any eviction silently resets the evicted extension's crash window (a fresh record starts at `crashes: 0`).
Why it matters: The comment misleads the next maintainer; the counter-reset side effect is undocumented. Blast radius is capped at >64 concurrent tracked extensions.
What correct looks like: Either store `last_activity_ms` and evict by it, or fix the comment to "evicts the record with the fewest crashes (ties: lexicographically first key)" and document the window-reset.
Confidence: HIGH — Verification: read.
### F-005 [S4] Misspelled public API: `reeanble` in `auto_reeanble_due` and `disabled_reeanble_ms`
Where: crates/plugin-host-wasm/src/supervisor.rs:108-109, 432
Code:
```rust
    /// `Disabled` policy timer in ms; 0 = user-action re-enable only.
    pub disabled_reeanble_ms: u64,
```
What is wrong: "reenable" is misspelled "reeanble" in two public names (`BackoffPolicy::disabled_reeanble_ms`, `ExtensionSupervisor::auto_reeanble_due`), while the corresponding setting key (`SETTING_REENABLE_MS` = "extensions.crash-disabled-reenable-ms") and all comments spell it correctly.
Why it matters: Public-API typo in a re-exported headline surface (`lib.rs:101`); grep-for-"reenable" misses the method.
What correct looks like: `disabled_reenable_ms` / `auto_reeanble_due` → `auto_reenable_due`.
Confidence: HIGH — Verification: read.
### F-006 [S4] No time "window" on the crash counter; every crash while Disabled slides the re-enable deadline
Where: crates/plugin-host-wasm/src/supervisor.rs:375-377
Code:
```rust
        let action = if crashes > restart_limit {
            record.disabled_at_ms = Some(now_ms);
            ActionTaken::Disabled
        }
```
What is wrong: (a) Doc 06 §5.6 says "a fourth crash within the window moves the extension to `Disabled`" — the code has no time window at all: crashes accumulate forever between re-enables/upgrades (the module header re-defines "window" as "between re-enables", an interpretation, disclosed). (b) Each crash recorded while already `Disabled` overwrites `disabled_at_ms` with the fresh `now_ms`, pushing the policy-timer deadline (`at_ms + disabled_reeanble_ms`, computed at 411 and 442) outward on every subsequent crash — undocumented deadline extension. (c) Theoretical: `at_ms + disabled_reeanble_ms` (411, 442) can overflow `u64` for a clock near `u64::MAX` with a non-zero timer (debug: panic; release: wrap → premature re-enable). Unreachable in practice with ms clocks.
Why it matters: Policy behavior drifts from the doc's wording; the sliding deadline is invisible to a reader of doc 06.
What correct looks like: Record `disabled_at_ms` only on the transition into `Disabled` (`if record.disabled_at_ms.is_none()`), and use `saturating_add` for deadlines.
Confidence: HIGH — Verification: read (all three sites traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SETTING_* consts (5 keys) | 77-85 | REAL | — | match `schemas/settings.toml:1408-1471` keys exactly (verified). |
| REASON_CAP / EVENT_RING / TRACKED_CAP | 87-92 | REAL | F-003 | 256/128/64; bounds asserted by test at 688-691. |
| `BackoffPolicy` (struct) | 98-110 | REAL | F-005 | field typo `disabled_reeanble_ms`. |
| `BackoffPolicy::from_schema_defaults` | 121-160 | REAL | — | typed refusals for missing/non-integer/default_ref entries; no silent fallback (verified against schema runtime contract in settings-schema audit). |
| `BackoffPolicy::apply_setting` | 169-201 | REAL | — | validates through `settings_schema::runtime::validate_value` BEFORE mapping; unknown key refused; invariants re-checked after write. |
| `BackoffPolicy::check_invariants` | 204-228 | REAL | — | multiplier ≥ 1, max ≥ initial, restart_limit ≥ 1; error messages actionable. |
| `BackoffPolicy::restart_delay_ms` | 234-247 | REAL | — | saturating mul; early cap-return; sequence 0/5s/30s/3min/5min-cap verified by test 511-516. |
| `ActionTaken` (enum) | 252-267 | REAL | — | doc 05 §3 `action_taken` vocabulary. |
| `SupervisorEvent` (struct) | 271-283 | REAL | — | reason doc "bounded to 256 chars" contradicted by F-003 mechanics (bytes vs chars). |
| `CrashRecord` (struct) | 286-293 | REAL | F-002/F-006 | no timestamps except `disabled_at_ms` → deadline not computable. |
| `RunState` (enum) | 297-318 | REAL | F-002 | `next_restart_ms` never `Some` (dead field). |
| `ExtensionSupervisor` (struct) | 325-330 | REAL | — | plain state, embedder-owned locking (documented). |
| `ExtensionSupervisor::new` | 340-346 | REAL | — | trivial; `# Errors` doc admits impossibility. |
| `ExtensionSupervisor::policy` | 349-351 | REAL | — | trivial getter. |
| `ExtensionSupervisor::record_crash` | 356-392 | REAL | F-003, F-006 | truncation bug; deadline slide; saturating crash counter; emits event per crash. |
| `ExtensionSupervisor::state` | 396-419 | REAL | F-002 | both `Enabled` arms hard-code `next_restart_ms: None`. |
| `ExtensionSupervisor::reenable` | 423-425 | REAL | — | removes record; fresh window (tested 588-599). |
| `ExtensionSupervisor::auto_reeanble_due` | 432-451 | REAL | F-006 | 0-timer short-circuit; overflow-theoretical add at 442. |
| `ExtensionSupervisor::drain_events` | 454-456 | REAL | — | `drain(..).collect()`; bounded by ring. |
| `ExtensionSupervisor::entry` | 462-475 | REAL | F-004 | eviction by fewest-crashes, not "oldest last activity". |
| `ExtensionSupervisor::push_event` | 477-482 | REAL | — | pops front at 128. |
| `tests::ok` (helper) | 490-495 | REAL | — | `unreachable!` on Err — test-only. |
| `schema_defaults_are_doc06_sequence` | 497-517 | REAL | — | mutation-sensitive: schema drift or formula break fails it. |
| `apply_setting_validates_through_the_schema` | 519-536 | REAL | — | runs green, but the multiplier=0 refusal is ALSO caught by `check_invariants` (205-211), so it does not uniquely prove the `validate_value` seam; no case where only the schema range would reject (e.g. initial-ms=700000 > max 600000). Sensitivity: PARTIAL (see §5). |
| `backoff_sequence_then_disabled_then_reeanble` | 538-600 | REAL | — | asserts full sequence, post-disable crash, event mirror, re-enable reset. |
| `policy_timer_reeanbles_only_after_the_window` | 602-627 | REAL | — | boundary at deadline−1/+0 verified. |
| `zero_timer_is_user_action_only` | 629-645 | REAL | — | `u64::MAX` clock with 0 timer → never fires. |
| `version_upgrade_resets_the_counter` | 647-676 | REAL | — | per-version keying verified both directions. |
| `event_ring_and_reasons_are_bounded` | 678-692 | REAL | F-003 | ring/map bounds real; reason bound only tested with ASCII `"x".repeat(10_000)` — the multi-byte wipe (F-003) passes undetected. Sensitivity: PARTIAL (see §5). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| schema_defaults_are_doc06_sequence | YES | exact-value asserts on schema + formula outputs (511-516). |
| apply_setting_validates_through_the_schema | PARTIAL | refusal asserts hold even if `validate_value` were deleted (invariants reject 0 too); unknown-key arm would still pass via the match fall-through. Schema-range-only rejection is untested. |
| backoff_sequence_then_disabled_then_reeanble | YES | exact `ActionTaken` vector equality (548-565). |
| policy_timer_reeanbles_only_after_the_window | YES | exact boundary times (613, 620). |
| zero_timer_is_user_action_only | YES | `u64::MAX` still disabled. |
| version_upgrade_resets_the_counter | YES | both old-version-disabled and new-version-fresh asserted. |
| event_ring_and_reasons_are_bounded | PARTIAL | ring/map YES (688-691); reason bound NO for multi-byte input — mutate `truncate(...)` to `truncate(0)` and this test still passes for the ASCII case it exercises; the F-003 bug is live and untested. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "exponential backoff — immediate retry once, then 5 s, 30 s, 5 min; a fourth crash … `Disabled`" | doc 06 §5.6 (quoted in module header 26-30) | REAL (with disclosed delta) | defaults 5000/×6/300000/3 produce 0/5s/30s then Disabled at 4th crash; module header 32-41 openly documents the geometric-vs-sketch delta (3 min, not 5 min, would be restart #4 — never scheduled under default limit 3). |
| `plugins.crashed { id, error, action_taken }` event exists | doc 05 §3:399 | FALSE (at runtime) | command-layer event is `#[ignore]`d gap: `orchrun-test-workspace-nofailfast.log.txt:3880`; supervisor events are only drained by its own tests. |
| "nothing depends on this crate but the apps that embed it" | plugin-host-wasm lib.rs:10-11 | TRUE vacuously / misleading | zero apps embed it (Cargo.toml sweep, F-001). |
| crash policy "config-driven through the settings layer (never re-stated in code)" | module header 13-14 | REAL | `from_schema_defaults` reads embedded schema; no literals in code (test 502-506 pins schema values). |
| matrix row `plugin-wasm-extensions` = not_started (all platforms) | docs/feature-matrix.toml:2329 | REAL (shipping view) / under-claim (code view) | host crate implemented + tested (baseline :4495-4564) but unlinked from every shipped binary — "not_started" is honest about shipping, silent about ~10k LOC of working host code. |
| Time is injected, "no sleeps, no `Instant::now`" | module header 56-58 | REAL | rg `Instant\|sleep` over the file → no matches. |

## 7. Dead / duplicated / unreachable code
- The whole file is production-dead (F-001): shipped binaries cannot reach it; the only callers are `crates/plugin-host-wasm/tests/plugin_crash_isolation.rs` (which supplies its own embedder loop at :69-87).
- Dead field: `RunState::Enabled::next_restart_ms` (F-002) — never `Some`.
- The `policy()` getter (349-351) has no non-test caller either.

## 8. Dependencies & flags
- `settings-schema` (embedded-schema runtime plane) + `toml` (value type for the `config.set-setting` seam) + `crate::error::HostError` — all used, no unused deps in this file.
- No feature gates on this module (crate has no `[features]` section — Cargo.toml:41-83).
- Crate compiles in baseline workspace build (build-release-workspace.log.txt, "Compiling plugin-host-wasm" at test-workspace.log.txt:10).

## 9. Open questions for the next agent
- A09 (instance.rs, lumenext.rs, lib.rs): confirm `WasmInstance` never consults `InstanceStatus::Faulted` → supervisor wiring inside the crate (my grep says it does not); check whether `lumenext` CLI is advertised as a user-facing tool anywhere (docs claim?).
- Who owns the `plugins.crashed` command-layer gap (handoff H-10) — is any Phase-7 wiring card actually scheduled, or is the settings surface (`--extensions-crash-*` CLI flags) shipping config for a policy that will never run?
- A03 (state.rs, imports.rs#part2): does any host-side code path emit SecurityEvent for traps that the supervisor could consume?
