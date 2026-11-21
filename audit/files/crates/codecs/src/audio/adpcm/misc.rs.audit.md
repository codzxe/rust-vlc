# AUDIT: crates/codecs/src/audio/adpcm/misc.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 413 / 17 items |
| File verdict | UNREACHABLE from shipped entry points (real, heavily tested code; both descriptors have no demuxer/host minter) |
| Reachable from shipped entry point? | NO. `decode_ima_qt_packet`/`decode_ima_ws` are called only by `AdpcmDecoder::decode`'s `Plan::ImaQt`/`Plan::ImaWs` arms (adpcm/mod.rs:622-635), which require a track descriptor `adpcm_ima_qt`/`adpcm_ima_ws` — rg over demux/ and apps/: no minter (see mod.rs#part1 audit). Other callers: unit tests + `fuzz_codec_adpcm` corpus seeds. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | ffmpeg 7.1.5 probes (local binary, /tmp scratch): `-encoders | grep ima_ws/ima_qt` (both encoders exist), forced NUT mux of `adpcm_ima_ws` → "No codec tag defined" (doc claim reproduced), `adpcm_ima_qt` stereo MOV generated; damaged-packet experiment: byte 0x59 planted in packet 2's ch1 header → ffmpeg decodes 344/345 packets, drops packet 2, and **ch1 output is permanently offset −120 vs the clean decode (21888/21888 remaining ch1 samples differ; ch0: 0 diffs)** — decisive oracle evidence for F-001. sed -n quote verification (128-131, 159-162, 177-186, 213-219); hand-checked ws arithmetic against test expectations ([2,15], [2,13,4,43] — exact); baseline log cites below. |

