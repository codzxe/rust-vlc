# AUDIT: crates/media-types/src/error.rs
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2025-06-27 |
| Lines / functions | 538 / 17 (9 types, 2 ctors, 2 impls, 4 fns) + 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli/src/pipeline_host.rs:153,1496,1523 constructs `media_types::error::ErrorContext`/`EngineError` on the open/decode path; also apps/web/src/source.rs:168, apps/desktop/src/bridge.rs:268, crates/input/src/error.rs:35, crates/codecs/src/audio/{vorbis,opus,flac}/error.rs (rg, 20+ files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p media-types` → 77 unit (76 pass, 1 ignored) + 6 + 2 integration, exit 0; `cargo test -p media-types error::` → 0 matched (filter), full run covers them; rg variant-usage census (22 variants) |

## 1. One-paragraph verdict

This is a real, disciplined error taxonomy: 14 pipeline stages, the seven MP §8
error kinds with structured detail enums, dotted stable context codes enforced
at construction, and a `Display` that is redaction-safe by construction
(structured data only; sniffed bytes capped at 16 hex pairs — test-pinned). I
ran the crate's tests myself: all error-module tests pass. It is consumed
across the whole tree (input, codecs, playback-engine, all three hosts). The
defects are hygiene-level: three taxonomy variants have zero constructors
anywhere (dead vocabulary), the `FormatInfo.sniffed` buffer is unbounded in
memory (only `Display` caps it), and the constructor-contract `assert` in
`ErrorContext::new` is one leaked non-literal away from a production panic.

## 2. What actually works (verified)

— `cargo test -p media-types` → 77 unit tests (76 passed, 1 ignored) + 6 + 2
integration, exit 0. Error-module tests observed passing:
`all_seven_kinds_display_structured_summaries`, `display_is_stage_bracketed_with_code`,
`sniffed_bytes_are_capped_at_sixteen_in_display`, `source_chain_delegates_to_std`,
`stage_names_are_stable_lowercase`, `context_codes_enforce_their_shape`.
— Variant-usage census (rg over crates/ + apps/, excluding this file):
`LimitKind::Memory` 31 files, `DecodeCause::BadState` 21, `PipelineStage::Control` 7,
`NetworkCause::ConnectionLost` 6 — the taxonomy is live, not decorative.

## 3. Findings

F-001 [S3]
Where: crates/media-types/src/error.rs:100-111,145-152,171-186
Code:
```rust
    /// Another process holds an exclusive lock.
    Locked,
```
```rust
    /// The GPU adapter backing presentation.
    Adapter,
```
```rust
    /// WASM extension memory ceiling.
    WasmMemory,
```
What is wrong: `AccessDenied::Locked`, `DeviceSlot::Adapter`, and
`LimitKind::WasmMemory` have ZERO constructors outside this file (rg census:
0 files each; every other variant has 2-31). They are vocabulary for
subsystems that do not exist yet, so every consumer match arm handles states
that cannot occur and tests can never exercise them.
Why it matters: dead enum variants inflate the "implemented surface" claim of
the taxonomy and mislead readers (and the feature matrix) into believing file
locking, GPU-adapter device errors, and WASM memory ceilings are detected
somewhere.
What correct looks like: either land the producing subsystem or gate the
variants behind the crate's own TODO ledger; at minimum document in the enum
that the variants are reserved.
Confidence: HIGH — Verification: ran rg census (`rg -l "AccessDenied::Locked" crates apps --glob '!*/error.rs'` → 0).

F-002 [S4]
Where: crates/media-types/src/error.rs:239-252
Code:
```rust
    #[allow(clippy::panic)]
    pub fn new(code: &'static str) -> ErrorContext {
        let valid = !code.is_empty()
            && code.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_'
            });
        assert!(
            valid,
            "error codes are lowercase dotted literals: got {code:?}"
        );
```
What is wrong: the guard is honest and documented ("every in-crate call site
passes a literal" — confirmed by sampling apps/web/src/source.rs:168,
apps/cli/src/pipeline_host.rs:1496,1523, crates/codecs/.../error.rs), but the
signature accepts ANY `&'static str`, including runtime strings promoted via
`Box::leak`/`once_cell`, which would panic in production rather than degrade.
Why it matters: an assert-based panic on a data-shaped input is exactly the
class checklist 6.2 flags; today it is unreachable, tomorrow it is a crash.
What correct looks like: return a `Result`/fallible constructor plus a
`new_unchecked` for literals, or debug_assert + fallback code.
Confidence: MEDIUM — Verification: read (call-site sampling, not exhaustive).
Note: workspace lints deny `panic!` — the `#[allow(clippy::panic)]` escape is
the crate's single sanctioned one (documented at :233-238).

