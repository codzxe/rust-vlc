# AUDIT: crates/demux/src/wav/format.rs#part1 (lines 1-440)
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 440 audited / 21 (2 enums + 3 structs with 10 methods, 2 private readers, 2 formula fns, `parse_fmt` head through 440; 9 constants, 3 GUID tables) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `apps/cli/src/containers/mod.rs:73` `"wav" => wav::open` → `apps/cli/src/containers/wav.rs:50` `WavDemuxer::new` → `crates/demux/src/wav/mod.rs:721` `format::parse_fmt(...)`). Baseline executes the whole lane through the real open path: `containers::wav::tests::corpus_wav_lanes_decode_through_the_open_path ... ok` (log 2841). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |
| Commands run | `rg -n "parse_fmt" crates/demux/src/wav/*.rs apps/cli/src/containers/wav.rs` (production call chain); `rg -n "u16le\(|u32le\("` (all 9 call sites listed + guarded — verified); formula check by hand vs doc examples (1017/2041/1012); baseline greps (format tests 1912-1921, wav lane 1927-1932, open-path lane 2838-2843); `sed -n` verification of every quote. |

Chunk boundary: nominal part1 = 1-440; `parse_fmt` (345-507) is split — my
range covers its head through the ADPCM `declared` acceptance (line 440); the
PCM fall-through, valid-bits and align checks (441-507) and the whole `#[cfg(test)]`
mod (509-881) are part2's (A07). The ADPCM comment/refuse block (430-440) is mine.

