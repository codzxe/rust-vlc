# AUDIT: crates/demux/tests/mp4_fixtures.rs

| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-12 |
| Lines / functions | 284 / 6 (4 `#[test]`s + 2 helpers + 2 const tables) |
| File verdict | REAL (a strong generated-fixture suite with real per-track assertions; defects are hygiene-level) |
| Reachable from shipped entry point? | N/A (integration test). Exercises `Mp4Demuxer` through the REAL access stack (`open_via_input` → `input::FileAccess`, tests/common/mod.rs:546-550). `Mp4Demuxer` itself is reachable via the CLI mp4 lane. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |

Commands run (adaptation: no cargo; static analysis + `sed -n` quote verification + fixture-byte inspection):
- `sed -n` verification of every quote below (verbatim)
- `ls tests/fixtures/generated/{valid,malformed}/` — all 21 referenced fixtures exist (7 valid incl. `.mov`/`.3gp`, 14 malformed)
- Byte-offset inspection of the valid MP4 fixtures (`moov`/`mdat` positions) — hard evidence for P-002: `mp4_h264_aac.mp4` moov@36 (front); `mp4_qt.mov` moov@11903, `mp4_chapters.mp4` moov@22389, `mp4_edts.mp4` moov@12594, `mp4_3gp_h263.3gp` moov@8824 (all moov-at-end); `mp4_frag*.mp4` moov@32
- `grep -rn "stszlie0\|mp4_mjpeg_pcm__"` — the mp4 malformed derivatives NOT in this file's list are exercised in `crates/codecs/tests/fuzz_survival.rs`, `crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs`, `crates/media-probe/tests/fixtures_matrix.rs`, `tests/integration/first-vertical/tests/recovery_matrix.rs` (no orphans)
- Per-test pass status: UNVERIFIED (no cargo); suite-level cites W00 baseline (1751 pass / 2 fail (golden drift) / 25 ignored)

## 1. One-paragraph verdict

This is what the malformed-input half of the MP4 demux story should look like: seven ffmpeg-authored valid fixtures (progressive faststart AND moov-at-end, QuickTime, fragmented with/without sidx, chapters, edit lists, 3GP H.263) each asserted on track count, codec, kind, fragmentation flag, chapter count, per-track packet presence and non-empty payloads; plus fourteen malformed derivatives run through both the fuzz harness (survival) and a typed pass that verifies start-refusals carry stage `Demux` and a `demux.mp4.*` code. Nothing is mocked, nothing is circular — the parents come from ffmpeg, the expectations from the card's spec table. The four findings are hygiene: a keyframe metric that is counted, misdescribed, and never asserted; a header that mislabels four moov-at-end fixtures as faststart; mid-walk errors swallowed without the typedness check the test name promises; and a weak `!= 0` oracle on the elst shift.

## 2. What actually works (verified)

- `valid_mp4_fixtures_demux_fully` (139-175): per-fixture asserts on track count, per-track codec string + media kind, `is_fragmented`, chapter count, `total > 0`, and `packets.len() == codecs.len()` — which (handles being dense, as they are for these two-track fixtures) forces **every** track to have emitted at least one packet. A regression in track enumeration, codec naming, fragmentation detection, or the demux walk fails this test.
- `chapters_fixture_surfaces_titled_markers` (178-187): count 3, all titles non-empty, strictly monotone starts (`w[0].start.0 < w[1].start.0`). Fails on any chapter-parse regression.
- `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` (254-284): the start-refusal path asserts `err.stage == PipelineStage::Demux` AND `err.context.code` starts with `"demux.mp4."` — a real typedness oracle, not just survival; packet payloads asserted non-empty ("fake packet"); both loops budgeted at 500 k events.
- Both malformed tests use the real access stack (`open_via_input`), not an in-memory mock — the plugin-api seam is exercised end to end.
- Coverage cross-check: the mp4-family malformed derivatives absent from this file's 14-entry list (`mp4_mjpeg_pcm__*` including the crafted `stszlie0`) are consumed by four other suites — nothing orphaned.
- Fixtures verified present on disk; a missing fixture would fail loudly (`fixture_path().must()` + `FixtureError::Missing`, testkit lib.rs:235-242).

## 3. Findings

