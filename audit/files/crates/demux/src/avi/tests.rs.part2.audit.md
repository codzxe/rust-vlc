# AUDIT: crates/demux/src/avi/tests.rs#part2 (chunk: lines 626-1250 of 3128)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 625 audited / 15 units (13 tests + 2 helpers; chunk starts at a clean test boundary — line 625 is the `#[test]` of the first test; the LAST test `rec_list_overrunning_parent_bound_clamps_with_damage` (1228) is cut mid-body by the chunk boundary at 1250 — its assertions live in part3/A07's range) |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | Test module for `AviDemuxer` — the subject is reachable (cli containers/avi.rs:40, web open.rs:251; baseline first-vertical lifecycle). All 13 chunk tests ran green in baseline: orchrun-test-workspace-nofailfast.log.txt:1620-1673 (4534/7/49 suite; zero avi failures). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` 620-1259 (full chunk + boundary context); `rg -n "^fn |^struct |^#\[test\]"` (inventory); per-test baseline grep `rg -c "test avi::tests::<name> \.\.\. ok"` → 13/13 green, exact lines 1620-1673; `sed -n` on every quoted range; `rg -rn "ffmpeg"` NOTE: my own `-rn` flag made rg *display* "ffmpeg" as "n" (–replace n) — display artifact, not file content; separately discovered the Bash output pipeline eats the 2-char sequence `[m` (probe: `printf 'A[movi B [movi_form..] C ffmpeg D [x'` → `Aovi B ovi_form..] C ffmpeg D [x`) — file bytes verified clean via `od -c` and the Read tool. No `[m` occurs in 626-1250, so all quotes below are pipeline-safe. |

