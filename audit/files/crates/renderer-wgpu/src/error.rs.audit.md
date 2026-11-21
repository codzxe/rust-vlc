# AUDIT: crates/renderer-wgpu/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 417 / 8 items (2 enums + 5 impls + alias + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `apps/cli/Cargo.toml:60` depends on renderer-wgpu; `apps/cli/src/pipeline_host.rs:913/:917/:918/:1321` call `RendererError::to_engine_error()` on the screenshot/present paths (`screenshot::encode_rgba(extent, &rgba).map_err(|err| err.to_engine_error())` :918). Variants minted in `staging.rs:269-300`, `surface.rs`, `renderer.rs`. Executed: baseline renderer_wgpu 60/60 unit tests incl. all 3 of this file's tests (orchrun log :4767-4781); the AVI smoke lane's screenshot (final-frame evidence) rides this fold. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` full read + quote verification (:270-278, :188-200) · `rg -n 'RendererError\|to_engine_error' apps/cli/src` (:913-918, :1321 — shipped callers) · `rg -n 'RendererError' crates/renderer-wgpu/src -l` (7 sibling consumers) · `rg -n 'enum CorruptDetail' -A 12 media-types/src/error.rs` (:85-96 — no Oversized variant: the fold vocabulary is forced) · `sed -n '4766,4800p' orchrun…log` (renderer_wgpu 60/60; error::tests 3/3 pass) |

## 1. One-paragraph verdict
A tight, complete typed-error vocabulary: 10 variants, each mapped four ways (stage tag, stable dotted code, engine fold, Display) with a full-coverage test that ran and passed in the baseline, and the fold is genuinely consumed by the shipped CLI's screenshot/present paths. The two defects are small: the "every error" test quietly omits one of the eleven code mappings (`ZeroExtent { from_frame: true }` → `render.frame.zero-extent` is never asserted), and `to_engine_error` folds both directions of a size mismatch into `CorruptDetail::Truncated` — a shared-vocabulary word that means "fewer bytes" — so an over-long buffer is reported to the engine as a truncation. Neither breaks behavior; both are honest-labeling gaps in a file whose entire job is honest labeling.

## 2. What actually works (verified)
— Exhaustive variant→{stage, code, kind} mapping: every one of the 10 variants has an arm in `stage()` (:132-145), `code()` (:150-167 — 11 arms, `ZeroExtent` splits by `from_frame`), `to_engine_error()` (:174-201), and `Display` (:215-256). No variant falls through.
— Shipped execution: `pipeline_host.rs:918` folds screenshot encode errors through `to_engine_error`; baseline `error::tests::*` 3/3 pass within renderer_wgpu's 60/60 (orchrun :4767-4781).
— Code strings are stable, lowercase, dotted (`render.surface.lost`, `render.upload.size-mismatch`, …) — matches doc 01 §3.4's ErrorContext shape; `with_source(self)` puts the RendererError on the std error chain (verified by test :405-407 `source(&lost).is_some()`).
— Display carries structured data only (:213-214 claim verified: every arm writes numbers/enum names, no buffer contents).

## 3. Findings

### F-001 [S4]
Where: crates/renderer-wgpu/src/error.rs:271-272
Code:
```
    #[test]
    fn every_error_names_stage_and_stable_code() {
        let cases: [(RendererError, RendererStage, &str); 10] = [
```
What is wrong: The test's name promises "every error", and `code()` has 11 distinct mappings, but the case array has 10 entries: `ZeroExtent` appears exactly once, with `from_frame: false` (`render.surface.zero-extent`, :278-288). The `from_frame: true` arm — `render.frame.zero-extent` (:153-155) — is never asserted anywhere in the file.
Why it matters: The `from_frame` split exists precisely so machine consumers can distinguish frame-side from surface-side zero extents; the frame-side code string is untested and could be renamed or broken (e.g., colliding with the surface string) without any test failing. A coverage hole in the file whose contract is "stable codes, additive-only once released" (:147-148).
What correct looks like: Add the 11th case: `ZeroExtent { extent: …, from_frame: true }` → `RendererStage::Configure`, `"render.frame.zero-extent"`.
Confidence: HIGH — Verification: read (array literal counted; `rg 'from_frame: true'` over the test module: zero hits).