F-003 [S4]
Where: crates/media-types/src/error.rs:74-81
Code:
```rust
pub struct FormatInfo {
    /// Demuxer's name for the container when one scored above threshold.
    pub name: Option<String>,
    /// The leading bytes that were sniffed (content, not credentials —
    /// `Display` caps them at 16 bytes of hex).
    pub sniffed: Vec<u8>,
}
```
What is wrong: `sniffed` is an unbounded `Vec<u8>`; only `Display` truncates
to 16 bytes (:322 `&info.sniffed[..info.sniffed.len().min(16)]`). An
over-eager probe that attaches a large buffer pays that memory on every
carried error (EngineError is Clone-able via kind).
Why it matters: bounded-by-doc-comment, not bounded-by-type; the cap lives in
the wrong layer.
What correct looks like: cap at construction (`Vec::truncate(16)` or
`[u8; 16]` + len) so the invariant is structural.
Confidence: HIGH (cap location verified by read; no producer currently
attaches >16 bytes — unverified that all future probes won't).
Mitigating: probe depth itself is budgeted elsewhere (LimitKind::ProbeDepth
consumers exist, 2 files).

## 4. Per-function table

| Function/Type | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PipelineStage` (enum, as_str, Display) | 13-70 | REAL | — | 14 stages, kebab names test-pinned; all variants used (census ≥2) |
| `FormatInfo` | 74-81 | REAL | S4 | unbounded `sniffed` (F-003) |
| `CorruptDetail` | 84-96 | REAL | — | 5 variants; ChecksumMismatch used (4 files) |
| `AccessDenied` | 98-111 | REAL | S3 | `Locked` dead (F-001) |
| `NetworkCause` | 113-128 | REAL | — | Tls used (3 files); TLS "never retried on mismatch" comment is policy, not code here |
| `DecodeCause` | 130-141 | REAL | — | all used |
| `DeviceSlot` | 143-152 | REAL | S3 | `Adapter` dead (F-001) |
| `DeviceCause` | 154-165 | REAL | — | all used |
| `LimitKind` | 167-186 | REAL | S3 | `WasmMemory` dead (F-001) |
| `ErrorKind` | 189-214 | REAL | — | exactly the 7 MP §8 names; mirrors doc 05 §2.13 media half |
| `ErrorContext::new` | 227-253 | REAL | S4 | assert guard (F-002); shape test via catch_unwind |
| `EngineError::new/with_source` | 266-287 | REAL | — | single chain root, documented |
| `Display for EngineError` | 289-297 | REAL | — | `[stage] summary (code)`; test-pinned exact string |
| `EngineError::message_summary` | 299-308 | REAL | — | used by playback-engine event.rs + recovery_env tests |
| `kind_summary` | 313-397 | REAL | — | structured only; 16-byte sniff cap test-pinned |
| `impl std::error::Error` | 399-405 | REAL | — | delegates source chain (test-pinned) |
| tests (7) | 407-538 | REAL | — | exact-string asserts; honest (see §5) |

## 5. Test honesty

| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all_seven_kinds_display_structured_summaries | YES | asserts exact rendered strings for all 7 kinds incl. sniffed hex |
| display_is_stage_bracketed_with_code | YES | exact full-Display equality |
| sniffed_bytes_are_capped_at_sixteen_in_display | YES | feeds 4096 bytes, asserts ≤16 pairs |
| source_chain_delegates_to_std | YES (weakly) | only asserts `is_some`, not the chain content |
| stage_names_are_stable_lowercase | YES | exact as_str equality (3 spot pins, not all 14) |
| context_codes_enforce_their_shape | YES | catch_unwind proves the malformed-code guard fires |

Mutation reasoning: replacing the sniff cap with `len()` fails test 3; swapping
any summary string fails tests 1-2. No tautologies.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Kinds are exactly the seven MP §8 names" | error.rs:3 / doc 01 §3.4 | REAL | ErrorKind 7 variants; command-layer twin commands/src/error.rs:63-103 mirrors them; bridge playback-engine/src/event.rs wire_error_code maps 1:1 |
| "Display never embeds credentials, cookies, auth headers or full user paths (MP §12)" | error.rs:4-7 | REAL | Display prints stage + structured summary + code only (:289-297); no URI field exists on EngineError; URIs only via SourceUri::redacted (ids.rs) |
| doc 05 §2.13 wire alignment: engine errors surface as the seven canonical codes | docs/05:344-352 | SHALLOW | The 1:1 kind bridge is real (wire_error_code), but seek refusals are remapped: `SeekFailure::NotSeekable => (Control, Unsupported)` (playback-engine/src/event.rs seek_error_wire) — doc 05 §2.4's `not-seekable` code never reaches the wire; `commands::ErrorCode::NotSeekable` has no production constructor |

## 7. Dead / duplicated / unreachable code

- Dead variants: `AccessDenied::Locked`, `DeviceSlot::Adapter`,
  `LimitKind::WasmMemory` (F-001).
- `commands::ErrorCode::NotSeekable` (consumer side of this taxonomy's
  bridge) is constructed only in tests — see §6; the dead mapping also lives
  at apps/cli/src/exit.rs:56,128.
- No duplication: this is the L0 taxonomy; the command layer
  (crates/commands/src/error.rs) intentionally re-declares the wire face
  (comment at commands/error.rs:8-12 documents the split).

## 8. Dependencies & flags

- No external deps (std only + schemars absent here — this type is NOT
  serde-serialized; the wire face is the commands crate's). No feature gates.
- Layering: L0 `media-types`, consumed upward by input/codecs/engine/hosts —
  consistent with boundaries.toml allowlist (check-architecture gate).

## 9. Open questions for the next agent

1. playback-engine agent: confirm whether any seek path ever produces
   `commands::ErrorCode::NotSeekable`/`OutOfRange` on the wire (I found the
   opposite mapping in event.rs seek_error_wire — doc 05 §2.4 violation).
2. Who attaches `FormatInfo.sniffed` in production, and with how many bytes?
   (media-probe agent; F-003 severity depends on it.)
3. The 1 ignored test in media-types (`cargo test -p media-types` → 1
   ignored) — not in this module; owner should confirm it is intentionally
   ignored (baseline notes 51 ignored workspace-wide).
