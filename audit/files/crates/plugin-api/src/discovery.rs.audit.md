# AUDIT: crates/plugin-api/src/discovery.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-21 |
| Lines / functions | 199 / 8 (+3 tests, 1 mock impl) |
| File verdict | UNREACHABLE (staged contract: zero implementors, zero host dispatch, zero callers of its error constructor) |
| Reachable from shipped entry point? | NO — triple-orphaned: (1) no implementor of `plugin_api::discovery::ServiceDiscovery` exists outside this file's test mock (`rg "impl ServiceDiscovery"` repo-wide → `device-discovery/src/volume.rs:228` implements the OTHER, domain trait `device_discovery::device::ServiceDiscovery`, and `discovery.rs:124` is the mock); (2) plugin-host-wasm never dispatches it — `rg discover/snapshot/changes` over plugin-host-wasm/src hits only a limits.rs:47 comment and the unrelated `lumen:v1.devices`/`devices_list` query import (imports.rs:865+); (3) the only shipped consumer of plugin-api, lumen-cli, uses Demuxer/Access/SourceStream (bench.rs:66, demux_host.rs:61, containers/*) — never discovery. `claim_mismatch` has zero callers outside its own test. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -rn "ServiceDiscovery\|DiscoveryChange\|claim_mismatch\|discovery::Device"` over crates/ apps/ tools/ tests/ (caller/implementor enumeration above); `sed -n '20,45p' crates/device-discovery/src/device.rs` (the two traits compared — different signatures: `discover(ctx)` vs `snapshot()`/`changes()`); `grep -n "plugin" apps/cli/Cargo.toml apps/desktop-tauri/Cargo.toml` (CLI depends on plugin-api only; tauri on neither); `rg "plugin_host_wasm"` over all Cargo.tomls (zero dependents — the wasm host itself is orphaned); `sed -n '130,165p' crates/plugin-api/src/kind.rs` (ServiceDiscovery IS extension_allowed_in_v1 + AppSingleton); `sed -n '55,80p' crates/plugin-api/src/codec.rs` (twin error compared); baseline: `orchrun-test-workspace-nofailfast.log.txt` (3/3 discovery tests `ok`; note plain baseline test runs #3/#4 never reached plugin-api — alphabetical fail-fast stop at desktop/first-vertical) |

## 1. One-paragraph verdict
A cleanly shaped, correctly typed host contract for service-discovery plugins — that nothing implements, nothing dispatches, and nothing calls. The trait's design is defensible (engine-driven polling, bounded passes, absence-as-data), its three tests pass in the orchestrator's no-fail-fast baseline run, and its doc admits the staging ("[PROPOSAL: placement, doc 06 §10 reconciliation]"). But the wave's hostile question gets a hard answer from this file: the ServiceDiscovery plugin kind is registry-allowed (`extension_allowed_in_v1` includes it, kind.rs:154-163) yet no discovery plugin exists (the only example extensions are metadata-provider and ui-panel), the wasm host never invokes `snapshot()`/`changes()`, and the claim-enforcement helper `claim_mismatch` — the module's only executable logic — has zero call sites. This is a contract waiting for its card, compiled into a crate whose other contracts (Demuxer/Access) the CLI does exercise; discovery is the unwritten half.

