# AUDIT: crates/device-discovery/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 156 / 4 (Discovery::{probe, report, volumes, ssdp}) |
| File verdict | REAL (honest typed-fallback service) — UNREACHABLE; its stated consumer (engine-host relay) does not exist |
| Reachable from shipped entry point? | NO — zero dependent manifests; playback-engine (the would-be relayer) does not depend on it and admits "wired-by-later 7.D1" (playback-engine/src/events.rs:106-107) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg (dependents, catalog wiring, DiscoveryScreen) exit 0/1; sed -n verifications exit 0; baseline citations |

## 1. One-paragraph verdict
The crate does exactly what it says and says exactly what it does: `Discovery::probe`
assembles a volume lane that is a TYPED REFUSAL (UnavailableVolumes, every method
Err(reason)) because no platform adapter implements `VolumeSource` yet — honest
capability reporting, no fake success, no fabricated events — plus a real SSDP socket
probe on native targets. Tests back every lane (38 unit + 5 fallback_paths + 5 ssdp +
3 volume_monitor, all green in baseline). The damning context: nothing depends on
this crate. The catalog's `devices.volume-added/removed` wire types exist
(commands/src/catalog/event.rs:290-307) and a UI screen exists (DiscoveryScreen, 16
vitest tests green), but the relay this crate exists to feed — "the source the engine
host relays into the catalog's rows" — is admitted by playback-engine itself as
"wired-by-later": the chain is three finished organs and no connective tissue.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt L2635-2721 (PACKAGE device-discovery
exit_code=0): unittests 38 ok; tests/fallback_paths.rs 5 ok; tests/ssdp.rs 5 ok
(1.04 s — real socket work); tests/volume_monitor.rs 3 ok (3.00 s — real watcher
delays).
— verified by read volume.rs:181-203: `UnavailableVolumes::because` + `enumerate`
returns `Err(self.reason.clone())` — the typed fallback is real, not a stub
returning empty success.
— verified by rg: no `Cargo.toml` in the workspace declares `device-discovery`;
`rg "device" crates/playback-engine/Cargo.toml crates/commands/Cargo.toml` -> no
matches; catalog event types defined at commands/src/catalog/event.rs:290-307 with
serde renames; playback-engine/src/events.rs:106-107 marks the relay
"wired-by-later 7.D1".
— verified by baseline ui-test.txt L50: `src/screens/discovery/DiscoveryScreen.test.tsx
(16 tests)` passing.

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/src/lib.rs:9-13
Code:
```
//! events::VolumeEvent`]s — the source the engine host relays into
//! the catalog's `devices.volume-added` / `devices.volume-removed`
//! rows (the relay itself is host wiring; the catalog emission ledger
```
What is wrong: the relay described in the present tense does not exist anywhere —
no crate depends on device-discovery, and the engine-side relay is explicitly
deferred ("wired-by-later 7.D1", playback-engine/src/events.rs:106). The crate is a
finished producer with zero consumers. Why it matters: any reader of this doc
(including gate docs) believes volume events reach the catalog; they never leave this
crate. docs/gates/phase7-exit.md calls 7.D1 "LANDED, tests green; UI exists" —
component-true, chain-false. What correct looks like: either the host-wiring card
lands or the doc says "consumer unwired". Confidence: HIGH — Verification: read
(manifest scan + playback-engine admission).
F-002 [S3]
Where: crates/device-discovery/src/lib.rs:81-85 + 96-131
Code: `pub struct Discovery {\n    volumes: std::sync::Mutex<Box<dyn VolumeSource>>,`
What is wrong: the crate's headline entry point (`Discovery::probe`) is unreachable
from every shipped binary — inherited from F-001 (zero dependents). The honest
capability table it computes (`DiscoveryReport`) is never read by any product path.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/device-discovery/src/lib.rs:17-19
Code:
```
//! The three wave-22 `platform-*` crates (L5) implement seams defined
//! by core crates; doc 01 §2.7's registry lists `device-discovery` as
```
What is wrong: the registry edge ("a dependency of every adapter") is aspirational —
the platform manifests explicitly EXCLUDE device-discovery from their allowlists
(platform-linux/Cargo.toml:6-10 et al., "device-discovery is issue #42 — the manifest
is an allowlist"). The lib doc's future tense ("will implement") keeps it honest, but
the layering story and the manifest reality point in opposite directions today.
Confidence: HIGH — Verification: read (platform manifests).

## 4. Per-function table (file < 170 LOC; table kept for the 4 methods)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Discovery::probe | 96-131 | REAL | F-002 | typed volume fallback minted once; live SSDP socket probe; report filled verbatim |
| Discovery::report | 135-137 | REAL | — | pure accessor |
| Discovery::volumes | 144-149 | REAL | — | poison-recovering lock (workspace convention) |
| Discovery::ssdp | 153-155 | REAL | — | Some only when native probe passed |

## 5. Test honesty
No tests in-file; the crate's suites (fallback_paths, ssdp, volume_monitor) cover the
lanes this file assembles — all green in baseline (cited above). fallback_paths.rs
pins that the fallback REFUSES (would fail if UnavailableVolumes ever returned Ok).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every method returns the typed DiscoveryError::Unavailable — ... never a fake success" | lib.rs:24-27 | REAL | volume.rs:199-203 + fallback_paths 5 ok |
| "real loopback UDP round-trips in tests/ssdp.rs" | lib.rs:36-37 | REAL | tests/ssdp.rs 5 ok in 1.04 s (baseline) |
| "the web build carries the typed BlockedOnWeb verdict" | lib.rs:37-39 | REAL-UNPROVEN | claimed from src/ssdp.rs (not this file; not executed on wasm here) |
| "the source the engine host relays into the catalog's rows" | lib.rs:10-13 | FALSE (as a system statement) | relay unwired: no dependent, playback-engine says wired-by-later |
| registry lists device-discovery as dependency of every adapter | lib.rs:18-19 | UNREACHABLE/aspirational | platform manifests exclude it |

## 7. Dead / duplicated / unreachable code
The entire public surface is unreachable (F-001/F-002). No intra-file dead code.

## 8. Dependencies & flags
std-only + `media-types` (path dep, Cargo.toml:46 — SourceUri/EngineError vocabulary);
zero crates.io deps ("C5 discipline" comment verified). testkit dev-only. No features
gating this file. No unsafe. No env reads.

## 9. Open questions for the next agent
- Doc auditors: check docs/gates/phase7-exit.md "7.D1 LANDED" against the unwired
  relay + blocked UI route (OF-1) — the row overstates the user-visible state.
- Which card owns the host wiring (engine host → catalog devices.volume-*)? If it
  was never scheduled, the "wired-by-later" note is a promise without a card.