### P-001 [S4] `drain`'s keyframe metric: comment lies about what is counted, and the value is never asserted
Where: crates/demux/tests/mp4_fixtures.rs:107-111, 126-128, 166
Code:
```rust
/// Drain every event (bounded), returning per-track packet counts and
/// the first video track's keyframe count.
fn drain(mp4: &mut Mp4Demuxer) -> (Vec<u64>, u64, u64) {
```
and
```rust
                if p.keyframe {
                    keyframes += 1;
                }
```
and the discard site:
```rust
        let (packets, _key, total) = drain(&mut mp4);
```
What is wrong: The counter increments on every keyframe packet regardless of track — there is no "first video track" filter, so the doc comment is false. Worse, `_key` is discarded by the only caller: no test in the file asserts anything about keyframes. MP4 sync-sample (`stss`) handling therefore has zero behavioral pinning here while the helper's documentation implies it is being measured.
Why it matters: The MKV side of this same wave found a keyframe-flag lie (cluster.rs C-001: all BlockGroup packets marked non-keyframe). A sibling metric that silently measures-and-drops the same flag on the MP4 side is exactly how such defects survive test suites.
What correct looks like: `assert!(key > 0)` per video-bearing fixture (a demux that never marks keyframes should fail), plus fix or delete the comment.
Confidence: HIGH — Verification: read.

### P-002 [S4] Header claims "+faststart progressive shapes"; four of five progressive fixtures are moov-at-end
Where: crates/demux/tests/mp4_fixtures.rs:9-11
Code:
```rust
//! The parents were authored with the sandbox ffmpeg (Lavf61) —
//! `-movflags +faststart` progressive shapes and `-movflags
//! +frag_keyframe+empty_moov` fragmented shapes — by the prior
```
What is wrong: Byte-offset inspection of the committed fixtures: `mp4_qt.mov` moov@11903, `mp4_chapters.mp4` moov@22389, `mp4_edts.mp4` moov@12594, `mp4_3gp_h263.3gp` moov@8824 — all four carry `moov` after `mdat` (the moov-at-end layout faststart exists to avoid). Only `mp4_h264_aac.mp4` (moov@36) matches the description.
Why it matters: The doc understates the suite's actual coverage — which is GOOD (both moov placements are demuxed and asserted, covering the wave's moov-at-end hostile question) — but a reader auditing "which shapes are covered" from the header gets the wrong map. Doc-vs-reality mismatch, benign direction.
What correct looks like: Describe the set as it is: one faststart progressive parent, four moov-at-end parents (QT/mov, chapters, edts, 3gp), two fragmented.
Confidence: HIGH — Verification: read + fixture byte inspection (offsets above).

### P-003 [S4] Mid-walk errors are swallowed unexamined, while the test name promises "only ever … typed"
Where: crates/demux/tests/mp4_fixtures.rs:274-282
Code:
```rust
            match mp4.next_event() {
                Ok(DemuxEvent::EndOfStream) | Err(_) => break,
                Ok(DemuxEvent::Packet(p)) => assert!(!p.data.is_empty(), "{fixture}: fake packet"),
                Ok(_) => {}
            }
```
What is wrong: The start-refusal path (261-272) verifies stage and code prefix — a real typedness check. The walk-error path accepts any `Err` and breaks: an `EngineError` minted at the wrong stage, with the wrong code, or even a panic-adjacent failure mode surfaced as Err would pass unnoticed. The test name (`malformed_mp4_fixtures_only_ever_refuse_or_demux_typed`) covers both halves; the body only checks one.
Why it matters: The distinction matters in practice — this demuxer's recovery design routes mid-walk damage to `Recovered` events (Ok) and stream truncation to Err; a regression that starts leaking arbitrary untyped errors mid-walk would still pass this test.
What correct looks like: `Err(e) => { assert_eq!(e.stage, Demux, ...); assert!(e.context.code.as_str().starts_with("demux.mp4."), ...); break; }`.
Confidence: HIGH — Verification: read.

### P-004 [S4] The elst oracle is "any non-zero shift" — a wrong shift passes
Where: crates/demux/tests/mp4_fixtures.rs:218-221
Code:
```rust
    assert!(
        pts != 0,
        "the edts fixture's first video PTS must be shifted, got {pts}"
    );
