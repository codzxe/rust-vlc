# AUDIT: crates/codecs/tests/fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 255 / 5 helpers + 6 tests |
| File verdict | REAL — committed-corpus survival run, all 6 tests executed live (3.74s), loud-fail fixtures |
| Reachable from shipped entry point? | N/A (integration test); exercises the SHIPPED MjpegDecoder via `fuzz_codec_mjpeg` (CLI "mjpeg" row) — harness drives both the free decode entry and the codec-api contract path (mjpeg/fuzz.rs:5-8, :47+) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/fuzz_survival-ba00cbfdf690b00c --nocapture` → 6 passed; 0 failed, exit 0, 3.74s; ls tests/fixtures/generated/ (22 mjpeg-lane fixtures present) |

## 1. One-paragraph verdict
This is what a survival suite should look like: 22 committed fixture files
(2 valid containers + 20 flip/trunc/garbage/noidx/stszlie derivatives, all
present on disk), 120 extracted frames asserted (`frames.len() == 60` per
container), systematic truncations every 128 bytes, single-byte flips
every 64 bytes, adversarial near-cap/over-cap geometries, and an
exhaustive AVI1 polarity sweep (all 256 values + truncations + doubled
markers + RST damage) — all through the full mjpeg decode pipeline. Unlike
the corpus-gated suites elsewhere in this repo, missing fixtures FAIL
LOUDLY (`unreachable!("{e}")` on read error, :52) rather than silently
passing. All 6 tests ran live. The only defect is an off-by-arithmetic
comment at the geometry cap whose described rejection case is never
constructed.

## 2. What actually works (verified)
— verified by `./target/debug/deps/fuzz_survival-ba00cbfdf690b00c
--nocapture` → `test result: ok. 6 passed; 0 failed` (exit 0, 3.74s of
real decode work). Fixture presence verified by `ls
tests/fixtures/generated/{valid,malformed}/` (avi_mjpeg_pcm + 10
derivatives, mp4_mjpeg_pcm + 10 derivatives).

## 3. Findings

F-001 [S4]
Where: crates/codecs/tests/fuzz_survival.rs:146-152
Code:
```rust
    // 2896×2896 ≈ 8.39 MiP — just above the 8 MiP cap is rejected;
    // 2890×2900 ≈ 8.38 MiP is the near-ceiling survivor case.
    let mut sof = vec![8u8];
    sof.extend_from_slice(&2900u16.to_be_bytes());
    sof.extend_from_slice(&2890u16.to_be_bytes());
```
What is wrong: (a) the arithmetic is wrong — 2896×2896 = 8,386,816 px,
which is BELOW 8 MiP (8,388,608), not "just above"; (b) the described
"just above the cap is rejected" case is never constructed: the near-cap
test builds 2890×2900 = 8,381,000 px (below the cap, survivor), and the
rejection test jumps to 8193×8193 = 67.1 M px (:158-171), six times over.
The tightest boundary — a geometry one pixel over the cap — is untested,
which is exactly where an off-by-one in `MAX_PIXELS` enforcement would
hide.
What correct looks like: a 8,388,609-px rejector (e.g. 2897×2896) beside
the survivor.
Confidence: HIGH — Verification: read + arithmetic (cap value per
mjpeg/fuzz.rs docs referencing `MAX_PIXELS`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| all_fixture_files | 15-45 | REAL | - | 22 paths, all present on disk |
| every_fixture_file_survives_the_harness | 47-57 | REAL | - | loud-fail on missing fixture |
| every_extracted_frame_survives_the_harness | 59-82 | REAL | - | asserts 60 frames/container |
| systematic_truncations_of_a_real_frame_survive | 84-102 | REAL | - | every 128 B + empty prefix |
| systematic_single_byte_flips_of_a_real_frame_survive | 104-122 | REAL | - | every 64 B, restore-after-flip |
| adversarial_geometries_survive_the_harness | 124-173 | REAL | S4 | F-001: no near-over-cap rejector |
| avi1_field_marker_mutations_survive_the_harness | 175-255 | REAL | - | 256 polarities, truncations, splice, RST damage |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 | PARTLY (by design) | contract is absence-of-panic/hang/unbounded-alloc only (fuzz.rs:7-10); wrong pixels/typed-error regressions do NOT fail these — the AC suite owns outcomes. Honest, documented, narrow |
| fixture presence | YES | unreachable! on read error — no silent skip |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every fixture file, every extracted frame, and systematic truncations/mutations ... pass through the harness" | fuzz_survival.rs:1-5 | REAL | 6/6 ran live, fixtures verified present |
| "2896×2896 ... just above the 8 MiP cap is rejected" | fuzz_survival.rs:146-147 | FALSE | arithmetic wrong (below cap) and case not constructed (F-001) |
| harness "Never panics, never hangs, never allocates without a ceiling" | mjpeg/fuzz.rs:45-46 | REAL-UNPROVEN | survived 6 live tests incl. 8193² over-cap; a sustained libFuzzer campaign is NOT wired (fuzz.rs:14-23 admits cargo-fuzz target unbuilt — Lead-owned dep) |
## 7. Dead / duplicated / unreachable code
None. (Directory contains additional malformed derivatives — absidx1,
negheight0, mkv lanes — not listed here; they belong to other suites.)
## 8. Dependencies & flags
codecs + test common module; fixtures from repo-root tests/fixtures
(common/mod.rs:30-32). No feature gates; runs in default test lane.
## 9. Open questions for the next agent
- mjpeg/fuzz.rs (file 16 of my list): confirm the contract-path pass
  covers configure→packet→sink→flush for EVERY input, not just the free
  entry.
- Add the one-pixel-over-cap rejector (F-001) — a 3-line fix.