## 1. One-paragraph verdict
The two decoders here are real, arithmetically verified against hand-computed expectations, and pinned by executed differential evidence on clean streams (MOV-carried IMA4 with 173 continuation seams; forced-decoder Westwood vectors — both bit-exact in the committed baseline run). I reproduced both ffmpeg oracle claims with the local 7.1.5 binary (ima_ws has an encoder but no muxer; ima_qt encodes into MOV). Two real defects: (1) on a damaged multi-channel IMA4 packet the decoder mutates the damaged channel's state (step index set to 89, predictor re-seeded) *before* validating it and then skips the packet — ffmpeg 7.1.5, which I ran on a byte-damaged stereo MOV, validates before adoption and keeps the channel's pre-damage state, producing a permanently offset channel for the rest of the stream where this decoder re-seeds from the next header: post-damage output provably diverges from the documented oracle, and the `WsChannel.step_index` "0..=88" invariant is transiently broken; (2) per-packet allocations (scratch, lows/highs, and the caller's fresh `buf` + copy) contradict mod.rs's "steady-state decode is allocation-free per packet" claim. Neither is reachable in the shipped build today (no container mints these descriptors), which caps the blast radius but not the finding.

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:301-310 → all 9 `adpcm::misc::tests` ok (sign extension, partial-vs-multiply forms, QT header split, continuation heuristic, short/wild skip, stereo interleave, WS mono/stereo/state/trailing); :826 `breadth_corpus_is_bit_exact_against_ffmpeg_reference ... ok` (the MOV IMA4 fixture with 346 continuation seams + WS vectors).
— verified by local ffmpeg 7.1.5 runs (this session): `adpcm_ima_qt` and `adpcm_ima_ws` encoders exist (`-encoders`); NUT refuses ima_ws ("No codec tag defined for stream 0" — doc line 44-48 claim TRUE); a stereo 22050 Hz ima_qt MOV encodes, and its clean decode matches the module's model (68-byte packets = 34×2ch, 64 instants each).
— verified by hand: `ws_expand` arithmetic on the test vectors — nibble 1 at step 7 → ((2·1+1)·7)>>3 = 2; nibble 7 → 13; stereo [2,13,4,43] exact; `qt_expand` partial-sum 7 at step 7 → 7+3+1+0 = 11 (vs multiply 13) — the two forms are correctly kept distinct.

## 3. Findings

### F-001 [S3] — damaged-channel state is mutated before validation; post-damage behavior diverges from the ffmpeg oracle (empirically demonstrated on the reference side)
Where: crates/codecs/src/audio/adpcm/misc.rs:177-186
Code:
```rust
        } else {
            state[ch].step_index = step_index;
            state[ch].predictor = predictor;
        }
        if state[ch].step_index > MAX_STEP_INDEX {
```
What is wrong: The wild header (step_index up to 127, 7 bits) is *adopted into the channel state first*; the `> 88` check then skips the packet, leaving `state[ch]` at `step_index = 89..=127` (violating the field's documented `0..=88` invariant, misc.rs:130) and the damaged header's predictor. ffmpeg 7.1.5 (which I ran on a byte-damaged stereo ima_qt MOV: ch1 header index forced to 89 in packet 2) rejects the packet *before* touching that channel's state — error `step_index[1] = 89`, packet dropped, decoding continues — and on the NEXT packet its continuation heuristic keeps the stale-but-valid state, producing a permanent −120 offset on ch1 for the remaining 21888/21888 samples (ch0: 0/21888 diffs — ffmpeg does advance earlier channels through the damaged packet, which this code also does). This decoder's mutated state (index 89 ≠ any legal next header) forces a re-seed on the next packet instead — its output realigns with the clean stream while ffmpeg's stays offset: the two decoders disagree on every post-damage sample of that channel.
Why it matters: The module's entire doctrine is bit-exact parity with the ffmpeg 7.1.5 oracle (doc lines 25-28, 44-48), but parity is only pinned on clean streams; on realistic damage (a flipped byte in one channel's header of a stereo file) the decoders diverge permanently. The `Recovered` skip outcome is also misleading: it reports "skipped 1" while silently having changed the channel's decode state in a way the reference does not.
What correct looks like: validate `step_index > 88` *before* any state mutation (`if step_index > MAX_STEP_INDEX { skip }` on the parsed value), and on skip restore/leave the channel's pre-packet state untouched (matching the measured reference: neither adoption nor nibble advance for the damaged channel).
Confidence: HIGH (reference side executed: ffmpeg damaged-vs-clean diff counts above; our side code-traced line by line; our-side execution UNVERIFIED — no cargo in sandbox) — Verification: ran (ffmpeg 7.1.5 damaged-MOV experiment, /tmp/qt_dmg.mov vs /tmp/qt_clean.mov) / read.
REQUEST-ORCH-RUN (to pin the our-side half): add a damaged-fixture case to `crates/codecs/tests/adpcm_breadth_differential.rs` (stereo ima_qt payload with ch1 header index 89 in chunk N) asserting the next-chunk output against ffmpeg's forced-decoder PCM — per this audit the assertion will FAIL against the current code, proving the divergence.

### F-002 [S3] — per-packet allocations contradict the documented "allocation-free per packet" decode claim
Where: crates/codecs/src/audio/adpcm/misc.rs:162 and 218-219 (+ caller mod.rs:580-581, 637)
Code:
```rust
    let mut scratch = vec![[0i16; 64]; channels];
```
```rust
    let pairs = data.len() / channels.max(1);
    let mut lows = vec![0i16; channels];
    let mut highs = vec![0i16; channels];
```
What is wrong: mod.rs:115-117 documents "the rotation reuses a freed slot via `get_mut`, so steady-state decode is allocation-free per packet", but every packet allocates: a fresh `buf` Vec in the caller's closure (mod.rs:581), grown by the flavor decoder, then *copied* into the pooled slot (mod.rs:637); `decode_ima_qt_packet` additionally allocates `scratch` per packet; `decode_ima_ws` allocates two small vecs per packet. The OUT_SLOTS rotation only avoids re-allocating the output `SharedBytes`.
Why it matters: The claimed D-1 discipline (allocation-free steady state) is the codec's stated performance contract; the actual steady state is 2-4 allocations plus a full buffer copy per packet. The PCM sibling decoder achieves the real thing (fills the pooled slot directly, `swap_groups_into`/`copy_from_slice` — pcm/decoder.rs:748-755).
What correct looks like: write nibble output directly into the pooled slot slice (pass `&mut [u8]` down like PCM does), keep the QT scratch and WS lows/highs as decoder fields (or stack arrays — channels ≤ 2).
Confidence: HIGH — Verification: read (all allocation sites traced; PCM contrast).
*(Cross-file addendum: recorded as F-006 in my mod.rs#part1 audit, where the false claim lives.)*

### F-003 [S4] — Westwood odd stereo tail is dropped silently; damage-model claim "every byte decodes" is false for stereo
Where: crates/codecs/src/audio/adpcm/misc.rs:217 and 50-56 (doc) + mod.rs:629-634 (caller)
Code:
```rust
    let pairs = data.len() / channels.max(1);
```
What is wrong: For stereo payloads with an odd byte count, `pairs = len/2` floors and the trailing byte is never decoded; the caller hardcodes `stats = Stats { skipped: 0, concealed: false }` for `ImaWs` (mod.rs:631-634), so the drop is invisible in the `DecodeOutcome`. The module doc says "The Westwood stream has no headers to damage — every byte decodes" (misc.rs:56).
Why it matters: The codec's damage model promises observable concealment (`Recovered`/`ConcealedFrame`) for every dropped byte; here a byte vanishes with an `Emitted` outcome. The test even names the behavior (`ws_trailing_byte_of_an_odd_stereo_payload_is_left_unconsumed`) without flagging the reporting gap.
What correct looks like: return the unconsumed-tail count in `MiscStats` (or a `concealed` flag) and let the caller map it to `ConcealedFrame`, as the WAV flavors do.
Confidence: HIGH — Verification: read (arithmetic + caller stats hardcoded).

### F-004 [S4] — dead store and dead stats field
Where: crates/codecs/src/audio/adpcm/misc.rs:174 and 136-141
Code:
```rust
        if state[ch].step_index == step_index {
            let diff = (predictor - state[ch].predictor).abs();
            if diff > 0x7F {
                state[ch].step_index = step_index;
```
What is wrong: (a) `state[ch].step_index = step_index` inside the `==` branch is a no-op (the branch condition asserts equality). (b) `MiscStats::frames` is written by both decoders but read only by tests — both production callers ignore it (mod.rs:622-627 reads only `skipped_packets`; mod.rs:629-630 drops the whole value).
Why it matters: Dead stores and unread fields are noise; the no-op line also obscures the heuristic's actual effect (only the predictor changes in that branch).
What correct looks like: delete the no-op assignment; either use `frames` in the caller's duration math (it currently recomputes from byte length) or make it test-only.
Confidence: HIGH — Verification: read (mod.rs call sites).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-56 | REAL | S4 | ffmpeg claims verified (see §2); "every byte decodes" false for stereo tails (F-003); damage model incomplete (F-001) |
| `STEP_TABLE` | 61-67 | REAL | — | 89 entries; deliberate duplication of ima/swf tables (documented, independently pinnable) |
| `INDEX_TABLE` | 70-73 | REAL | — | ffmpeg's 16-entry table |
| `MAX_STEP_INDEX`/`QT_CHUNK_BYTES` | 76-80 | REAL | — | 88 / 34 |
| `qt_expand` | 84-105 | REAL | — | partial-sum IMA form; clamp i16 + index 0..=88; no panics; i32 math cannot overflow (step ≤ 32767, diff ≤ ~57344) |
| `ws_expand` | 109-122 | REAL | — | multiply form; (2·delta+1)·step ≤ 15·32767 fits i32; clamps |
| `WsChannel` | 126-132 | REAL | S3 | `step_index` "0..=88" doc invariant transiently violated by F-001 path |
| `MiscStats` | 135-141 | REAL | S4 | `frames` unread in production (F-004b) |
| `decode_ima_qt_packet` | 147-200 | REAL | S3 | short-packet skip; per-channel 34B chunks; seed = sign_extend16 & !0x7F (verified vs ffmpeg semantics); continuation heuristic matches doc'd behavior; wild-index mid-packet state mutation (F-001); scratch alloc per packet (F-002); trailing bytes beyond 34·ch ignored (documented, unpinned); all-or-nothing output (out written only after full channel loop) |
| `decode_ima_ws` | 206-236 | REAL | S4 | headerless; per-pair per-channel packing; interleaved lows-then-highs (correct instant order); odd stereo tail dropped silently (F-003); channels=0 would count frames with no output — unreachable via contract (raw facts enforce 1..=2); per-packet lows/highs allocs (F-002) |
| `SignExtend` trait/impl | 240-252 | REAL | — | correct arithmetic-shift semantics; bits 0/≥32 guards |
| tests `samples` helper | 258-264 | REAL | — | as_chunks::<2> |
| 11 tests | 266-412 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| sign_extend_helper_matches_c_semantics | YES | pins 0/0x7FFF/0xFFFF/0x8000 |
| partial_and_multiply_forms_disagree_on_odd_steps | YES | pins 11 vs 13 — swapping the forms flips it |
| qt_header_word_splits_predictor_and_index | YES | asserts first sample = 0x2000−15 and 64 outputs |
| qt_continuation_heuristic_keeps_running_state | YES | pins keep (0x2002) vs re-seed (0x4000) — inverting the heuristic flips it |
| qt_short_packet_and_wild_index_are_skipped | YES | but does NOT pin the post-skip channel state — the F-001 divergence is invisible to it |
| qt_stereo_chunks_are_per_channel_then_interleaved | YES | asserts 512/−512 alternation |
| ws_mono_is_two_samples_per_byte_low_first | YES | hand-verified [2,15] |
| ws_stereo_packs_one_byte_per_channel_per_pair | YES | hand-verified [2,13,4,43] incl. cross-nibble index evolution |
| ws_state_persists_across_calls | YES | asserts predictor == last sample |
| ws_trailing_byte…left_unconsumed | YES | pins the drop (but not its reporting — F-003) |
| (differential) breadth_corpus_is_bit_exact_against_ffmpeg_reference | YES | clean-stream oracle; damaged-stream behavior NOT covered (F-001 gap) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "ffmpeg applies a continuation heuristic …" | misc.rs:15-20 | REAL | code 171-180 + test + 173-chunk MOV fixture bit-exact (baseline :826) |
| "ffmpeg 7.1.5 has an `adpcm_ima_qt` encoder — the committed fixture … bit-exact" | misc.rs:25-28 | REAL | encoder listed locally; differential ok in baseline |
| "no container in ffmpeg 7.1.5 muxes `adpcm_ima_ws` … vectors … verified bit-exact against ffmpeg's forced decoder" | misc.rs:44-48 | REAL | reproduced: NUT refuses ima_ws ("No codec tag defined"); differential ok |
| "A header index above 88 is the same skip (ffmpeg's hard error)" | misc.rs:54-55 | SHALLOW | ffmpeg does error+drop the packet, but its post-error channel state differs from ours — outputs diverge after damage (F-001, executed evidence) |
| "The Westwood stream has no headers to damage — every byte decodes" | misc.rs:56 | FALSE | stereo odd tails are dropped (F-003) |

## 7. Dead / duplicated / unreachable code
- Both decoders are unreachable from shipped entry points (no descriptor minter — see header row). `MiscStats::frames` dead in production (F-004b). `STEP_TABLE` duplicated across ima/swf/misc modules — deliberate, documented, test-pinned (acceptable).

## 8. Dependencies & flags
- No external deps; no unsafe; no feature gates. Uses only std + crate-internal types.

## 9. Open questions for the next agent
- W07-A04 (mod.rs part2): the `raw_out_bound` headroom for ImaQt (34B/ch → 128B/ch = 3.76×) is fine, but confirm my F-002 allocation finding's caller side (`buf` Vec + copy) when auditing `pooled_slot` (mod.rs:776-811).
- If the wave gate runs my REQUEST-ORCH-RUN from F-001, the expected result is a differential FAIL on the damaged fixture — that failure is the proof, not a regression to hide.
