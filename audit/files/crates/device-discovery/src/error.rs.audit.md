# AUDIT: crates/device-discovery/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 285 / 11 (2 enums — 6+3 variants, 2 Display impls, `impl Error`, 2 conversion fns, 1 accessor; 5 tests + 1 helper) |
| File verdict | UNREACHABLE (correct, tested error vocabulary — but no crate or app links this crate at all) |
| Reachable from shipped entry point? | NO — stronger than the other wave-10 orphans: `rg "device-discovery = \{" --glob '**/Cargo.toml'` → ZERO dependency edges anywhere (platform-linux/macos/windows mention it only in admission-record comments; no app lists it). The crate is compiled as a bare workspace member and linked into nothing — not even transitively. Within the crate, the vocabulary IS used (capability.rs:38, ssdp.rs:132,337,376-430, volume.rs per lib.rs:106) — but only library code and its own tests execute it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg "device-discovery" --glob '**/Cargo.toml'` → comments only, no edge; `rg -n "into_engine_error" .` (scoped) → zero DiscoveryError call sites (only streaming's SchemeError namesake + docs); `rg -n "DiscoveryError::" crates/device-discovery/src` → internal users census; `sed -n` on all quoted ranges + `od -c` on line 179; `rg -n -A20 "deps/device_discovery" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → error::tests 5/5 ok at 2381-2385, crate 38 tests |

## 1. One-paragraph verdict
A small, clean, honest-degradation error vocabulary: two enums with documented per-variant semantics, redaction-disciplined Display impls, a stable dotted `context_code` accessor, and a Clone-by-design documented rationale (the fallback stores a reason and hands out copies). The five in-file tests are real for what they assert — Display content, the probe-collapse, and the engine translation's stage/code/source chain — and all green in the baseline no-fail-fast run. Three problems, none local-code bugs: the entire crate is linkage-dead (zero Cargo.toml edges — not one binary in the repo contains this code, making it the most orphaned target of my assignment), the `into_engine_error` method that the doc comment calls THE boundary-host relay has zero callers in the whole repo (the relay host does not exist), and the engine-kind mapping table both flattens the enum's own documented distinctions (a per-pass `EnumerationFailed` becomes capability-verdict `Unsupported`; local file I/O becomes `Network/ConnectionLost`) and is never asserted by any test — the suite checks stage, code, and source but never the `kind`.

## 2. What actually works (verified)
— `error::tests` 5/5 green: orchrun-test-workspace-nofailfast.log.txt:2381-2385 (blocked_on_web, capacity, engine_error_carries_stage_context_and_source, reason_from_probe_error_collapses_io, unavailable_displays_its_reason); crate suite 38 tests, 0 failed.
— Internal use verified: `UnavailableReason::from_error` called at capability.rs:38 (`Capability::Unavailable(...)`); `DiscoveryError::Io`/`MalformedResponse` constructed at real socket/parse sites (ssdp.rs:132, 337, 376-430) — the vocabulary is not decorative inside the crate.
— Panic-safety: `u64::try_from(*limit).unwrap_or(u64::MAX)` (203-204) is total; no indexing, no unwrap/expect in production paths (verified by reading).

## 3. Findings

### F-001 [S3] The crate is linkage-dead: no Cargo.toml edge anywhere, no binary contains it
Where: crates/device-discovery/src/error.rs:16-17
Code:
```rust
/// Why a discovery call failed, typed — never a bare string guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
```
What is wrong: `rg "device-discovery = \{"` over every Cargo.toml in the repo returns nothing — no crate, no app, no test workspace depends on `device-discovery`. The platform-* crates name it only in admission-record comments ("device-discovery is issue #42").
Why it matters: the services-discovery matrix rows are honest (`sd-directory-watch` etc. all not_started, evidence empty), but the repo carries a full typed-discovery library (9 modules incl. real UDP SSDP loopback proof per lib.rs docs) that cannot be executed by any shipped or test binary except its own unit tests; it will bit-rot invisibly while its vocabulary claims to be "the honest-degradation" layer.
What correct looks like: at minimum a dev-dependency from an integration test that exercises the public `ServiceDiscovery` trait through a host-shaped consumer, or the planned platform-adapter edges landing before more library grows on top.
Confidence: HIGH — Verification: ran the Cargo.toml census (header).

### F-002 [S4] `into_engine_error` — the documented boundary relay — has zero callers
Where: crates/device-discovery/src/error.rs:180
Code:
```rust
    /// Translate into the engine error currency (stage
    /// [`media_types::PipelineStage::Discovery`]) — the boundary host
    /// relays crate errors into engine error reporting through this.
```
What is wrong: repo-wide rg for `into_engine_error` finds no call site on `DiscoveryError` (only streaming's unrelated `SchemeError::into_engine_error` and historical clippy warnings for other crates' dead methods). The "boundary host" that would relay through this seam does not exist in the tree.
Why it matters: this is the same dead-seam shape the crate's own docs promise ("the relay itself is host wiring") — a public API whose entire justification is a consumer that was never written; the unit test is its only caller.
What correct looks like: wire the relay (a host translating discovery failures into `EngineError` for the catalog/event surface) or mark the method `#[allow(dead_code)]`-with-comment until that card exists — today it is neither flagged nor used.
Confidence: HIGH — Verification: repo-wide caller census (header).

