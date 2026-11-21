# AUDIT: apps/web/src/render/present.rs#part2 (lines 428-875, `#[cfg(test)] mod tests`)
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = 428-875 (the entire tests module). Production code 1-427 audited by W12-A01 (part1) — not re-audited here except where a test's claim forces a re-read (noted inline). Boundary recorded so part1 does not re-audit 428-875. |
| Lines / functions | 448 / 13 (3 helpers + 9 tests + 2 fixture consts inline) |
| File verdict | REAL (strong, discriminating conversion tests, executed green; gaps are S4) |
| Reachable from shipped entry point? | Test code — executed by `cargo test --workspace` (baseline repair run, not run #3: the primary run aborted at the desktop zero-alloc failure before apps/web's tests ran). The production code under test is crate-internal, UNREACHABLE from shipped entry points (W01-A10 F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read present.rs 1-876 (full file; part1 read for context of every tested function); `rg -n "render::present::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (9/9 ok at 3186-3194); `rg -n "fn assert_err|macro_rules! assert_err" crates/testkit/src` (helper contract — asserts Err + label, panics on Ok); `rg -n "as_chunks" Rust std doc knowledge` (slice::as_chunks — stable in this repo's 1.99 toolchain, used by both code and tests). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
This test module is the strongest kind in this codebase: an independent f64 reference model of the same Kr/Kb derivation checked against the f32 pipeline within one quantization step across 3 matrices × 2 ranges × 7 YCbCr samples, exact studio-swing endpoints (black=0, white=255 asserted per matrix), a siting test that pins odd/even pixels to the reference with constant chroma, and mailbox tests that assert the full counter lattice (stored/displaced/converted/refused) rather than a single happy value. All 9 tests ran green in the no-fail-fast repair run. The defects are coverage gaps only: `take()`'s internal-inconsistency path (refused++ at take time) is unreachable by construction and untested, the 32 MiB bound is tested only at 36 MB (never at the boundary or one byte over), and the odd-dimension test asserts byte arithmetic but not the rendered pixels of its extra chroma row.

## 2. What actually works (verified)
— Verified by baseline `orchrun-test-workspace-nofailfast.log.txt:3186-3194` → all 9 `render::present::tests::*` `ok` (chroma_siting, conversion_matches_reference, endpoints_are_exact, mailbox_is_latest_wins, odd_dimensions, pts_rides, refused_offers, rgba_passthrough, upload_refusals).
— Verified by hand-check of `reference_rgb` (495-524) against `yuv_coefficients`/`convert_yuv` (part1 271-313): identical derivation (Kr/Kb pairs, 219/224/16 and 255/255/0 scales, `2·(1−Kr)` etc.), computed in f64 — a genuine independent oracle, not a re-call of the code under test.
— Verified by mutation reasoning: replace `y_scale` 219→220 in part1 and `endpoints_are_exact_for_every_matrix` fails (black (16,128,128) limited no longer maps to 0); swap the BGRA swizzle order and `rgba_passthrough_and_bgra_swizzle` fails ([3,2,1,4] assert at 711); break latest-wins (store into a second slot) and `mailbox_is_latest_wins_with_counted_drops` fails on `pts_ns == Some(2)` (836).

## 3. Findings

### F-001 [S4]
Where: apps/web/src/render/present.rs:811-846 (mailbox test) vs part1 393-407
Code (part1's untested branch, quoted for reference):
```
            Err(_) => {
                self.refused += 1;
                None
            }
```
What is wrong: `PresentMailbox::take()`'s conversion-failure branch (store validated OK, take-time `rgba_from_video` fails) has no test — it is unreachable through the public API because `store` re-runs the same validation (same inputs, same outcome), so the branch is dead-in-practice defensive code. Nothing pins that the defensive path counts `refused` and answers `None` rather than panicking.
Why it matters: defensive code that is provably unreachable today is exactly the code that silently breaks during a refactor of the validate/convert split (e.g. if `store` ever stops pre-checking the size bound).
What correct looks like: a test that constructs the inconsistency directly (e.g. via a `pub(crate)` test seam or by shrinking `MAX_UPLOAD_BYTES` under cfg(test)) and asserts the refused-count + `None` contract.
Confidence: HIGH — Verification: read (branch reachability traced: store 356-387 performs the identical checks take 398 re-performs).

### F-002 [S4]
Where: apps/web/src/render/present.rs:770-783
Code:
```
        // 3000×3000 RGBA = 36 MB > the 32 MiB bound.
        let mut oversize = i420_frame(
            3000,
```
What is wrong: the `MAX_UPLOAD_BYTES` bound (part1 72) is tested only with a 36 MB frame — 4 MB past the cap. The boundary itself (32 MiB exactly, and 32 MiB + 1) is unpinned, and the equivalent store-time gate (part1 377-379) is never tested with an oversize frame at all (the oversize test drives `rgba_from_video` directly, not `mailbox.store`).
Why it matters: an off-by-one in the comparison (`>` vs `>=`) or a unit slip (MiB vs MB) would pass both suites.
What correct looks like: boundary pair (32 MiB exact → Ok; +1 byte → `TooLarge`) through BOTH `rgba_from_video` and `PresentMailbox::store`.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: apps/web/src/render/present.rs:667-687
Code:
```
    fn odd_dimensions_round_chroma_up() {
        let frame = i420_frame(
            3,
            3,
            |_, _| (128, 128, 128),
            limited(ColorMatrix::Bt709),
            None,
        );
        let upload = rgba_from_video(&frame).must();
        assert_eq!(upload.width, 3);
```
What is wrong: the odd-dimension test asserts only shape arithmetic (17 source bytes, 36 RGBA bytes) — never a pixel value from the extra chroma row/column that div_ceil creates. The luma loop reading `cb[chroma_row + x / 2]` for y=2/x=2 (chroma row 1, col 1) is exactly the odd-edge indexing most likely to be off-by-one, and no test observes its output.
Why it matters: an indexing regression in the odd tail (e.g. `(y + 1) / 2` instead of `y / 2`) would keep every byte count identical and pass this test; only the siting test on a 2×2 even frame would catch it, and it does not exercise odd extents.
What correct looks like: a 3×3 (or 5×3) frame with distinct per-position fill, asserting at least the last-row/last-column pixels against `reference_rgb`.
Confidence: HIGH — Verification: read (index paths traced in part1 209-225).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| i420_frame (fixture) | 439-481 | REAL | — | builds packed-plane I420 with div_ceil chroma; fill(x,y) sampled at (2cx,2cy) for chroma — mirrors top-left siting; pts via ns TimeBase |
| limited (fixture) | 483-490 | REAL | — | Bt709 primaries/transfer + param matrix, Limited range |
| reference_rgb (oracle) | 495-524 | REAL | — | independent f64 re-derivation; round+clamp — not a re-call of production code |
| endpoints_are_exact_for_every_matrix | 529-577 | REAL | — | black/white exact per matrix; full-range vs reference; discriminating (mutation: y_offset ⇒ black ≠ 0) |
| conversion_matches_the_reference_within_one_lsb | 582-624 | REAL | — | 42 combinations (3×2×7), ≤1.0 per channel; strong |
| chroma_siting_is_top_left_420 | 629-665 | REAL | — | pins pixels[0]/pixels[3] to reference with shared chroma; discriminating for siting |
| odd_dimensions_round_chroma_up | 668-687 | REAL | S4 | shape-only; pixel edge unpinned = F-003 |
| rgba_passthrough_and_bgra_swizzle | 690-712 | REAL | — | [1,2,3,4]→[3,2,1,4]; discriminating |
| upload_refusals_are_typed | 716-784 | REAL | S4 | NV12/YUYV422/OpaqueHandle/Truncated/Zero/TooLarge all variant-asserted; boundary unpinned = F-002 |
| pts_rides_the_upload | 787-806 | REAL | — | 123_456_789 ns round trip; base re-asserted |
| mailbox_is_latest_wins_with_counted_drops | 811-846 | REAL | — | displaced=1, staged=SECOND (pts 2, white), converted=1, refused=0 — full lattice; take-failure branch untested = F-001 |
| refused_offers_count_and_keep_the_staged_frame | 849-874 | REAL | — | refusal does not displace; refused=1, displaced=0; good frame survives |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| endpoints_are_exact_for_every_matrix | YES | exact byte equality on studio-swing endpoints, per matrix |
| conversion_matches_the_reference_within_one_lsb | YES | independent f64 oracle, tight tolerance, 42 combos |
| chroma_siting_is_top_left_420 | YES | reference-equality on two pixels with different luma, shared chroma |
| odd_dimensions_round_chroma_up | NO (for pixel correctness) | asserts lengths only — F-003 |
| rgba_passthrough_and_bgra_swizzle | YES | exact byte equality both formats |
| upload_refusals_are_typed | YES (variant-level) | each refusal asserted to the exact enum variant via assert_err label |
| pts_rides_the_upload | YES | exact ns value + base conversion re-checked |
| mailbox_is_latest_wins_with_counted_drops | YES | wrong-order staging or uncounted displacement breaks a counter assert |
| refused_offers_count_and_keep_the_staged_frame | YES | staged-frame survival + counter pair |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the conversion is … the same YUV→RGB arithmetic … using the frame's OWN colorimetry (BT.601/709/2020 × limited/full)" | present.rs:11-19 (part1 module doc) | REAL | endpoints + one-LSB sweep tests cover all 3 matrices × 2 ranges, green |
| "at most ONE frame staged, the newest wins, displaced frames are COUNTED" | present.rs:31-38 | REAL | mailbox test lattice (811-846) |
| "the conversion happens at TAKE time so a displaced frame never pays for pixels" | present.rs:35-38 | REAL | converted==1 while stored==2/displaced==1 (843-844) |
| "an 8K frame is refused TYPED" | present.rs:68-71 | REAL (untested at 8K specifically) | 36 MB oversize refusal tested; 8K = 33.18 MP × 4 B ≈ 132 MB would also exceed — F-002 notes the unpinned boundary |

## 7. Dead / duplicated / unreachable code
`reference_rgb` duplicates the coefficient table of part1's `yuv_coefficients` (271-291) — INTENTIONAL duplication (the oracle must be independent); acceptable and documented as such (492-494). No dead tests.

## 8. Dependencies & flags
`testkit::{Must, assert_err}` (Must::must panics with context; assert_err asserts Err + label — contract verified by reading testkit); `media_types` frame/time vocabulary; `as_chunks::<4>` (stable slice method). No feature gates — the tests run in the default (decoder-less) and --all-features runs alike.

## 9. Open questions for the next agent
- W12-A01 (part1): the store-time size gate uses `rgba_bytes` (w×h×4) not the I420 plane total — confirm the intent (a 4:2:0 frame's RGBA expansion is what crosses the ABI, so the bound is on the OUTPUT — consistent) and whether doc 09 §4.2 states the bound in those terms.
- Does any doc claim the presenter is chroma-siting-exact to a standard (BT.601 siting conventions)? The tests pin top-left only.
