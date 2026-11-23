# AUDIT: crates/codecs/src/audio/opus/head.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 298 / 1 prod fn (`parse_head`) + `OpusHead` struct + `DECODE_RATE` const; 4 tests + 3 helpers |
| File verdict | REAL (RFC 7845 §5.1 OpusHead parser, pre-constructor panic-window gate; unit tests executed, pass) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:270 `"opus"` → `OpusDecoder` → `configure` → `parse_head` (crates/codecs/src/audio/opus/decoder.rs:290) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p codecs --lib 'audio::opus' --offline` -> 13 passed / 0 failed (includes this module's 4); sed verifications |

## 1. One-paragraph verdict
A correct, tight OpusHead parser that does exactly what its doc promises structurally: it closes the ruopus engine's constructor panic windows (channels ∈ {1,2} for family 0; streams ≥ 1; coupled ≤ streams; streams+coupled = channels; mapping entries in-slot-or-255) *before* any engine is built, and pins the decode rate at the RFC-mandated 48 kHz constant. Field offsets, LE endianness, HEAD_MIN=19 and the 21+channels multistream tail bound all match RFC 7845, and the arithmetic checks match the spec's slot model (coupled stream = 1 stream decoding 2 slots → slots = streams + coupled). Two documentation-level defects: the `output_gain` field doc states the application formula as `sample × gain / 256`, which is mathematically wrong (the decoder applies `10^(gain/5120)` — Q7.8 dB → linear — decoder.rs:307; the doc formula would turn a +1 dB field value of 256 into unity), and reserved mapping families 2..=254 (RFC-reserved, layout undefined) are parsed with the family-1 tail layout — a guess, contradicting the module's own "never guess" posture, though no such streams exist in the wild.

## 2. What actually works (verified)
— `cargo test -p codecs --lib 'audio::opus' --offline` -> `test result: ok. 13 passed; 0 failed` (family-0 and 5.1-multistream parses, all five panic-window rejections, four multistream-arithmetic rejections).
— Verified by read against RFC 7845 §5.1: layout offsets (magic 0..8, version 8, channels 9, pre-skip 10..12 LE, input rate 12..16 LE, gain 16..18 LE, family 18, streams 19, coupled 20, mapping 21..21+channels); version==1 gate; family-0 channels ≤ 2; `slots = streams + coupled == channels` is the spec's decoded-slot identity; mapping entries `< slots` or 255-silent (line 171-183).
— Verified downstream: `build_engine` (decoder.rs:122-141) constructs the mono/stereo engine only for family 0 (channels proven ∈ {1,2}) and the multistream engine only with parse-proven arithmetic; gain actually applied at decoder.rs:307 as `10f32.powf(f32::from(head.output_gain) / 5120.0)` — correct Q7.8-dB-to-linear per RFC.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/opus/head.rs:50-53
Code:
```
    /// Output gain, Q7.8 dB-scaled linear gain (RFC 7845 §4.2): the
    /// decoder applies `sample × gain / 256`. Every corpus lane
    /// measures 0 (unity) — applied anyway, honestly.
    pub output_gain: i16,