## 1. One-paragraph verdict
This chunk is the PCM-family and movi-walk test block, and it is the strong
kind: the tag/width matrix asserts exact codec descriptors, exact
`StreamFormat::Pcm` fields, empty damage, packet counts, and exact ladder
ticks; the refusal matrix pins six distinct typed skip reasons with sibling
survival; the timing test pins video ticks/base/DTS-mirror/keyframe/duration
and audio ticks/sample-frame math to the nanosecond; rec-list tests assert
payload order byte-exactly. Every test I traced would fail under the
corresponding mutation, and all 13 executed green in the no-fail-fast
baseline. The two findings are hygiene: the shared `drain_packets` helper
(part1's range) silently discards `Recovered` events after the first
assertion, and no test in this chunk asserts audio payload *bytes* (counts
and PTS only). The chunk boundary cuts the last test mid-body — flagged to
A07.

## 2. What actually works (verified)
— All 13 chunk tests green in baseline (exact log lines):
`block_align_disagreements…` :1620, `impossible_pcm_combinations…` :1635,
`mjpa_handler_is_filtered…` :1640, `odd_payloads_strip_the_word_pad` :1642,
`packets_carry_the_avi_ladder_timing` :1643, `pcm_wave_tag_matrix…` :1645,
`rec_list_overrunning_parent_bound…` :1654, `rec_lists_are_transparent`
:1655, `short_strh_skips…` :1664, `wave_format_extensible…` :1673 (plus
short_strf/channels_over_ceiling/float_pcm_without_sample_ladder — grep
count 1 each). Suite totals 4534/7/49; none of the 7 failures is avi.
— Timing arithmetic re-derived by hand: 1/30 s = 33,333,333.33 ns → test
asserts 33_333_333 (line 1136); 2/48000 s = 41,666.67 ns → test asserts
41_667 (line 1153); audio 8 bytes / block-align 4 = 2 sample-frames → ticks
[0, 2] (line 1148); block-align-lie test: strh dwSampleSize 5 → 10 bytes =
2 frames → ticks [0, 2] (line 1061). All exact.
— The tests exercise the PUBLIC seam: `AviDemuxer::new` → `start` →
`next_event` → `stream_format` (decoder-config seam) — no private-field
peeking, no mocks of the subject.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/avi/tests.rs:323-333 (helper in part1/A10's range;
the coverage fact lives in this chunk's tests)
Code:
```rust
fn drain_packets(demuxer: &mut AviDemuxer) -> Vec<media_types::Packet> {
    let mut packets = Vec::new();
    loop {
        match demuxer.next_event() {
            Ok(DemuxEvent::Packet(packet)) => packets.push(packet),
            Ok(DemuxEvent::EndOfStream) => break,
            Ok(_) => {}
            Err(err) => unreachable!("clean drain: {err}"),
```
What is wrong: `Ok(_) => {}` discards `Recovered` (and `Discontinuity`)
events. Tests in this chunk (e.g. `short_strh_skips_the_stream_siblings_survive`
:645, `mjpa_handler…` :1092) assert the FIRST damage event, then call
`drain_packets` — any ADDITIONAL damage the walk emits after that first
event is silently dropped and the test still passes.
Why it matters: the module's damage-accounting doctrine ("the container's
own account of what it did precedes the data") is exactly what these tests
should pin; a regression that emits spurious extra `Recovered` events would
be invisible to every drain_packets-based test. (`start_and_inspect` in
this chunk does it right — collects reasons.)
What correct looks like: `Ok(DemuxEvent::Recovered {..}) => unreachable!(
"unexpected extra damage")` in the tests that already asserted damage, or a
`drain_packets_returning_reasons` twin asserting the full reason list.
Confidence: HIGH — Verification: read of helper + all 13 call sites in
chunk. Blast radius limited: `start_and_inspect`-based tests (the majority
in this chunk) collect reasons correctly.

### F-002 [S4]
Where: crates/demux/src/avi/tests.rs:845-855 (matrix test) vs 1176-1177 (video)
Code:
```rust
        assert_eq!(packets.len(), 2);
        // PTS is each chunk's ladder position (start, AVI convention):
        // 0, then 4 sample-frames later on the 1/8000 base.
```
What is wrong: the PCM matrix tests (plain-tag :807-857 and extensible
:860-923) assert packet COUNT and PTS ticks but never the payload BYTES of
the audio packets — no `p.data.as_slice()` comparison anywhere in 626-1250
for audio. Video tests do assert bytes (`odd_payloads` :1176-1177
`assert_eq!(packets[0].data.as_slice(), b"abcde")`).
Why it matters: a pool/fill regression that corrupted only block-align-sized
(e.g. 4/8/16-byte) reads would pass every audio test in this chunk.
Mitigated repo-wide: corpus digest tests elsewhere cover byte-exactness
(baseline :909 `avi_extraction_matches_writer_declared_frame_digests ... ok`,
:1183 `avi_audio_extraction_matches_writer_declared_stream_digest ... ok`)
— so this is a chunk-local asymmetry, not a repo-wide hole.
What correct looks like: one `assert_eq!(packets[0].data.as_slice(),
payload.as_slice())` in the matrix loop.
Confidence: HIGH — Verification: read of both tests; baseline lines :909/:1183
confirm the digest coverage lives outside this chunk.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `short_strh_skips_the_stream_siblings_survive` | 626-646 | REAL | — | ShortStrh stream dropped ("strh-too-short", skipped=0), audio sibling survives, 1 packet. Would fail if containment broke (start would Err → unreachable! panics). Baseline :1664 |
| `short_strf_skips_the_stream_siblings_survive` | 649-707 | REAL | — | BOTH directions (39-byte video strf < 40 BITMAPINFOHEADER; 15-byte audio strf < 16 WAVEFORMATEX); asserts surviving track's ordinal and that ONLY its chunks deliver. Baseline green |
| `channels_over_ceiling_skips_the_stream_siblings_survive` | 710-743 | REAL | — | nChannels=0xFFFF → "audio-channels-over-ceiling", video survives, exactly 1 packet on track 0 |
| `start_and_inspect` (helper) | 756-785 | REAL | — | drains ALL events; Err → unreachable! (no swallowed errors); returns tracks/format/reasons/packets — the honest drain F-001 wants |
| `audio_case` (helper) | 790-804 | REAL | — | ssize×4-byte payload = exactly 4 sample-frames; deterministic content |
| `pcm_wave_tag_matrix_maps_every_width_and_kind` | 807-857 | REAL | S4 | 6 combos: descriptor + exact StreamFormat + zero damage + count + ticks [0,4]. F-002 (no byte assert). Baseline :1645 |
| `wave_format_extensible_resolves_subformat_guids` | 860-923 | REAL | — | 5 GUID combos incl. 24-bit/8ch and f64le/2ch; byte-level reproduction of ffmpeg's extensible strf (comment :861-864) — partial counter-evidence to mod.rs's fixture-only conformance story. No PTS assert (matrix test covers the shared ladder). Baseline :1673 |
| `impossible_pcm_combinations_skip_the_stream_siblings_survive` | 926-993 | REAL | — | 6 refusals, 5 distinct reasons strings incl. μ-law GUID and truncated-extensible; sibling survival + first-event-is-damage each |
| `float_pcm_without_sample_ladder_skips_the_stream` | 996-1027 | REAL | — | ssize=0 float → "pcm-sample-size-zero" (tightened per comment :997-1000) |
| `block_align_disagreements_are_damage_notes_not_skips` | 1030-1062 | REAL | — | the strongest here: strh dwSampleSize (5) pinned as ladder truth over lying block_align; ticks [0,2] exact; damage reason asserted present. Baseline :1620 |
| `mjpa_handler_is_filtered_like_any_unsupported_handler` | 1065-1093 | REAL | — | probe/demuxer stage agreement (V8); audio sibling survives. Baseline :1640 |
| `packets_carry_the_avi_ladder_timing` | 1100-1158 | REAL | — | exact video ticks [0,1,2] base 1/30 + DTS==PTS + keyframe + 33,333,333 ns; audio ticks [0,2] + 41,667 ns; with idx1 (build flag true). Baseline :1643 |
| `odd_payloads_strip_the_word_pad` | 1161-1179 | REAL | — | payload bytes exact; damage_total()==0 — pad is not damage. Baseline :1642 |
| `rec_lists_are_transparent` | 1182-1225 | REAL | — | hand-built (not via `build`) rec list; order r0,r1,t0 byte-exact. Baseline :1655 |
| `rec_list_overrunning_parent_bound_clamps_with_damage` | 1228-1250 (setup; body cut at boundary) | REAL | — | BOUNDARY: setup (rec list overrunning into idx1 region, V6) in my range; assertions 1260+ in A07's part3. Ran green as a whole: baseline :1654. Cross-ref to A07 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 13 | YES | concrete mutations reasoned: (a) tag→sample mapping flip (s24le as s16le) fails matrix `format` assert; (b) ladder off-by-one (audio ticks [0,5]) fails :855/:1061/:1148; (c) containment regression (start Err) hits `unreachable!("containment")` panics in every test; (d) pad leaking into payload fails :1176; (e) rec-list opacity (rec chunks dropped) fails :1224 count; (f) refusal-reason drift fails the 6-case matrix. No tautologies found; no `#[ignore]`; no mock of the subject (real AviDemuxer over mem_stream). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the randomized corpus showed ffmpeg's AVI muxer emitting 0xFFFE extensible strf for most multi-channel PCM" | tests.rs:747-750 | REAL-UNPROVEN | the extensible shapes ARE byte-level reproduced in :860-923 and green in baseline; the "randomized corpus" run itself is not committed in audit/runs/baseline (no corpus-generation log for AVI) — the claim's provenance is the comment only |
| probe/demuxer stage agreement for mjpa (V8) | tests.rs:1066-1068 | REAL | test green (baseline :1640); the probe side is W04-A01's signature/avi.rs — cross-ref |
| WAVE family vocabulary (tag 1 int 8/16/24/32, tag 3 float 32/64, 0xFFFE GUID) | mod.rs:85-89 doc vs these tests | REAL | matrix + extensible tests cover exactly that set, green |

## 7. Dead / duplicated / unreachable code
- None in the chunk. `start_and_inspect` and the inline loops are similar in
  shape but assert different things (reasons vs first-event) — not
  duplication worth flagging.

## 8. Dependencies & flags
- Test module: declared `#[cfg(test)] mod tests;` at mod.rs:1270-1271 (part2/
  A07's range) — unit tests, not shipped. The baseline log labels them
  `avi::tests::*` (demux lib test binary). No feature gates in the chunk;
  `header::PcmSample`/`StreamFormat` imported from the sibling module (real
  production types, not test doubles).

## 9. Open questions for the next agent
- A07 (part3, 1251+): the cut test `rec_list_overrunning_parent_bound…`
  asserts the clamp damage — verify its assertions pin the exact reason
  ("rec-extends-past-list-bound") and the salvaged chunk count, and audit
  `unknown_movi_chunks_skip_with_damage` (:1289) — its JUNK-splice pattern
  is the closest existing coverage to my part1 F-003 (undeclared-stream
  chunks), but it covers `Other` fourccs only, NOT `NNdc` tags of
  undeclared ordinals — the F-003 gap stands.
- A10 (part1, helpers 19-334): `build()`'s idx1 writer (flag `true`) —
  confirm it flags every entry AVIIF_KEYFRAME (the timing test's keyframe
  assert depends on it) and that `Stream::ShortStrh`/`Other` constructors
  produce the claimed corruption (I traced usage, not construction).
- Tooling warning for all agents: the Bash output pipeline silently eats
  the 2-char sequence `[m` (probe reproduced) and `rg -rn PAT` means
  `--replace n` — verify every verbatim quote containing `[m` with the
  Read tool or `od -c`, never trust sed/grep display alone for those lines.
