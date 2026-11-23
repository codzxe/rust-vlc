# AUDIT: crates/device-discovery/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 156 / 6 production (Capability, is_available, from_probe_error, Feature, key, DiscoveryReport::{new,set,get,iter}) + 3 tests |
| File verdict | REAL — UNREACHABLE (crate has zero dependents) |
| Reachable from shipped entry point? | NO (same as crate: no Cargo.toml dependency; playback-engine events.rs marks the wiring "wired-by-later 7.D1") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/device_discovery-80b8532926b93caf --list` -> 3 capability tests present; full run 38 passed / 0 failed, exit 0; rg producers scan; sed verifications |

## 1. One-paragraph verdict
A small, honest capability vocabulary: a two-value `Capability` verdict, three `Feature` lanes with stable dotted keys, and a `DiscoveryReport` table whose default is "unavailable: not probed" (fail-closed). It is actually fed by real probes — `lib.rs::Discovery::probe()` populates all three rows (volume lanes as a documented NotYetImplemented compile-time fact; the network lane from a real SSDP socket probe) — so this is not an empty vocabulary. The only defect is inherited: `from_probe_error` delegates to `UnavailableReason::from_error`, so a probe timeout or capacity breach is reported as "DependencyMissing" (the error.rs F-002 mislabel). The crate ships nowhere.

## 2. What actually works (verified)
— `./target/debug/deps/device_discovery-80b8532926b93caf` -> 38 passed (includes `capability::tests::*` 3/3, confirmed in `--list`): unprobed→NotYetImplemented("not probed"), set-replaces-not-appends, stable dotted keys.
— Producer scan: `rg DiscoveryReport` shows `lib.rs:94-124` `Discovery::probe()` sets all three features from real verdicts (SSDP probe via `ssdp::SsdpSearcher::probe()`, verified in ssdp.rs:317-337).

## 3. Findings

F-001 [S4]
Where: crates/device-discovery/src/capability.rs:37-39
Code:
```
    pub fn from_probe_error(err: &crate::error::DiscoveryError) -> Capability {
        Capability::Unavailable(UnavailableReason::from_error(err))
    }
```
What is wrong: pure delegation to `UnavailableReason::from_error`, so it inherits that function's collapse (error.rs F-002): a `TimedOut` probe (slow peer) or `CapacityExceeded` (local bound) is reported to capability consumers as `DependencyMissing("discovery integration")` — a permanent-looking capability verdict for a transient condition. Same root cause, same fix location. Confidence: HIGH — Verification: read (both functions quoted).

## 4. Per-function table (156 LOC > 100 → mandatory)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Capability enum | 16-25 | REAL | — | fail-closed two-value verdict |
| Capability::is_available | 29-32 | REAL | — | matches! only |
| Capability::from_probe_error | 36-39 | REAL | S4 | F-001 delegation inherits error.rs F-002 collapse |
| Feature enum + key() | 43-64 | REAL | — | 3 lanes, stable dotted keys (test-pinned) |
| DiscoveryReport::new | 73-78 | REAL | — | empty, fail-closed |
| DiscoveryReport::set | 82-90 | REAL | — | replace-in-place (test-pinned), linear scan over ≤3 rows |
| DiscoveryReport::get | 95-104 | REAL | — | unprobed → NotYetImplemented "not probed" (test-pinned) |
| DiscoveryReport::iter | 107-110 | REAL | — | |
| tests ×3 | 116-156 | REAL | — | honest-default, replace semantics, key stability |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unprobed_features_are_honestly_unavailable | YES | exhaustive match on verdict + detail substring (an Available default fails) |
| set_replaces_in_place_not_appends | YES | count==1 + verdict equality (append would give 2) |
| feature_keys_are_stable_dotted_ids | YES | exact string equality ×3 (catches key drift) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A Capability::Unavailable is a real implementation of graceful degradation: the lane says what is missing and why; it never fakes success" | capability.rs:8-11 | REAL | typed UnavailableReason throughout; lib.rs probe feeds real verdicts; default is unavailable |
| "unprobed features are honestly unavailable ('not probed'), never assumed available" | capability.rs:92-93 | REAL | L101-103 + test |
| "The stable dotted key (feature-matrix vocabulary)" | capability.rs:55-56 | REAL | keys are `discovery.*` dotted; whether any feature-matrix row cites them is the matrix auditors' question |

## 7. Dead / duplicated / unreachable code
None dead in-file; whole crate UNREACHABLE from shipped entry points. The vocabulary IS consumed crate-internally (lib.rs Discovery::probe/report, ssdp.rs probe) — not dead code within the crate.

## 8. Dependencies & flags
crate::error::UnavailableReason/DiscoveryError only. No features, no unsafe, no I/O.

## 9. Open questions for the next agent
- Does the feature-matrix.toml cite `discovery.volumes.enumeration` / `discovery.volumes.notifications` / `discovery.network.services` as evidence keys, and with what platform statuses? (matrix chunks belong to doc auditors — cross-check the keys' stability claim.)
- lib.rs `Discovery::probe()`'s volume verdict is a compile-time fact, not a runtime probe — verify the platform-* crates indeed still lack VolumeSource impls (their manifests say issue #42).
