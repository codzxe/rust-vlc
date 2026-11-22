# AUDIT: apps/cli/src/containers/ogg.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 702 / 5 production + 11 test items |
| File verdict | REAL-UNPROVEN (production + 5 of 6 tests run on committed fixtures) |
| Reachable from shipped entry point? | YES — main.rs:19 `run::run` → `lumen_dispatch` → dispatch.rs:82 `host.open_session` → host.rs:904 `open_source` → demux_host.rs:446-458 probe names `ogg` → containers/mod.rs:68 `"ogg" => ogg::open(stream, opts)` → ogg.rs:76 `open` → `demux::ogg::OggDemuxer`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `sed -n` quote verification (all quotes verbatim, below); `ls` fixture existence (both present). NO cargo (adaptation) — runtime claims UNVERIFIED. |

## 1. One-paragraph verdict

A clean delegation host (the Ogg page walk is `demux::ogg::OggDemuxer`'s; this file harvests header-chain blobs and wraps in the shared `SelectDemuxer`) with the assignment's best test ratio: 5 of 6 tests run on committed fixtures through the REAL open path and the REAL provider, including full-stream decode pins (vorbis 66 150 samples; opus decodes since 6.M-opus). The defects are documentary and structural: the module doc flatly claims "No Opus decoder exists in this build yet … decode refuses typed `cli.codec_unavailable` today" — FALSE, contradicted by this file's own test five hundred lines below and by pipeline_host.rs:276-282; the serial→track mapping is an arrival-order heuristic that assigns the FIRST OpusHead stream's headers to EVERY opus track (multi-stream files get the wrong blob, silently); and the corpus differential repeats the repo-wide skip-as-pass pattern with no recorded run.

## 2. What actually works (verified)

- Reachability chain above (each hop read in context).
- Both fixtures exist: `crates/codecs/tests/fixtures/vorbis_sine_44100_stereo.ogg`, `tests/fixtures/generated/valid/ogg_opus_48k_mono.opus` (verified `ls`).
- The blob shape matches the decoder's published parser: the join is consumed by `codecs::audio::vorbis::VorbisHeaders::from_extra_data` (headers.rs:220); ogg.rs imports no local grammar — the only container-specific parsing is `classify`'s magic prefixes.
- The start-time semantics are honestly tested twice (stamped-head skip at 1.1 s → 66 survivors of 67; past-the-end clamp to the 1.5 s extent) with the never-drop-unstamped rule pinned (ogg.rs:445-449, 486-489).
- MAX/4 clamp + `saturating_mul` (same shared shape as the other hosts).

## 3. Findings

### F-001 [S3] — Module doc claims Opus refuses typed; the build dispatches an Opus decoder

Where: apps/cli/src/containers/ogg.rs:27-31 (doc); contradicted by ogg.rs:518-519 (own test) and apps/cli/src/pipeline_host.rs:276-282

Code (ogg.rs:29-31, verbatim):
```rust
//!   Opus decoder exists in this build yet (the admission M-wave
//!   has not wired one) — the blob is the forward-honest config,
//!   and decode refuses typed `cli.codec_unavailable` today.
```

Code (ogg.rs:518-519, verbatim):
```rust
        // 6.M-opus landed: the opus decoder now EXISTS — the fixture
        // must decode, not refuse. Fresh open: the walk above consumed
```
Code (pipeline_host.rs:276-282, verbatim):
```rust
            "opus" => {
                // Requires the OpusHead ‖ OpusTags chain in
                // `extra_data` (the Ogg host's verbatim join; the
                // decoder's head gate refuses to guess the channel
                // geometry) — seeded from the facts.
                self.seeded(track, config, OpusDecoder::new)
```

What is wrong: the module doc's opus story is the pre-6.M-opus state; the provider dispatches `OpusDecoder` and the file's own corpus lane expects a full 192 000-sample decode (ogg.rs:657-660). Same staleness class as mkv.rs F-001 / mp4.rs F-001 (there h264, here opus) — the container-host docs were not updated when decoders landed.
Why it matters: the module docs are the recorded per-container "truth table" the wiring doc points to; three of four audited hosts now contain FALSE capability statements.
What correct looks like: doc regenerated per landing (or stripped of build-state claims that rot).
Confidence: HIGH — Verification: read (three-way contradiction is syntactic).

### F-002 [S3] — Serial→track mapping is an arrival-order heuristic; multi-opus-stream files get the wrong blob silently

Where: apps/cli/src/containers/ogg.rs:149-190

Code (ogg.rs:183-188, verbatim):
```rust
            "opus" => demuxer.serial_numbers().into_iter().find(|s| {
                demuxer
                    .header_packets(*s)
                    .first()
                    .is_some_and(|h| h.as_slice().starts_with(b"OpusHead"))
            }),
```

Code (ogg.rs:163-168, verbatim):
```rust
            Some("opus") => {} // every ogg opus stream shares the descriptor;
            // the queue is unnecessary: the ONLY opus truth this host
            // harvests (the joined OpusHead/OpusTags chain) is
            // identical in shape for any opus stream, and the corpus
            // lanes are single-stream. Multi-opus-stream mapping
            // waits on a demuxer-side track→serial accessor.
```

What is wrong: for vorbis/theora/flac/speex, tracks are matched to serials by popping per-codec queues built in BOS-arrival order — correct only while the demuxer's mint order is exactly arrival-order-minus-skeleton (an invariant asserted by comment, not by code). For opus, EVERY opus track gets the FIRST OpusHead-bearing stream's header chain: in a two-opus-stream file (e.g., dual-language audio) the second track is seeded with the first stream's OpusHead — wrong channel count/pre-skip/gain for that track, silently (the decoder will happily configure on the wrong head). The code comment admits it and defers to a demuxer-side accessor that does not exist.
Why it matters: wrong-blob seeding is exactly the class of bug that plays garbage or refuses confusingly; the defect is documented but unfixed and unguarded (no typed refusal for the ambiguous case).
What correct looks like: `OggDemuxer::serial_of(track)` on the demuxer (the comment's own proposal), or a typed refusal when >1 candidate serial exists.
Confidence: HIGH — Verification: read; multi-stream behavior UNVERIFIED at runtime (no corpus).

### F-003 [S2] — Corpus differential skip-as-pass, no recorded run

Where: apps/cli/src/containers/ogg.rs:568-576 (gate), 694-701 (gate helper)

Code (ogg.rs:569-575, verbatim):
```rust
        let Some(dir) = corpus_dir() else {
            eprintln!(
                // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
                "corpus(ogg-cli): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
                 — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
            );
            return;
        };
```

What is wrong: `corpus_ogg_lanes_through_the_open_path` (the only coverage of the 8 kHz vorbis lane, the 5.1 opus geometry lane, and the theora lane) returns Ok without asserting when the corpus is absent (it is absent here). No lumen_cli test ran in the W00 baseline (fail-fast at `first-vertical` — audit/runs/baseline/cargo-test.log:2499). The comment's escape hatch — "the differential table for the 2026-10-24 instance is in the worklog" — points at a worklog, not a run; the pins (32 256 / 192 000 / 2-packet theora) have no recorded green execution against this tree.
Confidence: HIGH — Verification: read + `ls` + baseline binary inventory.

### F-004 [S4] — FLAC-in-Ogg: a missing second header packet yields an EMPTY blob, not `None`

Where: apps/cli/src/containers/ogg.rs:193-199

Code (ogg.rs:196-199, verbatim):
```rust
                match headers.get(1) {
                    Some(h) => SharedBytes::from_vec(h.as_slice().to_vec()),
                    None => SharedBytes::from_vec(Vec::new()),
                }
```

What is wrong: a FLAC stream whose header stash lacks the second packet (mapping header only — damaged or nonconforming file) gets `extra_data = Some([])` instead of `None`. The FLAC decoder's configure then sees an empty STREAMINFO rather than an absent one — a different (and likely less clear) refusal path than the documented "no blob → decoder's own configure refusal" contract of every other host in this crate.
What correct looks like: `None` (let the codec refuse on absence, per the module doc's own rule at ogg.rs:239-243 of mkv.rs's twin).
Confidence: HIGH — Verification: read.

### F-005 [S4] — `classify` duplicates the demuxer's identification magic table in the app layer

Where: apps/cli/src/containers/ogg.rs:121-141

Code (ogg.rs:122-124, verbatim):
```rust
/// Classify one logical stream by its first stashed header packet's
/// magic — read-only mirror of the demuxer's own `identify()`
/// vocabulary (the descriptor names, never a second grammar: the
```

What is wrong: the demuxer's `identify()` exists (the comment cites it) but is not public, so the CLI re-states the magic constants. Small scale, same layering smell as mp4.rs F-003: container-format knowledge accumulating in the app because the demux crate under-exposes it. A demuxer that adds a codec (e.g. MIDI-in-Ogg) will mint a track this host cannot map (`_ => None` at ogg.rs:189 — no blob, silently).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 76-119 | REAL-UNPROVEN | — | Standard shared shape (MAX/4 clamp, seek-query validation, `saturating_mul`). |
| `classify` | 127-141 | REAL-UNPROVEN | S4 (F-005) | Magic-prefix mirror; correct prefixes (RFC 3533/5574/7845 shapes; `Speex   ` 8-byte form correct). |
| `harvest_facts` | 149-234 | REAL-UNPROVEN | S3 (F-002) | Arrival-order queues + opus first-match; FLAC second-packet rule (F-004); `frame_ns` from base with saturating math — theora pin (2 s) consistent. |
| tests: `codec_fixture`/`CountingSink`/`open`/`decoder_for`/`count_packets` | 253-313 | REAL-UNPROVEN | — | Real open path + provider; `decoder_for` helper hardcodes `tracks[0]` (fine for single-stream lanes). |
| `vorbis_fixture_opens_with_expected_facts` | 322-356 | REAL-UNPROVEN | — | Committed fixture; blob magic + span pin (66 150) + geometry seam pin. |
| `vorbis_fixture_decodes_the_full_stream` | 364-407 | REAL-UNPROVEN | — | Full decode, exact 66 150 pin with head-trim rationale. |
| `start_time_skips_the_stamped_head` | 418-450 | REAL-UNPROVEN | — | Watermark behavior pin (66 of 67) incl. the unjudgeable-pass rule. |
| `start_time_past_end_clamps_to_the_extent` | 456-490 | REAL-UNPROVEN | — | Clamp-to-extent pin. |
| `opus_fixture_opens_and_decodes` | 496-557 | REAL-UNPROVEN | — | Decodes opus (post-6.M); final pin is `sink.0 > 0` — weaker than the exact pins elsewhere, but geometry/mono pinned. |
| `corpus_ogg_lanes_through_the_open_path` | 568-691 | SHALLOW | S2 (F-003) | Skip-as-pass; the only theora/5.1-opus coverage. |
| `corpus_dir` | 694-701 | SHALLOW | S2 (F-003) | Gate — 5th copy of the same helper in this crate. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| vorbis facts/decode, start-time ×2, opus decode | YES | Committed fixtures, exact counts/stamps, real provider. (Runtime-UNPROVEN — no baseline run of this crate.) |
| corpus_ogg_lanes | NO (corpus-less env) | Returns before asserting (F-003). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No Opus decoder exists in this build yet … refuses typed `cli.codec_unavailable` today" | ogg.rs:29-31 | FALSE | pipeline_host.rs:276-282 dispatches `OpusDecoder`; ogg.rs:518-519/657-660 assert opus decodes. |
| "Vorbis extra_data = three-header chain joined VERBATIM" | ogg.rs:18-26 | REAL-UNPROVEN | Join code 200-206; decoder parser codecs/.../headers.rs:220; fixture test pins the magics (340-347). |
| "Duration IS declared (tail-granule probe)" | ogg.rs:49-51 | REAL-UNPROVEN | span pin 66 150 (ogg.rs:348-352). |
| "Theora dimensions are NOT harvested … never a guess" | ogg.rs:44-48 | REAL-UNPROVEN | `dimensions: None` (226). |

## 7. Dead / duplicated / unreachable code

- No dead production code.
- Duplication: `classify` (F-005), the corpus-gate helper (5th copy in the crate), the header-join concept duplicated against mkv.rs's `vorbis_chain` (different input shapes, same output format — acceptable but a shared `codecs`-side "header chain" type would remove the implicit coupling).

## 8. Dependencies & flags

- No feature gates: `mod ogg;` unconditional (mod.rs:29), arm at mod.rs:68. Deps: demux (ogg), media-types, plugin-api, commands.

## 9. Open questions for the next agent

- Does any real-world multi-opus-stream Ogg exist in the corpus (0077 is 5.1 single-stream)? F-002's wrong-blob path is unexercised either way.
- The vorbis head-trim pin (66 150 vs the pre-trim 66 278, ogg.rs:399-405) references "main's wave-7 vorbis head-trim" — cross-check the codecs crate's vorbis decoder audit (another wave) for whether the trim is real or the pin was fitted to observed output.
