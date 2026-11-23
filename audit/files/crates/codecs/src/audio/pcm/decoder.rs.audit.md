# AUDIT: crates/codecs/src/audio/pcm/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-27 |
| Lines / functions | 1572 / 15 non-test fns (+5 test helpers, 22 tests) |
| File verdict | REAL (with one UNREACHABLE-from-shipped sub-surface, F-001) |
| Reachable from shipped entry point? | YES: apps/cli/src/pipeline_host.rs:347-370 `HostDecoderProvider::decoder_for` (`codec if codec.starts_with("pcm_")` → `PcmDecoder::with_format` → `configure`/`decode`), wired via playback-engine `DecoderProvider` (pipeline.rs:120,1054); same shape in apps/web/src/host.rs:596-606 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `cargo test -p codecs --lib audio::pcm::decoder` → 22 passed / 0 failed (exit 0); rg/sed verifications (exit 0) |

## 1. One-paragraph verdict
This is a genuinely implemented, panic-hardened PCM core: the format matrix (u8/s16/s24/s32/f32/f64 × le/be × planar) is complete, channel==0/rate==0 are typed refusals, the whole-sample-frame prefix rule is layout-uniform and provably keeps planar plane boundaries, duration math floors with saturating/checked arithmetic, and the pooled conversion path is differentially pinned bit-exact against the free surface. I ran its 22 tests: all pass. The two hostile findings are about *which validation the shipped product actually runs*: the entire WAVEFORMATEX negotiation path (L374-512) is never exercised by any shipped host (both use `with_format` + engine-minted `extra_data: None`), and `with_format` — the only shipped seam — enforces no channel ceiling (parse_waveformatex caps at 64), leaving the shipped lane's channel bound to per-container harvest code of uneven strictness.

