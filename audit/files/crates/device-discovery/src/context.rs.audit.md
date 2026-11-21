# AUDIT: crates/device-discovery/src/context.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 (re-spawn §7.4, session 2) |
| Date | 2026-11-21 |
| Lines / functions | 82 / 5 production items (+2 tests) |
| File verdict | UNREACHABLE (real, baseline-tested budget struct in a dependency-island crate; its doc comments overclaim what the bounds guarantee) |
| Reachable from shipped entry point? | NO — proof: `rg "device-discovery\|device_discovery" --glob **/Cargo.toml` → only its own manifest plus future-card *comments* in platform-{linux,macos,windows} (issue #42, not yet implemented); `rg "device_discovery::" apps/ crates/` → only a doc-comment mention at crates/plugin-api/src/discovery.rs:3. Neither apps/cli nor apps/desktop-tauri links it. Matches W10-A10's lib.rs verdict (dependency island). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `Read context.rs` (full); `rg device-discovery --glob **/Cargo.toml` (zero dependents); `rg "DiscoveryContext\|MAX_DEVICES_CEILING"` (all uses inside crate + docs/01:711 sketch); `rg max_volumes` (exists ONLY in this file's doc comment); `rg scan_budget crates/device-discovery/` (zero readers outside context.rs); `rg device_discovery:: apps/ crates/` (comment only); `sed -n` quote verification on context.rs:5-10,17-24,27-29,42-51,71-81, monitor.rs:222-239, ssdp.rs:244-255; baseline: test-workspace-allfeatures.log.txt:2358-2400. |

## 1. One-paragraph verdict
The code is a small, correct, panic-free data class — two fields, two clamping constructors, two tests that both ran green in the baseline all-features run (context::tests::defaults_are_bounded and ::limits_clamp_to_the_ceilings, log :2364-2365; suite result 38 passed :2400). The defects are all in what the doc comments claim the bounds do: the device-ceiling doc promises "a runaway enumeration can never grow the process" while every consumer checks the bound only AFTER the full snapshot is allocated; the module doc claims ssdp "derives its receive windows from scan_budget's shape" when ssdp.rs contains zero references to either; `scan_budget` itself is dead — a public field documented as a binding budget that no code in the crate reads. And none of it is reachable: the crate is a dependency island no shipped binary links (consistent with the matrix's honest not_started/blocked services-discovery rows).

## 2. What actually works (verified)
— Both file tests compiled and PASSED in baseline run #4: `audit/runs/baseline/test-workspace-allfeatures.log.txt:2364-2365` (`test context::tests::defaults_are_bounded ... ok`, `test context::tests::limits_clamp_to_the_ceilings ... ok`), suite `38 passed; 0 failed` at :2400. (The plain workspace run #3 aborted fail-fast before this binary — no `device_discovery` in test-workspace.log.txt.)
— Constructor math verified by reading: `with_limits(100_000, 999 s)` → 1024 / 60 s; `with_limits(0, ZERO)` → 1 / 0 s (context.rs:48-49), exactly what the tests pin.
— The max_devices "typed capacity error" contract (context.rs:18-19) is real on both error-surfacing APIs: monitor.rs:262-268 (`poll_once` returns `DiscoveryError::CapacityExceeded`) and volume.rs:247-254 (`VolumesService::discover` returns it via `into_engine_error`).

## 3. Findings

### F-001 [S3]
Where: crates/device-discovery/src/context.rs:27-29
Code:
```rust
/// The per-pass device ceiling (D-3: a runaway enumeration can never
/// grow the process — the pass fails loudly instead).
pub const MAX_DEVICES_CEILING: usize = 1024;
```
What is wrong: The claim "a runaway enumeration can never grow the process" is false. Every enforcement point checks the bound only after the enumeration has fully materialized: monitor.rs:224 (`let snapshot = match self.source.enumerate(&self.ctx)`) runs before monitor.rs:234 (`if snapshot.len() > self.ctx.max_devices`), and volume.rs:237 (`Vec::with_capacity(volumes.len())`) plus a full conversion loop run before volume.rs:247's check. The ceiling bounds ACCEPTANCE of a result, not ALLOCATION of it — a source returning 10M volumes grows the process arbitrarily before the pass fails. Additionally the citation is wrong: D-3 is "Defensive Ingestion & The 'Dirty World' Principle" (docs/engineering-directives.md:51-74), a parsing-robustness directive; the memory-bound posture this constant claims belongs to D-1 (Memory & Allocation Discipline, :16) or "MP §12" as the crate cites elsewhere (error.rs:41 makes the same D-3 miscitation for "bounded memory").
Why it matters: This is exactly the kind of guarantee a future platform-* adapter author (issue #42) will trust: they will believe the context bounds enumeration memory and will not add their own cap inside `VolumeSource::enumerate`, where it actually needs to live.
What correct looks like: Doc says "a runaway enumeration's RESULT can never be used — the pass fails loudly; sources must bound their own allocation (D-1)" — or the trait contract requires sources to honor `max_devices` during enumeration, not after.
Confidence: HIGH — Verification: read monitor.rs:222-239, volume.rs:234-254; D-3 text read at engineering-directives.md:51-74.

### F-002 [S3]
Where: crates/device-discovery/src/context.rs:5-10
Code:
```rust
//! crate's own lanes: [`crate::monitor`] clamps its enumerate passes
//! to `max_volumes`, and [`crate::ssdp`] derives its receive windows
//! from `scan_budget`'s shape (its own deadline knob stays explicit —
//! a UDP search window is protocol-driven, see the MX discussion in
//! `src/ssdp.rs`).
```
What is wrong: "crate::ssdp derives its receive windows from scan_budget's shape" — it derives nothing. `rg "scan_budget|DiscoveryContext" crates/device-discovery/src/ssdp.rs` → zero hits. SsdpSearchConfig::new() hard-codes its window independently (ssdp.rs:249-254: `mx: 1, max_responses: 32, wait: Duration::from_millis(1500)` — "MX + 500 ms margin"), and `SsdpSearcher` never receives a `DiscoveryContext` at all. There is no code path from `scan_budget` to any SSDP window; the 3 s default vs 1.5 s window is parallel construction, not derivation. The parenthetical ("its own deadline knob stays explicit") concedes the knob but leaves the false "derives" claim standing.
Why it matters: A maintainer tuning `DiscoveryContext::new()`'s scan_budget (context.rs:38) will expect SSDP windows to follow; they will not. The doc oversells the context's "binding" nature — in the SSDP lane it binds nothing.
What correct looks like: "crate::ssdp does not use this context; its receive window is protocol-driven (MX + margin), see src/ssdp.rs."
Confidence: HIGH — Verification: read ssdp.rs:226-272; grep zero references.

### F-003 [S4]
Where: crates/device-discovery/src/context.rs:6-7
Code:
```rust
//! crate's own lanes: [`crate::monitor`] clamps its enumerate passes
//! to `max_volumes`, and [`crate::ssdp`] derives its receive windows
```
What is wrong: The doc names the field `max_volumes`; the field is `max_devices` (context.rs:21). `rg max_volumes` over the whole repo matches ONLY this doc line — the name does not exist anywhere. monitor.rs actually checks `self.ctx.max_devices` (monitor.rs:234).
Why it matters: A reader grepping for `max_volumes` to find the monitor's clamp finds nothing; intra-crate docs that misname their own public field are the "naming that lies" defect class.
What correct looks like: `to \`max_devices\``.
Confidence: HIGH — Verification: rg (repo-wide, one hit: this comment); monitor.rs:234 read.

### F-004 [S3]
Where: crates/device-discovery/src/context.rs:22-24, 49
Code:
```rust
    /// The advisory wall-clock budget for one pass. Implementations
    /// must not block materially beyond it.
    pub scan_budget: Duration,
```
and (context.rs:49):
```rust
            scan_budget: scan_budget.min(Duration::from_secs(60)),
```
What is wrong: `scan_budget` is dead config. `rg scan_budget crates/device-discovery/` hits only context.rs itself (declaration :24, default :38, clamp :49, tests :68/:75). No lane reads it: monitor passes have no deadline whatsoever (its only time knob is `MonitorConfig::poll_interval`), SSDP uses its own `wait`, and `VolumesService::discover` never consults it. The doc states a contract ("Implementations must not block materially beyond it") that binds nothing — the crate's own implementations ignore it. Secondary gaps in the same field: no lower clamp (`Duration::ZERO` passes through :49, while `max_devices: 0` is clamped up — asymmetric "never a black hole" policy applied to one field only), and the zero-case test asserts only `zero.max_devices == 1`, never `zero.scan_budget` (context.rs:76-80).
Why it matters: "Config declared but never read" plus a documented-but-unenforced contract: the field is public API surface that promises a budget guarantee the codebase does not deliver anywhere.
What correct looks like: Either the monitor's pass loop takes a deadline derived from `scan_budget` (e.g., pass `ctx` into a time-bounded enumerate contract), or the field is removed and the doc stops claiming a budget. If kept, clamp symmetrically (`scan_budget.max(Duration::from_millis(200))`, mirroring ssdp.rs:269) and pin the zero case in the test.
Confidence: HIGH — Verification: rg over the crate; monitor.rs:199-244 read (no deadline in `run`/`one_pass`); ssdp.rs read.

### F-005 [S4]
Where: crates/device-discovery/src/context.rs:48, 76-80
Code:
```rust
            max_devices: max_devices.clamp(1, MAX_DEVICES_CEILING),
```
and (context.rs:76-80):
```rust
        let zero = DiscoveryContext::with_limits(0, Duration::ZERO);
        assert_eq!(
            zero.max_devices, 1,
            "a zero ceiling is clamped up, never a black hole"
        );
```
What is wrong: The clamp RELAXES a caller's stricter bound: a host asking for a zero-device pass (dry-run probe) silently gets a one-device allowance. For a safety ceiling, clamping should tighten (toward the safe direction), not loosen. The in-code rationale ("never a black hole") assumes a 0 ceiling must mean "fail every pass", which is exactly what a caller asking for 0 has requested. This is a deliberate, documented, tested choice — hence S4, not higher — but it inverts the semantics of a limit.
Why it matters: A caller cannot construct a "discover nothing" context; the API silently rewrites their policy.
What correct looks like: Either refuse `max_devices: 0` at construction (typed error / panic-free assert surfaced in the constructor's doc) or document explicitly that 0 is normalized to 1 as a deliberate floor.
Confidence: HIGH — Verification: read; behavior pinned by the baseline-green test (allfeatures log :2365).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `DiscoveryContext` (struct, 2 pub fields) | 15-25 | REAL | S3 (F-001/F-004 via doc contracts) | Plain data, `Copy`, no panics. `max_devices` enforced post-hoc by consumers; `scan_budget` has zero consumers. |
| `MAX_DEVICES_CEILING` | 27-29 | REAL | S3 (F-001) | `1024`. Referenced only by `with_limits`' clamp (:48) and the test (:74) — the const itself is not the lane bound (lanes compare against the instance field). |
| `DiscoveryContext::new` | 31-40 | REAL | — | Defaults 256 / 3 s; pinned by baseline-green test (:2364). `#[must_use]` present. |
| `DiscoveryContext::with_limits` | 42-51 | REAL | S4 (F-005; F-004 floor gap) | `clamp(1, 1024)`, `min(60 s)`. No panics; both params honored except the 0-relaxation. |
| `Default for DiscoveryContext` | 54-58 | REAL | — | Delegates to `new()`; canonical clippy form. |
| `tests::defaults_are_bounded` | 64-69 | REAL | — | Pins both defaults; green in baseline #4. |
| `tests::limits_clamp_to_the_ceilings` | 71-81 | REAL | S4 (coverage gap, F-004) | Pins upper clamps and zero→1; does NOT assert `zero.scan_budget` nor any pass-through value (e.g. `with_limits(5, 500 ms)` unchanged). Green in baseline #4. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `defaults_are_bounded` | YES | Changing 256 or 3 s in `new()` fails the exact `assert_eq!`s. |
| `limits_clamp_to_the_ceilings` | YES (for what it covers) | Removing either clamp fails :74/:75; changing the 0-floor fails :77-80. Not covered: scan_budget floor (none exists), pass-through values, any consumer behavior. Both tests ran and passed in baseline run #4 (allfeatures log :2364-2365). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| `pub trait ServiceDiscovery { fn discover(&self, ctx: &DiscoveryContext) … }` — the context is a doc-sketched type | docs/01-architecture.md:711-712 (§2.4 sketch) | REAL (in an unreachable crate) | context.rs:16 defines it; device.rs:39 matches the signature verbatim (checked by W10-A07 and re-read here). Sketch specifies no fields — the budget vocabulary is the crate's addition. |
| "(c) Deps. `media-types`, `plugin-api`" for device-discovery | docs/01-architecture.md:716 (§2.4) and registry row :237 | FALSE-as-of-HEAD (stale, documented in-code) | Cargo.toml:46 declares only `media-types`; :47-53 records the plugin-api removal (audit T5, 2026-11-17). The crate's own header comment still repeats the stale row (Cargo.toml:4-5). |
| "MP §12 — bounded budgets, directive D-3" as this file's mandate | context.rs:1-3 | PARTIALLY FALSE | D-3 is dirty-world parsing (engineering-directives.md:51-74), not a memory/budget directive (see F-001). |
| services-discovery rows: sd-directory-watch not_started ×4 (evidence ""), sd-upnp blocked ×4 | docs/feature-matrix.toml:2594-2605, 2620-2631 | HONEST (consistent) | No shipped binary links this crate (proof in header row); an empty-evidence not_started row is the truthful status. Matches W10-A10's lib.rs audit conclusion. |

## 7. Dead / duplicated / unreachable code
- `scan_budget` field: dead — zero readers anywhere (F-004).
- `MAX_DEVICES_CEILING`: live but narrow — used only by `with_limits` and the test; no lane consults the const directly.
- The entire file (and crate): UNREACHABLE from `lumen` / `lumen-tauri` (proof in header). No duplication found; the `with_limits` clamp pattern is deliberately mirrored by `SsdpSearchConfig::with_limits` (ssdp.rs:259-271) and `MonitorConfig`'s floors — same idiom, no copy-paste drift.

## 8. Dependencies & flags
- File imports only `std::time::Duration`. No feature gates, no unsafe, no unwrap/expect/panic (verified by full read; consistent with the crate's C5 std-only discipline, Cargo.toml:22-28).
- Crate-level: `media-types` (path) is the sole production dep; `testkit` dev-only; workspace lints inherited (Cargo.toml:40-41). No flags gate any of this file's code.

## 9. Open questions for the next agent
- For whoever wires the platform-* volume-watcher cards (issue #42): the real memory bound must live INSIDE `VolumeSource::enumerate` implementations (F-001) — the context's post-hoc check cannot protect allocation. Consider making the bound part of the trait contract.
- W10-A08 (monitor.rs): confirm the intended split — `one_pass` (monitor.rs:234-239) treats over-budget snapshots as a counted failed pass WITHOUT surfacing the typed `CapacityExceeded` (a thread cannot return); the field doc's "the pass fails with the typed capacity error" (context.rs:18-19) is honored only by `poll_once` and `VolumesService::discover`. Not filed as a finding (deliberate, documented in monitor.rs), but the doc sentence overstates the thread lane.
- If `scan_budget` gains an enforcement site, add the missing floor clamp and the zero-case test assertion (F-004); `SsdpSearchConfig::with_limits` (ssdp.rs:269) already shows the symmetric-clamp idiom to copy.