## 1. One-paragraph verdict
Real, reachable, executed parser code on one of the two genuinely shipped
decode lanes (WAV + pcm_s16le): all 10 format tests pass in the baseline
(`orchrun-test-workspace-nofailfast.log.txt:1912-1921`) and the lane is proven
end-to-end through the real open path (`containers::wav::tests::corpus_wav_lanes_decode_through_the_open_path ... ok`,
log 2841). The PCM accepted-set is deliberately mirrored to the decoder's
(`parse_waveformatex`'s rules) to avoid demux-ok/decode-fail divergence — the
design concern is documented and the align re-check comment (490-495) shows
both layers enforce the same rule. The ADPCM formulas I re-derived by hand
match the documented corpus values (IMA 1024/2ch → 1017, 1024/1ch → 2041; MS
1024/2ch → 1012) and use checked arithmetic. Four hygiene defects: a
"two payloads" doc that lists three, an ADPCM comment that contradicts its own
code, an unvalidated `valid_bits` on the FLAC path that the PCM path refuses,
and panic-by-indexing private readers kept safe only by caller discipline.

## 2. What actually works (verified)
— 10/10 `wav::format::tests` pass: baseline log 1912-1921.
— Open-path execution: `containers::wav::tests::all_wav_fixture_flavors_open ... ok`
   (2839) and `corpus_wav_lanes_decode_through_the_open_path ... ok` (2841).
— ADPCM formulas re-derived: IMA `((ba−4ch)·2)/ch+1` ≡ 2·(ba/ch)−7 (the spec
   formula) → 1017 (stereo) / 2041 (mono) as documented (305); MS
   `(ba−7ch)/ch·2+2` → 1012 as documented (318). Both use `checked_mul`/
   `checked_add` — no overflow.
— All 9 `u16le`/`u32le` call sites in the file are length-guarded (16/20/40
   byte floors before every read — enumerated and verified); no panic path is
   live.
— The three SubFormat GUID byte tables match the Microsoft-registered values
   (PCM `…0001…`, FLOAT `…0003…`, FLAC `0000F1AC…` — mmreg.h order, LE first
   DWORD).

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/wav/format.rs:12-17
Code:
```
//! Beyond PCM, the parser recognizes the two non-PCM payloads the
//! randomized real corpus actually carries inside WAV containers —
//! **IMA ADPCM** (`wFormatTag 0x0011`), **MS ADPCM** (`0x0002`) and
//! **FLAC-in-WAV** (the `KSDATAFORMAT_SUBTYPE_FLAC` SubFormat GUID,
```
What is wrong: "the two non-PCM payloads" — then three are named (IMA ADPCM,
MS ADPCM, FLAC-in-WAV).
Why it matters: a count that contradicts its own list in the module's
orienting paragraph; readers (and downstream auditors quoting it) mis-scope
the accepted set.
What correct looks like: "the three non-PCM payloads".
Confidence: HIGH — Verification: read.
### F-002 [S4]
Where: crates/demux/src/wav/format.rs:430-440
Code:
```
        let samples_per_block = match (declared, derived) {
            (Some(declared), Some(derived)) => {
                // A declaration that disagrees with the block
                // geometry is a lie about arithmetic, not a variant:
                // trust the geometry, count nothing silently — the
                // typed refusal (the ADPCM decoder would decode the
                // declared count against the same bytes).
                if declared == 0 || declared > derived {
                    return Err(error::bad_format_params(offset));
                }
                declared
```
What is wrong: The comment says a disagreeing declaration is refused and the
geometry trusted; the code refuses only `declared > derived` and then uses
`declared` — an UNDER-declaration (`declared < derived`) is accepted and the
declaration wins, not the geometry.
Why it matters: `samples_per_block` drives duration/packet arithmetic; a
writer that under-declares produces drift while the comment promises the
refusal. Real writers declare the exact formula value, so the blast radius is
malformed files — but the doc/code divergence is the defect: a maintainer
"fixing" either side breaks the other's expectation.
What correct looks like: either refuse `declared != derived` (matching the
comment) or rewrite the comment to "an over-declaration is impossible
arithmetic and refused; an under-declaration is trusted as the writer's
framing".
Confidence: HIGH — Verification: read; both branches traced.
### F-003 [S4]
Where: crates/demux/src/wav/format.rs:391-400
Code:
```
            let codec = WavCodec::Flac { streaminfo };
            return Ok(WaveFormat {
                format_tag: TAG_EXTENSIBLE,
                channels,
                samples_per_sec: rate,
                block_align,
                bits_per_sample: bits,
                valid_bits_per_sample: Some(u16le(bytes, 18)),
                channel_mask: Some(u32le(bytes, 20)),
                codec,
            });
```
What is wrong: Two inconsistencies vs the PCM/float paths. (1) The FLAC
early-return records `valid_bits_per_sample` without any validation — the PCM
path (part2, 481-488) typed-refuses `valid == 0 || valid > bits`; the FLAC
path accepts and stores both. (2) `format_tag` stays `TAG_EXTENSIBLE` (0xFFFE)
here while the struct doc (244-245) promises "for extensible … the SubFormat
GUID's resolution" and the PCM/float paths store the resolved tag (1/3).
Why it matters: a FLAC-in-WAV with `wValidBitsPerSample = 0` (garbage) passes
with the oddity recorded as fact; and `format_tag` is semantically overloaded
(resolved for PCM/float, wrapper for FLAC) — any consumer branching on it
mis-handles one family.
What correct looks like: run the same valid-bits check before the FLAC
return (or document its exemption); add a `TAG_FLAC`-equivalent or document
that FLAC keeps the wrapper tag.
Confidence: HIGH — Verification: read; PCM-path checks confirmed at 481-488 (part2, quoted for context only).
### F-004 [S4]
Where: crates/demux/src/wav/format.rs:291-297
Code:
```
fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}
```
What is wrong: The private readers index directly — panic on out-of-bounds.
Today every call site is guarded (verified: 16-byte floor for 349-353, 40-byte
for 397-404, 20-byte for 421), so no live panic — but the safety is caller
discipline, unlike the ASF module's `guid::u16le` (asf/guid.rs:184-189) which
returns 0 on OOB. The crate bans panics in non-test code (lib.rs:38-39); one
future unguarded call site turns a malformed file into a crash.
Why it matters: latent panic surface in a parser reached from the shipped
open path; the pattern invites the exact class of bug found in asf/header.rs
this wave (F-001 there — the reverse: hardened there, not here).
What correct looks like: `bytes.get(at..at+2).map(...)` with a typed refusal,
or a `debug_assert!` + documented precondition.
Confidence: HIGH — Verification: read; all 9 call sites enumerated and checked.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const MAX_FMT_BYTES / MAX_CHANNELS | 36 / 41 | REAL | — | 4 KiB fmt cap (enforced by caller mod.rs:712); 64-channel ceiling mirrors decoder MAX_PCM_CHANNELS. |
| TAG_* / size constants | 44-66 | REAL | — | All match the WAVE spec values; FLAC_WAV_BYTES 74 = 40 + 34 STREAMINFO. |
| SUBFORMAT_{PCM,IEEE_FLOAT,FLAC}_GUID | 69-82 | REAL | — | Byte tables verified against the registered GUIDs (LE DWORD first). |
| `enum PcmSample` | 91-104 | REAL | — | The 6-shape vocabulary; mirrors registry descriptors. |
| `PcmSample::descriptor` | 109-118 | REAL | — | Exact doc 04 §3.1 strings (decode seam — the engine looks decoders up by these). |
| `PcmSample::bits_per_sample` / `bytes_per_sample` | 121-140 | REAL | — | Consistent pairs; S32Le/F32Le share 32. |
| `PcmSample::from_wave_tag` | 146-162 | REAL | — | Total: impossible combos → None → typed refusal downstream (474). |
| `enum WavCodec` | 168-199 | REAL | — | 4 families; FLAC carries `Option<[u8;34]>` STREAMINFO (decoder-config seam). |
| `WavCodec::descriptor` | 204-211 | REAL | — | adpcm_ima_wav / adpcm_ms / flac — honest names for undecoded-as-of-v0 codecs (engine reports no decoder — documented 19-20). |
| `WavCodec::unit_size` | 217-223 | REAL | — | PCM frame, ADPCM block_align, FLAC byte-quantum (documented honest unknown). |
| `WavCodec::samples_per_unit` | 228-235 | REAL | — | None for FLAC (needs frame grammar — never fabricated). |
| `struct WaveFormat` | 243-267 | REAL | S4 | Field docs honest ([VERIFY] oddity flagged); format_tag semantics inconsistent for FLAC (F-003). |
| `WaveFormat::codec_descriptor` / `unit_size` / `samples_per_unit` | 273-288 | REAL | — | Thin delegations, correct. |
| `fn u16le` / `fn u32le` | 291-297 | REAL | S4 | F-004: panic-capable private readers, currently all call sites guarded. |
| `ima_samples_per_block` | 306-313 | REAL | — | Formula re-derived ≡ spec; checked arithmetic; None when block_align ≤ 4·ch. |
| `ms_samples_per_block` | 319-326 | REAL | — | Formula re-derived ≡ ffmpeg; checked arithmetic; None when block_align ≤ 7·ch. |
| `parse_fmt` (head, 345-440) | 345-440 | REAL | S4 | 16-byte floor; zero-channels/rate and >64 channels refused (355-363); extensible resolved GUID-first (367-407); FLAC early-return (379-400, F-003); ADPCM block_align==0 refused (414-416), declared/derived reconciliation (425-452 spans the boundary; the comment-vs-code mismatch F-002 is mine at 431-440). |

## 5. Test honesty (tests at 509-881 are part2's; assessed as evidence for my code)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_plain_pcm16 (1917) | YES | Every field + unit_size + descriptor asserted. |
| parses_extensible_pcm24_like_ffmpeg_writes (1916) | YES | Byte-exact ffmpeg shape incl. cbSize 22, mask 0x3, GUID. |
| extensible_without_body_is_a_structural_lie (1915) | YES | The 0xFFFE-without-40-bytes path (368-372). |
| extensible_flac_guid_exposes_streaminfo_and_descriptor (1913) | YES | STREAMINFO capture (383-389) + "flac" descriptor. |
| adpcm_tags_parse_their_block_framing (1912) | PARTIAL | Covers declared==derived shapes; the under-declared acceptance (F-002) and the (Some,None) trust case (445-450, part2) unexercised. |
| unknown_tags_and_depths_are_typed_refusals (1919) | YES | from_wave_tag None paths. |
| valid_bits_below_container_bits_are_refused (1920) | PARTIAL | Pins the PCM path only — FLAC path's missing check (F-003) unexercised. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| WAV container demux (fmt plain/extensible/float, ADPCM, FLAC-in-WAV) | docs/feature-matrix.toml `container-wav-*` rows (evidence cites `crates/demux/src/wav/**`) | REAL | Open-path corpus lanes decode (baseline 2841); format tests 10/10 (1912-1921). |
| "one accepted-set, no demux-ok/decode-fail divergence for PCM" | format.rs:6-8 | REAL-UNPROVEN | The mirror claim is enforced by identical rules (align check 490-495 + decoder re-check per comment); no test in this file cross-runs the decoder's parser against the same blobs — the corpus open-path test (2841) is the indirect proof. |
| "the two non-PCM payloads" | format.rs:12-13 | FALSE | F-001: three named. |

## 7. Dead / duplicated / unreachable code
None. Every constant/type/method has a consumer (parse_fmt, wav/mod.rs, or the
container host). The `u16le`/`u32le` readers duplicate asf/guid.rs's — with
worse discipline (F-004).

## 8. Dependencies & flags
No feature gates; std-only; no unsafe. Reached from the shipped CLI. Baseline
ran format tests in the default selection (release profile deviation per INDEX.md).

## 9. Open questions for the next agent
- A07 (format.rs#part2): the `(Some(declared), None)` ADPCM arm (445-450)
  accepts any nonzero declared samples_per_block when the geometry cannot
  derive one (block_align ≤ preamble) — e.g. IMA stereo with block_align 8
  and declared 1000 passes with physically impossible framing. Worth a
  finding in your range; my F-002 covers only the comment half.
- A07: confirm the test mod pins `format_tag` for the FLAC family (F-003's
  second half) — if a test asserts 0xFFFE, the inconsistency is load-bearing.
- Whoever consumes `channel_mask`: it is recorded everywhere but I found no
  reader outside tests — verify before calling it dead.