## 2. What actually works (verified)
— `discovery::tests::{snapshot_and_change_drain, absence_is_data, claim_mismatch_error_shape}` → all `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (plugin_api unittest section). These prove the trait is object-dispatchable, the drain-then-None cadence works, absence is `Ok(vec![])`, and the mismatch error carries its claim through `Error::source`.
— The two-trait split is real and documented on both sides (device.rs:26-28 names plugin-api's trait as "the HOST surface"; this file names device-discovery's as the domain trait) — not accidental duplication.

## 3. Findings

### F-001 [S3]
Where: crates/plugin-api/src/discovery.rs:47-70
Code:
```rust
pub trait ServiceDiscovery: Send + Sync {
    fn discovers(&self) -> Vec<&'static str>;
```
What is wrong: the contract is completely unwired. No production implementor exists (the sole `impl` is the in-file test mock at l.124); plugin-host-wasm contains no dispatch — its only discovery-adjacent surface is `lumen:v1.devices`/`devices_list`, the query direction (plugins ASKING about devices, imports.rs:863-897), never this trait's provide direction; `claim_mismatch` (l.86-97) — the doc-06-§7.2 "capability honesty" enforcement — has zero callers, so a manifest `discovers` lie is currently unenforced anywhere.
Why it matters: the plugin kind `ServiceDiscovery` is loadable per the v1 registry rule (kind.rs:154-163), so a manifest can claim it, and the resulting plugin would expose… nothing: no host path calls it. The "Allowed as a WASM extension in v1 (§1.4)" module-doc claim (l.5) is true at the registry layer and hollow at the execution layer.
What correct looks like: land the host-side dispatch with the discovery card (poll task → snapshot/changes on the sandboxed instance, claim_mismatch at registration), or mark the kind builtin/not-yet in the registry rule so manifests cannot claim an inert kind.
Confidence: HIGH — Verification: read + repo-wide rg enumerations (documented in the header table).

### F-002 [S4]
Where: crates/plugin-api/src/discovery.rs:86-93
Code:
```rust
pub fn claim_mismatch(claimed: &str) -> EngineError {
    EngineError::new(
        PipelineStage::Discovery,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
```
What is wrong: a manifest capability lie (claiming to discover `mdns` while serving nothing) is typed as `Corrupt{InvalidStructure{offset: None}}` — the kind reserved for malformed byte structures. There is no structure and no offset; the real identity lives entirely in the code string `plugin.discovery.claim-mismatch`.
Why it matters: taxonomy stretch: consumers branching on ErrorKind will treat a capability lie as data corruption. The twin `codec_claim_mismatch` (codec.rs:62-74) makes the same choice at stage `Plugin`, so this is a consistent crate convention — but consistency in a stretch is still a stretch.
What correct looks like: an ErrorKind meaning "capability/claim violation" (or Unsupported with the claim in context), with the stage carrying the subsystem.
Confidence: HIGH — Verification: read (both constructors compared).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Device` (struct) | 24-36 | REAL (data, unused) | — | four plain String fields; data-not-UI-copy discipline documented; no validation of `uri` (SourceUri parse is the Access layer's job per doc). |
| `ServiceDiscovery::discovers` | 51 | REAL (contract, undispatched) | — | Vec<&'static str> allocation per call — fine at polling cadence. |
| `ServiceDiscovery::snapshot` | 61 | REAL (contract, undispatched) | — | bounded-pass contract documented; absence-is-data rule stated (MP §8). |
| `ServiceDiscovery::changes` | 69 | REAL (contract, undispatched) | — | drain semantics, None = nothing new. |
| `DiscoveryChange` (enum) | 73-82 | REAL (data, unused) | — | Added/Removed{id}; plain. |
| `claim_mismatch` | 86-97 | REAL (dead) | S4 | F-002 taxonomy; zero callers. |
| `Mismatch` + `Display` + `Error` | 99-113 | REAL (dead) | — | carries the claim; shape test-proven. |
| `UpnpMock` impl (test) | 119-141 | REAL | — | poison-recovering lock, take-drain — correct mock. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `snapshot_and_change_drain` | YES (for the contract shape) | exercises trait-object dispatch, one-event drain, second-drain None. It tests the MOCK against the trait — no production code under test exists; it pins the contract's ergonomics, not any behavior. |
| `absence_is_data` | YES (same scope) | empty snapshot is Ok(vec![]). |
| `claim_mismatch_error_shape` | YES | stage assert + source-string contains the claim. |
Scope caveat: all three tests can only fail against this file's own definitions — there is no production implementation they could catch a break in. They are contract-shape pins, honestly useful the day a real impl lands.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Allowed as a WASM extension in v1 (§1.4)" | module docs l.5 | TRUE-at-registry / HOLLOW-at-execution | kind.rs:154-163 includes ServiceDiscovery in extension_allowed_in_v1; no host dispatch exists (F-001). |
| "this minimal host surface is staged here [PROPOSAL: placement…]" | module docs l.3-5 | honest | the staging is admitted. |
| "enforced by the sandbox for extensions" (manifest `discovers` capabilities) | module docs l.12-13 | FALSE today | claim_mismatch has zero callers; nothing compares a manifest's `discovers` against the provider's (F-001). |
| "domain trait: `device_discovery::ServiceDiscovery` (L2)" | module docs l.2-3 | REAL | device.rs:26-45 defines it; VolumesService implements it (volume.rs:228) — separate trait, correctly referenced. |

## 7. Dead / duplicated / unreachable code
- The entire module is production-dead (F-001); mock is cfg(test).
- Cross-crate duplication of the WORD ServiceDiscovery (two traits by design, both sides document the split — acceptable, noted for completeness).

## 8. Dependencies & flags
- media-types only (+ std). No feature gates. Crate-level hygiene note: `cargo fmt --check` fails on `crates/plugin-api/tests/testkit_wiring.rs` (baseline INDEX row 7) — same crate, different file; recorded here as crate context.

## 9. Open questions for the next agent
- A03 (plugin-host-wasm state.rs / imports.rs part2): confirm no dispatch table row ever constructs a ServiceDiscovery instance — F-001's "no dispatch" claim rests on the greps above.
- Whoever audits `device-discovery` (later wave): VolumesService implements the domain trait — is IT reachable from the CLI? (This file's audit only covers the plugin-api side.)
- The plugins/ asset question (my w12-g001): both example extensions are non-discovery kinds — consistent with F-001.
