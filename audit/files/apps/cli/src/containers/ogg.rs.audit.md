# AUDIT: apps/cli/src/containers/ogg.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-21 |
| Lines / functions | 702 / 15 (3 prod fns + 12 test items) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` main → `open_source` (apps/cli/src/demux_host.rs:362) → `containers::open` (apps/cli/src/containers/mod.rs:47) → `"ogg" => ogg::open` (mod.rs:68)) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | `rg -n "ogg::open"` → mod.rs:68 dispatch arm; `rg -n "FlacInOgg"` crates/demux/src/ogg/mod.rs → is_header_packet:171-174 + flac_streaminfo_rate offsets; Read crates/demux/src/ogg/mod.rs:1073-1121 (mint_track), :937-999 (complete_packet), :1273-1282 (stash_header — verbatim stash); Read crates/codecs/src/audio/flac/decoder.rs:519-549 (configure → parse_streaminfo(extra_data verbatim)); Read crates/codecs/src/audio/flac/metadata.rs:139-178 (STREAMINFO_LEN=34, offsets 0..18); `rg codec-opus crates/codecs/Cargo.toml` → default feature (line 56); `ls /tmp/my-project/corpus-v4/media` → absent; `rg containers::ogg audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → all 6 tests ok (lines 2824-2829); test-workspace.log.txt has NO lumen_cli unittest run (aborted at desktop zero_alloc_transport before reaching lumen-cli) |

## 1. One-paragraph verdict
The Ogg open path is genuinely wired and the vorbis/opus lanes are proven end-to-end by the baseline no-fail-fast run (open → demux → decode → sample counts pinned to ffprobe/codec-card references). But the FLAC-in-Ogg lane is broken by construction: the harvest hands the demuxer's verbatim second header packet — which by the demuxer's own magic table starts with the 4-byte metadata-block header `00 00 00 22` (crates/demux/src/ogg/mod.rs:173) — to a decoder that demands the bare 34-byte STREAMINFO body at offset 0 (crates/codecs/src/audio/flac/decoder.rs:540, metadata.rs:158-176). Every field the decoder parses is shifted 4 bytes; no fixture or test anywhere covers FLAC-in-Ogg. Multi-opus-stream files map every opus track to the first stream's headers. The module doc asserts no Opus decoder exists "today" while the default build ships one and the file's own test decodes opus. The corpus differential test — "the user's standing mandate" — asserts nothing in every recorded run.

