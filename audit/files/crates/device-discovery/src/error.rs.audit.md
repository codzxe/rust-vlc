# AUDIT: crates/device-discovery/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Lines / functions | 285 / 6 production (2 enums, from_error, 2 Display, context_code, into_engine_error) + 5 tests |
| File verdict | REAL — UNREACHABLE (crate has zero dependents) |
| Reachable from shipped entry point? | NO (no Cargo.toml lists `device-discovery`; platform crates mention it only as future issue #42 comments; playback-engine events.rs:106 says "wired-by-later 7.D1") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/device_discovery-80b8532926b93caf` -> 38 passed / 0 failed, exit 0 (binary 18:08 > source 17:35); second prebuilt copy aa385b2b511099c2 -> 38 passed; rg dependent scan; sed verifications |

## 1. One-paragraph verdict
A small, disciplined typed-error vocabulary: six `DiscoveryError` variants with a stable dotted `context_code()` each, a Clone-by-design reason model for the fallback lane, and an engine-boundary translation that keeps the full typed cause on the source chain. Everything it claims is implemented and test-pinned (38/38 crate tests pass). The defects are classification compromises it half-admits: every I/O failure — including `PermissionDenied` on a socket bind — becomes `ErrorKind::Network{ConnectionLost}` at the engine boundary, and `UnavailableReason::from_error` collapses timeouts, capacity breaches and malformed responses into "DependencyMissing", which is a mislabel for a slow peer or a bound. The crate ships nowhere.

## 2. What actually works (verified)
— `./target/debug/deps/device_discovery-80b8532926b93caf` -> `38 passed; 0 failed` (0.15s), includes this file's 5 error tests. The `engine_error_carries_stage_context_and_source` test asserts stage=Discovery, code="discovery.timed-out", and the source chain renders the original — the translation is genuinely lossless at the source level.

## 3. Findings

F-001 [S4]
Where: crates/device-discovery/src/error.rs:193-195
Code:
```
            DiscoveryError::Io { .. } => ErrorKind::Network {
                cause: media_types::error::NetworkCause::ConnectionLost,
            },
```
What is wrong: any I/O failure (bind EPERM, NotFound, ENOMEM) is reported as Network/ConnectionLost. The doc comment (L173-178) admits the seven-kind taxonomy forces a "nearest fixed class" choice, but `Inaccessible` exists in the same taxonomy (see transcode error.rs) and would be nearer for EACCES/EPERM. Source chain keeps the truth, so this is a classification smudge, not data loss. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/device-discovery/src/error.rs:103-106
Code:
```
            other => UnavailableReason::DependencyMissing {
                what: "discovery integration",
                detail: other.to_string(),
            },
```
What is wrong: `TimedOut` (a slow peer), `CapacityExceeded` (a local bound) and `MalformedResponse` (a dirty peer) all collapse to "DependencyMissing" — a capability verdict ("this lane cannot work here") for conditions that are per-pass, not per-capability. The doc for `EnumerationFailed` (L24-25) itself distinguishes per-pass from capability verdicts; this collapse erases that distinction in the fallback reason. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/device-discovery/src/error.rs:199-205
Code:
```
            DiscoveryError::CapacityExceeded {
                limit, attempted, ..
            } => ErrorKind::ResourceLimit {
                limit: LimitKind::QueueSize,
```
What is wrong: every capacity breach maps to `LimitKind::QueueSize` even when the bounded thing is a "volume list" (the enum's own doc example) — the limit kind is wrong for non-queue bounds; budget/attempted are carried correctly. Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 300 LOC; table for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DiscoveryError enum | 16-64 | REAL | — | 6 variants, Clone, typed bounds (what/limit/attempted) |
| UnavailableReason enum | 69-91 | REAL | — | NotYetImplemented/BlockedOnWeb/DependencyMissing; web-target policy documented |
| UnavailableReason::from_error | 93-109 | REAL | S4 | F-002 collapse |
| Display (UnavailableReason) | 111-123 | REAL | — | test-pinned prefixes |
| Display (DiscoveryError) | 125-150 | REAL | — | all 6 variants rendered; capacity names both numbers (tested) |
| Error impl | 152 | REAL | — | |
| context_code | 154-167 | REAL | — | 6 stable dotted codes, additive-only surface |
| into_engine_error | 179-213 | REAL | S4 | F-001/F-003 mapping compromises; source chain verbatim (`.with_source(self)`) — test-pinned |
| tests (5) | 216-285 | REAL | — | Display content, stage/code/source, io-collapse; only TimedOut's engine mapping tested (5 of 6 kinds unpinned) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unavailable_displays_its_reason | YES | substring of nested reason in output |
| blocked_on_web_reason_displays_the_policy | YES | exact prefix |
| capacity_error_names_the_bound | YES | both numbers present |
| engine_error_carries_stage_context_and_source | YES | exact stage + code + source content |
| reason_from_probe_error_collapses_io | YES | exhaustive match on collapse result |
Gap: `into_engine_error` mappings for Unavailable/EnumerationFailed/MalformedResponse/Io/CapacityExceeded are not pinned by any test (only TimedOut is).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the reason is data, not a consumed one-shot" (Clone on purpose) | error.rs:6-8 | REAL | derive(Clone); volume.rs fallback stores/copies (per doc; volume.rs is another agent's file) |
| "codes are additive-only surface" | error.rs:155-157 | REAL-UNPROVEN | nothing ships the crate, so no compat surface exists yet |
| "the precise typed cause always rides the source chain (`self`, verbatim)" | error.rs:174-176 | REAL | `.with_source(self)` L212 + test asserts source renders original text |
| "never a bare string guess" | error.rs:15 | REAL | all variants carry structured fields (though `detail: String` at failure sites — SSDP datagram-bound, see ssdp.rs:75 read bound) |

## 7. Dead / duplicated / unreachable code
None dead in-file. Crate UNREACHABLE from shipped entry points (zero dependents; issue #42 future wiring per platform-crate manifest comments).

## 8. Dependencies & flags
media-types (EngineError/ErrorKind/NetworkCause/LimitKind/PipelineStage), std only otherwise. No features, no unsafe.

## 9. Open questions for the next agent
- When the L5/L2 seam unification lands (doc header L2-4), will `from_error`'s collapse (F-002) be the merged behavior? Flag before wiring.
- ssdp.rs / volume.rs / monitor.rs (other agents) — verify every `MalformedResponse`/`EnumerationFailed` detail is bounded (datagram cap confirmed for ssdp; volume.rs:138 unverified here).
