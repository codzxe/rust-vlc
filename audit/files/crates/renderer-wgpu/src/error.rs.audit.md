# AUDIT: crates/renderer-wgpu/src/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 416 / 8 production fns + 3 tests |
| File verdict | REAL (typed error vocabulary, exercised by the full renderer-wgpu lib suite — 60/60 pass live) |
| Reachable from shipped entry point? | YES — renderer-wgpu shipped to apps/cli + apps/desktop; `RendererError` flows out of `Renderer::render`/`present` (renderer.rs:205-214) and `to_engine_error` is the Presenter boundary fold |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A disciplined typed-error vocabulary: 10 variants, every one stage-tagged with a stable dotted code, Display without secrets, and an EngineError fold carrying structured kinds (Device/Unsupported/Corrupt) with the renderer error as the chain source. It is genuinely consumed (staging.rs, params.rs, surface.rs, screenshot.rs all raise it) and tested (60/60 renderer-wgpu lib tests pass live, 3 of them this file's). Two diagnostic-level defects: a zero-extent **frame** error raised during the upload leg is stage-tagged `Configure` (the from_frame:true arm is also the one arm the exhaustive-looking stage/code test table omits), and over/under-length readback/encode buffers fold to `CorruptDetail::Truncated` even when `found > expected` ("truncated" is then a lie by half — the sites I checked only ever produce found < expected, so the mislabel is latent).

## 2. What actually works (verified)
- `timeout 100 ./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 --test-threads=2` → `60 passed; 0 failed` (includes this file's 3 tests: stage/code table, engine-fold kinds, stable stage names).
- Cross-checked raise sites: staging.rs:269/279, surface.rs:208-298 (write_rect short-buffer), screenshot.rs:40 — all found-<-expected at every site, so the `Truncated` vocabulary currently matches reality.

## 3. Findings

F-001 [S4 — zero-extent frame error carries the wrong stage tag; the arm is untested]
Where: error.rs:133-136 (stage), 153-158 (code), 272-354 (test table)
Code:
```
            RendererError::SurfaceLost
            | RendererError::ZeroExtent { .. }
            | RendererError::NoSurface => RendererStage::Configure,
```
What is wrong: `ZeroExtent { from_frame: true }` is raised by the **upload** leg (staging.rs `stage()`, renderer.rs render path) but is stage-tagged `Configure` (surface-construction) — a frame dimension problem reported as a surface-configuration problem. The test `every_error_names_stage_and_stable_code` enumerates 10 cases (one per variant) but includes only the `from_frame: false` arm of ZeroExtent, so the mislabeled arm is also the untested arm. The dotted code (`render.frame.zero-extent`) is distinct and correct, so routing by code is unaffected; the stage tag is the diagnostic that lies.
What correct looks like: `ZeroExtent { from_frame: true, .. } => RendererStage::Upload` + a test row for it.
Confidence: HIGH — Verification: read + ran suite.

F-002 [S4 — `Truncated` kind for potentially over-length buffers]
Where: error.rs:188-200
Code:
```
            RendererError::UploadSizeMismatch { expected, found } => ErrorKind::Corrupt {
                detail: CorruptDetail::Truncated {
                    expected: *expected,
                    found: *found,
                },
            },
```
What is wrong: all three size-mismatch variants fold to `CorruptDetail::Truncated`, which names only the short case. Every current raise site guards with `pixels.len() < needed` (surface.rs:296-299, screenshot.rs:38-41, staging.rs:278-283) so `found < expected` always holds today, but the type permits `found > expected` and the kind would then misname the corruption. Latent vocabulary defect, zero current impact.
Confidence: HIGH — Verification: read (all raise sites cross-checked).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RendererStage::as_str / Display | 36-55 | REAL | — | stable lowercase, tested |
| RendererError::stage | 132-145 | REAL | S4 | F-001 ZeroExtent-from-frame → Configure |
| RendererError::code | 150-168 | REAL | — | 11 stable dotted codes (ZeroExtent split by from_frame) |
| RendererError::to_engine_error | 173-208 | REAL | S4 | F-002; otherwise structured kinds + chain source, tested |
| Display | 211-257 | REAL | — | structured payloads only, no secrets |
| std::error::Error | 260 | REAL | — | |
| RendererResult alias | 263 | REAL | — | |
| tests ×3 | 270-415 | REAL | — | pass live; table gap noted in F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_error_names_stage_and_stable_code | YES | per-variant stage+code equality (gap: from_frame:true arm absent) |
| engine_error_folds_stage_present_with_structured_kinds | YES | exact ErrorKind matches + source chain presence |
| stage_names_are_stable_lowercase | YES | string locks |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every failure names the exact pipeline leg it died on" | error.rs:4-5 | FALSE for one arm | F-001: frame zero-extent names Configure, dies on Upload |
| "additive-only once released" (codes) | error.rs:147-148 | UNVERIFIED | no released consumers to check against; codes are stable in-tree |
| "never a panic, MP §8" | error.rs:8 | TRUE (this crate's paths) | all raise sites typed; 60/60 suite |

## 7. Dead / duplicated / unreachable code
None. All variants are raised somewhere in the crate (rg: every variant has ≥1 raise site).

## 8. Dependencies & flags
media-types error vocabulary (EngineError/ErrorKind/CorruptDetail), crate-internal params/surface types. No wgpu dependency in this file.

## 9. Open questions for the next agent
- None blocking. If a raise site ever appears that can produce `found > expected`, F-002 becomes a live mislabel.
