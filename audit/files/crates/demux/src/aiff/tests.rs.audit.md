# AUDIT: crates/demux/src/aiff/tests.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Lines / functions | 537 / 26 (6 helpers + 20 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test module, `#[cfg(test)]`, gated behind `mod tests` in aiff/mod.rs:71-72); the code UNDER test is reachable (see aiff/mod.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read; `sed -n` on every quoted range; `rg -n "pts|ticks" crates/demux/tests/wav_aiff_fixtures.rs` (no per-packet PTS assertions in the differential suite); `rg -n "aifc_raw_u8" crates/demux/tests/wav_aiff_fixtures.rs` (u8 lane covered at fixture level); fixture sizes via Truth table (frames 8_820 × 2ch × 2B = 35,280 B < 64 KiB → all fixtures single-packet); baseline nofailfast log lines 1534-1551 (20/20 aiff::tests ok) |

## 1. One-paragraph verdict
A genuinely hostile unit suite for the AIFF demuxer: damage-reason ordering
(`Recovered` before the prefix packet), SSND offset semantics, odd-payload pad
walking, the AIFC vocabulary, prefix-then-typed-truncation, span/frame accounting,
and trait misuse are all pinned with exact codes and bytes, and all 20 tests pass in
the committed baseline. Two honesty defects: the test named
`pts_ladder_and_span_count_sample_frames` exercises a **single** packet (1,200-byte
payload < the 64 KiB packet ceiling), so the mid-stream PTS ladder
(`frames_emitted` increments) is verified nowhere in the repo — deleting the
increment in mod.rs:550 would pass the entire aiff suite; and
`error_constructors_keep_their_codes` is a weaker near-duplicate of
aiff/error.rs's own constructor test that ends in two assertion-free constructions.
The mod.rs audit's two error-misclassification paths (declared-short COMM/SSND) have
zero coverage here — the suite pins only the genuinely-truncated variants.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` lines 1534-1551: all 20 `aiff::tests` `... ok`.
— verified by read + baseline line 2229: the differential lane (`every_aiff_fixture_matches_the_ffmpeg_truth`) pins 4 AIFF fixtures (pcm_s16be/s24be, sowt s16le, raw u8) bit-exactly against ffmpeg MD5 dumps, including the nice cross-check that the `sowt` fixture's MD5 equals the little-endian WAV fixture's (`5d105ceb…`, wav_aiff_fixtures.rs:135 vs 150).
— verified by read: `odd_comm_payload_takes_its_pad` (197-216) fails if the walk drops `pad_for` (SSND header would misparse); `declared_ssnd_past_actual_delivers_prefix_then_truncated` (285-312) pins the exact three-step sequence Recovered(2, "ssnd-extends-past-stream") → prefix packet → `demux.aiff.truncated`.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/aiff/tests.rs:144-151 (test spans 143-165)
Code:
```rust
fn pts_ladder_and_span_count_sample_frames() {
    let samples = vec![0u8; 300 * 4];
    let mut aiff = started_aiff(minimal_aiff(2, 8_000, 16, &samples));
    match aiff.next_event() {
        Ok(DemuxEvent::Packet(p)) => match p.pts {
            Stamp::Present(ts) => assert_eq!(ts.ticks, 0),
            Stamp::Missing => unreachable!("pts must be present"),
        },
```
What is wrong: The payload is 1,200 bytes — under `MAX_PACKET_BYTES` (64 KiB), so
`step()` emits **one** packet and the test checks only `pts == 0`. The "ladder"
has exactly one rung. The mid-stream increment (`self.frames_emitted +=
want_frames`, aiff/mod.rs:550) and the `pts_ticks` clamp are dead-untested: a
mutation deleting the increment (every subsequent packet would carry pts 0) passes
this file's suite AND the differential suite — `drain_aiff`
(wav_aiff_fixtures.rs:193-210) collects payload bytes, damage reasons and the
terminal code but never asserts a per-packet PTS, and every fixture payload
(8,820 frames × ≤4 B ≈ 35 KiB) is also single-packet.
Why it matters: PTS is the A/V-sync input; a demuxer that timestamps every packet
at zero after the first would demux "bit-exactly" (payload MD5s unchanged) while
breaking playback timing — exactly the class of defect this audit exists to catch,
and the one test whose name promises to catch it does not.
What correct looks like: One test with `samples.len() > MAX_PACKET_BYTES` (e.g.
2 × 64 KiB + tail) asserting the PTS of packets 1..n (ticks 0, 16384@8 kHz-equivalent
ladder, …) and the final packet's duration; or per-packet PTS assertions in the
differential drain.
Confidence: HIGH — Verification: read (packet arithmetic: mod.rs:515 `want_frames =
(whole_bytes / frame).min(MAX_PACKET_BYTES / frame)`; single packet for ≤64 KiB
payloads; fixture sizes from Truth table)

### F-002 [S4]
Where: crates/demux/src/aiff/tests.rs:521-537
Code:
```rust
fn error_constructors_keep_their_codes() {
    assert_eq!(error::not_aiff(b"FORM").context.code, "demux.aiff.not-aiff");
    assert_eq!(error::no_comm().context.code, "demux.aiff.no-comm");
    assert_eq!(error::no_ssnd().context.code, "demux.aiff.no-ssnd");
    assert_eq!(
        error::unsupported_compression(0).context.code,
        "demux.aiff.unsupported-compression"
    );
    let _ = CommChunk {
        channels: 1,
        num_frames: 0,
        sample_size: 16,
        sample_rate: 44_100,
        compression: None,
    };
    let _ = chunks::MARK;
}
```
What is wrong: A weaker near-duplicate of `aiff::error::tests::
errors_carry_demux_stage_and_aiff_codes` (error.rs:156-180), which already covers
all ten codes plus stage and Display format. This one re-checks four codes and adds
nothing except two assertion-free constructions (`let _ = CommChunk {…}` and
`let _ = chunks::MARK;`) whose only function is touching the symbols.
Why it matters: Duplication invites drift (add a code to error.rs's list, forget
here — or vice versa) and the trailing constructions are coverage theater that a
line-count metric would count as "tested surface".
What correct looks like: Delete this test (error.rs owns constructor codes), or
reduce it to the two constructions with a comment saying why they need touching
(e.g. dead-code lint), under an honest name.
Confidence: HIGH — Verification: read (error.rs:156-180 compared)

### F-003 [S4]
Where: crates/demux/src/aiff/tests.rs:271-282 (representative site of the gap)
Code:
```rust
fn truncated_comm_is_typed_truncation() {
    let full = minimal_aiff(1, 44_100, 16, &[0x12, 0x34]);
    // Cut 4 bytes into COMM's payload (head is 12, chunk header 8).
    let cut: std::sync::Arc<[u8]> = full[..24].into();
```
What is wrong: Coverage gap, not a wrong assertion. The suite pins the
**physically-cut** COMM (stream ends mid-chunk → `demux.aiff.truncated`, correct),
but has no case for a **declared-short** COMM (AIFC ckSize 18-21) or a
declared-short SSND (ckSize < 8) — the exact inputs where mod.rs misclassifies
structure as truncation with a fabricated `expected` (aiff/mod.rs audit F-001/F-002,
S3). Also untested: duplicate COMM/SSND damage paths (`duplicate-comm`/
`duplicate-ssnd`, mod.rs:329-330/354-355) and seek landing exactly at the end
(mod.rs:594-598 allows it by design).
Why it matters: The one behavior defect found in the demuxer lives on an untested
path; a regression test here is how the S3s would have been caught pre-merge.
What correct looks like: Add cases: AIFC COMM with ckSize ∈ {18,21} asserting
`bad-structure` (will fail today — that is the point), SSND ckSize 4, duplicate
COMM, seek target == total_frames expecting a legal end-landing.
Confidence: HIGH — Verification: read (all 20 tests enumerated; no such cases exist; `rg -n "duplicate" tests.rs` → none)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `drain` (helper) | 17-31 | REAL | — | asserts clean drain; `unreachable!` on Err — only used on files expected to drain |
| `started_aiff` (helper) | 33-38 | REAL | — | new+start, panics loudly on refusal |
| `form` (helper) | 42-51 | REAL | — | builds FORM with correct big-endian size patch |
| `comm_chunk` (helper) | 53-71 | REAL | — | big-endian COMM + pstring tail + even pad |
| `ssnd_chunk` (helper) | 73-85 | REAL | — | `declared_extra` enables over/under-declaration lies |
| `mark_and_inst_chunks` (helper) | 87-106 | REAL | — | spec-shaped MARK/INST bodies for skip-tolerance |
| `drains_classic_pcm16be_whole_frames` | 108-124 | REAL | — | payload equality, codec, damage 0, offset/block — strong |
| `big_endian_samples_arrive_verbatim` | 126-141 | REAL | — | pins no byte-order normalization at the demuxer |
| `pts_ladder_and_span_count_sample_frames` | 143-165 | SHALLOW | F-001 | single packet; ladder = one rung; span end + pool quantum pinned |
| `mark_and_inst_chunks_are_skip_tolerant` | 167-178 | REAL | — | payload survives, zero damage events |
| `ssnd_offset_skips_into_the_sound_data` | 180-194 | REAL | — | offset semantics exact |
| `odd_comm_payload_takes_its_pad` | 196-216 | REAL | — | pad-walk correctness; would fail on `pad_for` regression |
| `aifc_compression_vocabulary_demuxes` | 218-244 | REAL | — | 4 of 5 vocab lanes end-to-end (raw u8 covered in fixtures) |
| `truncated_head_is_typed` | 246-268 | REAL | — | FORM-cut → truncated; FORM…WAVE → not-aiff |
| `truncated_comm_is_typed_truncation` | 271-282 | REAL | F-003 | genuine truncation pinned; declared-short sibling missing |
| `declared_ssnd_past_actual_delivers_prefix_then_truncated` | 284-312 | REAL | — | the three-step contract, exact reason + bytes + code |
| `missing_comm_and_ssnd_are_typed` | 314-337 | REAL | — | no-comm / no-ssnd |
| `zero_or_offset_past_ssnd_is_no_ssnd` | 339-354 | REAL | — | offset past payload → no-ssnd |
| `comm_declarations_disagreeing_with_ssnd_are_damage` | 356-373 | REAL | — | bytes-are-authority damage note, full payload still delivered |
| `non_normalizable_rate_is_structural` | 375-393 | REAL | — | denormal exponent-0 shape → bad-structure |
| `unsupported_depths_are_typed` | 395-425 | REAL | — | classic 8-bit and 12-bit NONE refusals |
| `trailing_partial_frame_is_damaged_and_dropped` | 427-446 | REAL | — | `ssnd-not-frame-aligned` + whole-frame prefix delivered |
| `trait_misuse_is_typed_not_a_panic` | 448-477 | REAL | — | not-started for next_event/seek AND start-after-misuse (pins the permanent sticky poisoning — see mod.rs audit F-004) |
| `seek_maps_frames_and_reanchors` | 479-509 | REAL | — | landing 500/1000, past-end refusal, reanchor byte count; exact-end landing untested |
| `probe_recognizes_both_forms` | 511-518 | REAL | — | 5 assertions incl. short-header false |
| `error_constructors_keep_their_codes` | 520-537 | SHALLOW | F-002 | duplicate of error.rs's stronger test + 2 assertion-free constructions |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pts_ladder_and_span_count_sample_frames | NO — for the mid-stream ladder | single packet; only pts[0] checked (F-001); span end and pool quantum would catch other breakage |
| error_constructors_keep_their_codes | only for 4 of 10 codes | error.rs's own test covers all (F-002) |
| all other 18 | YES | exact byte/code/reason assertions traced against mod.rs behavior; mutation reasoning per §2-3 |
Baseline: 20/20 ok (nofailfast 1534-1551); the two SHALLOW entries pass for the wrong reason — they would not fail for the defect class their names describe.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The ffmpeg-generated fixtures get their differential + mutated-family suite in wav_aiff_fixtures.rs" | module doc lines 4-6 | REAL | file exists, 8/8 green in baseline (log 2223-2234) |
| (implied) "PTS ladder" tested | test name line 143-144 | FALSE-ish | one rung only (F-001) — name overpromises |

## 7. Dead / duplicated / unreachable code
- `error_constructors_keep_their_codes` — duplicated weaker twin (F-002).
- `minimal_aiff`'s silent fallback in test_support.rs:35-38 (`Err(_) =>
  44100-bytes`) is dead for every u32 rate (encoder cannot fail in u32 range:
  integral, < 2^64, exponent ≤ 16414 < 0x7FFF) — if it ever fired it would
  silently build a 44.1 kHz file for a different requested rate. Cross-note for
  whoever owns aiff/test_support.rs (not in my list).

## 8. Dependencies & flags
Test-only: uses `super::*` surface, `media_types::time`, `plugin_api` event types,
and the crate's own test_support. No features, no unsafe, no external deps. House
no-panic style honored via `unwrap_or_else(|e| unreachable!(...))` (acceptable in
tests per crate lint policy — flagged style, not a defect).

## 9. Open questions for the next agent
- Add the multi-packet PTS test (F-001) — the same gap likely exists in the WAV
  twin's tests (`crates/demux/src/wav/tests.rs`, W04-A04's wave) — worth a
  cross-check.
- Should `start()` after a misuse error recover instead of refusing forever
  (pinned at lines 472-476)? Design question for the plugin-api owner.