### F-003 [S4] The engine-kind mapping flattens documented distinctions and is never asserted by any test
Where: crates/device-discovery/src/error.rs:184-198
Code:
```rust
            DiscoveryError::Unavailable { .. } | DiscoveryError::EnumerationFailed { .. } => {
                // "The pass could not be served here" — the typed
                // cause (and, for enumeration, the per-pass detail)
                // rides the source chain.
                ErrorKind::Unsupported { format: None }
            }
```
What is wrong: (a) `EnumerationFailed`'s own doc (26-27) says the failure "is per-pass, not a capability verdict", yet the translation maps it to `ErrorKind::Unsupported` — the capability-verdict kind — indistinguishable from `Unavailable` for any caller branching on `kind` (the source chain keeps the text, not the semantics). (b) `DiscoveryError::Io` — which per the field docs can be a filesystem probe (context: "socket/filesystem API") — maps to `Network/ConnectionLost`, a wrong class for local I/O. (c) No test asserts `err.kind` at all: `engine_error_carries_stage_context_and_source` (259-271) checks stage, code, and source text only, so any kind-row swap (Timeout↔ConnectionLost, Corrupt↔Unsupported) passes the suite.
Why it matters: the mapping table is the method's entire logic; it is both semantically lossy where the enum took care to distinguish, and untested where it distinguishes correctly.
What correct looks like: assert the kind for at least one representative of each mapping row; split `EnumerationFailed` to a transient-appropriate kind (or document the flattening in the variant doc, not just the method doc).
Confidence: HIGH — Verification: read both mapping and test; no kind assertion exists in the file.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum DiscoveryError` (6 variants) | 17-64 | REAL (unreachable) | S3 | Clone-by-design rationale documented (6-11) and honored (volume fallback); Eq derive sound (ErrorKind is Eq). |
| `enum UnavailableReason` (3 variants) | 70-91 | REAL | — | Adds the doc-09 §10 web verdict — used by ssdp web gate. |
| `UnavailableReason::from_error` | 96-108 | REAL | — | Collapse: Unavailable→clone; Io→"host I/O" DependencyMissing; other→to_string detail. The `other` branch (e.g. MalformedResponse) untested — noted in §5. |
| `Display for UnavailableReason` | 111-123 | REAL | — | Prefix conventions asserted by 2 tests. |
| `Display for DiscoveryError` | 125-150 | REAL | — | All six variants rendered; bound/attempt numbers asserted by capacity test. |
| `impl std::error::Error` | 152 | REAL | — | Blanket. |
| `DiscoveryError::context_code` | 158-167 | REAL | — | Six stable dotted codes; asserted by the engine test (`discovery.timed-out`). |
| `DiscoveryError::into_engine_error` | 180-213 | REAL (dead) | S4 | F-002/F-003; `with_source(self)` keeps the typed cause — verified by test's source assertion. |
| test helper `render` | 220-222 | REAL | — | to_string wrapper. |
| test `unavailable_displays_its_reason` | 224-236 | REAL | — | Contains-check on the reason detail. |
| test `blocked_on_web_reason_displays_the_policy` | 238-245 | REAL | — | starts_with prefix pin. |
| test `capacity_error_names_the_bound` | 247-256 | REAL | — | Both numbers present in Display. |
| test `engine_error_carries_stage_context_and_source` | 258-271 | REAL | — | Stage + code + source chain pinned; kind NOT (F-003c). |
| test `reason_from_probe_error_collapses_io` | 273-284 | REAL | — | Io→DependencyMissing("host I/O") pinned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unavailable/blockED/capacity/reason_from_probe | YES | Each pins a concrete Display fragment or variant shape. |
| engine_error_carries_stage_context_and_source | PARTIALLY | Stage, code, source yes; the kind-mapping table (the method's core) NO — F-003c. |
| (missing) `from_error` `other` branch | NO test exists | MalformedResponse/CapacityExceeded/TimedOut → "discovery integration" collapse never asserted. |
All 5 green: orchrun-test-workspace-nofailfast.log.txt:2381-2385.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Services-discovery rows (sd-directory-watch, sd-podcast, sd-upnp) status not_started/blocked, evidence "" | docs/feature-matrix.toml (~2595-2631) | REAL (honest) | The crate exists but is linkage-dead (F-001) — matrix correctly claims no capability. |
| "the boundary host relays crate errors into engine error reporting through this" | error.rs:171-172 | PAPER-ONLY | No boundary host exists; zero callers (F-002). |
| "the failure is per-pass, not a capability verdict" (EnumerationFailed) | error.rs:26-27 | FALSE at the engine boundary | F-003a: the translation maps it to capability-verdict `Unsupported`. |
| "the typed fallback … returns a copy from every method — the reason is data, not a consumed one-shot" | error.rs:6-9 | REAL | Clone derive + capability.rs:38 usage (read). |

## 7. Dead / duplicated / unreachable code
Whole crate linkage-dead (F-001). Within the crate: `into_engine_error` has no caller (F-002). No duplication; the PlatformError-mirror shape is a documented deliberate precursor to seam unification.

## 8. Dependencies & flags
std + `media-types` (L0) only (per crate Cargo.toml audited earlier: no external deps, workspace lints inherited). No feature gates, no unsafe. Redaction: this file's own strings carry no content — but `MalformedResponse { detail }` and `EnumerationFailed { detail }` embed caller-rendered strings; the discipline lives at the ssdp.rs/volume.rs call sites (flagged to their owner in §9).

## 9. Open questions for the next agent
- ssdp.rs/volume.rs owner (wave-10 A0?/A0?): verify the `detail` strings built at the MalformedResponse/EnumerationFailed construction sites embed no response bytes or paths (MP §12) — error.rs cannot police its callers.
- Whoever owns the wave-22 platform adapters: the promised `VolumeSource` implementors are the crate's only plausible route to linkage (F-001).
