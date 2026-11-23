# AUDIT: crates/device-discovery/src/context.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 82 / 4 production items (struct, ceiling const, new, with_limits, Default) + 2 tests |
| File verdict | REAL (works as written, tested) — two false doc claims; crate UNREACHABLE |
| Reachable from shipped entry point? | NO — same crate as monitor.rs (zero dependents repo-wide; see monitor audit F-002) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg for `max_volumes` (doc-only), rg for `scan_budget|DiscoveryContext` in ssdp.rs/volume.rs; baseline citation |

## 1. One-paragraph verdict
The context type is a two-field bounds struct with correct, tested clamping (`max_devices` into [1, 1024], `scan_budget` capped at 60 s; zero clamped up to 1, pinned by test). Both in-file tests pass in the baseline. The defects are documentation: the module doc (a) names a field `max_volumes` that does not exist anywhere in the codebase — the field is `max_devices` (rg proves the string occurs only in the doc comment itself), and (b) claims `crate::ssdp` "derives its receive windows from `scan_budget`'s shape" — ssdp.rs contains no reference to `scan_budget` or `DiscoveryContext` at all (its deadline knob is independent). Both claims are contradicted by the code they cite. The crate-level reachability failure is inherited from monitor.rs's audit (zero dependents).

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt (PACKAGE device-discovery): `test context::tests::defaults_are_bounded ... ok`, `test context::tests::limits_clamp_to_the_ceilings ... ok` (within the 38-passed lib run). Consumers within the crate verified: monitor.rs:153/234/262 reads `ctx.max_devices`; volume.rs:247-250 enforces it with the typed capacity error.

## 3. Findings
F-001 [S4]
Where: crates/device-discovery/src/context.rs:5-8
Code:
```
//! The context is advisory to implementations and binding to this
//! crate's own lanes: [`crate::monitor`] clamps its enumerate passes
//! to `max_volumes`, and [`crate::ssdp`] derives its receive windows
//! from `scan_budget`'s shape
```
What is wrong: two false claims in one sentence. (a) No field named `max_volumes` exists — `rg -n 'max_volumes' crates/device-discovery/src/*.rs` matches ONLY this doc line; the field is `max_devices` (context.rs:21, enforced at monitor.rs:234 and volume.rs:247). (b) ssdp.rs never reads `scan_budget` or `DiscoveryContext` — `rg 'scan_budget|DiscoveryContext' crates/device-discovery/src/ssdp.rs` returns nothing; its window is its own explicit knob. Why it matters: the doc describes a coupling between the context and the SSDP lane that does not exist; a future maintainer trusting it will "tune" scan_budget expecting SSDP behavior to change. Confidence: HIGH — Verification: ran both rg commands.

F-002 [S4]
Where: crates/device-discovery/src/context.rs:46-51
Code:
```
    pub fn with_limits(max_devices: usize, scan_budget: Duration) -> DiscoveryContext {
        DiscoveryContext {
            max_devices: max_devices.clamp(1, MAX_DEVICES_CEILING),
            scan_budget: scan_budget.min(Duration::from_secs(60)),
        }
    }
```
What is wrong: a caller requesting zero devices gets ONE device — the floor silently rewrites a (nonsensical but explicit) request instead of refusing it, and there is no way to express "discovery disabled" through the context. Deliberate (test comment: "a zero ceiling is clamped up, never a black hole") and harmless today; recorded as a policy wart. Confidence: HIGH — Verification: read + test at 76-80.

## 4. Per-function table (file <100 LOC — table included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DiscoveryContext (struct) | 15-25 | REAL | — | two bounded fields, per-field docs honest |
| MAX_DEVICES_CEILING | 27-29 | REAL | — | 1024 |
| DiscoveryContext::new | 31-40 | REAL | — | 256 / 3 s |
| DiscoveryContext::with_limits | 42-51 | REAL | S4 | 0→1 floor wart (F-002) |
| Default | 54-58 | REAL | — | delegates to new |
| tests (2) | 60-82 | REAL | — | exact clamp values pinned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| defaults_are_bounded | YES | exact 256 / 3 s |
| limits_clamp_to_the_ceilings | YES | 100 000→1024, 999 s→60 s, 0→1 all pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "crate::monitor clamps its enumerate passes to max_volumes" | context.rs:6-7 | FALSE (field name) | field is max_devices; `max_volumes` occurs only in this doc line |
| "crate::ssdp derives its receive windows from scan_budget's shape" | context.rs:7-8 | FALSE | ssdp.rs has zero references to scan_budget/DiscoveryContext |
| "the pass fails with the typed capacity error beyond it — ... not a silent truncation" | context.rs:17-20 | REAL | monitor.rs:234-239, volume.rs:247-250; poll_once test pins CapacityExceeded |

## 7. Dead / duplicated / unreachable code
Crate unreachable from shipped entry points (monitor.rs audit F-002 — zero Cargo.toml dependents). Within the file: nothing dead; `scan_budget` is read by no implementation in the crate (only the doc claims ssdp uses it) — an advisory field with zero in-crate readers, which is what makes F-001's second half true.

## 8. Dependencies & flags
std only. No features, no unsafe, no unwrap/expect.

## 9. Open questions for the next agent
- ssdp.rs auditor: does the SSDP lane take ANY context-derived bound today? If not, `scan_budget` is a write-only field — worth folding into that file's findings.
- If device browsing ever ships, the `max_volumes`/`max_devices` doc mismatch should be fixed before the context becomes a public contract.
