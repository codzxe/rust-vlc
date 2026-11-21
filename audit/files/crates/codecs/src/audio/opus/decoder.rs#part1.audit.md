# AUDIT: crates/codecs/src/audio/opus/decoder.rs#part1 (lines 1-547)
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = lines 1-547 (module docs, `Engine`, `build_engine`, `OpusDecoder` + full `Decoder` impl, `PooledWindow`, `pooled_sample_slot`); part2 = lines 549-853 (`#[cfg(test)] mod tests`) — W07-A07. I read part2's test bodies only as evidence for part1 behavior claims (cited below). |
| Lines / functions | 547 / 14 (11 fns/methods + `Engine` enum + 2 consts + struct) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.opus/.ogg` → apps/cli/src/containers/mod.rs "ogg" arm → demux `ogg` (OpusHead → `Clock::Opus48000`, base 1/48000 — demux/src/ogg/mod.rs:204) → apps/cli/src/pipeline_host.rs:276-283 `"opus" => self.seeded(track, config, OpusDecoder::new)` (OpusHead‖OpusTags seeded from facts)). Baseline: opus unit tests green (test-workspace.log.txt:446-449, 514) and the ffmpeg differential corpus green (`:1196-1197` corpus_0047_mono, corpus_0077_surround_51). No baseline run opens an .opus end-to-end (rc-smoke AVI-only, INDEX.md #20). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `rg -n "OpusDecoder" apps crates` (pipeline_host.rs:61/281, apps/web/src/host.rs:647); `rg -n "Clock::Opus48000\|TimeBase" crates/demux/src/ogg/mod.rs` (granule-domain base 1/48000 confirmed — decoder's `ts.base == base` filter matches the demuxer's stamps); `sed -n '414,475p' apps/cli/src/pipeline_host.rs` (ReseedingDecoder: seek/switch bumps generation → header-less reconfigure → OpusDecoder::configure resets `raw_next=0`); Read on decoder.rs 1-547 + 628-723 (test evidence); `rg -n "opus" audit/runs/baseline/test-workspace.log.txt` (executed evidence). |

## 1. One-paragraph verdict
A genuinely implemented ruopus wrapper with the two admission conditions honored: the pre-constructor panic gate (geometry closed by `parse_head` before any engine build — head.rs is W07-A08's, cross-referenced) and the valid-timeline window (pre-skip head-trim, granule end-trim, seek re-anchor with honest `Missing` positions). Engine errors are typed, never panics; the interleaver's direct indexing is arithmetically bounded (I traced `scratch_offset + frame·channels + source ≤ samples·channels − 1` in all granule/discontinuity combinations — no panic path); the pooled-slot rotation is the proven FLAC/Vorbis pattern. The one substantive defect is in the seek-landing window arithmetic: after the generation reconfigure that every seek triggers, the raw counter restarts at zero and the head-trim `start = raw_before.max(pre_skip)` re-applies the STREAM-HEAD pre-skip to the SEEK-LANDING prefix packet — every mid-page Opus seek silently drops 312 samples (6.5 ms) of valid audio (F-001, S2); the in-file seek test pins only the pts (`Missing`), never the window size, so the trim is unpinned. Hygiene: two per-packet heap clones (`head.clone()`, `permutation.clone()`) in the hot path.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:446-449,514` → all five in-crate decoder tests `ok`; `:1196-1197` → the ffmpeg differential corpus (`corpus_0047_mono`, `corpus_0077_surround_51`) `ok` (baseline run #3).
— Verified by reading demux/src/ogg/mod.rs:129-133,196-213: the Ogg demuxer stamps Opus packets in the granule domain with base `{num:1, den:48000}` — the decoder's `ts.base == base` stamp filter (341-344) matches the real container stamps, so end-trim actually engages on Ogg lanes.
— Verified by arithmetic trace (read): the interleaver index bound `scratch_offset + (window−1)·channels + source < decoded.len()` holds for every combination of granule present/absent and discontinuity true/false (end < start collapses to `window == 0` → NeedMoreInput at 416-421).

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/audio/opus/decoder.rs:400-410 (with 299-303 and 382-399)
Code:
```
        let raw_after = raw_before + packet_samples as u64;
        // 3. The valid-timeline window: head-trim at pre-skip,
        // tail-trim at the stamped granule (binding condition 2).
        let start = raw_before.max(u64::from(head.pre_skip));
        let end = granule
            .map(|g| raw_after.min(u64::try_from(g).unwrap_or(raw_after)))
            .unwrap_or(raw_after);
        let window = end.saturating_sub(start);
```
What is wrong: The head-trim `start = raw_before.max(pre_skip)` conflates "raw counter below pre-skip" with "the stream head". Every seek bumps the engine generation (pipeline_host.rs:216-218 documents the seek/switch path reconfiguring decoders header-less; `ReseedingDecoder::configure` → `OpusDecoder::configure` resets `raw_next = 0`, line 301). The seek-landing prefix packet arrives unstamped (`Ogg stamps only page-completing packets`) with `discontinuity = true`: the `else if packet.discontinuity` branch (397-399) only un-anchors the pts — the window still computes `start = max(0, 312) = 312`, so the landing prefix packet's first `pre_skip` samples (312 @ 48 kHz = 6.5 ms; "measured 312 on every corpus lane", module doc 29-31) are dropped as if they were encoder look-ahead. They are not: pre-skip is a once-per-stream head concept (RFC 7845 §4.2); after a mid-stream seek the landing audio is valid from sample 0 of the packet.
Why it matters: Every mid-page Opus seek loses 6.5 ms of audio and accumulates a 6.5 ms A/V offset per seek — wrong behavior in a realistic case (seeking is core player functionality), and it is invisible to the test suite: `seek_landing_is_missing_until_the_granule_reanchors` (654-723) captures the window size in `Seen.window` but asserts only the pts values — the 312-sample trim ships unpinned.
What correct looks like: Track pre-skip consumption once (e.g., a `pre_skip_consumed: bool` set when the first post-OPEN packet trims, cleared only on true stream-open — not on generation reconfigure), or apply the head-trim only when `packet_index == 0 && !packet.discontinuity`; the landing prefix should then emit its full window with pts `Missing` (the position stays honestly unknown — that part is right).
Confidence: MEDIUM — Verification: read (full trace through configure/decode/ReseedingDecoder/Ogg stamping); not executed. REQUEST-ORCH-RUN: a test that reconfigures, decodes one unstamped discontinuity packet, and asserts `seen[0].window == full packet samples` (it will observe `samples − 312` today).

### F-002 [S4]
Where: crates/codecs/src/audio/opus/decoder.rs:328 and 425-426
Code:
```
        let head = match self.head.clone() {
            Some(head) => head,
```
(and)
```
        let permutation = self.permutation.clone();
        let gain = self.gain;
```
What is wrong: Two heap clones per packet in the decode hot path: `OpusHead` (contains `mapping: Vec<u8>` up to 255 entries for family ≥ 1) and the permutation `Vec<u8>` (≤ 255). Both exist only to appease the borrow checker around the `fill` closure and `pooled_sample_slot`; a field-disjoint borrow split (as `mix()` in audio-engine does with `mem::take`+restore, or cloning only the scalar fields) avoids them.
Why it matters: Per-packet allocation/copy in the steady-state decode path contradicts the file's own D-1 discipline framing (72-77); for a multistream stream it is ~510 bytes copied per packet on top of ruopus's own per-packet output `Vec` (engine API, unavoidable).
What correct looks like: Destructure `&mut self` into disjoint field borrows (head/permutation by reference, pool/slot_next by mutable reference) so no clone is needed.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/codecs/src/audio/opus/decoder.rs:341-344 (with 346-358)
Code:
```
        let granule: Option<i64> = match packet.pts {
            Stamp::Present(ts) if ts.base == base && ts.ticks >= 0 => Some(ts.ticks),
            _ => None,
        };
```
What is wrong: For any container whose pts is NOT in the 1/48000 granule base (Ogg is; Matroska/WebM stamps in its own timescale), `granule` is `None` and the decoder silently substitutes its own raw-counter timeline — the container's stamp is discarded entirely (pts minted from `raw − pre_skip` at 458-462), rather than converted. Not a panic and not a wrong sample emission, but the end-trim (binding condition 2) never engages off-Ogg, and the emitted pts can drift from the container's authoritative timeline if the stream ever carries non-decodable padding packets (raw counts only decoded samples).
Why it matters: The Opus lane is wired for ogg only today (containers registry), so blast radius is future mkv/webm lanes; the behavior difference is undocumented — the doc (36-40) frames the granule rule as universal.
What correct looks like: Convert foreign-base stamps into the granule domain (base ratio math) or document that end-trim/re-anchor are Ogg-only and mkv pts passes through untouched.
Confidence: MEDIUM — Verification: read; no non-Ogg Opus lane exists to test (containers/mod.rs has no mkv-opus arm wired to this decoder's descriptor yet).

## 4. Per-function table (part1: lines 1-547)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs + `SAMPLE_SLOTS`/`MAX_PACKET_SAMPLES` consts | 1-96 | REAL | — | 5760 = 120 ms × 48 kHz (RFC 6716 §3 ceiling) — checked, correct. |
| `Engine` enum + `decode` | 101-120 | REAL | — | Thin dispatch to ruopus `decode_packet`; typed `PacketError`, no panic surface here. |
| `build_engine` | 124-143 | REAL | — | Safety relies entirely on `parse_head` having closed ruopus's constructor panic windows (channels∈{1,2} family 0; streams/coupled/mapping arithmetic family ≥1) — head.rs is W07-A08's; the family-0 test (726-753) pins the 3-channel refusal. |
| `max_output_len` | 147-149 | REAL | — | 5760×channels×4; bounded. |
| `OpusDecoder` struct | 189-226 | REAL | — | Field docs (216-225) accurately describe anchored semantics. |
| `OpusDecoder::new` | 230-233 | REAL | — | Trivial default. |
| `accepts` | 237-239 | REAL | — | Exact "opus"; pinned by test 583-588. |
| `output_format` | 244-251 | REAL | — | F32/interleaved/48 kHz preview; pinned by 746-751. |
| `pre_skip` / `decoded_total` | 256-265 | REAL | — | Diagnostics getters; `pre_skip` pinned by 752. |
| `Decoder::configure` | 269-314 | REAL | F-001 (counter reset) | Descriptor gate; mandatory OpusHead blob (`opus.head.missing`, doctest 165-188); builds engine AFTER parse_head (panic-gate order correct); resets `raw_next=0`/`anchored=true` — the reset is the seek-landing trim root; gain = 10^(q/5120) — RFC 7845 §4.2 formula, exact. |
| `Decoder::decode` | 316-488 | REAL | F-001, F-002, F-003 | Stale-guard first; BadState gates; whole-sample multiple check (359-370); granule filter; re-anchor/monotonic-max arithmetic (382-414) — traced sound except the landing pre-skip trim; window bound `usize::try_from…unwrap_or(usize::MAX)` unreachable (window ≤ packet_samples); interleaver index bounds proven (see §2); gain multiply branch on exact `gain == 1.0`; duration ns saturating; pts Missing when unanchored — honest. Empty packet: no NeedMoreInput convention (unlike FLAC/PCM) — empty data goes to the engine and maps to typed `opus.packet.decode` corrupt; whether any demuxer emits zero-length Opus packets is UNVERIFIED. |
| `Decoder::flush` | 490-497 | REAL | — | Stateless-per-packet claim matches decode (no cross-packet state besides counters). |
| `PooledWindow` + `pooled_sample_slot` | 500-547 | REAL | — | Same proven pattern as FLAC (flac/decoder.rs:629-674); fill errors typed; fresh-slot get_mut failure typed (not `unreachable!` — stricter than audio-engine's, fine). |

## 5. Test honesty (part2 range, cited as evidence)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seek_landing_is_missing_until_the_granule_reanchors (654-723) | PARTIALLY | Pins pts Missing for landing prefix + `granule − window − 312` for the re-anchored packet — but never asserts the prefix's `window` (captured in `Seen.window`, unasserted): F-001's 312-sample trim would not be caught. |
| configure_requires_and_validates_the_head (726-753) | YES | Exact error codes (`opus.head.missing`, `opus.head.family0_channels`) + negotiated shape. |
| decode_before_configure_is_typed_bad_state (756+) | YES | BadState class pinned. |
| hostile_packets_never_panic_and_isolate_to_the_packet (786+) | YES (panic class) | Would catch engine panics; executed green (baseline :449). |
| descriptor_gate_accepts_only_opus (583-588) | YES | Direct. |
| doctest (165-188) | YES | `expect_err` + exact code. |
| External: tests/opus_corpus_differential.rs | YES | ffmpeg float-decoder comparison within tolerance class; executed green (baseline :1196-1197). Gain≠0 path untested — honestly documented (module doc 59-61). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ruopus 0.1.2 exact-pinned, default-features off, std only" | module docs 7-13 | REAL | Cargo.toml:137 `ruopus = { version = "=0.1.2", optional = true, default-features = false, features = ["std"] }`. |
| Binding condition 1 (pre-constructor panic gate closed by parse_head) | module docs 21-26 | REAL | configure:290-291 parses head before build_engine:291; family-0 3-channel refusal pinned (741). head.rs internals = W07-A08. |
| Binding condition 2 (pre-skip + end-trim window) | module docs 27-43 | REAL at open / BUG at seek landing | Open path correct (test 659-673: first pts == 0); seek landing re-trims pre-skip — F-001. |
| "family 255 (discrete) passes through, honestly undefined" | module docs 51-52 | REAL | permutation None (294-298); order.rs has no row (baseline :458 `unknown_counts_have_no_row`). |
| "Output gain … 10^(q/5120) … gain≠0 untested honest gap" | module docs 56-61 | REAL (honest gap) | Formula matches RFC 7845 §4.2; gap is self-declared. |
| "Interleaved native-endian F32 at 48 kHz" | module docs 63-70 | REAL | decode:469-476 `to_ne_bytes` (449-450); differential green. |
| "D-1 pooled-slot rotation … slots sized once at configure" | module docs 72-77 | REAL (output side) | pooled_sample_slot + configure:299; note F-002 for the per-packet clones the claim's spirit excludes. |

## 7. Dead / duplicated / unreachable code
No dead code in part1. `pooled_sample_slot` is the third verbatim-pattern copy (MJPEG/FLAC/Vorbis lineage, self-acknowledged at 506-509) — acceptable, consistent. `decoded_total`/`pre_skip` getters are used by the differential tests.

## 8. Dependencies & flags
`ruopus =0.1.2` optional behind `codec-opus` (Cargo.toml:79), in `default` (57) — shipped workspace builds compile it. No unsafe in this file. `head.rs`/`order.rs`/`error.rs` are sibling modules (A08/A05 own them; error.rs is mine, next).

## 9. Open questions for the next agent
- part2 (W07-A07): add the missing window-size assertion to the seek-landing test (see F-001 REQUEST-ORCH-RUN).
- W07-A08 (head.rs): confirm `parse_head` rejects `pre_skip` on family ≥ 1 heads only per RFC (it is meaningful for all families) and that `output_gain` parsing is the full Q7.8 signed range.
- Vorbis decoder owner: the module doc says this design mirrors "the vorbis decoder's measure_shift design" — check whether vorbis carries the SAME seek-landing re-trim bug class (its pre-skip analog is the "first-sample" trim); if yes, this is a family-wide S2.
