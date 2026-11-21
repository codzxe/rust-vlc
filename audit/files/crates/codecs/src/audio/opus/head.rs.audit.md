# AUDIT: crates/codecs/src/audio/opus/head.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 298 / 12 (1 const pair, 1 struct, 1 parser, 8 test helpers/tests) |
| File verdict | REAL (clean, RFC-conformant parser; every branch either verified by hand against RFC 7845 or pinned by a typed test) |
| Reachable from shipped entry point? | YES (call chain: `lumen` open → `OpusDecoder::configure` crates/codecs/src/audio/opus/decoder.rs:290 `let head = parse_head(blob)?` ← decoder dispatched at apps/cli/src/pipeline_host.rs:276-282 `"opus" =>` on the Ogg lane). Execution: opus suites green in baseline #3 (INDEX.md). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "pre_skip\|output_gain\|DECODE_RATE\|parse_head" opus/decoder.rs` (consumer semantics — gain applied at decoder.rs:307/426-447, pre_skip at 403/460, rate constant everywhere) · `rg "ruopus" Cargo.toml` (external =0.1.2, `codec-opus` feature) · registry lookup for ruopus source (absent — engine panic claims UNVERIFIABLE in-sandbox) · `sed -n` quote verification |

## 1. One-paragraph verdict
A small, disciplined parser that does exactly what its module doc promises: closes every ruopus constructor panic window (family-0 channel count, multistream slot arithmetic, mapping range) with typed refusals before any engine is built. I verified the field offsets byte-for-byte against RFC 7845 §5.1 (magic 0-7, version 8, channels 9, pre-skip 10-11 LE, input rate 12-15 LE, gain 16-17 LE, family 18, streams 19, coupled 20, mapping 21..), the length gates (19/21+channels), the slot arithmetic (streams+coupled == channels, no u16 overflow at 255+255), and the 255-silent mapping exemption — all correct. The two downstream doc claims (gain applied, pre-skip trimmed) are true in the consumer (decoder.rs:307, 403, 426-447, 460). The only gap is test coverage of two legal-input branches, and the engine-panic claims themselves rest on an admission record I cannot re-verify in this sandbox.

## 2. What actually works (verified)
— Field offsets, endianness (all LE per RFC), and gate arithmetic verified by hand against RFC 7845 §5.1; family-0 geometry (streams 1, coupled = channels==2) is arithmetically consistent with the multistream rule.
— Typed rejections pinned by tests: `rejects_the_constructor_panic_windows_typed` (5 codes) and `rejects_broken_multistream_arithmetic` (4 codes) ran green in baseline #3 (INDEX.md: 1681 passed).
— Consumer semantics confirmed: `output_gain` is applied (decoder.rs:307 `self.gain = 10f32.powf(gain/5120)`, 444-447 per-sample multiply), `pre_skip` is honored (403, 460), decode rate is the constant 48 kHz (129, 137, 332, 474) — the head.rs doc claims are honest.

