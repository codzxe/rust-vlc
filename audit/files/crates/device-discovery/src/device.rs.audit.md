# AUDIT: crates/device-discovery/src/device.rs (whole file, 123 lines)
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-21 |
| Lines / functions | 123 / 4 items (trait + 3 types + 2 impls) + 2 tests |
| File verdict | UNREACHABLE (real vocabulary types — nothing shipped reaches them) |
| Reachable from shipped entry point? | NO — the crate's only dependents (platform-linux/macos/windows) never reference it; those crates themselves have no dependents; `discover()` has zero production callers |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg -n "DeviceKind::Service\|ServiceProtocol::Upnp" crates/device-discovery/src` (constructed nowhere); `rg -n "impl ServiceDiscovery"` (one prod impl: volume.rs:228; plugin-api's is its own trait); `rg -n "\.discover\(" crates apps` (only tests + unrelated vaapi DriDiscovery); `rg -n -i "device" crates/platform-{linux,macos,windows}/src` (comment-only — dependency unused); `rg -n "upnp" crates/plugin-api/src/discovery.rs` (vocabulary match verified); `rg -c "device-discovery" docs/feature-matrix.toml product.toml` (zero — crate unclaimed); baseline INDEX #3 (crate tests green). |

## 1. One-paragraph verdict
A 123-line vocabulary file: one trait (`ServiceDiscovery`), one data struct (`Device`), two enums — all correct, small, and honest about untrusted labels. Its problems are entirely about existence: `DeviceKind::Service(ServiceProtocol::Upnp)` is **constructed nowhere in the codebase** (SSDP is a verdict-and-parse module that emits no Devices, so the "network service … SSDP/UPnP today" doc line overstates), and the trait's "domain port" is unwired — one production implementation (local volumes), zero production callers, with the three platform crates that declare the dependency never using it. The crate is not even cited by a feature-matrix row; it is fully dead weight in the shipped product.

## 2. What actually works (verified)
— The data model compiles and is exercised by its own tests (`device_is_plain_data` :107-116 — would fail if `SourceUri::parse` mangled the scheme; `service_protocol_names…` :119-122 — pins `"upnp"`), green in baseline `cargo test --workspace` (INDEX #3, 2026-11-21).
— The doc claim "matches plugin-api's examples (doc 06 §7.2)" is real: plugin-api's `ServiceDiscovery::discovers()` example vocabulary is `vec!["upnp"]` (crates/plugin-api/src/discovery.rs:126) — identical string.
— "The two crates deliberately share no types beyond media-types" (:13-14) is real: device-discovery's only deps are media-types + workspace lints; no plugin-api edge exists.

## 3. Findings

### F-001 [S3] `DeviceKind::Service` is dead vocabulary; the doc claims SSDP/UPnP devices exist "today"
Where: crates/device-discovery/src/device.rs:63-65 (with :71-76)
Code:
```
    /// A network service (SSDP/UPnP today; mDNS and friends join with
    /// their own cards and verdicts).
    Service(ServiceProtocol),
```
What is wrong: no code in the repository constructs `DeviceKind::Service` — not ssdp.rs (which is a feasibility-verdict + parse-only module and builds no `Device` values; `rg "Device \{|DeviceKind" crates/device-discovery/src/ssdp.rs` → no matches), not tests, nothing. The doc's "SSDP/UPnP today" implies the Service lane is live today; the truth (per ssdp.rs's own header, "the honest feasibility verdict + the provable subset") is that SSDP produces zero devices.
Why it matters: a sorting/filtering "vocabulary" that no producer emits is aspirational API surface masquerading as a capability; a UI filtering on `DeviceKind::Service` would show an eternally empty section and a reader of the doc would believe UPnP discovery is wired.
What correct looks like: either ssdp.rs's parseable-subset path mints `DeviceKind::Service(Upnp)` entries, or the doc says "recorded verdict, no devices produced yet (ssdp.rs §verdict)" and the variant is `#[allow(dead_code)]`-documented or removed until a producer exists.
Confidence: HIGH — Verification: read (repo-wide grep for constructors).

