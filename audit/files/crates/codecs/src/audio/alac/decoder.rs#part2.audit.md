# AUDIT: crates/codecs/src/audio/alac/decoder.rs#part2 (lines 443-884)
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 442 (chunk part2 of 885; part1=1-442 is W07-A05's) / 15 units in range |
| File verdict (part2) | REAL-UNPROVEN (genuine, bit-exact-pinned decode core; reachable by call chain; no baseline run plays an ALAC lane in the shipped binary) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open → playback-engine `pipeline.rs:2362` → `DecoderProvider::decoder_for` apps/cli/src/pipeline_host.rs:293-297 `"alac" => self.seeded(track, config, AlacDecoder::new)`; MP4 mints the descriptor at crates/demux/src/mp4/mod.rs:450 `b"alac" => Some("alac")`, MKV at mkv/header.rs:812). Execution UNPROVEN in shipped binary (baseline rc-smoke AVI-only, INDEX.md #20); bitexact/fuzz suites green in baseline #3. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n "samples_out\|cookie"` (field usage) · read playback-engine pipeline.rs:1290-1354 (seek reconfigure), 2557-2724 (LimbSink first), 850-963 (ObserveWindow anchor), seek.rs:426-450 (complete_seek) — the pts→clock-anchor chain · read mp3/decoder.rs:560-599 + aac/decoder.rs:540-604 (sibling pts strategies) · `rg "alac"` in demux/src (descriptor minting) · `rg S24` media-types/src/frame.rs (S24 = native endian — write_i24 verified) · `sed -n` quote verification |

## 1. One-paragraph verdict
The decode core itself is solid: the cookie gates are typed and mirrored from the engine's own limits, the 32-bit-justified→S16/S24/S32 narrowing is shift-correct (20-bit rides S24 as `pcm << 4`, exactly as documented), `write_i24` matches media-types' native-endian S24 semantics, pooled slots are bounds-checked, and five ffmpeg-MD5-pinned fixtures cover stereo/mono × 16/24-bit plus the truncation tripwire. The fatal flaw is the pts: `decode` stamps every frame from a configure-relative sample counter (`samples_out`) and ignores `packet.pts` entirely, while the engine's seek completion re-anchors the media clock on "the first audio frame's own timestamp" (playback-engine seek.rs:441) — and the seek path reconfigures the decoder (pipeline.rs:1308), resetting the counter to 0. Consequence: after any seek on an ALAC lane, the first delivered frame carries pts 0, the clock re-anchors at 0, and the `Seeked` report answers `actual: 0` regardless of the target. FLAC (frame-header absolute) and Vorbis (stamp-anchored) satisfy the anchor contract; ALAC — and its counter-twin siblings MP3 and AAC — do not, and no test pins seek-time pts on any of them.

## 2. What actually works (verified)
— PCM bit-exactness: `crates/codecs/tests/alac_bitexact.rs` pins 5 fixtures (stereo/mono × 16/24-bit + noise) against ffmpeg MD5s — ran green in baseline #3 (INDEX.md: 1681 passed / 1 unrelated fail). This exercises my range's `decode_into` narrowing (457-477), `write_i24` (491-502), and the pooled fill.
— `write_i24` native-endian layout verified against media-types/src/frame.rs:20-21 ("Signed 24-bit (3 bytes), native endian").
— Narrowing algebra verified by hand: `shift = 32 − width_bits` with width_bits the container width (16/24/32); source bits land in the container's high bits; the module doc's `pcm << 4` for 20-bit-in-S24 holds.
— Cookie damage gates: in-file test `cookie_damage_is_typed_per_rule` (817-845) asserts all six typed codes by mutation.

## 3. Findings
### F-001 [S2] Counter-based pts ignores the container stamp — every seek re-anchors the clock at 0
Where: crates/codecs/src/audio/alac/decoder.rs:687-688 (with 636)
Code:
```
        let pts = i64::try_from(self.samples_out).unwrap_or(i64::MAX);
        self.samples_out = self.samples_out.saturating_add(u64::from(meta.samples));
```
What is wrong: `packet.pts` (the MP4/MKV sample timestamp) is never read; pts is `samples_out`, a counter reset to 0 by `configure` (line 636). The seek path reconfigures every decoder with the new generation (playback-engine pipeline.rs:1301-1308 `limb.decoder.configure(&config)?` — header-less, re-seeded through `ReseedingDecoder` apps/cli/src/pipeline_host.rs:442-447), so the first post-seek frame's pts is 0 in the 1/rate base. The engine then does `complete_seek`: `let actual = audio_anchor_ns.unwrap_or(inflight.landing_ns); clock.reanchor(Timestamp::new(actual, NANO_BASE))` (seek.rs:441-442) — `audio_anchor_ns` is the first delivered audio frame's pts (pipeline.rs:861-921, 2527-2531), i.e. 0. The clock is re-anchored at media time 0 and the `Seeked { requested, actual }` payload reports 0, whatever the target was.
Why it matters: seeking an ALAC file (MP4 and MKV lanes both dispatch `"alac"`) reports position 0 and desynchronizes the clock from the content by the seek distance; in an ALAC+video file this is an instant A/V position lie. The design's own words (seek.rs:426-428 "anchor on the first audio frame's own timestamp") show the frame pts is expected to be the absolute media position — FLAC delivers that from frame headers (flac/decoder.rs:241-252), Vorbis via stamp-measured shift; the counter family does not.
What correct looks like: stamp pts from the packet's own pts when `Stamp::Present` (resolving to the sample domain), falling back to the counter only for `Missing` — or keep a configure-time base offset derived from the first stamped packet (the Vorbis shift pattern).
Confidence: HIGH — Verification: read; every link of the chain traced (decoder → LimbSink.first → ObserveWindow.first_audio_ns → complete_seek). UNVERIFIED only in execution: no baseline run seeks an ALAC file (rc-smoke is AVI; ttf_seek.rs uses an AAC cover-art fixture). Siblings MP3 (decoder.rs:580-588, `samples_fed`) and AAC (decoder.rs:576-578, `samples_fed` reset at :480) share the defect — flagged for their agents.
### F-002 [S3] Pooled-slot rotation re-allocates per frame under the shipped decode-ahead regime; cursor update diverges from the family twin
Where: crates/codecs/src/audio/alac/decoder.rs:745-761 (with 723-744)
Code:
```
    if slots.len() < SAMPLE_SLOTS {
        slots.push(fresh);
        *next = slots.len() % SAMPLE_SLOTS;
    } else {
        slots[*next] = fresh;
        *next = (*next + 1) % SAMPLE_SLOTS;
    }
```
What is wrong: same rotation as vorbis (4 slots, reuse only when `Arc::get_mut` succeeds): with more than 4 frames outstanding downstream — the shipped "16-packet bursts into a 3 s ring" decode-ahead regime (apps/cli/src/pipeline_host.rs:603-605) — every decode allocates a fresh `vec![0u8; slot_len]` (up to 65536×8ch×4 = 2 MB at the frame-length cap). The module doc claims "steady-state decodes refill a slot in place — zero per-frame allocation after warm-up" (decoder.rs:91-95): true only in the unserviced paced regime. Additionally `*next = slots.len() % SAMPLE_SLOTS` (757) sets the cursor to 0 when the 4th slot is pushed (index 3), unlike the vorbis twin (`slots.len() - 1`) — harmless (the scan visits all slots) but the two "verbatim pattern" copies have drifted.
Why it matters: per-frame allocation churn in the decode hot path contradicting the D-1 claim; the drift between the three pooled_sample_slot copies (vorbis/mp3/alac) is the classic copy-paste divergence the checklist targets.
What correct looks like: pool depth ≥ the decode-slice budget; one shared implementation instead of three drifted copies.
Confidence: MEDIUM (allocation behavior reasoned from Arc semantics + documented burst regime; not measured).
### F-003 [S4] `cookie` field is write-only dead state
Where: crates/codecs/src/audio/alac/decoder.rs:566-567 and 638
Code:
```
    /// The cookie bytes handed to the engine (None until `configure`).
    cookie: Option<Vec<u8>>,
```
(assigned at 638: `self.cookie = Some(cookie.to_vec());`)
What is wrong: `self.cookie` is assigned once per configure and never read — not by decode (the engine keeps its own copy), not by the Debug impl (586-593 prints config and samples_out only). It is a dead heap copy of the extra_data blob.
Why it matters: dead state that costs a per-configure allocation and misleads readers into thinking re-decode re-seeds from it.
What correct looks like: delete the field.
Confidence: HIGH — Verification: rg "cookie" over the file — only the 638 write touches `self.cookie`.

## 4. Per-function table (part2 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `decode_into` (tail; fn begun part1:392) | 443-487 | REAL | — | narrowing loops correct (2/3/4-byte paths, `as_chunks_mut`+zip bounds-safe); `dst.len()` gate typed (447-456); BlockMeta carries negotiated format |
| `write_i24` | 491-502 | REAL | — | native-endian S24 verified vs media-types frame.rs:20-21 |
| `decode_frame` (free surface) | 513-525 | REAL | — | throwaway engine; fuzz/differential entry (fuzz.rs); truncate to exact |
| `struct AlacDecoder` + fields | 562-581 | REAL | F-001 (samples_out), F-003 (cookie) | |
| `const SAMPLE_SLOTS` | 584 | REAL | F-002 | 4 |
| `impl Debug for AlacDecoder` | 586-593 | REAL | — | finish_non_exhaustive; honest |
| `AlacDecoder::new` | 598-600 | REAL | — | |
| `AlacDecoder::accepts` | 604-606 | REAL | — | exact "alac" |
| `Decoder::configure` | 610-642 | REAL | F-001 (636 reset) | gates before any state mutation (630-631 error leaves old state intact); pool/scratch reset correct |
| `Decoder::decode` | 644-700 | REAL | F-001 | stale drop first; empty packet → NeedMoreInput (656-659, contract-true — better than vorbis); exactly one frame per packet, `Emitted { frames: 1 }` (699) — contract-true; duration math guarded (`.max(1)`) |
| `Decoder::flush` | 702-708 | REAL | — | stateless, contract-true Drained |
| `struct PooledBlock` | 711-715 | REAL | — | |
| `pooled_sample_slot` | 723-763 | REAL | F-002 | fresh-slot `get_mut` None arm typed BadState (752) — unreachable by construction, honest |
| test `cookie_parse_accepts_the_passthrough_shapes` | 789-814 | REAL | — | bare/36-byte/48-byte shapes |
| test `cookie_damage_is_typed_per_rule` | 817-845 | REAL | — | 6 typed codes |
| test `truncation_maps_to_the_typed_tripwire` | 848-875 | REAL | — | pins `alac.frame.truncated` + `alac.frame.empty` |
| test `output_width_follows_the_bit_depth` | 878-883 | REAL | — | pins the width mapping |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| in-file 4 (table above) | YES | typed codes / exact tuples asserted |
| bitexact `decode_file` suite (tests/alac_bitexact.rs:71-107,167+) | YES | ffmpeg MD5 of interleaved PCM — any narrowing/interleave regression flips the digest; pts only asserted MONOTONIC (bitexact.rs:148-157), so F-001 is invisible to it |
| alac_fuzz_survival.rs | YES (survival class) | no panic/abort on hostile bytes |
| NOT COVERED: seek-time pts, post-reconfigure pts, pooled-slot steady-state allocation | — | F-001/F-002 unpinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero per-frame allocation after warm-up" | decoder.rs:91-95 (module doc) | SHALLOW | F-002: false under the shipped 16-packet burst regime |
| "output … by `sample >> (32 − width)`, … a 20-bit stream rides S24 as `pcm << 4`" | decoder.rs:52-60 | REAL | verified by hand; bitexact MD5s pin it |
| "`flush` is immediately drained (no inter-frame state)" | decoder.rs:534-535 | REAL | 702-708 |
| "the SAME bytes are then handed to the engine untouched" | decoder.rs:45-48 | REAL | engine_for strips the same prefixes (310-311) and passes body verbatim |
| prior-session engine-bug suspicion "NOT reproduced" | decoder.rs:63-87 | REAL (honest ledger) | differentials bit-exact; claim is scoped to the 0.6.1 engine + correct packetization |

## 7. Dead / duplicated / unreachable code
F-003 (`cookie` field). `pooled_sample_slot` is the third drifted copy of the vorbis/mp3 pattern (F-002). The `MAX...` DoS-mirror gates are live.

## 8. Dependencies & flags
symphonia-codec-alac 0.6.1 exact-pinned (module doc; record docs/dependencies/records/symphonia-alac-0.6.1.md). No feature gate; compiled and dispatched in the shipped build (pipeline_host.rs:293-297). No unsafe. No panic path found: all slices bounded by validated cookie fields (frame_length ≤ 65536 mirrored before slot sizing) and `dst.len()` gates.

## 9. Open questions for the next agent
- MP3 (W07-A0x) and AAC (W07-A0x) decoders share F-001 (`samples_fed` counters reset by seek reconfigure; aac/decoder.rs:480, mp3 sibling) — the seek-anchor lie is family-wide; the fix belongs in one place (packet-pts stamping) and one test (seek an audio lane, assert `Seeked.actual` ≈ target).
- ttf_seek.rs's `0029-cover-audio.m4a` — which codec? If AAC, the seek-anchor bug is reachable in an existing test fixture and the test's assertions need checking (it may assert actual==0 and be pinning the bug).
- Does any MKV lane mint A_ALAC with a CodecPrivate in the 24-byte bare shape (skip_cookie_prefixes accepts it, but nothing in-repo proves the mkv harvest path)?
