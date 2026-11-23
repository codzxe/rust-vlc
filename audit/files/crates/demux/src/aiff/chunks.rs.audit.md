# AUDIT: crates/demux/src/aiff/chunks.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 595 / 17 (9 prod fns + 2 const-groups + comm_bytes + 7 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli `apps/cli/src/containers/aiff.rs:30` (`use demux::aiff::AiffDemuxer`) → `AiffDemuxer::start` (crates/demux/src/aiff/mod.rs:286-401 walk) → `chunks::parse_comm` :236 / `chunks::parse_ssnd` :336 / `extended80_to_f64` :171. `f64_to_extended80` and `SsndChunk` are NOT on the shipped path (test-support / dead). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=1 aiff::chunks` -> 7 passed, exit 0; sed quote verifications (byte-exact); rg caller sweeps (SsndChunk: zero constructors workspace-wide; f64_to_extended80: only aiff/test_support.rs) |

## 1. One-paragraph verdict
The AIFF chunk parsers are real, bounds-disciplined, and live-verified: COMM's
80-bit extended sample-rate decode is mathematically exact on its accepted
domain (integrality + u32 gate ⇒ ≤32 mantissa significant bits ⇒ exact f64),
the compression vocabulary is typed and refusal-honest, and the walk in
mod.rs clamps every payload read before these functions see bytes
(`.min(MAX_COMM_BYTES).min(extent−payload)` at mod.rs:334-336, `size.min(8)`
at :357). Seven unit tests pass live with exact error-code assertions. Defects:
the fixture-builder `f64_to_extended80` accepts a 1344-wide window above 2^64
and silently encodes the WRONG value (u64 saturation), the `SsndChunk` struct
is dead code, and the public `u16be`/`u32be` are unchecked-index foot-guns
(current callers all safe — verified).

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-5e0c7194d741938b aiff::chunks`
-> "test result: ok. 7 passed" exit 0 (round-trip of 14 corpus rates,
non-normalized rejections incl. the canonical ffmpeg 44100 bytes
`40 0E AC 44 00…`, full AIFC vocabulary, typed refusals with exact
`demux.aiff.*` context codes). Caller bounds verified by reading
crates/demux/src/aiff/mod.rs:286-401 (head len-check before u32be, fixed
8-byte raw, want-clamped reads, typed truncations). AIFF fixtures also
exercised by demux's wav_aiff_fixtures suite (baseline green).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/aiff/chunks.rs:196-204
Code:
```
pub fn f64_to_extended80(rate: f64) -> Result<[u8; 10], EngineError> {
    if !rate.is_finite()
        || rate <= 0.0
        || rate.fract() != 0.0
        || rate >= 18_446_744_073_709_552_960.0
    {
        return Err(error::bad_structure(0));
    }
    let bits = rate as u64;
```
What is wrong: the guard constant is 2^64 + 1344 (verified: 2^64 =
18,446,744,073,709,551,616; constant = 18,446,744,073,709,552,960), not 2^64
as its doc comment claims ("practically: not a nonnegative integer below
2^64"). For rate in [2^64, 2^64+1344) the guard passes, `rate as u64`
saturates to u64::MAX (2^64−1), and the function returns Ok encoding of
2^64−1 — a silently WRONG value that breaks the round-trip contract its
twin test enforces. Also dead checks: `exponent >= 0x7FFF` is unreachable
(exponent ≤ 64+16383−1 = 16446), and `unwrap_or(64)` on a u32→i32 conversion
that cannot fail. Why it matters: fixture builders that mis-encode produce
fixtures that lie about their own headers; the magic constant suggests a
typo that review missed. What correct looks like: `rate >= 18_446_744_073_709_551_616.0`
(2^64) or `u64::try_from(rate)` with a typed refusal. Blast radius limited:
only caller is aiff/test_support.rs (verified by rg), never the shipped path.
Confidence: HIGH — Verification: read + python3 arithmetic (2^64 + 1344).

F-002 [S4]
Where: crates/demux/src/aiff/chunks.rs:139-147
Code:
```
/// The parsed SSND facts (absolute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsndChunk {
    /// First sound byte (chunk payload start + 8 + `offset`).
    pub data_start: u64,
```
What is wrong: dead code. `SsndChunk` is never constructed anywhere in the
workspace (rg over crates/ + apps/: zero constructors; mod.rs:370-375 builds
its own private `SoundRegion` with an extra `effective_end` field instead).
`parse_ssnd` returns a bare `(u32, u32)` tuple. A reader will expect
parse_ssnd to yield this type. What correct looks like: delete it, or make
parse_ssnd return it.
Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: crates/demux/src/aiff/chunks.rs:151-157
Code:
```
pub fn u16be(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

/// Read a big-endian `u32` at `data[at]`.
pub fn u32be(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}
```
What is wrong: `pub` unchecked indexing — any future caller that forgets the
documented "bounds are the caller's" contract gets a panic, in a crate whose
house rules (lib.rs:38-39) forbid panics in non-test code. All current
callers are safe (verified: chunks.rs floor-checks at :242/:337; mod.rs:301
after the 12-byte len check, :322 on a fixed [u8;8]) — but the safety is
convention, not type. What correct looks like: return Option or take
`&[u8; 2]`/`&[u8; 4]`.
Confidence: HIGH — Verification: read + caller audit.

F-004 [S4]
Where: crates/demux/src/aiff/chunks.rs:269-271
Code:
```
        // The pstring name follows (1 length byte + chars, padded to
        // even); advisory display text — parsed for the skip math,
        // not kept.
```
What is wrong: the comment is false — the pstring is never parsed anywhere;
the walk's skip math (mod.rs:324-326) uses the declared chunk `size` + pad,
not the pstring length. The comment claims work that does not exist.
What correct looks like: "ignored; the walk skips by declared chunk size".
Confidence: HIGH — Verification: read + mod.rs:324-326.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts FORM/COMM/SSND/MARK/INST + compression tags | 17-57 | REAL | — | correct fourcc tags |
| MAX_COMM_BYTES/COMM_MIN/AIFC_MIN | 34-43 | REAL | — | enforced at mod.rs:331-336 (verified) |
| AiffCodec::descriptor | 88-104 | REAL | — | matches registry names; `_ =>` arm catches 4 (and any invalid — unreachable via resolve_compression which gates 16/24/32) |
| AiffCodec::bytes_per_sample | 107-116 | REAL | — | correct |
| u16be / u32be | 151-158 | REAL | S4 (F-003) | unchecked pub indexers; callers currently safe |
| extended80_to_f64 | 171-186 | REAL | — | exact on accepted domain (proof in §1); rejects denorm/pseudo-zero/inf/NaN/unnormal; is_finite check is dead (cannot be false) |
| f64_to_extended80 | 196-220 | REAL | S3 (F-001) | saturation window above 2^64; dead exponent guard; test-support only |
| parse_comm | 236-289 | REAL | — | floor checks before every index; rate gate (integral, ≤u32::MAX); channels 1..=64 typed; AIFC tail read at [18..22] guarded by floor 22 |
| resolve_compression | 298-327 | REAL | — | typed vocabulary {NONE,sowt}×{16,24,32}, fl32@32, fl64@64, raw@8; all else refused |
| parse_ssnd | 336-341 | REAL | — | len ≥ 8 checked; returns (offset, blockSize) |
| tests: 7 fns | 343-594 | REAL | — | all pass live; exact error-code pins; canonical-ffmpeg byte pattern check |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| extended80_roundtrips_the_corpus_rates | YES | 14 rates exact-equality round-trip |
| extended80_rejects_non_normalized_shapes | YES | five malformed shapes + canonical 44100 bytes assert Some(44100) |
| classic_comm_parses | YES | all five fields pinned |
| aifc_comm_resolves_the_compression_vocabulary | YES | 5 codecs' descriptors + one refusal with exact code |
| unsupported_widths_and_compressions_are_typed | YES | 8/12-bit, ulaw, ima4 refusals with exact codes |
| broken_comms_are_typed | YES | short, zero-channel, hand-encoded non-integral rate, short AIFC tail |
| ssnd_header_parses | YES | ok pair + 7-byte refusal |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "COMM is 18 bytes classic, 22 + pstring for AIFC; anything larger is a lying tail, skipped positionally" | chunks.rs:34-36 | REAL | mod.rs:331-336 clamps reads to MAX_COMM_BYTES and extent; verified read |
| "sowt is the little-endian corner [VERIFY-P0: doc 04 §2.2's AIFF note]" | chunks.rs:61-62 | REAL (open VERIFY tag left in) | resolve_compression maps sowt→PcmLe; tests pin pcm_s16le; tag itself is debt (S4-grade, folded into hygiene) |
| "not a nonnegative integer below 2^64" (f64_to_extended80 doc) | chunks.rs:193-195 | FALSE for [2^64, 2^64+1344) | F-001 arithmetic |

## 7. Dead / duplicated / unreachable code
- `SsndChunk` (F-002) — dead.
- `is_finite` guard in extended80_to_f64 (:185) — dead (value bounded by
  2^16383, never inf).
- `exponent >= 0x7FFF` + `unwrap_or(64)` in f64_to_extended80 (:206-209) — dead.
- `MARK`/`INST` consts (:30-32) — referenced by docs/skip logic? Verified:
  mod.rs skip branch mentions them in comment only; consts are used... rg
  shows no non-doc use of `chunks::MARK`/`chunks::INST` outside their
  declarations — dead-but-documentary. Folded here.

## 8. Dependencies & flags
None beyond media_types + crate::avi::riff::FourCC (documented reuse).
No unsafe. No feature gates. Std-only per crate rules.

## 9. Open questions for the next agent
- aiff/mod.rs (not assigned to me) holds the actual chunk-walk bounds; I
  verified the COMM/SSND read clamps (:331-366) but the seek/effective_end
  arithmetic (:420-435, :591-599) deserves its own audit — especially
  `data_start` when SSND `offset` runs past the chunk (mod.rs stores
  effective_end = data_start; confirm no inversion when data_start >
  declared_end).
- apps/cli/src/containers/aiff.rs harvest path untested here.
