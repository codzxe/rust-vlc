# AUDIT: crates/subtitles/src/ass/layout.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-21 |
| Lines / functions | 584 / 17 (incl. 7 tests) |
| File verdict | UNREACHABLE (real, library-internal, tests baseline-executed green; no shipped entry point) |
| Reachable from shipped entry point? | NO — `lumen` CLI / `lumen-tauri` never touch crates/subtitles (see F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 1 |
| Commands run | `rg "subtitles" --glob **/Cargo.toml` → only fuzz/Cargo.toml:50; `rg "use subtitles|subtitles::" apps/ crates/ fuzz/` → only fuzz targets; `rg "cue_layout|model_positioning|..."` (callers); `sed -n` quote verifies; baseline: test-workspace.log.txt + test-workspace-allfeatures.log.txt (fail-fast, stop before `subtitles`) AND orchrun-test-workspace-nofailfast.log.txt:5269-5540 (full no-fail-fast run: subtitles lib 267/0, layout tests ok :5273-5280) |

## 1. One-paragraph verdict
The numpad-grid and margin→rational-anchor math in this file is genuinely implemented, arithmetically correct (traced by hand against `compositing::layout::place_cue` and `model::position::resolve` semantics; unit tests assert exact reduced fractions 5/96, 43/48, 5/48), and its 7 tests are baseline-executed green (orchrun-test-workspace-nofailfast.log.txt:5273-5280, within the lib's 267/0 at :5540). But it is a library island: nothing outside `crates/subtitles` depends on the crate, only `render.rs` calls `cue_layout`/`cue_layout_at` and only `render.rs`'s own tests call the renderers, `model_positioning` has no production caller at all. The file's honest-documentation culture (HONEST NOTE, documented deviations) is real; its "parser seam" claim is not. (Correction v2 of this audit: the first draft claimed zero baseline execution, based on INDEX.md's table; the unlisted orchrun artifact refutes that — see F-002.)

## 2. What actually works (verified)
— Cross-checked `cue_layout(BOTTOM_LEFT, {20,20,30}, (384,288))` by hand: u=20/384=5/96, v=(288−30)/288=43/48 — matches test assertion layout.rs:473-474; `place_cue` (compositing/layout.rs:264-317) consumes these anchors correctly. Verified by read+hand-computation; execution proven by baseline orchrun-test-workspace-nofailfast.log.txt:5273-5280 (all 7 layout tests ok).
— `margin_percent` round-half-up verified on the tie case m=36, e=288 → 12.5% → 13: `(3600+144)/288 = 13`. Correct.
— validate/margin_percent/cue_layout arithmetic: no underflow (`play_res.0 - margins.right` guarded by `margins.right > play_res.0` check at :223), no u64 overflow (`u32*100+u32/2 < 2^40`).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/ass/layout.rs:1-584 (whole file; evidence via dependency census)
Code:
```
[crates/subtitles/Cargo.toml]
[dependencies]
media-types = { path = "../media-types" }
```
What is wrong: The entire crate has exactly one reverse-dependency in the repo: `fuzz/Cargo.toml:50`. `rg "use subtitles|subtitles::"` over apps/ + crates/ + fuzz/ hits only `fuzz/fuzz_targets/fuzz_sub_{srt,ass,vtt}.rs`. Neither `apps/cli` (the shipped `lumen` binary) nor `apps/desktop-tauri` references it; baseline `lumen engine-info` (audit/runs/baseline/lumen-engine-info.txt) lists features `first-vertical:avi-mjpeg-pcm, audio.null-device, video.software-compositor` — no subtitle lane exists.
Why it matters: Every line of this file is dead weight in the shipped build. The wave's hostile question — "is any subtitle path reachable in the shipped build?" — answers NO for the whole crate, this file included.
What correct looks like: A demux/pipeline wiring (sidecar probe → parse → compositing) in apps/, or an honest UNREACHABLE marker in the matrix instead of platform "implemented/partial" statuses.
Confidence: HIGH — Verification: read + rg census (commands above).

### F-002 [S3] — CORRECTED v2
Where: crates/subtitles/src/ass/layout.rs:389-584 (test module) + audit/runs/baseline/INDEX.md (verdict table)
Code:
```
#[cfg(test)]
mod tests {
    use super::{
        AssAlignment, AssHorizontal, AssMargins, AssVertical, DEFAULT_PLAY_RES, cue_layout,
```
What is wrong: The baseline EVIDENCE INDEX is misleading, not the tests. First draft of this audit (v1) claimed "no baseline run executed any subtitles test" — that was wrong: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269-5540` shows the full subtitles suite running **267 passed; 0 failed**, with this file's 7 tests green at :5273-5280. The error came from INDEX.md: its verdict table cites only the two fail-fast runs (#3 `test-workspace.log.txt`, stopping at `-p desktop --test zero_alloc_transport`; #4 all-features, stopping at `-p first-vertical --test evidence`), neither of which reaches the alphabetically-later `subtitles` package — and INDEX.md does not list ANY of the 10 `orchrun-*` artifacts in its directory despite AGENT-INSTRUCTIONS.md stating "audit/runs/baseline/INDEX.md lists everything".
Why it matters: An auditor working from INDEX.md alone (as instructed) reaches a false no-evidence conclusion — this one did, in five files, before catching it. The index-everything contract is load-bearing for every wave; a 3 MB full-suite log sitting unlisted beside it is exactly the drift the Devil Audit exists to catch.
What correct looks like: INDEX.md gains rows for the orchrun-* artifacts (or an explicit pointer that later orchestrator runs are unindexed), and the gate uses --no-fail-fast so the headline runs themselves cover the whole workspace.
Confidence: HIGH — Verification: read of all three baseline logs (Running-line censuses; subtitles result lines :5540).

### F-003 [S3]
Where: crates/subtitles/src/ass/layout.rs:7-9
Code:
```
//! * [`model_positioning`] — the parser/cue-model vocabulary
//!   ([`Positioning`], WebVTT-flavored integer percents). This is the
//!   seam the 5.M2a parser half consumes when it builds
```
What is wrong: The claimed consumer does not exist. `rg "model_positioning"` across the crate hits only this file (def + tests). The parser builds its `Positioning` directly: parser.rs:636 `Some(s) => (Some(s.clone()), Positioning::new(s.alignment, None, None))` — margins are dropped entirely, `model_positioning` is never called. The "seam" the module doc advertises was never wired.
Why it matters: Dead public API sold as the integration point; a reader auditing "how do style margins reach the cue model?" is pointed at code that nothing calls.
What correct looks like: Either parser.rs:636 calls `model_positioning(style.alignment, style_margins, play_res)`, or the doc says "unused vocabulary, kept for the styling card".
Confidence: HIGH — Verification: rg census + read of parser.rs:630-646.

### F-004 [S3]
Where: crates/subtitles/src/ass/layout.rs:313-323
Code:
```
    let (u, horizontal) = match alignment.horizontal() {
        AssHorizontal::Left => (
            Rational::new(margins.left, play_res.0)?,
```
What is wrong: For `AssHorizontal::Center` (:318) `cue_layout` hard-codes `u = 1/2` and silently discards BOTH MarginL and MarginR. libass/VSFilter inset the centered text box to `[marginL, W−marginR]` and center within it — an asymmetric pair (e.g. MarginL=10, MarginR=100, common in fansub scripts to dodge on-screen logos) shifts the cue center; this code pins frame center regardless. The module doc (:29-31) asserts the deviation but frames it as the natural reading, and the record doc's "margins = anchor fractions of PlayRes … libass-style" (sub-ass-ssa.md:52-53) hides it.
Why it matters: Wrong placement vs the reference renderer in realistic asymmetric-margin centered cues (the ASS DEFAULT alignment 2 is centered); deviation is documented but mislabeled "libass-style".
What correct looks like: `u = (margins.left + (W − margins.right)) / 2 / W` for Center, or an explicit damage/counter when L≠R on centered alignment.
Confidence: MEDIUM — libass source not consulted in-repo (same limitation the file itself admits at :34-38 for mid-alignment); VSFilter behavior from spec knowledge.

### F-005 [S4]
Where: crates/subtitles/src/ass/layout.rs:353-356
Code:
```
/// # Errors
/// [`AssRenderError::ZeroPlayRes`]; [`AssRenderError::Cue`]
/// defensively for rational/anchor construction (the pre-checked
/// invariants make those unreachable).
```
What is wrong: For `cue_layout_at` this is false: `pos` (:361) is NOT pre-checked against `play_res`, so `pos.0 > play_res.0` reaches `Anchor::new` and fails `AnchorOutOfRange` → `AssRenderError::Cue` on a routine path (an ASS `\pos` outside the frame). The "unreachable" claim holds only for the margin-validated `cue_layout`.
Why it matters: Doc claims an invariant that doesn't exist; a caller relying on "Cue is never returned" would mishandle out-of-frame positions.
What correct looks like: Document `Cue` as the typed out-of-frame refusal for `cue_layout_at`, or pre-check `pos` with a dedicated error variant.
Confidence: HIGH — Verification: read of :357-387 + compositing/layout.rs:98-109 (`Anchor::new` rejects `num > den`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DEFAULT_PLAY_RES` | 58-60 | REAL | — | 384×288 libass default; consumed by parser.rs `play_res()` fallback + render tests |
| `AssHorizontal` (enum) | 62-71 | REAL | — | trivial, correct |
| `AssVertical` (enum) | 73-82 | REAL | — | trivial, correct |
| `AssAlignment` consts (9 + `ALL`) | 89-120 | REAL | — | numpad order 1..=9; `ALL` drives the 9/9 grid sweep |
| `AssAlignment::new` | 126-131 | REAL | — | typed `BadAlignment` for non-1..=9; no panics |
| `AssAlignment::get` | 134-137 | REAL | — | const trivial |
| `AssAlignment::horizontal` | 141-147 | REAL | — | `_ => Right` safe: constructor bounds 1..=9 |
| `AssAlignment::vertical` | 151-157 | REAL | — | same pattern, correct |
| `AssAlignment::to_model_alignment` | 161-176 | REAL | — | 3×3 mapping correct; verified vs `Alignment::code()` |
| `AssMargins` + `ASS_DEFAULT`/`ZERO` | 183-208 | REAL | — | 10/10/10 matches the ASS `Default` template |
| `validate` | 212-238 | REAL | — | typed refusals; allows margin==extent (boundary test-pinned :571-582) |
| `margin_percent` | 245-248 | REAL | — | round-half-up u64 math; tie case hand-verified; no overflow |
| `model_positioning` | 262-294 | REAL (dead) | S3 | ZERO production callers (F-003); math itself correct vs `Positioning::resolve` |
| `cue_layout` | 307-344 | REAL | S3 | callers: render.rs:428,559 (which have no production callers themselves); center-margin deviation F-004 |
| `cue_layout_at` | 357-387 | REAL | S4 | callers: render.rs:427,558; out-of-frame pos → reachable `Cue` error, doc says unreachable (F-005) |
| tests (7 fns) | 400-583 | REAL | S3 | genuine assertions (exact fractions, typed errors, boundary margin==extent); executed green in baseline orchrun log :5273-5280 (INDEX-drift finding F-002 documents why the two INDEX-cited runs miss them) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| numpad_grid_maps_all_nine_to_the_model_matrix | YES | asserts concrete `code()` strings per alignment + spot horizontal/vertical |
| bad_alignment_is_typed | YES | exact `Err(BadAlignment{value})` equality for 0/10/255 |
| model_positioning_carries_margins_as_rounded_percents | YES | pins 5/90, resolve (19,259), 95/10, None/None — mutation of rounding or flip math breaks it |
| cue_layout_builds_exact_rational_anchors | YES | pins reduced fractions (5,96),(43,48),(5,48) and `reference` |
| middle_alignment_anchors_the_frame_center | YES | `with.anchor == without.anchor` + (1,2) — breaks if MarginV were applied |
| cue_layout_at_pins_the_pos_anchor_exactly | YES | (1,2) anchor + `assert_ne` vs margins + typed ZeroPlayRes |
| margin_and_play_res_violations_are_typed | YES | exact error variants + the margin==extent OK boundary |

All 7 are mutation-sensitive. All 7 executed green in the baseline no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt:5273-5280, within lib result 267/0 at :5540).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "margins = anchor fractions of PlayRes 384×288, libass-style scaling" | docs/formats/sub-ass-ssa.md:52-53 | SHALLOW | fractions real (cue_layout :313-334) but "libass-style" hides the center-margins-ignored deviation (F-004); margins scale only with the actual play_res, defaulting 384×288 |
| "9/9 alignment-grid positions pixel-exact through the compositor's own place_cue" | docs/formats/sub-ass-ssa.md:71-72 | REAL | the 9/9 sweep test exists (render.rs tests, ~:697, A08's part) and ran green in baseline orchrun log :5306 (`nine_position_grid_lands_pixel_exact ... ok`) |
| "libass mid-alignment (4–6) MarginV policy unverified (conservative center, test-pinned)" | docs/formats/sub-ass-ssa.md:94-95 | REAL | matches :486-502 test; honest |
| "267/0 lib tests" (G-2 evidence) | docs/feature-matrix.toml:914 | REAL | orchrun-test-workspace-nofailfast.log.txt:5540: `267 passed; 0 failed; 0 ignored` for the subtitles lib |
| matrix row `subtitle-ass-ssa` status partial ×3 platforms | docs/feature-matrix.toml:913 | UNREACHABLE (shipped-build sense) | code exists, tests green; crate not in any shipped binary (F-001); "partial" describes library scope, not player capability |

## 7. Dead / duplicated / unreachable code
- `model_positioning` + its `Percent` flip arithmetic: dead in production (F-003).
- Whole file unreachable from shipped entry points (F-001); its only live consumers are `render.rs` functions that themselves have only test callers.

## 8. Dependencies & flags
- Imports only crate-internal modules (`ass::render::AssRenderError`, `compositing::layout`, `model::position`) — consistent with the crate's "std-only, zero external deps" manifest claim (crates/subtitles/Cargo.toml:26-27, verified: no `[dependencies]` beyond media-types, which this file does not even use).
- No feature flags gate this file. No unsafe. No panics (all `?`-propagated, `Percent::new` typed).

## 9. Open questions for the next agent
- A08 (render.rs part2): confirm from the test side that no test constructs `AssRenderCue` from real parsed output (I found none in part1 callers — the parser→render bridge appears to not exist anywhere).
- A10 (parser.rs part2): `parse_event_line` drops margins into the model path (:636) — check whether `AssStyle` margins are consumed by ANY production code or only re-exported for the never-landed styling card.
- Whoever audits docs/formats/sub-ass-ssa.md: the "230/0 current tree" claim has no reproducible run (F-002 applies crate-wide).
