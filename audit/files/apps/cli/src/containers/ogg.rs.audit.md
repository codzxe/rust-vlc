# AUDIT: apps/cli/src/containers/ogg.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-24 |
| Lines / functions | 703 / 3 prod (+6 test helpers, 6 tests) |
| File verdict | REAL (flac-in-ogg lane FALSE/broken — see F-001) |
| Reachable from shipped entry point? | YES: main.rs -> host.rs:904 open_source -> demux_host.rs:458 containers::open -> "ogg" arm (mod.rs:63) -> ogg::open (ogg.rs:76) |
| Findings | S0: 0 S1: 1 S2: 1 S3: 2 S4: 2 |
| Commands run | prebuilt `lumen_cli-5f13e10be716a8b0 containers::ogg --test-threads=1` -> 6 passed exit 0; `... corpus_ogg --nocapture` -> prints SKIP (corpus absent); crafted /tmp/flac_in_ogg.oga (python, ogg CRC) -> `printf 'quit\n' \| ./target/debug/lumen open /tmp/flac_in_ogg.oga` -> `error: [decode.unsupported] ... (flac.streaminfo.rate): sample rate 0 is invalid`; same driver on vorbis fixture -> Playing->Stopped, exit 0; `--start-time 1.1` -> `ok: time 1.100000s (73.3%)` |

## 1. One-paragraph verdict
Real, wired, mostly honest Ogg host — vorbis (symphonia-codec-vorbis 0.6.1,
codecs/src/audio/vorbis/decoder.rs:107) and opus decode end-to-end through the
REAL CLI path (I executed both). But FLAC-in-Ogg is DEAD: the host seeds the
second header packet verbatim — which by the demuxer's own parser carries a
4-byte METADATA_BLOCK_HEADER before the 34-byte STREAMINFO — into a decoder
that parses the FIRST 34 bytes as the native body. I crafted a spec-shaped
flac-in-ogg stream and the shipped binary refuses it (`flac.streaminfo.rate:
sample rate 0`). No test anywhere covers flac-in-ogg. Plus: --start-time on
ogg is effectively decorative (66/67 packets pass; clock says 1.1s while
content plays from 0:00), the only theora lane self-skips without the corpus,
and the module doc still claims Opus has no decoder.

## 2. What actually works (verified)
— verified by `./target/debug/deps/lumen_cli-5f13e10be716a8b0 containers::ogg --test-threads=1` ->
`test result: ok. 6 passed` (vorbis facts/geometry 2ch/44.1k, 67 packets,
66 150 decoded sample frames; opus fixture decode >0 frames mono 48k;
start-time clamps). Also `printf 'quit\n' | ./target/debug/lumen open
crates/codecs/tests/fixtures/vorbis_sine_44100_stereo.ogg` -> full
Idle->Opening->Buffering->Playing->Stopped->Idle, exit 0 (the real session
path decodes vorbis through symphonia). Baseline test log:3038-3043 (ok x6).

## 3. Findings
F-001 [S1]
Where: apps/cli/src/containers/ogg.rs:191-199 (harvest) vs ogg.rs:36-39 (doc)
Code:
```
            if codec == "flac" {
                // The native STREAMINFO block (the mapping header's
                // successor) — the FLAC decoder's 34-byte truth.
                match headers.get(1) {
                    Some(h) => SharedBytes::from_vec(h.as_slice().to_vec()),
```
What is wrong: `headers.get(1)` is the second header packet VERBATIM, and the
demuxer's own parser proves that packet is NOT the bare STREAMINFO —
crates/demux/src/ogg/mod.rs:304-306: "native block header (4 bytes) +
min/max blocksize (4) + min/max frame size (6), then the 20-bit sample rate
at byte offset 14" (rate at packet offset 14 = body offset 10 + 4-byte
prefix). The FLAC decoder's parse_streaminfo
(crates/codecs/src/audio/flac/metadata.rs:168-174) reads `body[10..18]` of
whatever it is handed — the first 34 bytes. Handed the 38-byte packet, it
parses the 4-byte block header + the first 30 body bytes: garbage rate,
channels, bps, MD5. EXECUTED PROOF: crafted spec-shaped file (0x7F"FLAC"
mapping header page; 4-byte block header 0x80||len-34 + valid 34-byte
STREAMINFO rate 44100; EOS page), `lumen open /tmp/flac_in_ogg.oga` ->
`error: [decode.unsupported]: unsupported in flac (flac.streaminfo.rate):
sample rate 0 is invalid`. Every real .oga flac file hits this (or worse: a
garbage non-zero rate misconfigures the decoder). Why it matters: FLAC is a
P0 matrix row (docs/feature-matrix.toml:699-708 "implemented"), the ogg row
(:165) claims "FLAC-in-Ogg mappings", and this lane is completely broken
with zero test coverage — the only flac-in-ogg test in the tree is a 9-byte
identify() magic assert (demux/src/ogg/mod.rs:1425-1426). The module doc
claim "the FLAC decoder's 34-byte STREAMINFO requirement" (ogg.rs:38-39) is
FALSE. What correct looks like: seed `h.as_slice()[4..]` (strip the
METADATA_BLOCK_HEADER) after validating the block type/length, plus an
end-to-end flac-in-ogg fixture. Confidence: HIGH — Verification: ran
(crafted-file execution + three source sites read).

