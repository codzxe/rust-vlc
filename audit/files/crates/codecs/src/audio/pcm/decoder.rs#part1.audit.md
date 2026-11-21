# AUDIT: crates/codecs/src/audio/pcm/decoder.rs#part1 (lines 1-524 of 1572)
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 524 / 17 (constants + 8 fns + 3 structs + doc-claim block) |
| File verdict (part1) | REAL |
| Reachable from shipped entry point? | YES for the byte engine (`whole_frame_prefix`/`decode_payload`/`pooled_convert_slot` run under `PcmDecoder::decode` on the AVI/WAV/MP4 PCM lanes: `apps/cli/src/pipeline_host.rs:347-370` `PcmDecoder::with_format` for `pcm_*`; baseline rc-smoke AVI decoded 60/60 audio frames). `parse_waveformatex` (374-512) is NOT reached by any shipped host: every shipped PCM lane seeds facts via `with_format` and passes `extra_data: None` (`apps/cli/src/demux_host.rs:160-164` "None for AVI"; `apps/cli/src/containers/mp4.rs:194-203` routes PCM through the `pcm` pair) — it is exercised only by unit tests + the doctest. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "mod tests\|swap_parity\|swapped"` (tests start line 790 — parts 2/3); read `pcm/mod.rs:100-229` (PcmFormat invariants: bps ∈ {1,2,3,4,8}, `is_little_endian` None only for U8); read `pipeline_host.rs:347-370`, `demux_host.rs:140-221`, `containers/mp4.rs:192-216` (facts routing); baseline `orchrun-test-workspace-nofailfast.log.txt:422-451` (all 30 pcm decoder/fuzz tests pass). |

## 1. One-paragraph verdict
Part 1 is the honest core of the one shipped audio decoder: the whole-sample-frame prefix rule (with its planar-buffer claim, which I re-derived and confirmed: `L = m·channels·bps` ⟺ equal planes of whole samples), the endianness normalization twins (`swapped`/`swap_groups_into`, byte-identity pinned by a test that passed in baseline), the bounded 4-slot conversion pool, and a paranoid WAVEFORMATEX parser (channel ceiling, GUID resolution, checked align arithmetic — the W2-15 overflow hardening is real and tested). The two real criticisms: the entire WAVEFORMATEX/`extra_data` negotiation seam is dormant in every shipped host (all ship the `with_format` pair), and that dormant parser is stricter than the container world (19–39-byte legal forms refused). No panic path, no unbounded allocation, no silent lie found in this range.

## 2. What actually works (verified)
- Baseline rc-smoke (`audit/runs/baseline/rc-smoke-engine-only.log.txt`, INDEX #20): AVI MJPEG+PCM lane `a_frames=60 a_decoded=60` — the part-1 byte engine (`decode_payload` zero-copy path) executed end-to-end in the shipped binary.
- Baseline tests (orchrun log:429,431,432,435,443): `native_endian_interleaved_decode_is_zero_copy`, `planar_clean_passes_through_with_planar_layout`, `planar_truncation_keeps_plane_boundaries_at_whole_frames`, `swap_groups_into_is_byte_identical_to_the_reference_swapped`, `zero_copy_path_never_materializes_a_pool_slot` — all `ok`.
- `pcm/mod.rs:149-157` read: `bytes_per_sample` ∈ {1,2,3,4,8} — the `frame_bytes ≥ 1` invariant `whole_frame_prefix` relies on holds by enum construction.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/pcm/decoder.rs:374-512 (representative quote 375-384)
Code:
```rust
fn parse_waveformatex(bytes: &[u8], format: PcmFormat) -> Result<Facts, PcmError> {
    if !matches!(bytes.len(), 16 | 18) && bytes.len() < 40 {
        return Err(unsupported(
            "pcm.waveformat.length",
```
What is wrong: the whole `extra_data`-as-WAVEFORMATEX negotiation seam (module doc channel 1, lines 16-23) is unreachable from any shipped entry point. The CLI's PCM lanes build the decoder with `PcmDecoder::with_format(channels, rate)` (`pipeline_host.rs:359`, `apps/web/src/host.rs:606`) and configure with the engine-minted `extra_data: None`; the demux-host doc states PCM blobs are "`None` for AVI (its `strf` pair above is the PCM truth)" (`demux_host.rs:160-161`) and the MP4 host routes PCM through the `pcm` pair (`containers/mp4.rs:194-203`). So the 139-line parser — including the SubFormat-GUID resolution and the W2-15 overflow hardening — compiles into the shipped binary but is driven only by tests/doctest.
Why it matters: dead-in-shipped-lane surface is exactly where drift hides: a future host that DOES pass a blob will exercise code no shipped run has ever executed; and the module doc (lines 16-27) presents the blob channel as a live sanctioned path without saying no container currently uses it.
What correct looks like: either wire one lane (AVI `strf` verbatim through `extra_data`, cross-checked as designed) or mark the seam `[PROPOSAL]`-only in the doc and the registry so its dormant status is explicit.
Confidence: HIGH — Verification: read + `rg` of all `PcmDecoder` construction sites (`pipeline_host.rs:359`, `bench.rs:316`, `web/src/host.rs:606` — all `with_format`).

### F-002 [S4]
Where: crates/codecs/src/audio/pcm/decoder.rs:375-385
Code:
```rust
    if !matches!(bytes.len(), 16 | 18) && bytes.len() < 40 {
        return Err(unsupported(
            "pcm.waveformat.length",
            format!(
                "stream-facts blob must be a 16-byte PCMWAVEFORMAT, \
                 18-byte WAVEFORMATEX or >= 40-byte WAVEFORMATEXTENSIBLE, \
                 got {} bytes",
```
What is wrong: WAVEFORMATEX blobs of 19–39 bytes — schema-legal forms with `cbSize > 0` (e.g. the common 20-byte `cbSize=2` shape) — are refused typed, though their 18-byte prefix is fully parseable and ffmpeg accepts any `strf` ≥ 16 for PCM.
Why it matters: an interop strictness on the (currently dormant, F-001) seam; a file ffmpeg plays would be refused here.
What correct looks like: accept any length ≥ 18 with `cbSize == len - 18` consistency (or ≥ 16 reading only the fixed prefix), refusing only genuine structural lies.
Confidence: HIGH — Verification: read; the strictness is deliberate and test-pinned (`waveformatex_rejects_short_long_and_wrong_tags ... ok`, baseline log:438) — so this is a policy critique, not an accident.

### F-003 [S4]
Where: crates/codecs/src/audio/pcm/decoder.rs:389-393
Code:
```rust
    let tag = u16le(0);
    let channels = u16le(2);
    let rate = u32le(4);
    let block_align = u16le(12);
    let bits = u16le(14);
```
What is wrong: `nAvgBytesPerSec` (bytes 8..12) is read by nobody and never cross-checked (`rate × block_align`), and `rate` has no sanity ceiling (`0xFFFF_FFFF` Hz passes, yielding ~0 ns durations downstream); `cbSize` (16..18 on the 18-byte form) is likewise unread.
Why it matters: consistency fields left unchecked are exactly the lies a damaged blob can tell; an absurd rate flows into `AudioFormat.rate` and the clock limbs.
What correct looks like: cross-check avg-bytes (tolerant of rounding) and bound the rate (e.g. ≤ 768_000 Hz, the family ceiling).
Confidence: HIGH — Verification: read (impact bounded: typed gates cover zero values; the dormant seam compounds F-001).

### F-004 [S4]
Where: crates/codecs/src/audio/pcm/decoder.rs:313-325
Code:
```rust
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
```
What is wrong: slots are only reusable at an exact length match; a stream whose packet sizes vary (damaged tails → shorter conversions, or containers with varying chunk sizes) never hits the warm path and allocates a fresh buffer per packet (the evict branch, 335-339), defeating the "steady-state decoding allocates nothing per buffer (D-1)" claim of lines 66-68 for that regime.
Why it matters: bounded (≤ 4 slots retained) but the D-1 claim is stated unconditionally; variable-size packet streams pay an allocation per buffer.
What correct looks like: size-class the pool (reuse any slot with `len >= needed`, slice on handout — the `bytes.slice(0..exact)` pattern the mp2 decoder uses) or state the fixed-size precondition in the claim.
Confidence: MEDIUM — Verification: read; steady-state AVI/WAV chunks are fixed-size in practice, so the defect needs a varying-size stream to bite.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc claims | 1-68 | REAL | — | Prefix-rule math for planar buffers re-derived and confirmed; "output ≤ input" holds (`swapped`/`to_vec` of the prefix only). |
| `WAVE_FORMAT_PCM` / `WAVE_FORMAT_IEEE_FLOAT` / `MAX_PCM_CHANNELS` | 79-91 | REAL | — | Ceiling 64 mirrors demuxer's; agreement claimed pinned by attack suites (pcm fuzz tests pass, baseline:447-451). |
| `PcmPlan::new` | 111-144 | REAL | — | Zero-channel/zero-rate typed gates; saturating frame_bytes (wasm32-safe); swap derivation correct vs `is_little_endian` (verified against pcm/mod.rs:193-207). |
| `PcmPlan::output_format` | 147-158 | REAL | — | Planarity mapped to AudioLayout. |
| `PcmPlan::duration_nanos` | 163-168 | REAL | — | Saturating mul + checked div; floor semantics documented and test-pinned (`duration_floors_to_whole_nanoseconds`, baseline:425). |
| `Payload` / `Prefix` structs | 172-196 | REAL | — | Carriers. |
| `whole_frame_prefix` | 198-209 | REAL | — | checked_div defensiveness; invariant frame_bytes ≥ 1 holds by construction; planar-boundary claim TRUE (math in §1). |
| `decode_payload` | 215-231 | REAL | — | Zero-copy when no swap and whole packet; conversion only on need. |
| `swapped` | 238-244 | REAL | — | Cold-path reference; chunks(bps) on the already-truncated prefix. |
| `swap_groups_into` | 259-284 | REAL | — | Hot-path twin; 2-byte specialization; ragged-tail parity; byte-identity test passed (baseline:435). |
| `CONVERT_SLOTS` / `pooled_convert_slot` | 286-341 | REAL | S4 | Rotation bounds re-derived: no OOB (`% count` loop; evict index < CONVERT_SLOTS); `unreachable!` at 330 is input-unreachable (fresh refcount-1 allocation, documented); exact-length matching perf gap = F-004. |
| `parse_waveformatex` | 374-512 | REAL (UNREACHABLE from shipped hosts — F-001) | S3/S4 | No OOB (length gate ≥ 16 before any indexed read; GUID slice guarded by ≥ 40); channel ceiling, checked align (`u16::try_from`), tag/bits cross-checks all typed; strictness F-002, unchecked fields F-003. |

## 5. Test honesty
Tests for this range live at lines 790+ (parts 2/3 — not my range). From the baseline log all 30 pcm decoder tests pass, including the ones pinning this range's behavior (`waveformatex_accepts_16_and_18_byte_forms`, `waveformatex_rejects_short_long_and_wrong_tags`, `waveformatextensible_length_ladder_pins_the_physical_bound`, `waveformatextensible_resolves_guids_and_decodes`, `channel_counts_beyond_the_ceiling_are_typed_refusals`, `converted_path_reuses_pooled_slots_and_stays_bit_exact`, `swap_groups_into_is_byte_identical_to_the_reference_swapped`). Would they fail if broken? Yes — they assert exact error codes, byte-identical outputs, and slot-reuse counts. The `swap_groups_into` differential is a genuine two-implementation pin, not a tautology (one impl is the spec, the other the optimization).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "extra_data as WAVEFORMATEX — the container's own out-of-band init bytes, verbatim" (lines 16-23) | module doc | REAL-UNPROVEN / dormant | Code real and tested (baseline:437-441); no shipped container passes a PCM blob (F-001 chain). |
| "for planar data it is exactly the largest split into whole equal planes of whole samples" (lines 40-45) | module doc | REAL | Math re-derived: L multiple of channels×bps ⟺ c equal planes of whole samples; `planar_truncation_keeps_plane_boundaries_at_whole_frames ... ok` (baseline:432). |
| "steady-state decoding allocates nothing per buffer (D-1)" (lines 66-68) | module doc | SHALLOW | Holds only for fixed-size packets; exact-length pool matching allocates per buffer on varying sizes (F-004). |
| "Silence is never fabricated" (lines 52-56) | module doc | REAL (part-1 side) | `decode_payload` drops tail, emits shorter prefix — concealment decided in part 2's decode; `partial_tail_is_concealed_not_dropped ... ok` (baseline:430). |

## 7. Dead / duplicated / unreachable code
- `parse_waveformatex` + the 3 GUID/tag constants (346-358): unreachable from shipped entry points (F-001) — library/doctest/test surface.
- `swapped` vs `swap_groups_into`: intentional duplication, pinned byte-identical by test — acceptable, documented (lines 246-258).

## 8. Dependencies & flags
- No external deps; `pcm` module unconditional (audio/mod.rs:59-61) — always in the shipped build.
- No unsafe; single `unreachable!` (line 330) with documented invariant (crate convention "no panics on any input" not violated — path not input-reachable).

## 9. Open questions for the next agent (part 2/3)
- Part 2 (`configure`/`decode`/`flush`): confirm the Recovered outcome bookkeeping uses `Payload.dropped_tail` honestly, and that re-`configure` after seek keeps the `seeded` facts (the `with_format` seam) — the module doc (517-521) promises it.
- Does any host ever deliver a PCM `extra_data` blob (F-001)? If wave 8+ audits a container host that does, re-rate F-001.
