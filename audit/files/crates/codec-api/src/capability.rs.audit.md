# AUDIT: crates/codec-api/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 353 / 21 items (8 prod types+methods, 1 helper fn, 3 tests) |
| File verdict | UNREACHABLE (fully implemented and tested vocabulary; zero production consumers) |
| Reachable from shipped entry point? | NO — `rg "CodecCapability\|ProfileDescriptor\|LevelDescriptor\|DimensionLimits\|HwPreference"` across crates/ + apps/ hits exactly 3 files: this file, codec-api/src/lib.rs (module decl + re-exports), codec-api/tests/contract_suite.rs:714 (`assert_send::<CodecCapability>()`). No codec implementation constructs a capability; no registry exists to match one. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg for all five public type names repo-wide (consumers listed above); rg "CodecCapability::new" (zero non-test construction sites); rg "capability" baseline test log (3 unit tests ok :240-242); read of lib.rs re-export block; spec-bound check on MAX_DIMENSION vs AV1/VP9 (below) |

## 1. One-paragraph verdict
A small, clean, honestly-scoped vocabulary module: opaque profile/level strings with shared validation, per-axis dimension limits with a 16-bit cap, an ADR-001 hardware-posture enum, and a capability struct with three pure predicates — "absence is not rejection" semantics throughout. The three unit tests are real (all ran green in baseline :240-242) and would fail under the obvious mutations. The problem is that none of it is used: the registry that card 1.D1-R designed this for (doc 01 §2.5(b) `register()`) never landed — every decoder in the tree is hand-wired in `apps/cli/src/pipeline_host.rs:330-377` with hard-coded `match` arms, not capability matching — so this entire public API is dead weight compiled into the shipped `codec-api` dependency, reachable only from its own tests and one `assert_send` line. Two vocabulary-level nits: control-character rejection is ASCII-only while the doc promises "no control characters", and the 65,535-px cap claim ("covers every codec family's documented maximum") is off by one against AV1/VP9, whose headers allow 65,536.

## 2. What actually works (verified)
— `rg -n "capability" audit/runs/baseline/test-workspace.log.txt` → `capability::tests::{opaque_descriptors_reject_empty_and_control_bytes, dimension_limits_validate_per_axis, capability_predicates_treat_absence_as_unconstrained}` all `ok` (:240-242).
— Validation logic re-derived by auditor: `DimensionLimits::new` enforces `1..=65535` and `min ≤ max` per axis independently (width pair and height pair) — correct; `accepts` is the exact conjunction of the four bounds — correct; `supports_profile`/`supports_level` empty-list-accepts-anything matches the documented "absence is not rejection" contract.

## 3. Findings