## 2. What actually works (verified)
- `containers::ogg::tests::{vorbis_fixture_opens_with_expected_facts, vorbis_fixture_decodes_the_full_stream, start_time_skips_the_stamped_head, start_time_past_end_clamps_to_the_extent, opus_fixture_opens_and_decodes, corpus_ogg_lanes_through_the_open_path}` all `ok` — verified by `rg "containers::ogg" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → lines 2824-2829 (note: the primary test-workspace.log.txt aborted at desktop's zero_alloc_transport failure before ever running lumen-cli's unit tests; only the no-fail-fast repair run exercised this file).
- Vorbis full-stream decode pinned: 67 packets = ffprobe nb_read_packets, 66,150 sample frames, 2 channels — real fixture `crates/codecs/tests/fixtures/vorbis_sine_44100_stereo.ogg` (exists, 8,422 bytes).
- Opus decode proven by the same run (opposite of the module doc's claim, see F-003).

## 3. Findings

### F-001 [S2] FLAC-in-Ogg `extra_data` is the raw 38-byte packet, not the 34-byte STREAMINFO body the decoder parses — every decoded field is shifted by 4 bytes
Where: apps/cli/src/containers/ogg.rs:191-199 (and demux/decoder seam)
Code:
```rust
        let extra_data = serial.map(|serial| {
            let headers = demuxer.header_packets(serial);
            if codec == "flac" {
                // The native STREAMINFO block (the mapping header's
                // successor) — the FLAC decoder's 34-byte truth.
                match headers.get(1) {
                    Some(h) => SharedBytes::from_vec(h.as_slice().to_vec()),
                    None => SharedBytes::from_vec(Vec::new()),
                }
```
What is wrong: `header_packets(serial)` returns packets stashed VERBATIM (crates/demux/src/ogg/mod.rs:1273-1282 `stash_header` pushes `data` unmodified). For FLAC-in-Ogg, `headers[1]` is the STREAMINFO packet whose first 4 bytes are the metadata-block header — the demuxer's own magic table proves the shape: `OggCodec::FlacInOgg => ... || data.starts_with(&[0x00, 0x00, 0x00, 0x22])` (crates/demux/src/ogg/mod.rs:173; `0x22` = 34 = STREAMINFO body length), and its clock parser `flac_streaminfo_rate` reads the 20-bit rate at packet bytes 14-16, i.e. body offset 10 behind a 4-byte prefix. `FlacDecoder::configure` feeds this blob straight to `parse_streaminfo` (crates/codecs/src/audio/flac/decoder.rs:529-540), which requires the 34-byte body at offset 0 (metadata.rs:158-176: min_block_size=body[0..2], packed rate/channels/bps=body[10..18]). With the 4-byte `00 00 00 22` prefix, `body.len()=38 ≥ 34` passes the length gate, and every field is read from the wrong offsets: min_block_size reads the block header (0), max_block_size reads 0x0022, and the packed rate/channels/bps field reads real-body bytes 6..14 (min/max frame-size bytes). Result on valid FLAC-in-Ogg input: either a typed `flac.streaminfo` Unsupported/Corrupt refusal with nonsense diagnostics, or — if the shifted bits land in-domain — a silently misconfigured decoder (wrong rate/channels/bit-depth → garbage audio). The comment "the FLAC decoder's 34-byte STREAMINFO requirement" describes what the decoder wants, while the code delivers the prefixed packet.
Why it matters: FLAC-in-Ogg is a first-class Ogg family this host claims to harvest; the config seam is wrong for every such file, and nothing in the repo would notice — there is no FLAC-in-Ogg fixture or test anywhere (`ls tests/fixtures/generated/valid | rg flac` → only `mkv_h264_flac.mkv`; `rg flac crates/demux/tests/ogg_fixtures.rs` → no match; this file's tests have no flac lane).
What correct looks like: slice the body off the packet (`h.as_slice().get(4..).filter(|b| b.len() >= 34)`), or have the demuxer expose a typed STREAMINFO accessor; add a FLAC-in-Ogg fixture test that decodes and pins the sample count.
Confidence: HIGH — Verification: read (demuxer stash/magic/clock-parser code + decoder configure/parse code traced end-to-end; not executed — no fixture exists to run). REQUEST-ORCH-RUN: `ffmpeg -f lavfi -i sine=44100 -t 1 -c:a flac /tmp/flac.ogg && /home/z/my-project/target/release/lumen open /tmp/flac.ogg` to confirm the refusal/garbage shape at the gate.

### F-002 [S2] Every opus track maps to the FIRST OpusHead stream — multi-opus-stream files configure later tracks with the wrong header chain
Where: apps/cli/src/containers/ogg.rs:183-188
Code:
```rust
            "opus" => demuxer.serial_numbers().into_iter().find(|s| {
                demuxer
                    .header_packets(*s)
                    .first()
                    .is_some_and(|h| h.as_slice().starts_with(b"OpusHead"))
            }),
```
What is wrong: unlike vorbis/theora/flac/speex (per-codec FIFO queues built at ogg.rs:153-174), the opus arm ignores the track and `find`s the first serial whose first header is `OpusHead`. With two opus logical streams, both minted tracks receive stream #1's OpusHead‖OpusTags chain. A second stream with different pre-skip/channel count/gain configures its decoder from the wrong facts — silent wrong decode or a confusing refusal. The code admits it in prose ("Multi-opus-stream mapping waits on a demuxer-side track→serial accessor") — an admission is not a fix.
Why it matters: valid RFC 7845 files may carry multiple opus streams; the other four codecs in this same function handle the case correctly, so opus is an inconsistency with real wrong-behavior consequences, not a stylistic choice.
What correct looks like: an opus queue mirroring the vorbis queue (`Some("opus") => opus_q.push_back(serial)` at ogg.rs:163-168 and `"opus" => opus_q.pop_front()`), or the demuxer-side track→serial accessor the comment asks for.
Confidence: HIGH — Verification: read (control flow traced; no multi-opus fixture exists in the repo to execute).

### F-003 [S3] Module doc claims no Opus decoder exists and decode refuses `cli.codec_unavailable` — FALSE in the default build this file ships in
Where: apps/cli/src/containers/ogg.rs:27-31 (module docs)
Code:
```rust
//! * **Opus `extra_data` = OpusHead ‖ OpusTags** (RFC 7845's
//!   identification + comment headers, joined the same way). No
//!   Opus decoder exists in this build yet (the admission M-wave
//!   has not wired one) — the blob is the forward-honest config,
//!   and decode refuses typed `cli.codec_unavailable` today.
```
What is wrong: `codec-opus` is in the codecs crate's `default` feature set (crates/codecs/Cargo.toml:56), and this file's own `opus_fixture_opens_and_decodes` (ogg.rs:496-557, comment: "6.M-opus landed — the refusal shape inverted when the decoder arrived") passed in the baseline no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt:2825). The doc's "today" is stale; a reader auditing capability would conclude opus is refused when it decodes.
Why it matters: this repo's docs are the claim surface under audit; an in-code capability claim contradicting the shipped binary is exactly the drift the audit exists to catch (same class as the hard-coded `engine-info` literal called out in the baseline INDEX).
What correct looks like: delete the "No Opus decoder exists … refuses typed … today" sentence and state the 6.M-opus landing, matching the test.
Confidence: HIGH — Verification: read + baseline run cited.

### F-004 [S3] The corpus differential test asserts nothing in every recorded run — "skips loudly" is one harness-captured eprintln
Where: apps/cli/src/containers/ogg.rs:569-576
Code:
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
What is wrong: `/tmp/my-project/corpus-v4/media` does not exist in the baseline sandbox (verified by `ls`), and no baseline run sets `LUMEN_CORPUS`. So `corpus_ogg_lanes_through_the_open_path` took the early-return path and reported `ok` (orchrun-test-workspace-nofailfast.log.txt:2824) while executing zero of its ~20 assertions — the 0085 vorbis decode pin (32,256), the 0047/0077 opus pins, the 0090 theora lane and its typed-refusal assert never ran. The eprintln is swallowed by the test harness for passing tests; nothing in CI can distinguish "differential passed" from "differential skipped".
Why it matters: the test is labeled "the user's standing mandate" and is the only place the multi-lane Ogg behavior (5.1 opus geometry, theora cadence, theora typed refusal) is asserted; all of that evidence is absent from every recorded run while the green `ok` implies otherwise.
What correct looks like: `#[ignore]` + an explicit skipped/pended marker (visible in `--list`/ignored counts), or assert the corpus presence when an env var demands evidence (fail, not skip, under a `LUMEN_CORPUS_REQUIRED=1` CI lane).
Confidence: HIGH — Verification: read + `ls /tmp/my-project/corpus-v4/media` (absent) + baseline log line 2824 shows `ok`.

### F-005 [S4] Dead/misleading empty-blob arm for a missing FLAC second header
Where: apps/cli/src/containers/ogg.rs:197-199
Code:
```rust
                match headers.get(1) {
                    Some(h) => SharedBytes::from_vec(h.as_slice().to_vec()),
                    None => SharedBytes::from_vec(Vec::new()),
                }
```
What is wrong: the demuxer defers FLAC track minting until the second (STREAMINFO) packet arrives (crates/demux/src/ogg/mod.rs:953-998: `flac_clock_now` → `mint_track`), so a minted flac track always has ≥2 stashed headers and `headers.get(1)`'s `None` arm is unreachable for any track that reaches this code. When unreachable code is also wrong-shaped (an empty blob masquerading as config, which the decoder rejects with "streaminfo body is 0 bytes"), it misleads readers into thinking the arm is live.
Why it matters: dead defensive arms that return plausible-but-invalid values are how future refactors smuggle in silent failures.
What correct looks like: `None => None` (omit extra_data; the decoder's `flac.streaminfo.missing` refusal is the honest signal) or a debug_assert documenting the invariant.
Confidence: MEDIUM — Verification: read (demuxer mint path traced; the exact reachability depends on the damage path where a header-budget-exceeded stream still mints — stash_header refuses at budget, headers.len() stays 1, and mint still runs at ogg mod.rs:998, which CAN leave a flac track with 1 header; so the arm is reachable only through header-budget damage, where an empty blob still misleads vs a typed refusal).
Confidence note: the budget path (`ogg/mod.rs:1274-1279` notes damage and skips the stash while mint proceeds) makes this arm reachable-but-damaged-input-only; severity stays S4.

### F-006 [S4] Speex is classified, queued, and harvested but absent from the module's codec enumeration and from every test
Where: apps/cli/src/containers/ogg.rs:136-137 and 171 and 182
Code:
```rust
    } else if first.starts_with(b"Speex   ") {
        Some("speex")
```
What is wrong: the module doc enumerates vorbis/opus/theora/flac as the harvest vocabulary (ogg.rs:10-43) and never mentions speex, yet `classify` recognizes it, `speex_q` maps it, and the generic join builds its blob — all untested (no speex fixture, no corpus lane, no doc row). The theora typed-refusal honesty pattern is documented; speex's identical refusal shape is undocumented.
Why it matters: undocumented-but-live classification is how phantom capability creeps in — nothing pins speex behavior, so a regression here is invisible.
What correct looks like: one doc bullet for speex (blob shape, typed decode refusal) plus one fixture test mirroring the theora lane's refusal assert.
Confidence: HIGH — Verification: read (`rg -n speex` in this file; `rg -ln speex` across fixtures/tests → no lane).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 76-119 | REAL | — | Happy path verified by 5 fixture tests (baseline no-fail-fast). `unwrap_or(i64::MAX/4)` duration fallback is safe; `saturating_mul(1000)` µs→ns correct; typed errors via `wire_error` on demuxer seek failure. Passes `no_video`/`no_audio` through. No panic paths (`unwrap`/`expect` absent). |
| `classify` | 127-141 | REAL | — | Faithful mirror of demuxer `identify()` magics (verified against crates/demux/src/ogg/mod.rs:247-295). Skeleton → None (fishead handled by demuxer, never queued) ✓. |
| `harvest_facts` | 149-234 | REAL (defective) | S2 | F-001 (flac blob misalignment), F-002 (opus first-match). Queue correspondence for vorbis/theora/flac/speex is sound: tracks mint in BOS arrival order (demux mint_track pushes in arrival order; skeleton skipped both sides), per-codec FIFOs absorb cross-codec mint-order skew (flac's deferred mint at 2nd header vs vorbis's BOS mint). `frame_ns` = num×1e9/den with saturating ops — correct under the seconds-per-tick base convention (vorbis (1,44100) → sample clock; theora (2,1) → 2 s/frame verified by test). `dimensions: None` honest per doc. |
| `tests::codec_fixture` | 253-257 | REAL | — | Fixture exists (8,422 bytes). |
| `tests::CountingSink::accept` | 261-272 | REAL | — | Counts via `frames_in` — real accounting, not a no-op. |
| `tests::open` (helper) | 275-280 | REAL | — | Uses the REAL `open_source` path ✓ (not a mock of the subject). |
| `tests::decoder_for` | 285-299 | REAL | — | Real `HostDecoderProvider`. |
| `tests::count_packets` | 302-313 | REAL | — | Full demux walk to EndOfStream. |
| `tests::vorbis_fixture_opens_with_expected_facts` | 321-356 | REAL | — | Would fail if harvest broken: pins base (1,44100), span_end 66,150, blob prefixes + >3,000 bytes, geometry (2,44100). Passed (baseline no-fail-fast:2829). |
| `tests::vorbis_fixture_decodes_the_full_stream` | 363-407 | REAL | — | Pins 67 packets / 66,150 samples / 2 ch — mutation of harvest or decoder breaks it. Passed (…:2828). |
| `tests::start_time_skips_the_stamped_head` | 417-450 | REAL | — | Pins start_position_ns 1.1e9, 66 survivors, no stamped packet < target. Passed (…:2827). Shows 65 unstamped packets pass the watermark unjudged — see demux_host part2 cross-ref in §9. |
| `tests::start_time_past_end_clamps_to_the_extent` | 455-490 | REAL | — | Clamps 600 s → 1.5 s extent. Passed (…:2826). |
| `tests::opus_fixture_opens_and_decodes` | 495-557 | REAL | — | Proves opus decoder exists (contradicts module doc, F-003). Fixture exists at repo-root tests/fixtures/generated/valid/ogg_opus_48k_mono.opus. Passed (…:2825). |
| `tests::corpus_ogg_lanes_through_the_open_path` | 567-691 | SHALLOW (in recorded runs) | S3 | F-004: skipped in every recorded run; assertions real but never executed. |
| `tests::corpus_dir` | 694-701 | REAL | — | Env-override present; default absent in sandbox. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| vorbis_fixture_opens_with_expected_facts | YES | Concrete mutation: swap the blob join to skip the setup header → `windows(7)` setup-header assert fails. Break span_end harvest → 66,150 assert fails. |
| vorbis_fixture_decodes_the_full_stream | YES | Drop the decoder wiring → unreachable! panic; mis-trim → 66,150 pin fails. |
| start_time_skips_the_stamped_head | YES | Watermark that dropped nothing → seen=67 ≠ 66; watermark that dropped unstamped packets → seen=1. |
| start_time_past_end_clamps_to_the_extent | YES | Remove clamp → start_position_ns 600e9 ≠ 1.5e9. |
| opus_fixture_opens_and_decodes | YES | Remove opus decoder → unreachable!("opus decoder_for"). Sink.0>0 is weak but channel pin (1) and decode loop add coverage. |
| corpus_ogg_lanes_through_the_open_path | NO (in recorded runs) | Early-returns before every assert when corpus absent — the recorded "ok" proves nothing (F-004). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No Opus decoder exists in this build yet … decode refuses typed `cli.codec_unavailable` today" | module doc ogg.rs:27-31 | FALSE | codec-opus default feature (codecs/Cargo.toml:56) + opus_fixture_opens_and_decodes ok (baseline no-fail-fast:2825). |
| "Vorbis extra_data = three-header chain joined VERBATIM" | ogg.rs:18-26 | REAL | Blob prefix + setup-header-window + >3,000-byte asserts passed; decoder consumed it to 66,150 samples. |
| "FLAC-in-Ogg extra_data = the SECOND header packet … the FLAC decoder's 34-byte STREAMINFO requirement" | ogg.rs:36-39 | FALSE (the seam it describes) | F-001: second packet carries a 4-byte block header (demux magic 00 00 00 22, ogg/mod.rs:173); decoder parses offset 0 (flac/decoder.rs:540). |
| "Duration IS declared … `--start-time` clamps against the real extent" | ogg.rs:49-51 | REAL | start_time_past_end_clamps_to_the_extent passed (66 survivors, 1.5 s clamp). |
| "Seek: … the demuxer's `seek` is a pure query (position restored), so the open-time validation moves nothing" | ogg.rs:52-55 | REAL-UNPROVEN | Code passes a TimeSpan and discards only the validation result (ogg.rs:92-99); purity is a demuxer-side property not executed here (no test asserts position after seek). |
| "Theora dimensions are NOT harvested … never a guess" | ogg.rs:44-48 | REAL-UNPROVEN | `dimensions: None` at ogg.rs:226; the assert-bearing theora lane (0090) never ran (F-004). |

## 7. Dead / duplicated / unreachable code
- ogg.rs:197-199 `None => SharedBytes::from_vec(Vec::new())` — reachable only via the demuxer's header-budget damage path (see F-005); misleading shape.
- No other dead code; `classify`/queue duplication of demuxer vocabulary is deliberate (documented mirror) and verified in sync.

## 8. Dependencies & flags
- Uses `demux::ogg` accessors (`serial_numbers`, `header_packets`) — public API, no feature gates on this path; `codec-opus`/`codec-vorbis` are codecs-crate defaults (Cargo.toml:47-56), so the tested lanes match shipped builds.
- Test-only env knob `LUMEN_CORPUS` (S4-class sandbox literal, annotated `lumen-allow-literal` — consistent with the repo's lint-literal gate).

## 9. Open questions for the next agent
- Does the engine's "reanchor path" (claimed at ogg.rs:415-416 to own exact mid-stream seek) actually exist? The open-time watermark passes 65/67 unstamped packets after a 1.1 s `--start-time` (test at ogg.rs:417-450) — if no reanchor exists downstream, `--start-time` on Ogg decodes ~1 s of pre-target audio. I will check `SelectDemuxer` in demux_host.rs#part2 (my chunk) — W13-A02 owns part1 (`MediaFacts`/`TrackFact::audio_geometry` definitions).
- Whether `Track.language` stays None for Ogg (Track mint sets `language: None`, ogg/mod.rs:1106) — contrasts with the baseline's garbage `Some("voe")` on MP4 tracks; confirm the MP4 side is demux-wave territory.
- REQUEST-ORCH-RUN (repeated from F-001): `ffmpeg -f lavfi -i sine=44100 -t 1 -c:a flac /tmp/flac.ogg && /home/z/my-project/target/release/lumen open /tmp/flac.ogg` — expected to expose F-001 (typed flac.streaminfo error or garbage geometry) on a valid file.
