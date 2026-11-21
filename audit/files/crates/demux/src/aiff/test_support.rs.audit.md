# AUDIT: crates/demux/src/aiff/test_support.rs (file: 58 lines)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 58 / 3 units (`extended80`, `minimal_aiff`, `mem_stream`) |
| File verdict | REAL (test-support surface) |
| Reachable from shipped entry point? | This file: NO by design (module doc :2-3 "nothing in the demuxer itself uses it" — verified: consumers are aiff/tests.rs (7 call sites), aiff/fuzz.rs (3), the aiff/mod.rs doctest :160). Its consumers executed green in baseline: 32 `aiff::*` tests (:1523+) and the doctest `aiff::AiffDemuxer (line 142) ... ok` (:6366). The subject (AiffDemuxer) IS shipped-reachable via the probe registry (AiffSignature registered; containers/aiff.rs wired). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` full file; extended80 fallback constant hand-verified (44100 = 0x400E exponent 16398 = 16383+15, mantissa 0xAC44×2^48 — byte-exact for the 80-bit Apple extended form); `rg -n "minimal_aiff\("` (10 in-tree call sites — all channels ≥ 1, bits = 16: div-zero not reachable in-tree); `rg -c "test aiff::" baseline` → 32 green; `sed -n` on both quoted ranges. |

## 1. One-paragraph verdict
A tight 58-line builder whose layouts are correct: the 18-byte COMM
(channels/f frames/bits/extended80 rate — field order and widths match the
AIFF spec), SSND with 8-byte offset+blockSize head, big-endian throughout
(AIFF is a BE format — all `to_be_bytes`), FORM size patched to the real
total, and the 44100 fallback constant is byte-exact 80-bit extended (I
re-derived exponent and mantissa by hand). `mem_stream` reuses the AVI seam
like its ASF twin. Two hygiene findings: `minimal_aiff` divides by
`channels × bytes-per-sample` with no zero guard (a pub fn that panics on
`channels=0`/`bits=0` — no in-tree caller does), and the conversion-error
fallback silently writes the 44100 constant for whatever rate actually
failed — unreachable today (u32 → extended80 cannot fail: exact ≤ 2^53,
integral, finite, in range) but wrong-by-construction if ever reached.

## 2. What actually works (verified)
— Layout arithmetic: COMM = 2+4+2+10 = 18 bytes (:31 `18u32.to_be_bytes()`
matches); SSND size = samples+8 (:40); FORM size = len−8 (:44-45);
`with_capacity(54 + n)` = 12 (FORM head) + 8+18 (COMM) + 8+8 (SSND head)
+ n = 54 + n ✓ exact.
— The fallback constant `[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]`
decodes to exactly 44100.0 in IEEE 80-bit extended (verified by hand:
exponent 0x400E = 16398 → 2^15 scale; mantissa 0xAC44 = 44100 at bit 63).
— 32 aiff tests + doctest green in baseline; the builder feeds all of them.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/aiff/test_support.rs:24-25
Code:
```rust
pub fn minimal_aiff(channels: u16, rate: u32, bits: u16, samples: &[u8]) -> Arc<[u8]> {
    let frames = (samples.len() / (channels as usize * usize::from(bits).div_ceil(8))) as u32;
```
What is wrong: division by zero when `channels == 0` or `bits == 0` (the
denominator is 0 in both cases). The function is `pub` on a `pub mod`
(test-support surface), and the doc constrains only `samples`
("must be whole sample frames") — nothing tells the caller that zero
channels or zero bits panic.
Why it matters: a fuzz harness or future test iterating edge-case
parameter combos hits a panic in builder code, not in the demuxer under
test — the failure misattributes. (No in-tree caller passes zero:
all 10 call sites use channels 1-2, bits 16 — verified by rg.)
What correct looks like: `debug_assert!(channels > 0 && bits > 0)` plus a
documented precondition, or a typed refusal mirroring the demuxer's own
zero-channel handling.
Confidence: HIGH — Verification: read + caller grep.

### F-002 [S4]
Where: crates/demux/src/aiff/test_support.rs:35-38
Code:
```rust
    match crate::aiff::chunks::f64_to_extended80(f64::from(rate)) {
        Ok(ext) => b.extend_from_slice(&ext),
        Err(_) => b.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]), // 44100
    }
```
What is wrong: the fallback hardcodes the 44100 encoding regardless of the
`rate` argument. If `f64_to_extended80` ever failed, the built file would
silently declare 44100 Hz for a different requested rate — a fixture that
lies about its own content instead of failing.
Why it matters: unreachable today (u32 → f64 is exact up to 2^53, integral,
finite, and well inside extended80 range — the conversion cannot fail for
any u32), which makes this dead defensive code whose only possible future
effect is a wrong fixture. The AIFF demuxer's tests would then pin
behavior against a file whose rate field contradicts the test's intent.
What correct looks like: propagate the error (`?` — the function already
returns Arc, so it would need a Result signature, or
`unreachable!("u32 rates always encode")` to make the unreachability
explicit and loud).
Confidence: HIGH — Verification: read + conversion-range reasoning + the
constant's byte-exactness proof (§2).
Blast radius: zero today; wrong-fixture-if-ever-reached.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-3 | REAL | — | "nothing in the demuxer itself uses it" verified |
| `extended80` | 9-18 | REAL | — | thin delegate to chunks::f64_to_extended80 (A01's file); error contract documented |
| `minimal_aiff` | 20-47 | REAL | S4 ×2 | F-001 (div-by-zero on zero channels/bits), F-002 (wrong-rate fallback); layouts/endianness/size-patching all correct (§2) |
| `mem_stream` | 49-58 | REAL | — | delegates to AVI seam — no duplication |

## 5. Test honesty
This file is test support; its consumers' honesty is A03's (aiff/tests.rs,
aiff/chunks.rs). The builder itself produces spec-shaped BE AIFF verified
by arithmetic; nothing here asserts anything.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the WAV twin of crate::wav::test_support" | :1-2 | REAL | same shape: builders + shared mem_stream delegate |
| "`samples` must be whole sample frames" | :21-22 | REAL | frames computed by exact division; callers' payloads are frame-multiples |
| "(the ffmpeg-compatible normal form)" | :9-11 | REAL-UNPROVEN | normality vs ffmpeg's writer not re-derived here (chunks.rs is A01's; its `extended80_rejects_non_normalized_shapes` test is green :1526) |

## 7. Dead / duplicated / unreachable code
- F-002's Err arm — dead for all u32 inputs.
- No duplication; delegation is the pattern.

## 8. Dependencies & flags
- `plugin_api::SourceStream`, `crate::aiff::chunks` (workspace-internal);
  std-only; no feature gates; compiled unconditionally (pub test-support —
  same workspace-wide decision as avi/wav/asf).

## 9. Open questions for the next agent
- A03 (aiff/chunks.rs): confirm `f64_to_extended80` cannot fail for finite
  integral f64 < 2^64 (my range reasoning says any u32 is safe — the Err
  path in F-002 is then provably dead; if your analysis finds a failing
  u32, F-002 upgrades to S3).