### F-002 [S3] The domain port is unwired: one impl, zero callers, and the platform crates' dependency edges are unused
Where: crates/device-discovery/src/device.rs:32-40 (trait) — evidence: crates/platform-{linux,macos,windows}/Cargo.toml deps vs. their src
Code:
```
pub trait ServiceDiscovery {
    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<Device>, EngineError>;
}
```
What is wrong: exactly one production implementation exists (`impl ServiceDiscovery for VolumesService`, volume.rs:228), and `discover(&…)` is called only by the crate's own tests (tests/fallback_paths.rs:86). The three crates that depend on `device-discovery` (platform-linux, platform-macos, platform-windows) contain **zero** references to it (`rg -i "device" crates/platform-*/src` hits only comments about audio devices), and those three crates themselves have no dependents — so the dependency declarations are dead edges on top of a dead crate. No shipped entry point (`lumen` CLI, `lumen-tauri`) is anywhere in this chain.
Why it matters: unused dependencies bloat the build and misrepresent the architecture (a reader of platform-linux/Cargo.toml concludes Linux integrates services discovery); the "domain port" doc language suggests a functioning seam where none is consumed.
What correct looks like: wire `VolumesService`/`VolumeMonitor` into a platform integration that something ships, or drop the dependency edges until the integration card exists.
Confidence: HIGH — Verification: read (grep evidence above; platform-* consumer check; workspace Cargo.toml scan in my job.rs audit).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ServiceDiscovery` trait + `discover` | 32-40 | REAL | S3 | F-002: 1 impl (volume.rs:228), 0 prod callers; "absence of devices is `Ok(vec![])`" is a contract its only impl must honor (volume.rs owner to confirm) |
| `Device` struct | 44-54 | REAL | — | plain data; `uri: SourceUri` validated; label documented untrusted (V7) |
| `DeviceKind` enum | 59-66 | REAL | S3 | F-001: `Volume` variant live (volume.rs `to_device`), `Service` variant dead |
| `ServiceProtocol` enum | 71-76 | REAL | — | single variant; vocabulary-only |
| `ServiceProtocol::as_str` | 82-87 | REAL | — | "upnp" — matches plugin-api's discovers() vocabulary |
| `Display for ServiceProtocol` | 89-93 | REAL | — | delegates to as_str |
| test `device_is_plain_data` | 107-116 | REAL | — | trivial-but-honest field/scheme roundtrip |
| test `service_protocol_names_match_the_manifest_vocabulary` | 119-122 | REAL | — | pins the string |

## 5. Test honesty
Both tests assert exact values and would fail if `SourceUri::parse` regressed (scheme mangling) or the vocabulary string changed. Gap: no test constructs a `DeviceKind::Service` — consistent with F-001 (nothing can).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A network service (SSDP/UPnP today; …)" | device.rs:63-64 | FALSE | F-001: no producer of `Service` devices exists |
| "as_str … matches plugin-api's examples" (doc 06 §7.2) | device.rs:79-80 | REAL | plugin-api/src/discovery.rs:126 `vec!["upnp"]` |
| "the two crates deliberately share no types beyond media-types" | device.rs:13-14 | REAL | no plugin-api dependency in device-discovery/Cargo.toml |
| "labels … untrusted text (V7 — never interpolate them into markup …)" | device.rs:5-7 | REAL | this file does no interpolation; the contract is on the (nonexistent) UI consumer |
| "Implementations must be bounded and non-blocking beyond ctx's budget" | device.rs:29-31 | REAL-UNPROVEN | only impl is volume.rs — that file's owner must confirm boundedness |
| (no feature-matrix row cites device-discovery at all) | docs/feature-matrix.toml | — | `rg -c "device-discovery"` → 0 matches: the crate is unclaimed as well as unreachable |

## 7. Dead / duplicated / unreachable code
The dead chain, end to end: `Device`/`DeviceKind::Service`/`ServiceProtocol` (no producer) ← `device-discovery` crate (only self-tests exercise it) ← platform-{linux,macos,windows} (dependency declared, never referenced) ← nothing (no workspace member depends on platform-*). `lumen` CLI and `lumen-tauri` are not in the chain. The `DeviceKind::Volume` lane is real code (volume.rs `to_device`) with no shipped consumer.

## 8. Dependencies & flags
`media_types` only. No feature gates, no unsafe. Crate compiled clean in the workspace build (baseline #1); its tests green in baseline #3.

## 9. Open questions for the next agent
- volume.rs owner (wave-10): confirm `VolumesService::discover` honors the "absence of devices is Ok(vec![])" contract and `ctx`'s budget — device.rs's trait contract is your file's obligation.
- ssdp.rs owner: your module is cited as the reason `ServiceProtocol::Upnp` exists; if your verdict is "no devices produced", F-001's doc fix belongs with your card.
- Orchestrator/architecture wave: the three unused `device-discovery` dependency edges in platform-*/Cargo.toml are a `cargo-udeps`-class defect worth a cross-cutting finding.
