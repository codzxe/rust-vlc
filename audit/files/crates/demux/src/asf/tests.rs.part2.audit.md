# AUDIT: crates/demux/src/asf/tests.rs#part2 (lines 527-1052)
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 526 audited / 13 (11 tests + `FixtureTruth` struct + `FIXTURES` truth table; helpers at 460-526 are part1's) |
| File verdict | REAL (test code); subject lane UNREACHABLE from shipped entry points (see header.rs#part2 audit: no asf probe signature, no containers/mod.rs arm) |
| Reachable from shipped entry point? | Tests: cfg(test) (asf/mod.rs `#[cfg(test)] mod tests;`). Subject: NO — `AsfDemuxer::new` has no caller outside tests; `apps/cli/src/containers/mod.rs:50-75` has no asf arm; media-probe has no ASF signature. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | baseline grep for all 11 test names (all `ok`, log 1561-1588); `sed -n` quote verification; `ls -la crates/demux/src/asf/fixtures/` + arithmetic cross-check of all 10 FIXTURES entries against committed file sizes and object offsets (10/10 consistent); `rg -n "fn seek" -A 60 asf/mod.rs` (linear-scan + budget gate traced, mod.rs:699-761); `rg -n "undeclared" asf/mod.rs` → mod.rs:477-480 silent-skip site; `rg -n "MAX_LINEAR_SEEK_PACKETS"` (=4096, mod.rs:160). |

Chunk boundary: nominal part2 = 527-1052. Audited tests 528-880 plus
`FixtureTruth` (889-916) and `FIXTURES` entries 918-1052; the final table entry
(`asf_adpcm_ms.wma`, starting 1051) and the fixture-differential tests
(`fixture_dir`/`generate`/walk/cross-check, 1053+) are part3's (A07). Helpers
`payload_bytes`/`data_packet`/`object`/`ms_base`/`span_at` (460-526) are
part1's (A02), read for context only.

## 1. One-paragraph verdict
Eleven executed, mutation-sensitive tests covering the ASF-b packet layer:
byte-exact field parsing with data ranges sliced back into the source packet,
multi-payload ordering, four typed malformed-packet refusals, fragment
reassembly (including abandoned predecessors), four typed assembler refusals
(including the `u32::MAX` declared-size memory ceiling), preroll-subtracted
stamps, undeclared-stream skipping, the min==max packet-size gate, indexed
seek/reanchor, the bounded linear scan, and the scan-budget refusal. All pass
in the baseline (`orchrun-test-workspace-nofailfast.log.txt:1561-1588`). The
`FIXTURES` truth table in my range is arithmetically self-consistent to the
byte — I re-derived header+data+index offsets vs file sizes for all 10 entries
and all close exactly, and all 10 committed fixture files' sizes match the
table. Two defects: the undeclared-stream test blesses a fully silent skip
that contradicts the crate's own "never silent" damage policy, and the section
comment claims the fixtures are "not committed" when 11 of them are.

## 2. What actually works (verified)
— 11/11 tests pass: baseline log 1561,1568,1571-1573,1575,1577,1579,1580,1583,1588.
— FIXTURES truth table byte-consistency (hand-verified, 10/10 entries):
   e.g. `wmv_wmv2_64x48.wmv`: header 587 + data 6_450 = simple_index offset
   7_037 + size 86 = file_size 7_123 = actual committed file (7123 bytes);
   audio-only `asf_pcm_s16le.wma`: 456 + 25_650 = 26_106 = file size ✓.
— Preroll arithmetic pinned behaviorally: payload pts 150 with preroll 100 →
   emitted ticks 50 (line 736); 250 → 150 (line 747).
— The memory-ceiling refusal is real: declared object size `u32::MAX` with one
   byte fed → `demux.asf.payload.oversized` (683-686), not an allocation.

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/asf/tests.rs:752-760 (subject site: asf/mod.rs:477-480)
Code:
```
#[test]
fn payloads_for_undeclared_streams_are_skipped_not_fatal() {
    let wfx = waveformatex(1, 1, 1000, 2, 16);
    let stray = object(9, true, 1, 10, &[0xAA; 4]); // Header declares stream 1 only.
    let packets = vec![data_packet(0, 20, &[stray], 48)];
    let mut asf = started(asf_with_packets(&wfx, 0, 48, &packets, None));
    assert_eq!(ok(asf.start()).len(), 1);
    assert!(matches!(ok(asf.next_event()), DemuxEvent::EndOfStream));
}
```
What is wrong: The test pins the SKIP but not its observability. The
implementation site is a bare `continue` — `asf/mod.rs:477-480`: "A payload for
a stream the header never declared: skipped, never fatal (the demux recovery
seam). `continue;`" — no damage note, no `Recovered` event, no counter. The
test's single behavioral assertion (`next_event == EndOfStream`) cements that
silence: any future damage-note would FAIL this test, so the suite actively
locks in zero observability for dropped payloads.
Why it matters: the crate's own damage policy is "repairs are counted and
surfaced, never silent" (`crates/demux/src/avi/header.rs:382-383`, DamageLog
doc). On a real file whose Data Object carries payloads for a stream the
header dropped (damaged-header case — exactly the recovery scenario the ASF
suite tests elsewhere), media bytes vanish with no note, no count, no event.
Every other skip in this suite (unknown header objects, command streams,
codec-list disagreements) is damage-noted; this one alone is dark.
What correct looks like: assert AND implement a `Recovered { skipped: n,
reason: "payload-undeclared-stream" }` (or a damage-total bump) before EOS —
the same shape `non_audio_video_stream_types_skip_with_notes` (part1) pins for
header-level skips.
Confidence: HIGH — Verification: read (test + mod.rs site traced; DamageLog policy doc quoted).

### F-002 [S4]
Where: crates/demux/src/asf/tests.rs:882-884
Code:
```
// ---------------------------------------------------------------------------
// Layer 2: the ffmpeg fixture differential (regenerated, not committed)
```
What is wrong: The comment says the fixture family is regenerated at test time
and not committed — but 11 fixture files ARE committed under
`crates/demux/src/asf/fixtures/` (ls verified: asf_adpcm_ms.wma …
wmv_wmv2_wma_av.wmv), `fixture_dir()` (1085-1088) prefers the in-tree dir with
the note "ASF-d: committed §1.2 family, byte-stable", and all 10 FIXTURES
`file_size` truths match the committed files' actual sizes exactly (6942,
10102, 7123, 7119, 7129, 26321, 10499, 16899, 26106, 6936 bytes).
Why it matters: a reader concludes the differential needs ffmpeg on the test
machine (and that committed bytes don't exist to diff against); in fact the
baseline ran it hermetically from the tree (`fixture_family_walks_match_the_measured_truth ... ok`,
log 1567, in a sandbox where the fallback generation path was never taken).
Stale comments in the evidence chain are exactly how later audits mis-derive
provenance.
What correct looks like: "Layer 2: the ffmpeg fixture differential (committed
§1.2 family in src/asf/fixtures, byte-stable; ffmpeg regeneration is only the
fallback when the tree is absent)".
Confidence: HIGH — Verification: read + ls + size arithmetic (10/10 match).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `packet_fields_parse_and_close_exactly` | 528-552 | REAL | — | Every parsed field pinned incl. `replicated` tuple, `data_range` sliced back into source bytes, and the padding-closure arithmetic (551). |
| `multi_payload_packets_parse_in_order` | 554-574 | REAL | — | Order + per-payload flags + data ranges for 2 payloads in one packet. |
| `malformed_packets_refuse_typed_never_guess` | 576-606 | REAL | — | 4 mutations → 3 distinct typed codes (bad-flags ×2, bad-length, bad-replication); the compressed 1-byte replication form refused-not-guessed (598-605). |
| `fragments_reassemble_into_whole_objects` | 608-651 | REAL | — | 2-fragment object → exact concat buffer + pts/keyframe; new media-object number abandons the incomplete predecessor (pinned: predecessor's bytes never surface). |
| `assembler_refusals_are_typed` | 653-696 | REAL | — | Wrong-offset continuation, oversized first fragment, `u32::MAX` declared size (the MP §12 allocation guard), first-fragment-without-replication — all typed. |
| `walk_emits_whole_objects_with_preroll_subtracted_stamps` | 698-750 | REAL | — | Preroll subtraction pinned on the 1 kHz sample==ms clock (50/150 ticks); cross-packet reassembly emitted as one packet; EOS after. |
| `payloads_for_undeclared_streams_are_skipped_not_fatal` | 752-760 | REAL | S3 | F-001: pins a fully silent skip. |
| `non_fixed_packet_sizes_refuse_typed` | 762-780 | REAL | — | Patches FPO min-packet-size at byte offset 122 (30 header + 24 object + 68 payload — double-pins the FPO field table) → typed `variable-size-unsupported`. |
| `simple_index_seek_reanchors_the_walk` | 782-825 | REAL | — | SeekSupport::Fast{indexed:true}; 1500ms → bucket 1 → landing 1000 (bucket time), resume at entry packet 2 (pts 2000/3000); past-end clamps to last bucket. Landing-is-bucket-time (not packet pts) is pinned and commented. |
| `indexless_files_seek_by_bounded_linear_scan` | 827-858 | REAL | — | SlowLinear; 600ms lands on first packet with send ≥ target (packet 2, ticks 1000 — at-or-AFTER semantics, documented); rewind-to-zero pinned. |
| `indexless_seeks_past_the_scan_budget_refuse_typed` | 860-880 | REAL | — | 5000 packets > 4096 budget → any seek (even the 5ms target — deliberately the cheapest) refuses `demux.asf.seek.too-slow`. Faithfully pins the TOTAL-count gate at mod.rs:737-739 (refuses even a 2-packet scan on a long file — conservative; the gate itself is mod.rs's defect surface, A02's chunk). Name reads as "the seek goes past the budget" when it is the FILE that does — comment disambiguates. |
| `struct FixtureTruth` | 889-916 | REAL | — | Measured-truth schema; every field documented with its oracle (ffprobe/ffmpeg). |
| `const FIXTURES` (10 entries in range) | 918-1052 | REAL | — | Byte-consistent with committed files (10/10 sizes; all header/data/index offset arithmetic closes exactly — hand-verified). `fpo`'s 4th tuple slot doc'd "min/max packet" holds one scalar (min==max is the only supported shape — fine but the label lies slightly). `asf_adpcm_ms.wma` entry straddles the part2/part3 boundary at 1051. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| packet_fields_parse_and_close_exactly | YES | Field-by-field equality + slice-back-into-source + padding arithmetic (551). |
| multi_payload_packets_parse_in_order | YES | Per-payload stream/flags/mo + data ranges (563-573). |
| malformed_packets_refuse_typed_never_guess | YES | Exact code strings per mutation; a guessed-parse (Ok) fails `err_of`. |
| fragments_reassemble_into_whole_objects | YES | Exact concat buffer (626-629); wrong-offset or lost fragments break it. |
| assembler_refusals_are_typed | YES | Four exact codes incl. the allocation guard (683-686) — an OOM attempt would abort the test process, the typed refusal doesn't. |
| walk_emits_whole_objects_with_preroll_subtracted_stamps | YES | Preroll off-by-one shifts ticks 50→150 or −50 → matches! guard fails (736, 747). |
| payloads_for_undeclared_streams_are_skipped_not_fatal | PARTIAL | Pins non-fatality and silence; would NOT catch a missing damage note (F-001 — it enforces the absence of one). |
| non_fixed_packet_sizes_refuse_typed | YES | Wrong byte patched → wrong code or no refusal. |
| simple_index_seek_reanchors_the_walk | YES | Landing bucket arithmetic + resumed packet sequence + clamping (805-824). |
| indexless_files_seek_by_bounded_linear_scan | YES | Landing packet + subsequent pts sequence (845-857). |
| indexless_seeks_past_the_scan_budget_refuse_typed | YES | 5000-packet file with seek → exact too-slow code; a scan-anyway implementation would return Ok and fail. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ffmpeg 7.1.5 sha256-pinned fixture differential incl. live ffprobe codec + -count_packets cross-check" | docs/feature-matrix.toml:216 (container-asf-wmv evidence) | REAL | Baseline 1567 (`fixture_family_walks_match_the_measured_truth ok`) + 1953 (`fixture_family_cross_checks_against_live_ffprobe ok`); truth table byte-consistent with committed fixtures (this audit, 10/10). |
| "36-test asf unit suite" | feature-matrix.toml:216 | REAL | 36 asf tests in baseline 1555-1589. |
| "(regenerated, not committed)" | tests.rs:883 | FALSE | F-002: 11 committed fixture files; in-tree-first fixture_dir(). |
| The whole lane's user-facing claim | matrix row 215 windows/macos "implemented" | FALSE | See header.rs#part2 audit §6 — open path has no asf arm on ANY platform; these strong tests exercise an unwired lane. |

## 7. Dead / duplicated / unreachable code
- The tested lane is unreachable from shipped entry points (containers/mod.rs
  has no asf arm; media-probe has no signature) — the tests are the lane's
  only execution context today.
- No dead code inside the chunk. `ms_base`/`span_at`/`object` (part1's) are
  used throughout; `FixtureTruth.generate` strings double as the fallback
  generation recipe (consumed by `generate()`, part3).

## 8. Dependencies & flags
cfg(test) module; std only; fixture I/O via std::fs/std::process (Command for
the ffmpeg fallback, part3's range). No unsafe. Baseline ran the full suite
(release profile deviation per INDEX.md).

## 9. Open questions for the next agent
- A02 (asf/mod.rs): the total-count seek gate (mod.rs:737) refuses a 5ms seek
  on a 5000-packet indexless file that a 2-packet scan would answer —
  deliberate worst-case bound or over-refusal worth a scan-budget counter?
- A09 (asf/data.rs): the `payload.data_range` indices are trusted into
  `&bytes[range]` at mod.rs:482 — confirm parse_packet validates them against
  the packet length (my chunk's tests only exercise well-formed ranges plus
  the four refusal shapes).
- Whoever wires the asf open path: F-001's silent skip becomes user-visible
  data loss with zero diagnostics the day real files hit it.