F-002 [S2]
Where: apps/cli/src/containers/ogg.rs:409-416 (doc), test 418-450
Code (doc):
```
    /// `--start-time` mid-stream — the honest Ogg watermark shape:
    /// the demuxer stamps only each page's last-completing packet
```
What is wrong: the Ogg demuxer stamps only page-terminal packets, so the
SelectDemuxer watermark can only drop those; the in-file test PINS the
result — `assert_eq!(seen, 66, ...)` of 67 packets at a 1.1 s target on a
1.5 s file. The 65 unstamped packets (content from 0:00) flow to the
decoder. EXECUTED: `lumen open <vorbis.ogg> --start-time 1.1` answers
`ok: time 1.100000s (73.3%)` while the delivered audio content starts at
0:00 — position display and content disagree by the whole skip. Why it
matters: `--start-time` on ogg is effectively non-functional (skips ~1
packet), realistic user-facing wrong behavior; other containers stamp every
packet (PS: every PES; wav/adts: every frame) so ogg is uniquely broken
here. The code doc admits it ("The exact mid-stream seek is the ENGINE's
reanchor path") — an admitted design gap, not a hidden bug. What correct
looks like: page-granule apportioning at the demuxer, or an open-time
reanchor. Confidence: MEDIUM-HIGH (packet delivery proven by test + run;
exact playback symptom inferred) — Verification: ran + read.

F-003 [S3]
Where: apps/cli/src/containers/ogg.rs:183-188
Code:
```
            "opus" => demuxer.serial_numbers().into_iter().find(|s| {
                demuxer
                    .header_packets(*s)
                    .first()
                    .is_some_and(|h| h.as_slice().starts_with(b"OpusHead"))
            }),
```
What is wrong: for EVERY opus track the harvest resolves the FIRST OpusHead
serial — a file with two opus streams gives both tracks the first stream's
header chain (wrong channels/pre-skip for track 2). The code comment
(:163-168) admits it ("Multi-opus-stream mapping waits on a demuxer-side
track→serial accessor"). Same-codec multi-stream ordering for vorbis/flac/
speex queues (:150-156) is an arrival-order assumption with no multi-stream
host test. What correct looks like: pop a per-codec queue like the others,
or the demuxer accessor. Confidence: HIGH — Verification: read.

F-004 [S3]
Where: apps/cli/src/containers/ogg.rs:567-576
Code:
```
    fn corpus_ogg_lanes_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
            eprintln!(
```
What is wrong: the ONLY theora lane (facts, 1/2-fps base, frame_ns, typed
refusal) plus the 0085/0047/0077 corpus pins live in a test that returns
"ok" without asserting anything when /tmp/my-project/corpus-v4/media is
absent. EXECUTED: `--nocapture` prints "corpus(ogg-cli): ... absent and
LUMEN_CORPUS unset — SKIP"; the corpus dir does not exist in this sandbox,
and the baseline log (:3038) shows plain "ok" — indistinguishable from a
real pass. Theora host behavior is UNVERIFIED in every run I can see.
Confidence: HIGH — Verification: ran.

F-005 [S4]
Where: apps/cli/src/containers/ogg.rs:29-31
Code:
```
//!   Opus decoder exists in this build yet (the admission M-wave
//!   has not wired one) — the blob is the forward-honest config,
//!   and decode refuses typed `cli.codec_unavailable` today.
```
What is wrong: stale FALSE doc — the opus decoder landed
(pipeline_host.rs:276-282 wires OpusDecoder; my run of
opus_fixture_opens_and_decodes decodes PCM). Understates capability; still
a doc/reality contradiction. Confidence: HIGH — Verification: ran.

F-006 [S4]
Where: apps/cli/src/containers/ogg.rs:86-87
Code:
```
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
```
What is wrong: same latent clamp-panic seam as ps.rs (negative duration
would panic); Ogg granules are non-negative so unreachable today. Also
ogg.rs:198 fabricates an EMPTY blob (`Vec::new()`) for a malformed
flac stream instead of `None`. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 76-119 | REAL | — | clamp + pure seek query + wrapper; same shape as ps/avi |
| classify | 127-141 | REAL | — | prefix mirror of demux identify (mod.rs:247-295) — verified magic-for-magic; disagreement is harmless (queue never popped) |
| harvest_facts | 149-234 | REAL | S1/S3 | flac blob off-by-4-bytes (F-001); opus multi-stream (F-003); vorbis/theora/speex queues arrival-order assumption |
| tests x6 | 321-691 | mostly honest | S3 | vorbis/opus/start-time lanes honest and pass; corpus lane self-skips (F-004); NO flac/speex/multi-stream lane exists |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| vorbis_fixture_opens_with_expected_facts | YES | exact base 44.1k, blob chain shape, span 66150, geometry (2,44100) |
| vorbis_fixture_decodes_the_full_stream | YES | 67 packets + 66 150 sample frames (trim pin) |
| start_time_skips_the_stamped_head | YES — and it PINS the weak behavior | asserts 66/67 survive (F-002) |
| start_time_past_end_clamps_to_the_extent | YES | clamp to 1.5 s pinned |
| opus_fixture_opens_and_decodes | WEAK | asserts sink.0 > 0 only — a decoder emitting one frame passes; count not pinned (corpus 192k pin is in the skipped lane) |
| corpus_ogg_lanes_through_the_open_path | NO when corpus absent | returns early, reports ok (F-004) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the FLAC decoder's 34-byte" + "STREAMINFO requirement; the mapping header is not part of it." | ogg.rs:38-39 | FALSE (the blob it seeds is the 38-byte packet, not a 34-byte body) | executed: crafted file -> flac.streaminfo.rate refusal (F-001) |
| "Opus decoder exists in this build yet" | ogg.rs:29 | FALSE (stale, understates) | opus decode test passed in my run |
| "Duration IS declared" | ogg.rs:49-51 | REAL | span_end 66150 asserted, test passes |
| "the three-header chain joined" + "VERBATIM" (Vorbis blob join) | ogg.rs:18-19 | REAL | decode test 66 150 frames through VorbisHeaders::from_extra_data |
| "Theora dimensions are NOT harvested" | ogg.rs:44-48 | REAL-UNPROVEN | code says None (:226); theora lane corpus-gated (F-004) |

## 7. Dead / duplicated / unreachable code
speex harvest arm (classify :136-137, queue :156, :182) feeds a codec no
provider arm serves — demuxes then refuses typed (honest but untested).
FLAC-in-Ogg harvest is effectively dead output (F-001). No other dead code.

## 8. Dependencies & flags
No feature gates; release-lane reachable. Decoders reachable from here:
vorbis (symphonia-codec-vorbis 0.6.1), opus (OpusDecoder), flac (broken lane,
F-001); theora/speex refuse typed `cli.codec_unavailable`.

## 9. Open questions for the next agent
- bench.rs:300-334 hardcodes "no decoder in this build (first vertical:
  mjpeg + pcm)" for everything but mjpeg/pcm — decode-bench LIES about the
  registry (executed: vorbis -> "not benched"); host.rs:1643-1658 engine-info
  hardcodes decoders [mjpeg, pcm_s16le] — same stale mirror. Both understate
  the real provider; flag to the host.rs/bench.rs owners.
- Does any demuxer-side track→serial accessor exist yet (F-003's blocker)?
