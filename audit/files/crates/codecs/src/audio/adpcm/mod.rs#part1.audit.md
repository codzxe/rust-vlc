# AUDIT: crates/codecs/src/audio/adpcm/mod.rs#part1 (lines 1–693)
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 693 (of 1386; part2 = 694–1386 → W07-A04) / 18 items |
| File verdict | REAL (WAV pair reachable+executed; breadth five arms UNREACHABLE from shipped entry) |
| Reachable from shipped entry point? | YES for `adpcm_ms`/`adpcm_ima_wav` (chain: `lumen open` → playback-engine `DecoderProvider::decoder_for` → apps/cli/src/pipeline_host.rs:298-303 `codec.starts_with("adpcm_")` → `self.seeded(track, config, AdpcmDecoder::new)`; descriptors minted by demux/src/wav/format.rs:207-208 and demux/src/asf/header.rs:406-409). NO for the five breadth flavors (`adpcm_yamaha/ima_oki/swf/ima_qt/ima_ws`): rg over the whole repo shows no demuxer/host mints those descriptors — tests/fuzz only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 (F-006 added after auditing misc.rs — same wave, same agent; the claim lives in this part's range) |
| Commands run | rg "AdpcmDecoder|adpcm::" crates (registration + callers); rg "adpcm_yamaha|adpcm_swf|…" (breadth descriptor minters — none); sed -n quotes (48-53, 192-208, 436-445, 553-558, 645-676); rg "MAX_CHANNELS" → demux/src/wav/format.rs:41 (=64, matches the doc claim); rg "header_bytes" adpcm (ms=7*ch, ima=4*ch); rg adpcm in audit/runs/baseline/test-workspace.log.txt:822-843 (differential tests pass); `ffmpeg -decoders | grep -i oki` → only `adpcm_ima_oki` exists in ffmpeg 7.1.5 (lettered `adpcm_oki` claim TRUE). |

## 1. One-paragraph verdict
Part1 is the decoder's front half: module docs, the `Plan` enum, `AdpcmDecoder` state, the two configure gates (`configure_wav` WAVEFORMATEX/coefficient intake, `parse_raw_facts` dispatch) and the `Decoder::configure`/`decode`/`flush` contract. The code is real, panic-free by construction (bounds-checks traced below), and bit-exactness against ffmpeg 7.1.5 is proven by committed baseline runs, not just claimed. The defects found are documentation lies about repo state, a doctest whose bytes contradict its own prose, and two latent-contract traps in `decode`/`configure_wav` that today's flavor set cannot trigger. The five "breadth" flavors are dead code from the shipped CLI's perspective — no demuxer mints their descriptors — which the module docs admit, while simultaneously (and falsely) claiming the demux crate "carries AVI only".

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:822-843 → `adpcm_breadth_differential`: 4 passed (`breadth_corpus_is_bit_exact_against_ffmpeg_reference`, `committed_fixtures_decode_bit_exact`-style corpus checks); `adpcm_corpus_differential`: 2 passed (`committed_fixtures_decode_bit_exact`, `real_corpus_decodes_bit_exact`); unit tests `audio::adpcm::…` all ok. This backs the doc's "bit-exactly against the ffmpeg 7.1 reference" claim with an executed run.
— verified by `ffmpeg -hide_banner -decoders | grep -i oki` → `adpcm_ima_oki` only: ffmpeg 7.1.5 indeed ships NO `adpcm_oki` decoder, so the lettered-OKI typed refusal (`adpcm.oki.no_reference`, mod.rs:481-492) rests on a true premise.
— verified by rg "adpcm_ms" crates/demux → wav/format.rs:207-208, asf/header.rs:406-409: the WAV/ASF demuxers mint the descriptors that make the WAV pair reachable through pipeline_host.rs:298-303.

## 3. Findings

### F-001 [S3] — module doc claims the demux crate "carries AVI only": FALSE
Where: crates/codecs/src/audio/adpcm/mod.rs:48-52
Code:
```rust
//! Sega `.adp`, Dialogic `.vox`, SWF/FLV sound tags, QuickTime chunks,
//! Westwood AUD chunks — none of which have demuxers yet (future
//! cards; the demux crate currently carries AVI only). Until those
```
What is wrong: The demux crate carries mp4, mkv, avi, ogg, wav, aiff, asf, ps, ts, ivf, adts, es (see crates/demux/src/* and audit/files/crates/demux/src/*). "AVI only" was true months ago; it is false now.
Why it matters: The doc uses this false premise to explain why the breadth flavors have no container lane. A reader auditing reachability is pointed away from the fact that WAV and ASF demuxers DO mint `adpcm_ms`/`adpcm_ima_wav` today. Stale capability claims are exactly the class of doc rot this audit exists to flag.
What correct looks like: "the demux crate carries AVI, WAV (which mints the ADPCM WAV pair), ASF, MP4, …; no demuxer mints the breadth descriptors yet."
Confidence: HIGH — Verification: read (demux source tree + rg evidence above).

### F-002 [S4] — doctest bytes contradict the doctest prose
Where: crates/codecs/src/audio/adpcm/mod.rs:194-208
Code:
```rust
/// The WAV flavors — the demuxer's `fmt ` payload as facts (IMA WAV,
/// mono, 8 kHz, 256-byte blocks, 249 samples per block):
///
/// let fmt = [
///     0x11, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00, 0x00, 0x20,
///     0x00, 0x00, 0x00, 0x04, 0x00, 0x02, 0x00, 0xF9, 0x00,
/// ];
```
What is wrong: The prose says "256-byte blocks"; the bytes encode nBlockAlign at [12..14] = 0x0400 = 1024. The trailing fields are shifted: wBitsPerSample reads 0x0200=512, cbSize reads 0xF900=63744, and byte 19 of wSamplesPerBlock is missing (array is 19 bytes, not 20). The doctest passes only because `parse_waveformat` reads tag/channels/rate/block_align and ignores the misaligned tail.
Why it matters: This example is the documentation of the facts wire format; anyone copying it ships a 1024-byte-block fmt blob believing it is 256. The prose numbers (249 samples/block) are the test-helper's (tests `ima_fmt` at mod.rs:890-901 uses block_align param + 249), not the example bytes'.
What correct looks like: `0x11,0x00, 0x01,0x00, 0x40,0x1F,0x00,0x00, 0x00,0x20,0x00,0x00, 0x00,0x01, 0x04,0x00, 0x02,0x00, 0xF9,0x00` (20 bytes: block align 0x0100, bits 4, cbSize 2, samples/block 249).
Confidence: HIGH — Verification: read (field offsets counted by hand against WAVEFORMATEX layout used by parse_waveformat:305-341).

### F-003 [S4] — `decode` fall-through reports `Emitted { frames: 1 }` even when no frame was emitted
Where: crates/codecs/src/audio/adpcm/mod.rs:663-675
Code:
```rust
        if stats.skipped > 0 {
            Ok(DecodeOutcome::Recovered {
                skipped: stats.skipped as u32,
                reason: RecoverReason::CorruptInput,
            })
        } else if stats.concealed {
            Ok(DecodeOutcome::Recovered {
                skipped: 0,
                reason: RecoverReason::ConcealedFrame,
            })
        } else {
            Ok(DecodeOutcome::Emitted { frames: 1 })
        }
```
What is wrong: If `samples.is_empty()` while `stats.skipped == 0 && !stats.concealed`, the decoder returns `Emitted { frames: 1 }` having accepted zero frames into `out`. The comment at 645-648 asserts "Every other path decoded at least one sample" — true for the current seven flavors (raw flavors emit ≥1 sample per non-empty byte; SWF/IMA-QT short packets count as skipped; MS/IMA tails ≥ header emit seed samples) but the invariant is enforced nowhere.
Why it matters: A future flavor (or a refactor of a flavor decode fn) that returns an empty buf without counting a skip makes the decoder report a phantom frame to the pipeline — the exact class of frame-accounting drift the engine's `Emitted` counting relies on.
What correct looks like: `else if samples.is_empty() { Ok(DecodeOutcome::NeedMoreInput) } else { Ok(DecodeOutcome::Emitted { frames: 1 }) }` — or a debug_assert on the invariant.
Confidence: HIGH on the code shape, MEDIUM on unreachability — Verification: read (traced all seven flavor entry points at 583-635 + swf.rs/ima.rs/ms.rs doc'd damage models); not executed.
### F-004 [S4] — `configure_wav`'s `_` arm silently means "adpcm_ima_wav"
Where: crates/codecs/src/audio/adpcm/mod.rs:437-444
Code:
```rust
            _ => {
                if tag != ima::TAG_WAVE_FORMAT {
                    return Err(unsupported(
                        "adpcm.waveformat.tag",
                        format!("wFormatTag {tag:#06x} is not the IMA-WAV tag 0x0011"),
```
What is wrong: `configure_wav` is only called with `"adpcm_ms" | "adpcm_ima_wav"` (mod.rs:510), so `_` == `"adpcm_ima_wav"` today; but the match arm does not name it. Adding a third WAV flavor to `accepts`+`configure` routing without touching this arm silently parses the new flavor as IMA.
Why it matters: Classic copy-paste-dispatch trap; the IMA tag check would then mask the bug as a confusing "not the IMA-WAV tag 0x0011" error for a non-IMA flavor.
What correct looks like: `\"adpcm_ima_wav\" => { … }` with a `_ => unreachable/typed refuse` arm naming the routing bug.
Confidence: HIGH — Verification: read.
### F-005 [S4] — wBitsPerSample never validated for the WAV flavors
Where: crates/codecs/src/audio/adpcm/mod.rs:316-340
Code:
```rust
        let tag = u16::from_le_bytes([extra[0], extra[1]]);
        let channels = u16::from_le_bytes([extra[2], extra[3]]);
        let rate = u32::from_le_bytes([extra[4], extra[5], extra[6], extra[7]]);
        let block_align = u16::from_le_bytes([extra[12], extra[13]]);
```
What is wrong: The 4-bit-per-sample field (offset 14) is ignored; a fmt blob claiming wBitsPerSample 8 or 16 is accepted and decoded as 4-bit nibbles.
Why it matters: A mis-tagged stream decodes to garbage at half/double rate with a success outcome instead of a typed refusal — the decoder's own "honest gate" doctrine would refuse. Real-world blast radius small (every real ADPCM WAV writes 4).
What correct looks like: read [14..16], refuse `!= 4` with `adpcm.waveformat.bits`.
Confidence: HIGH — Verification: read (no other read of offsets 14-15 in part1; rg confirms no bits check).

### F-006 [S3] — "steady-state decode is allocation-free per packet" is false: every packet allocates a fresh intermediate Vec + full copy
Where: crates/codecs/src/audio/adpcm/mod.rs:115-118 (claim) and 580-581, 637 (refutation)
Code:
```rust
/// Pooled output slots (the PCM `CONVERT_SLOTS` precedent): the
/// rotation reuses a freed slot via `get_mut`, so steady-state decode
/// is allocation-free per packet.
```
```rust
        let samples = pooled_slot(&mut pool, &mut slot_next, bound, |bytes| {
            let mut buf = Vec::new();
```
```rust
            bytes[..buf.len()].copy_from_slice(&buf);
            buf.len()
```
What is wrong: The `pooled_slot` rotation only reuses the *output* `SharedBytes`. Inside the fill closure every packet allocates a fresh `buf` Vec (grown by the flavor decoders' `out.extend_from_slice`/`push`), decodes into it, and then the whole decoded payload is *copied again* into the pooled slot (line 637). The breadth flavor decoders allocate further per-packet scratch (`misc.rs:162` scratch, `misc.rs:218-219` lows/highs).
Why it matters: The D-1 "allocation-free steady state" is this decoder's stated performance contract; the real steady state is 2-4 heap allocations plus one full buffer copy per packet. The PCM sibling does it correctly — its flavor fillers write directly into the pooled slot (`swap_groups_into` / `dst.copy_from_slice`, pcm/decoder.rs:748-755) with no intermediate Vec.
What correct looks like: Pass the pooled slot's `&mut [u8]` (or a cursor) into the flavor decoders and write samples directly, as the PCM decoder does; delete the `buf` Vec.
Confidence: HIGH — Verification: read (allocation/copy sites traced end-to-end: mod.rs:580-639, misc.rs:162/218-219; PCM contrast). Discovered while auditing misc.rs (same agent, W07-A03); the false claim itself sits in this part's line range, hence recorded here.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-93 | REAL | S3 | Claims differential-tested (verified vs baseline); one FALSE repo-state claim (F-001); damage-model description matches code |
| module decls (`fuzz`,`ima`,`misc`,`ms`,`oki`,`swf`,`yamaha`) | 95-101 | REAL | — | fuzz pub, flavor modules pub(crate) |
| `OUT_SLOTS` | 115-118 | REAL | — | pool depth 4, matches PCM precedent |
| `RAW_TAG` | 120-123 | REAL | — | 0x0000 marker, used by parse_raw_facts (part2) |
| `enum Plan` | 126-151 | REAL | — | 7 variants; breadth variants unreachable from shipped CLI (no descriptor minter) |
| `Plan::channels` | 154-164 | REAL | — | trivial, correct |
| `Plan::rate` | 166-175 | REAL | — | trivial, correct |
| `Plan::output_format` | 177-184 | REAL | — | S16/interleaved/rate — matches doc claim |
| `AdpcmDecoder` struct + Default | 251-272 | REAL | — | state vectors + slot pool; no Drop issues (SharedBytes Arc-style) |
| `AdpcmDecoder::new` | 279-281 | REAL | — | trivial |
| `AdpcmDecoder::accepts` | 288-300 | REAL | — | 8 descriptors incl. refuse-only `adpcm_oki`; matches test at 995-1009 |
| `AdpcmDecoder::parse_waveformat` | 305-341 | REAL | S4 | len≥16, channels 1..=64 (matches demux MAX_CHANNELS=64, wav/format.rs:41), rate≠0, block_align≠0; ignores bits/sample (F-005); no panics (fixed offsets after len check) |
| `AdpcmDecoder::configure_wav` | 346-463 | REAL | S4 | MS lane: tag 0x0002, ≤2ch, ≥22B, wNumCoef==7, 28B coef tail, exact `STANDARD_COEFFICIENTS` equality, block_align ≥ 7*ch+1; IMA lane: tag 0x0011, block_align ≥ 4*ch+1; `_` arm trap (F-004) |
| `Decoder::configure` | 467-537 | REAL | — | gate → oki typed refuse → facts required (`adpcm.facts.missing`) → per-descriptor plan; state vectors sized/reset per (re)configure; no partial-state on error (plan assigned last) |
| `Decoder::decode` | 539-676 | REAL | S4 | stale-packet guard via codec-api drop_stale_packet (decode.rs:203); empty→NeedMoreInput; bound math (part2 fns) verified safe against all 7 flavors' max expansion (≤8× for SWF w2 stereo, ≤4×+seeds WAV; copy_from_slice cannot OOB); per-packet `plan.clone()` (small Copy-ish enum, negligible); Emitted fall-through (F-003); duration = instants×1e9/rate, u128→u64 cast safe for real inputs |
| `Decoder::flush` | 678-683 | REAL | — | Drained; correct (blocks self-contained; raw flavors stateless w.r.t. flush) |
| `struct Stats` | 686-691 | REAL | — | trivial ledger |

## 5. Test honesty (tests in part2's range 882+ exercise part1 code; assessed for part1 items)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `gate_accepts_exactly_the_seven_flavors` (995-1016) | YES | asserts accepts() for all 8 descriptors and rejects `adpcm_ima`; also asserts typed code `adpcm.codec_descriptor` for the DVI gap |
| `lettered_oki_is_the_honest_typed_gap` (1018-1031) | YES | pins `adpcm.oki.no_reference` + `ErrorKind::Unsupported` |
| `missing_facts_and_zero_block_align…` (1033-1051) | YES | pins `adpcm.facts.missing` and `adpcm.waveformat.block_align` codes |
| `missing_facts_and_skeleton_damage…` (1053-1100) | YES | pins tag/channel/rate/short-skeleton/unknown-descriptor codes against mutated facts |
| baseline `real_corpus_decodes_bit_exact` / `breadth_corpus_is_bit_exact…` | YES | MD5-oracle differentials; any decode mutation flips the hash (test-workspace.log.txt:822-843 all ok) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bit-exactly against the ffmpeg 7.1 reference on the real corpus" | mod.rs:33-37 | REAL | baseline test-workspace.log.txt:822-843 (corpus+breadth differential, all passed) |
| "no ffmpeg 7.1.5 decoder or encoder exists" (lettered OKI) | mod.rs:28, 481-492 | REAL | `ffmpeg -decoders` in this sandbox (7.1.5-0+deb13u1): no `adpcm_oki`, only `adpcm_ima_oki` |
| "the demux crate currently carries AVI only" | mod.rs:50-52 | FALSE | demux tree: wav/asf/mp4/mkv/ogg/ts/ps/aiff/ivf/adts/es all present; F-001 |
| "channels … outside 1..=64 … demuxer's MAX_CHANNELS ceiling" | mod.rs:320-322 | REAL | demux/src/wav/format.rs:41 `pub const MAX_CHANNELS: u16 = 64` |
| "an MP4 cannot be opened" context: ADPCM row reachability | baseline INDEX.md | n/a (part1-specific) | adpcm reachable via WAV/ASF lanes only; breadth five UNREACHABLE (no descriptor minter) |

## 7. Dead / duplicated / unreachable code
- `Plan::Yamaha/ImaOki/Swf/ImaQt/ImaWs` + their decode dispatch arms (mod.rs:601-635) and the `yamaha_state`/`oki_state`/`ima_state` fields: unreachable from `lumen`/`lumen-tauri` — no demuxer or host mints those descriptors (rg evidence in header). Alive only via tests + `fuzz_codec_adpcm`. Not dead weight (documented as future-container lanes) but dead in the shipped build.
- No duplication found within part1.

## 8. Dependencies & flags
- Std-only path confirmed for this file (no external crates used; the codecs crate's claxon/symphonia deps live in other modules). No feature gates on part1 code — compiled unconditionally.

## 9. Open questions for the next agent
- Part2 owner (W07-A04): verify `wav_out_bound`'s `+ 2*channels*blocks` seed term against actual seed bytes (MS seeds are 2 samples/channel = 4*channels bytes, more than the term's 2*channels); my arithmetic says the nibble term's slack covers it (per-block margin ≥ 32 bytes mono, ≥ 52 stereo, MS; IMA-WAV similar) — confirm when auditing part2:743-751.
- Does the pipeline's `DecodeOutcome::Recovered` accounting assume zero frames delivered? `decode` can deliver a frame AND return Recovered (skipped>0 after partial decode) — if the engine counts frames strictly from the outcome, counts drift (part1:663-667).
