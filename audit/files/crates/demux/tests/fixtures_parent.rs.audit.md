# AUDIT: crates/demux/tests/fixtures_parent.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 346 / 7 (1 helper + 6 #[test]) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test (`cargo test --test fixtures_parent`), test-only. Code under test reachable: AviDemuxer <- apps/cli/src/containers/avi.rs:40. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg/sed/ls over tests/fixtures, common/mod.rs, testkit; baseline citation. No cargo runs. |

## 1. One-paragraph verdict
A genuine end-to-end acceptance suite: it opens the committed 425,516-byte parent AVI through the REAL file-access stack (`input::FileAccess`, common/mod.rs:546-550), pins the test-side SHA-256 against the committed whole-file digest first (so a broken hasher cannot vacuously pass), then asserts per-frame video sha256 equality in order, the concatenated-PCM stream digest, zero damage, exact PTS ladders, tolerance-bounded at-or-below seek landings, and a from-start-tail proof for re-anchor with a typed past-end refusal plus D4 continuation. All 6 tests pass in the baseline (cargo-test-workspace.txt ~line 2271: `test result: ok. 6 passed; 0 failed`). Two minor defects: the fuzz-harness smoke test asserts nothing (a gutted harness body would pass it), and the PTS-ladder expectations hardcode 60 frames / 1470-tick audio steps in Rust instead of reading them from the expectation TOML.

## 2. What actually works (verified)
— verified by ls: `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (425,516 bytes, exists) + `tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml` (sha256 `1f5620...d271a`, duration 2_000_000 µs, 4 seek targets, tolerance 33_333 µs = exactly one 30 fps frame — not vacuous).
— verified by read: `open_via_input` goes through `input::FileAccess::new().open(&uri)` (common/mod.rs:546-550) — not an in-memory stand-in, so the header's "real file-access stack" claim (line 2) is TRUE.
— baseline: `Running tests/fixtures_parent.rs` -> 6 named tests `... ok` -> `test result: ok. 6 passed; 0 failed; 0 ignored` (cargo-test-workspace.txt lines ~2259-2271).
— landing-frame arithmetic (lines 316-324) checked: `div_ceil` of the µs-truncated landing recovers the frame index; `.min(video_sample_count)` + the earlier len==count assert (263-267) make the slice index panic-free.

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/fixtures_parent.rs:220-226
Code (verbatim):
```
    let bytes = testkit::fixture_bytes(PARENT).must();
    demux::avi::fuzz::fuzz_demux_avi(&bytes);
}
```
What is wrong: `parent_events_survive_the_fuzz_harness` has zero assertions — it is an absence-of-panic smoke only (the comment admits "no panic, no hang"). If `fuzz_demux_avi`'s body were gutted to return immediately, this test still passes; it pins only that the harness does not explode on these specific bytes, and "no hang" is enforced only by CI timeout, not by the test. Why it matters: the test's name promises "events survive" — the body verifies only "harness returned". Blast radius: small (fuzz.rs has its own unit tests). Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/demux/tests/fixtures_parent.rs:176-178
Code (verbatim):
```
    assert_eq!(video_pts, (0..60).collect::<Vec<i64>>());
    let audio_want: Vec<i64> = (0..60).map(|i| i * 1470).collect();
    assert_eq!(audio_pts, audio_want);
```
What is wrong: the PTS-ladder expectations hardcode 60 frames and the 1470-tick audio step (44100/30 = 1470) in Rust, duplicating values the expectation TOML already records (`video_sample_count = 60`, rate 44100, fps 30/1). A fixture regeneration that changed the frame count fails this test with a confusing hardcoded-60 mismatch instead of reading the new truth (the count assert at 153-157 would fail first, masking the real coupling). Fragile duplication, not dishonesty. Confidence: HIGH — Verification: read + TOML inspection.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_parent | 19-26 | REAL | — | real FileAccess stack; unreachable! on failure (test-appropriate) |
| sha256_impl_matches_the_recorded_file_digest | 29-36 | REAL | — | pins hasher vs committed digest + size — anti-vacuity keystone |
| parent_track_table_matches_the_expectation | 39-109 | REAL | — | exact codec/format/fps-reciprocal/duration; unwrap_or("") fallbacks make it stricter, not weaker |
| parent_demuxes_fully_per_frame_hash_and_pcm_digest | 112-179 | REAL | — | per-frame sha256 in order, PCM digest, recovered==0, damage_total==0, exact PTS ladders (F-002 hardcode) |
| parent_seek_targets_land_within_tolerance | 182-217 | REAL | — | tolerance 33_333 µs (one frame) + at-or-below direction assert |
| parent_events_survive_the_fuzz_harness | 220-226 | SHALLOW | S4 | no assertions (F-001) |
| parent_reanchor_continues_the_stream_within_tolerance | 229-346 | REAL | — | from-start tail equality per target, typed past-end refusal, D4 EOS continuation |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sha256_impl_matches_the_recorded_file_digest | YES | fixed digest vs implementation |
| parent_track_table_matches_the_expectation | YES | exact values incl. 2_000_000 µs span conversion |
| parent_demuxes_fully... | YES | 60 per-frame hashes + stream digest + both PTS ladders + zero damage |
| parent_seek_targets... | YES for landing errors | tolerance is one frame, direction pinned; a broken seek landing on frame+1 fails the at-or-below assert |
| parent_events_survive_the_fuzz_harness | NO (panic-only) | gutted harness passes (F-001) |
| parent_reanchor... | YES | tail-hash equality is exact; past-end code pinned |

Golden-file caveat (checklist 6.4): the expectation TOML and the AVI are both produced by `cargo xtask fixtures` (TOML header: "Generated by `cargo xtask fixtures` — DO NOT EDIT"), so a generator bug would produce mutually-consistent fixture+expectation and this suite would not catch it. That is the inherent golden-file trade; the header's own wording "the writer-declared truth" (line 5) is honest about it, and the independent oracle is the ffprobe differential in corpus tests (not this file).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "demuxes fully through the real file-access stack" | fixtures_parent.rs:1-2 | REAL | open_via_input -> input::FileAccess (common/mod.rs:546-550) |
| "every video packet hash-matches the expectation's per-frame sha256" | fixtures_parent.rs:2-4 | REAL | lines 152-165 |
| "the track/duration metadata matches the writer-declared truth" | fixtures_parent.rs:4-5 | REAL | lines 39-109 (with the golden-file caveat above) |

## 7. Dead / duplicated / unreachable code
None beyond F-002's duplicated constants. `mod common;` pulls the 550-line shared helper module; unused halves are `#[allow(dead_code)]`-documented (common/mod.rs:2-6) — acceptable shared plumbing.

## 8. Dependencies & flags
dev-dependency on testkit (fixture_path/fixture_bytes/Must), input (FileAccess), media-types, plugin-api. Zero external crates (hand-rolled SHA-256 in common, pinned by the first test). No feature gates. Fixture resolution: workspace `tests/fixtures/` (testkit/src/lib.rs:235-242), files verified present.

## 9. Open questions for the next agent
- corpus.rs / fuzz_loop_stable.rs auditors: does any test exercise `fuzz_demux_avi` with assertions (not just survival), covering F-001's gap?
- If the fixture is ever regenerated with a different frame count, F-002's hardcoded ladders must be updated by hand — worth a follow-up card.