```
What is wrong: The test guards exactly one regression — elst handling collapsing the shift to zero. A shift of the wrong magnitude, wrong sign, or applied to the wrong timeline (e.g., media_time added instead of subtracted) passes. The fixture is deterministic (committed bytes), so an exact expected tick value (or at least a plausible range from the generating ffmetadata) is checkable.
Why it matters: Edit-list math is the classic silent-corruption area of MP4 demuxing; a one-bit oracle invites exactly the drift it should catch.
What correct looks like: Pin the expected first PTS in ticks for the committed fixture (it is immutable), or compute the expectation from the parsed `elst` and cross-check.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Valid` + `VALID` table | 26-97 | REAL | — | 7 specs, all fixtures verified on disk; chapters fixture's hidden chapter-trak note (73-74) matches the 2-codec expectation. |
| `started` | 99-105 | REAL | — | Real access stack; `unreachable!` on failure = fail-loud for valid fixtures (appropriate). |
| `drain` | 107-137 | REAL (defect) | S4 | P-001: comment-vs-code lie + discarded metric; otherwise sound (dense-handle growth, budget, non-empty payload assert). |
| `valid_mp4_fixtures_demux_fully` | 139-175 | REAL | — | See §2; would fail on enum/codec/fragmentation/chapter/walk regressions. |
| `chapters_fixture_surfaces_titled_markers` | 178-187 | REAL | — | Count + non-empty titles + strict monotonicity. |
| `edts_fixture_shifts_the_first_pts` | 192-222 | REAL (weak oracle) | S4 | P-004: `!= 0` only. |
| `MALFORMED` table | 224-239 | REAL | — | 14 of the committed mp4-family derivatives; the rest verified consumed by other suites (no orphans). |
| `malformed_mp4_fixtures_survive_the_fuzz_harness` | 243-249 | REAL | — | Zero assertions by design (the harness's documented survival contract, fuzz.rs:6-10); honest as the `+FB` AC — it is the fixed-input complement to the (never-gated) libFuzzer campaign per the fuzz.rs audit. |
| `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` | 254-284 | REAL (defect) | S4 | P-003: walk-error half unverified despite the name; start-refusal half is a genuine typedness check. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `valid_mp4_fixtures_demux_fully` | YES | Concrete codec/kind/count/fragmentation/chapter expectations per fixture; every track must emit packets. |
| `chapters_fixture_surfaces_titled_markers` | YES | Exact count, non-empty titles, strict monotone starts. |
| `edts_fixture_shifts_the_first_pts` | YES (narrowly) | Only guards the shift collapsing to zero (P-004). |
| `malformed_mp4_fixtures_survive_the_fuzz_harness` | NO (by contract) | Survival-only; the fuzz harness swallows errors; zero assertions in the test body. Honest per the harness's documented contract, but contributes pass-count, not behavior. |
| `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` | PARTLY | Start-refusals: YES (stage+code pinned). Mid-walk errors: NO (P-003). |

Mutation reasoning (not run — no cargo): flipping `is_fragmented()` to constant-false fails specs 3/4; dropping the chapters parse fails tests 1 (chapters count) and 2; zeroing the elst media_time application fails test 3; emitting empty packets fails the "fake packet" assert in both valid and malformed walks.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "demux through the real access stack" | header, line 4 | REAL | `open_via_input` → `input::FileAccess` (common/mod.rs:546-550). |
| "no panic, no hang, no unbounded allocation, nothing fake" | header, line 7 | REAL | Event budgets (115, 276), non-empty payload asserts (129, 279); "nothing fake" = the payload assert. |
| "the deterministic malformed derivatives (trunc/flip/garbage) run through both the `+FB` fuzz harness and a full typed pass" | header, lines 5-7 | REAL | Tests at 243-249 and 254-284; "typed" half-true per P-003. |
| "`-movflags +faststart` progressive shapes" | header, lines 9-11 | FALSE (4 of 5 progressive fixtures are moov-at-end) | P-002: byte offsets measured. Coverage is better than claimed. |
| "fixtures on disk are only half the contract" | header, lines 12-13 | REAL | The wiring (this suite) exists and asserts. |

## 7. Dead / duplicated / unreachable code

- P-001's keyframe counter: computed, misdescribed, discarded — dead metric.
- No dead tests; no `#[ignore]`; all four tests run in the default target.

## 8. Dependencies & flags

No feature gates. Runtime deps: committed fixtures under `tests/fixtures/generated/` (existence-checked by testkit), `input` crate via `common`, `testkit::Must`. No ffmpeg/ffprobe needed at test time (fixtures are pre-generated — unlike the mkv attachments suite, this one is CI-reproducible everywhere).

## 9. Open questions for the next agent

- Where is MP4 keyframe (`stss`/sync-sample) behavior actually asserted? Not here (P-001); if `mp4/tests.rs` does not pin it either, the MP4 mirror of the MKV BlockGroup keyframe lie (cluster.rs C-001) would be invisible. (mp4/tests.rs owner.)
- Is the demuxed MP4 packet stream pinned anywhere the way AVI's is (`.oracle.sha256` files exist for `avi_mjpeg_pcm` and `mp4_mjpeg_pcm` — who consumes the mp4 one)? (corpus/compat suites' owners.)
- The 3GP fixture exposes only one track (h263, no audio) — was a 3GP audio (AMR/AMR-WB) lane ever generated? The codec map (`h263` descriptor) suggests the demuxer names it; nothing here exercises 3GP audio refusal. (fixtures generator owner: `tools/xtask/src/cmd/fixtures.rs`.)