## 3. Findings
### F-001 [S4] Two legal-input branches have no test: silent mapping (255) and non-zero output gain
Where: crates/codecs/src/audio/opus/head.rs:171-174 (with tests at 261-297)
Code:
```
    if let Some((index, slot)) = mapping
        .iter()
        .enumerate()
        .find(|(_, s)| **s != 255 && u16::from(**s) >= slots)
```
What is wrong: the `**s != 255` exemption (RFC 7845: 255 = silent/unused channel) is never exercised — no test builds a head with a 255 mapping entry, so a regression that rejects legal silent channels (dropping the exemption) passes the suite. Likewise every fixture and test uses `output_gain = 0` (head.rs:208, 560-560 in decoder.rs), so the non-zero i16 gain parse (103) is unpinned; a sign/endianness regression there would surface only in production audio level.
Why it matters: limited blast radius (multistream-with-silent-channel streams and gained streams are rare), but these are exactly the "legal input refused" edges a fuzz corpus rarely covers deliberately.
What correct looks like: one test head with mapping `[0, 255, 1]` (channels 3, streams 2, coupled 1... any closing geometry) and one with gain 0x0180 (+1.5 dB) asserting `output_gain == 0x0180`.
Confidence: HIGH — Verification: read; all 9 test cases enumerated (261-297) — none touch either branch.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const DECODE_RATE | 34-35 | REAL | — | 48_000; used by decoder for base/rate/format |
| const HEAD_MIN | 37-38 | REAL | — | 19 = family-0 head size ✓ RFC |
| struct OpusHead (+9 fields) | 41-65 | REAL | — | carries everything the decoder consumes |
| parse_head | 71-194 | REAL | — | offsets verified vs RFC 7845 §5.1; length gates 72/131 correct; slot arithmetic 161-170 overflow-free (u16, ≤510); 255-silent exemption 174; trailing OpusTags ignored by design |
| test head_v0 / head_v1_51 (builders) | 202-222 | REAL | — | byte-exact builders |
| test parses_the_family0_geometry | 224-232 | REAL | — | would fail on any offset regression |
| test parses_the_51_multistream_geometry | 234-241 | REAL | — | streams/coupled/mapping asserted |
| test code_of / rejected (helpers) | 243-259 | REAL | — | must-reject form |
| test rejects_the_constructor_panic_windows_typed | 261-281 | REAL | — | 5 typed codes |
| test rejects_broken_multistream_arithmetic | 283-297 | REAL | — | 4 typed codes; F-001 gap noted |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_the_family0_geometry | YES | exact field assertions on a byte-built head |
| parses_the_51_multistream_geometry | YES | mapping vector asserted |
| rejects_the_constructor_panic_windows_typed | YES | 5 distinct typed codes (truncated/magic/version/channels.zero/family0_channels) |
| rejects_broken_multistream_arithmetic | YES | 4 distinct typed codes |
| (gap) 255-silent acceptance, gain≠0 parse | NO test exists | F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The ruopus constructors panic on channel counts outside {1,2} and rates outside {8,12,16,24,48} kHz" | head.rs:10-13 (citing admission record R6) | REAL-UNPROVEN | ruopus =0.1.2 is an external crate; no registry source in sandbox (checked ~/.cargo, target/) — the record docs/dependencies/records/ruopus-0.1.2.md is the only evidence; the wrapper's gates are strictly safer than claimed even if the panic window were narrower |
| "version must be exactly 1" | head.rs:15-16 | REAL | code 87-93 |
| "family ≥ 1 ⇒ streams ≥ 1, coupled ≤ streams, streams+coupled = channels, mapping < slots" | head.rs:17-22 | REAL | code 145-183, tests 284-297 |
| "the decode rate is ALWAYS 48 kHz" | head.rs:23-27 | REAL | decoder.rs:129/137/332/474 |
| "the decoder applies `sample × gain / 256`" (head.rs doc on output_gain) | head.rs:50-53 | REAL (semantics differ in form, same value) | decoder.rs:307 uses `10^(gain/5120)` = 10^(dB/20) — the Q7.8 field is dB×256, so gain/5120 = dB/20 ✓ equal math |
| "pre-skip: samples to discard from the decode head" | head.rs:45-47 | REAL | decoder.rs:403, 460 |

## 7. Dead / duplicated / unreachable code
None. `OpusHead.input_rate` is advisory and unused by decode (by RFC design — it feeds nothing; kept for diagnostics). All fields have consumers or documented advisory status.

## 8. Dependencies & flags
No direct deps (error types from super). The `codec-opus` feature gates ruopus (=0.1.2, optional) at the crate level (crates/codecs/Cargo.toml:79,137); head.rs itself is feature-independent and always compiled with the opus module. No unsafe, no panics (all indexing behind the 72/131 length gates — verified each slice bound).

## 9. Open questions for the next agent
- opus/decoder.rs (its agent): the head-hold equivalent — does an Opus stream whose FIRST stamped page carries pre-skip trim correctly at a seek landing (decoder.rs:652-717 suggests yes, `granule − window − pre_skip`)? Cross-check with the vorbis F-005 empty-packet question.
- Whoever owns the admission records: the ruopus R6 panic-window claim is load-bearing for this file's entire raison d'être; a registry-side re-verification (or a vendored copy) would close the REAL-UNPROVEN.