### F-001 [S3] Entire capability vocabulary has zero production consumers — designed for a registry that does not exist
Where: crates/codec-api/src/capability.rs:1-256; evidence lib.rs:107/115/130, contract_suite.rs:714
Code:
```
//! Descriptors are *claims* made by implementations and matched by the
//! registry; this crate defines the shape and pure predicates only.
//! Selection policy (priority, enable/disable, user preference) is the
//! registry's (MP §6) — deliberately absent here.
```
What is wrong: No implementation makes the claim and no registry does the matching. Repo-wide rg for the five public type names finds only the defining file, the lib.rs re-export block, and one `assert_send::<CodecCapability>()` in the contract test. No decoder (mjpeg, h264 sw, av1, vp9, flac, …) constructs a `CodecCapability`; decoder selection in the shipped pipeline is a hard-coded `match` on codec-name strings in `apps/cli/src/pipeline_host.rs` ("h264" | "avc1" => H264SwDecoder::new, etc.). The `register()` registry surface this module was built for is explicitly a future card (crates/codecs/Cargo.toml:19-30 "plugin-api joins when the registry/`register()` surface lands").
Why it matters: ~250 lines of public API in a shipped dependency (apps/cli/Cargo.toml:50 codec-api) that nothing exercises — the classic designed-ahead-of-need pattern. Worse for audit economics: its predicates (profile/level/dimension matching) LOOK like the mechanism that guards decode selection, inviting a reader to believe a 4096×2160 refusal or a profile mismatch is capability-gated somewhere. It is not; nothing reads these predicates in production.
What correct looks like: Either wire it (decoders declare capabilities; the pipeline host consults them) or move it behind a `registry` feature until that card lands.
Confidence: HIGH — Verification: read + exhaustive rg (all crates, all apps, non-md).
### F-002 [S4] Control-character rejection is ASCII-only; doc promises "no control characters"
Where: crates/codec-api/src/capability.rs:87-92 (and the claim at :30-33)
Code:
```
fn validate_opaque(value: &str, code: &'static str) -> Result<String, EngineError> {
    if value.is_empty() || value.chars().any(|c| c.is_ascii_control()) {
        return Err(unsupported(code));
    }
    Ok(value.to_string())
}
```
What is wrong: `is_ascii_control` covers U+0000–U+001F and U+007F only. Unicode control-ish characters — U+0085 (NEL), U+2028/U+2029 (line/paragraph separators), and bidi overrides like U+202E — pass validation into strings the struct docs call "already user-renderable" (:47, :74).
Why it matters: A bidi override in a profile name renders backwards in any UI ("high" + U+202E + junk). Today's only minters are in-tree developers, so the spoofing surface is theoretical — S4 — but the doc overstates the guarantee.
What correct looks like: `c.is_control() || c == '\u{2028}' || …` or `unicode_normalization`-grade filtering, or weaken the doc to "no ASCII control characters".
Confidence: HIGH — Verification: read; `char::is_ascii_control` semantics are std-documented.
### F-003 [S4] "65 535 px covers every codec family's documented maximum" is false for AV1 and VP9 (both allow 65,536)
Where: crates/codec-api/src/capability.rs:97-101, 112
Code:
```
/// vocabulary bound, not a performance promise `[PROPOSAL]` — 65 535 px
/// covers every codec family's documented maximum (containers store
/// dimensions as 16-bit fields) while keeping absurd claims out of the
/// registry.
```
What is wrong: AV1 and VP9 code their frame dimensions as 16-bit "minus one" fields, so their spec maxima are 65,536×65,536, not 65,535. The rationale conflates container fields (16-bit raw, max 65,535 — correct) with codec maxima. A legal-per-spec 65,536-wide AV1 stream cannot be represented by any `DimensionLimits`.
Why it matters: Zero practical impact (no such streams exist in any real corpus, and the module has no consumers — F-001), but the stated coverage claim is checkably wrong, and this repo's whole posture is checkable claims.
What correct looks like: Cap at 65,536, or state "covers every container-storable dimension; AV1/VP9's spec maximum 65,536 is excluded deliberately".
Confidence: HIGH — Verification: read; AV1 seq_header `max_frame_width_minus1` u(16) and VP9 `frame_width_minus_1` u(16) are public spec syntax (auditor knowledge; no spec text in sandbox — UNVERIFIED against the spec documents themselves).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `unsupported` (helper) | 18-24 | REAL | — | Unsupported/Decode-stage error mint; consistent with module doc. |
| `ProfileDescriptor` + `new`/`as_str`/`Display` | 34-57 | REAL | S4 | Wrap/validate/render; ASCII-only control check (F-002). |
| `LevelDescriptor` + `new`/`as_str`/`Display` | 61-84 | REAL | S4 | Structural twin of ProfileDescriptor (duplicated bodies, acceptable for two distinct vocabularies). |
| `validate_opaque` | 87-92 | REAL | S4 | Shared validation; F-002. |
| `DimensionLimits` (struct) | 102-108 | REAL | — | Private fields — sound invariant (min≤max, 1..=cap) enforced at construction. |
| `DimensionLimits::MAX_DIMENSION` | 112 | REAL | S4 | 65,535; coverage claim off-by-one vs AV1/VP9 (F-003). |
| `DimensionLimits::new` | 120-141 | REAL | — | Per-axis validation verified correct (re-derived). |
| `min_width`/`min_height`/`max_width`/`max_height` | 144-161 | REAL | — | Plain accessors. |
| `DimensionLimits::accepts` | 165-170 | REAL | — | Correct conjunction; test pins boundary (1921 rejected at max 1920). |
| `HwPreference` (enum) | 182-196 | REAL | — | 3 variants; docs reference product.toml policy accurately (product.toml:191 `hw_decode_policy = "software-only"` exists). |
| `CodecCapability` (struct) | 206-220 | REAL | — | Public fields (profiles/levels mutable by design — "data, not policy"). |
| `CodecCapability::new` | 225-233 | REAL | — | Minimal software capability. |
| `supports_profile` | 237-239 | REAL | — | Empty-list-accepts-all; documented. |
| `supports_level` | 243-245 | REAL | — | Same. |
| `accepts_dimensions` | 250-255 | REAL | — | None → true; documented. |
| test `opaque_descriptors_reject_empty_and_control_bytes` | 262-282 | REAL | — | Ran ok (:240); would fail if validation loosened ("" / "\n" / "high\x00" all asserted rejected). |
| test `dimension_limits_validate_per_axis` | 284-318 | REAL | — | Ran ok (:241); pins zero/over-cap/inverted rejections + cap legality. Mutation: drop `min <= max` → the (1921,16,1920,1088) case fails. |
| test `capability_predicates_treat_absence_as_unconstrained` | 320-352 | REAL | — | Ran ok (:242); pins empty-accepts-all AND declared-narrows (professional profile rejected, 4096 rejected at max 3840). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| opaque_descriptors_reject_empty_and_control_bytes | YES | Exact error kind + code asserted for three bad inputs; equality round-trip checked. |
| dimension_limits_validate_per_axis | YES | Six invalid tuples + boundary acceptance; each rejection asserts the typed code. |
| capability_predicates_treat_absence_as_unconstrained | YES | Both the unconstrained default and the narrowed (declared) behavior asserted with concrete accept/reject pairs. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Descriptors are claims made by implementations and matched by the registry" | capability.rs:5-6 | PAPER-ONLY | No implementation claims, no registry matches (F-001) — the sentence describes a system that does not exist. |
| "no control characters" (validation posture) | capability.rs:30-33 | SHALLOW | ASCII control only (F-002). |
| "65 535 px covers every codec family's documented maximum" | capability.rs:98-99 | FALSE | AV1/VP9 spec max is 65,536 (F-003). |
| "Absence is not rejection" (empty lists accept anything) | capability.rs:202-205 | REAL | Implemented (:237-255) and test-pinned (:328-329). |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-001): compiled into codec-api (a shipped dep), referenced by re-exports nobody imports and one Send assertion. `ProfileDescriptor`/`LevelDescriptor` are structural duplicates of each other (new/as_str/Display ×2) — acceptable as two distinct vocabularies, but a macro or generic would halve it.

## 8. Dependencies & flags
Depends only on media-types (error types, CodecDescriptor) — in-workspace, no externals, no unsafe, no features. Compiled unconditionally as part of codec-api (`pub mod capability;` lib.rs:107). Wave-6 hostile question: contributes nothing to shipped H.264 reachability — it is the vocabulary a future registry would have used.

## 9. Open questions for the next agent
- If a later wave wires `register()`/the registry (doc 01 §2.5(b)): does decoder selection then migrate from pipeline_host.rs's string match to capability matching? That migration is the test of whether this module was worth its card.
- media-types' `CodecDescriptor` — same ASCII-only control validation? (F-002 pattern may repeat there.)