```
What is wrong: the stated application formula is false. The decoder applies `sample × 10^(gain/5120)` (decoder.rs:307: `self.gain = 10.0f32.powf(f32::from(head.output_gain) / 5120.0);`) — Q7.8 *decibels* converted to linear. The doc's `sample × gain / 256` is a linear scaling by the raw Q7.8 value: a field value of 256 (+1 dB, correct linear ×1.122) would be unity ×1.0 under the doc formula, and a field value of 512 (+2 dB, ×1.259) would be ×2.0. The implementation is right; the doc misstates it. Why it matters: the next maintainer "fixing" the decoder to match this doc would break gain handling for every non-zero-gain stream. What correct looks like: "the decoder applies `sample × 10^(gain/256/20)`". Confidence: HIGH — Verification: read (both sites quoted).
F-002 [S4]
Where: crates/codecs/src/audio/opus/head.rs:128-131, 184-193; module doc 15-16
Code (128-131):
```
    // Multistream family (1 = Vorbis-ordered surround, 255 = discrete;
    // the arithmetic is family-agnostic on our side — the engine
    // consumes streams/coupled/mapping the same way).
    if blob.len() < 21 + usize::from(channels) {
```
Code (15-16):
```
//! * version must be exactly 1 (RFC 7845 §4: decoders may reject
//!   other versions; we refuse typed, never guess),
```
What is wrong: RFC 7845 reserves mapping families 2..=254 with no defined layout; this parser applies the family-1 tail layout (streams+coupled+mapping after byte 18) to *any* nonzero family and, on self-consistent arithmetic, hands the result to the multistream engine as a discrete passthrough (decoder.rs:292-297 gives the Vorbis reorder only to family 1). That is a layout guess for reserved families, contradicting the module's own "never guess" rule — a reserved-family stream that happens to satisfy the family-1 arithmetic decodes under an assumed geometry instead of being refused typed. No such streams exist in practice (the families are unused), so blast radius is nil today. What correct looks like: accept families {0, 1, 255} only; refuse 2..=254 typed (`opus.head.family_reserved`). Confidence: HIGH — Verification: read; RFC reserved-family status from the public spec (family values 2..254 reserved) — the precise decoder-facing RFC verb is UNVERIFIED (no spec copy in-repo to quote).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_head` | 71-194 | REAL | S4 | Correct RFC layout + all panic-window gates; reserved-family layout assumption (F-002) |
| `OpusHead` (struct/fields) | 41-65 | REAL | S4 | Gain field doc formula false (F-001) |
| `DECODE_RATE` | 34-35 | REAL | — | 48 000 per RFC 7845 §4.2; used as the only decode rate |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_the_family0_geometry | YES | All fields asserted incl. derived coupled=1 |
| parses_the_51_multistream_geometry | YES | streams/coupled/mapping asserted |
| rejects_the_constructor_panic_windows_typed | YES | 5 distinct rejection codes (family0 3ch, 0ch, bad magic, short, version 2) |
| rejects_broken_multistream_arithmetic | YES | slot mismatch, coupled>streams, mapping range, missing tail |
| (gap) | — | No test for a mapping entry 255 (silent) — accepted by line 174 but never exercised; no test for family 255 discrete or a reserved family 2..254 (F-002's behavior is unpinned) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ruopus constructors panic on channel counts outside {1,2} … this parser closes the WHOLE geometry before any engine is built" | head.rs:10-13 | REAL | Gates at 95-100, 108-116, 145-183; `build_engine` (decoder.rs:122-141) relies on them |
| "the decode rate is ALWAYS 48 kHz … the rate side of the panic contract is a constant" | head.rs:23-27 | REAL | `DECODE_RATE = 48_000` used at decoder.rs:129,137,249,332,474 |
| "the decoder applies `sample × gain / 256`" | head.rs:51 | FALSE (formula) | F-001: actual application is `10^(gain/5120)` (decoder.rs:307) |
| "we refuse typed, never guess" | head.rs:16 | SHALLOW | F-002: reserved families 2..254 are parsed under an assumed family-1 layout |

## 7. Dead / duplicated / unreachable code
- None. Single consumer chain (decoder.rs configure), all fields consumed.

## 8. Dependencies & flags
- No deps of its own; sits behind `codec-opus` (`dep:ruopus`, codecs/Cargo.toml:79) in the default feature set — shipped in the release lane.

## 9. Open questions for the next agent
- Does ruopus's multistream constructor itself validate mapping families, or would a reserved-family stream (F-002) reach engine state with a guessed geometry? The engine's accepted-family set is UNVERIFIED (ruopus source not read; pin 0.1.2).
- Opus granule/pre-skip trimming lives in decoder.rs, not here — the head fields this module extracts (`pre_skip`) are consumed there; an auditor of decoder.rs should confirm pre-skip is actually applied.