### F-002 [S4]
Where: crates/renderer-wgpu/src/error.rs:188-200
Code:
```
            RendererError::UploadSizeMismatch { expected, found } => ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: *expected,
                    found: *found,
                },
            },
            RendererError::ReadbackSizeMismatch { expected, found }
            | RendererError::EncodeSizeMismatch { expected, found } => ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: *expected,
                    found: *found,
                },
            },
```
What is wrong: All three size-mismatch variants fold into `CorruptDetail::Truncated`, whose definition is "Fewer bytes than the structure requires" (media-types/src/error.rs:88-89). A mismatch where `found > expected` (an over-long buffer — equally "decode-boundary corruption" per :88-89 of this file) is reported to the engine as a *truncation*. The renderer-side name (`*SizeMismatch`) is accurate; the engine-side kind mislabels the direction.
Why it matters: The engine's structured kinds are the machine-readable corruption taxonomy (doc 01 §3.4); a consumer keying on `Corrupt::Truncated` to mean short-buffer will misclassify long-buffer corruption. The numbers (`expected`/`found`) are still present so a human can tell, which keeps this S4. The shared vocabulary offers no better variant (verified: `CorruptDetail` = InvalidStructure/Truncated/BadIndex/ChecksumMismatch/Timing only, media-types error.rs:85-96), so this is a vocabulary limitation surfaced at the fold — the fix belongs in media-types (an `Oversized`/`SizeMismatch` variant).
What correct looks like: A shared `CorruptDetail::SizeMismatch { expected, found }` (or `Oversized`) used for the `found > expected` direction; until then, a doc note on `to_engine_error` that mismatches in either direction fold to `Truncated`.
Confidence: HIGH — Verification: read (both files compared; vocabulary exhaustively enumerated).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RendererStage` enum + `as_str` + `Display` | 20-55 | REAL | — | 6 stages, stable lowercase names; verified by test :411-415. |
| `RendererError` enum (10 variants) | 63-127 | REAL | — | Every variant carries diagnostic payload (extent/format/expected-found/crop); no unit-only variants. |
| `RendererError::stage` | 131-145 | REAL | — | Total mapping; `SurfaceLost`→Configure is a taxonomy choice (surface-lifecycle class), consistent with its non-fatal doc (:65-67). |
| `RendererError::code` | 149-168 | REAL | S4 | 11 stable codes; the `from_frame: true` arm untested (F-001). |
| `RendererError::to_engine_error` | 172-208 | REAL | S4 | Stage Present + MP §8 kinds + chain source; F-002 direction mislabel on mismatches. |
| `Display` | 211-258 | REAL | — | Structured, non-empty for all variants (test :360); no secrets. |
| `impl std::error::Error` | 260 | REAL | — | No source override — chain root (matches test). |
| `RendererResult<T>` alias | 262-263 | REAL | — | Used by staging/surface/renderer/screenshot. |
| tests (3) | 265-416 | REAL | S4 | All pass in baseline; F-001 coverage gap in the first. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `every_error_names_stage_and_stable_code` (:271) | PARTIAL | Would catch wrong stage/code for 10 of 11 mappings; the 11th (`render.frame.zero-extent`) is untested (F-001) — a mutation renaming that string or making both arms return the same code passes. |
| `engine_error_folds_stage_present_with_structured_kinds` (:365) | YES | Asserts stage=Present, Device{Display,Lost} for SurfaceLost, Corrupt::Truncated{16,12} for upload mismatch, Device{Display,FormatUnsupported} for mode, and `source().is_some()` — kills fold regressions. |
| `stage_names_are_stable_lowercase` (:411) | YES (weakly) | Spot-checks 3 of 6 names — adequate for constants. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every failure names the exact pipeline leg it died on" | error.rs:4-5 | REAL | `stage()`/`code()` total mapping verified; test passes in baseline. |
| "never a panic, MP §8" | error.rs:8 | REAL | No unwrap/panic in the file; typed errors only. |
| "context codes are the machine-readable identity… additive-only once released" | error.rs:147-148 | REAL-UNPROVEN | Codes exist and are tested (10/11 — F-001); "additive-only" is a process promise, not code. |

## 7. Dead / duplicated / unreachable code
None. All variants are minted by sibling modules (staging.rs:269-300, surface.rs, renderer.rs) and the fold is called by the shipped CLI (pipeline_host.rs:913/:917/:918/:1321).

## 8. Dependencies & flags
`media_types::error` (shared vocabulary), `crate::params::{Extent2D, Rect}`, `crate::surface::PresentMode` — all used. No feature gates, no unsafe, no external deps. renderer-wgpu is a CLI+desktop dependency (apps/cli/Cargo.toml:60, apps/desktop/Cargo.toml:42).

## 9. Open questions for the next agent
- `screenshot.rs` and `synthetic.rs` (my next files) consume `RendererResult` — check their mismatch errors can actually occur with `found > expected` (if not, F-002 is theoretical for those legs; upload path in staging.rs can, in principle, see either direction).
- media-types owner: consider `CorruptDetail::SizeMismatch` for direction-neutral corruption (F-002's root cause).