## 2. What actually works (verified)
— verified by `cargo test -p codecs --lib audio::pcm::decoder` → `test result: ok. 22 passed; 0 failed` (includes: zero-copy pointer-identity test `zero_copy_path_never_materializes_a_pool_slot`, differential `swap_groups_into_is_byte_identical_to_the_reference_swapped` over bps∈{2,3,4,8}×adversarial lengths, pool-bound + bit-exact `converted_path_reuses_pooled_slots_and_stays_bit_exact`, duration floor `duration_floors_to_whole_nanoseconds` incl. u64::MAX saturation, channel-ceiling boundary 64-accepted/65-refused, planar truncation plane-boundary test).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/pcm/decoder.rs:374-512 (and resolve_facts L656-671)
Code:
```rust
fn parse_waveformatex(bytes: &[u8], format: PcmFormat) -> Result<Facts, PcmError> {
    if !matches!(bytes.len(), 16 | 18) && bytes.len() < 40 {
```
What is wrong: the whole WAVEFORMATEX/WAVEFORMATEXTENSIBLE negotiation surface (139 lines + 8 of the 22 tests) is dead in the shipped product. Both shipped hosts route PCM through the typed seam and configure with the engine-minted `extra_data: None` config: apps/cli/src/pipeline_host.rs:347-360 (`codec if codec.starts_with("pcm_")` … `let mut decoder = codecs::audio::pcm::PcmDecoder::with_format(channels, rate)?; decoder.configure(config)?;`) and apps/web/src/host.rs:596-606 (identical). No in-repo shipped caller ever passes a WAVEFORMATEX blob to this decoder. Why it matters: the richest untrusted-input validation in the file (length ladder, tag/GUID resolution, bits/align cross-check, the W2-15 g1 channel ceiling) never executes in production; a regression there ships invisibly. What correct looks like: either wire `extra_data` for PCM lanes (the doc's channel 1) or mark the surface library-only and shrink the claim. Confidence: HIGH — Verification: read (callers enumerated by rg over apps/ + crates/ non-test code).
Sub-verdict: parse_waveformatex surface = UNREACHABLE (from shipped entry points; reachable via public `PcmDecoder::new()` API and tests only).

F-002 [S3]
Where: crates/codecs/src/audio/pcm/decoder.rs:16-23 (module doc)
Code:
```rust
//! 1. **`extra_data` as WAVEFORMATEX** `[PROPOSAL]` — the container's
//!    own out-of-band init bytes, verbatim (AVI `strf` / WAV `fmt `:
//!    16-byte PCMWAVEFORMAT, 18-byte WAVEFORMATEX, or the ≥ 40-byte
//!    WAVEFORMATEXTENSIBLE whose SubFormat GUID names the real format
//!    — vendor tail bytes past offset 40 are legal per the MS schema
//!    and unread; the engine hands the blob on exactly as it hands AVC
//!    config records). Cross-checked against the descriptor's
//!    width/tag at `configure`.
```
What is wrong: "the engine hands the blob on exactly as it hands AVC config records" is FALSE for the shipped engine. AVC lanes go through `seeded()`/`ReseedingDecoder` (pipeline_host.rs:346, seeded_config L202-210 seeds `extra_data` from harvested facts); PCM lanes go through `with_format` with the raw engine config whose `extra_data` is `None` (pipeline_host.rs:348-360, comment: "the engine passes no extra_data"). The doc describes an engine behavior that does not exist. Claim-level verdict: FALSE. Why it matters: a reader auditing PCM input validation is pointed at a path that never runs (compounds F-001). Confidence: HIGH — Verification: read both host call sites.

F-003 [S3]
Where: crates/codecs/src/audio/pcm/decoder.rs:618-636
Code:
```rust
    pub fn with_format(channels: u32, rate: u32) -> Result<PcmDecoder, EngineError> {
        if channels == 0 {
```
What is wrong: the only shipped seam accepts any nonzero u32 channels and rate — no ceiling — while the (unshipped) blob path enforces `MAX_PCM_CHANNELS = 64` (L407-416). The in-file comment (L128-135) argues extreme channel counts are safe because "every sane packet then decodes to zero whole sample-frames and skips honestly", but that only holds when `channels × bps` exceeds the packet ceiling: with `pcm_u8` and channels=1000, a 1000-byte packet decodes to one instant and emits an `AudioFrame` declaring 1000 channels (L759-767) downstream. Shipped-lane exposure depends on harvest strictness, which is uneven: MKV bounds 1..=64 (apps/cli/src/containers/mkv.rs:343 `if channels == 0 || channels > 64 …`), AVI bounds via demux MAX_PCM_CHANNELS=64 (crates/demux/src/avi/header.rs:349,777), but MP4's `audio_geometry` only checks `channels > 0` (apps/cli/src/containers/mp4.rs:389 `(channels > 0 && rate > 0).then_some(...)`), so a crafted `sowt`/`ipcm` entry with channelcount=65535 configures a 65535-channel decoder. No panic/OOM (frame_bytes saturates, L136-137), but the layer that owns channel validation validates asymmetrically. Confidence: HIGH — Verification: read + rg of all `fact.pcm` producers.

F-004 [S4]
Where: crates/codecs/src/audio/pcm/decoder.rs:118-123 and 417-422
Code:
```rust
        if rate == 0 {
            return Err(unsupported(
                "pcm.rate.zero",
                "a PCM stream needs a nonzero sample rate",
            ));
        }
```
What is wrong: rate validation is zero-check only on both seams; rate = 4_000_000_000 configures fine and every duration floors to ~0 ns (L163-168 `checked_div`). No crash (division is guarded by plan construction), pure quality gap; container harvests bound rate inconsistently (MKV ≤4,096,000, MP4 ≤65535 via u16). Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/codecs/src/audio/pcm/decoder.rs:386-393
Code:
```rust
    let u16le = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let u32le =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let tag = u16le(0);
```
What is wrong (minor): `nAvgBytesPerSec` (offset 8) and `wValidBitsPerSample` (offset 18) are never cross-checked (align/bits/channels/rate are); the channel mask is explicitly out of scope (documented gap, mod.rs:63-65). Permissive but bounded and documented — hygiene note only. Indexing is safe (length gate L375-385 guarantees ≥16 bytes). Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PcmPlan::new | 111-144 | REAL | — | zero-ch/rate typed refusals; saturating frame_bytes (wasm32-safe) |
| PcmPlan::output_format | 147-158 | REAL | — | declares source layout verbatim |
| PcmPlan::duration_nanos | 163-168 | REAL | — | floor div, saturating mul; pinned by test incl. u64::MAX |
| whole_frame_prefix | 198-209 | REAL | — | checked_div; frame_bytes ≥1 by construction |
| decode_payload | 215-231 | REAL | — | free-surface twin of contract path |
| swapped | 238-244 | REAL | — | reference converter (differential anchor) |
| swap_groups_into | 259-284 | REAL | — | bps==2 fast loop; total-function ragged-tail parity pinned by test |
| pooled_convert_slot | 307-341 | REAL | — | bounded 4-slot rotation; eviction path correct; `unreachable!` at L330 is a genuine invariant (fresh Arc refcount 1) |
| parse_waveformatex | 374-512 | REAL (tests) / UNREACHABLE from shipped | F-001 | full validation ladder; never run by shipped hosts |
| PcmDecoder::new | 604-606 | REAL | — | |
| PcmDecoder::with_format | 618-636 | REAL | F-003/F-004 | the shipped seam; no channel/rate ceilings |
| PcmDecoder::resolve_facts | 643-673 | REAL | — | conflict = typed error, never silent pick (pinned by test) |
| Decoder::configure | 677-706 | REAL | — | `_planar` strip; re-configure replaces plan (pool stays bounded) |
| Decoder::decode | 708-777 | REAL | — | stale-gen guard first; zero-copy vs pool split at L740; sink error propagated (L768) |
| Decoder::flush | 779-787 | REAL | — | stateless; BadState pre-configure (pinned) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 22 tests | YES | Assertions are on values/pointers/outcome enums, not tautologies; `native_endian_interleaved_decode_is_zero_copy` asserts allocation pointer identity; `converted_path_reuses_pooled_slots_and_stays_bit_exact` runs 64 packets with varying content against the free-surface reference; ceiling test pins boundary 64-accepted/65-refused; differential swap test covers bps∈{2,3,4,8}×{0,1,2,7,64,…} |
Ran them: 22 passed, 0 failed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the engine hands the blob on exactly as it hands AVC config records" | decoder.rs:21-22 | FALSE | pipeline_host.rs:348-360 passes engine-minted `extra_data: None` for pcm_; AVC uses seeded() (L346, L202-210) |
| "output is at most the input … bounded by the packet" (MP §12) | decoder.rs:60-68 | REAL | read: every path is prefix-truncate or swap (length-preserving); pool bounded at CONVERT_SLOTS=4 |
| "a conflict is a typed negotiation failure, never a silent pick" | decoder.rs:29-30 | REAL | resolve_facts L658-669 + test `with_format_seeds_facts_and_conflicts_are_typed` |
| "Silence is never fabricated" | decoder.rs:53-56 | REAL | decode L722-733: empty→NeedMoreInput, sub-frame→Recovered{skipped:1,CorruptInput}; no zero-fill anywhere |

## 7. Dead / duplicated / unreachable code
- `parse_waveformatex` + `resolve_facts`'s `extra` arm + 8 WAVEFORMATEX tests: unreachable from shipped entry points (F-001); alive via public library API and in-repo tests (transcode/tests use `PcmDecoder::new()`, but those are tests).
- `decode_payload`/`swapped`: reachable via `decode_pcm` free surface (mod.rs:281-289) used by fuzz entry (fuzz.rs, audited separately) — not dead.

## 8. Dependencies & flags
std-only in this module; `codec-api` (Decoder/DecodeOutcome/drop_stale_packet, codec-api/src/decode.rs:203-209), `media-types`. PCM is in-house (mod.rs:5-37 records the no-symphonia decision). No feature gates touch pcm (Cargo.toml: in-house decoders unconditional).

## 9. Open questions for the next agent
- Does demux/src/mp4 bound `sowt`/`ipcm` AudioSampleEntry channelcount? The CLI harvest (apps/cli/src/containers/mp4.rs:389) does not — combined with F-003 a crafted MP4 configures a 65535-channel PCM decoder. MP4-stbl auditor should confirm.
- Who consumes `AudioFrame.format.channels` downstream (audio-engine convert.rs) and does it cap channels before allocating?
