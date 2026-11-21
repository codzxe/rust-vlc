# AUDIT: crates/demux/src/aiff/chunks.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 594 / 26 (2 fns + 1 fn encode/decode pair + 2 parsers + 2 readers + 2 struct impls (3 methods) + 17 consts + 2 enums/structs; 7 tests + 1 helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → probe names `aiff` (`media-probe/src/signature/mod.rs:134`) → `apps/cli/src/containers/mod.rs:53` `"aiff" => aiff::open(...)` → `AiffDemuxer` pre-pass walk (`aiff/mod.rs:315-401`) → `parse_comm` (mod.rs:349) / `parse_ssnd` (mod.rs:366)). `f64_to_extended80` is test/fixture-only (see F-003). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n` on chunks.rs (full read), aiff/mod.rs:310-400 (walk — MAX_COMM_BYTES enforced at 335, positional skip), aiff/tests.rs:495-540 (the `let _ = chunks::MARK;` site), test_support.rs:1-40 + `od -c` line 23 (confirmed `#[must_use]`, an earlier cat artifact); `rg -n "SsndChunk\|CommChunk\|f64_to_extended80\|chunks::MARK\|chunks::INST"` (dead-vocabulary proof); `rg -n "aiff" apps/cli/src/containers/` (dispatch); baseline `orchrun-test-workspace-nofailfast.log.txt:1523-1529` (7/7 file tests ok) |

## 1. One-paragraph verdict
Real, careful parsers with a genuinely correct self-built 80-bit extended float (layout, bias 16383, explicit integer bit, denormal/pseudo-zero/inf/NaN/unnormal all refused — pinned by tests including the canonical ffmpeg 44100 bytes) and a disciplined typed-refusal vocabulary. The defects are all around the edges: a layer of dead exported vocabulary (`SsndChunk` never constructed or consumed anywhere; `MARK`/`INST` consts never matched — the walk skips positionally; `MARK`'s only reference is a `let _ =` discard inside an unrelated error-code test), a comment claiming the AIFC pstring is "parsed for the skip math" when nothing parses it, a sub-ulp rounding path that falsifies the documented "non-integral rates are typed refusals" invariant, and a release binary that ships a test-only encoder plus dead defensive checks. The `MAX_COMM_BYTES` ceiling the module promises is real — enforced by the caller at `aiff/mod.rs:335`.

## 2. What actually works (verified)
— All 7 tests executed and passed: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1523-1529` (`aiff::chunks::tests::* ... ok`, 7 lines).
— `MAX_COMM_BYTES` (4096) is not decorative: enforced at `aiff/mod.rs:335` `let want = (size as usize).min(chunks::MAX_COMM_BYTES).min(...)` — verified by sed on the walk.
— 80-bit extended math re-derived by hand: exponent = `u16::from_be_bytes([bytes[0] & 0x7F, bytes[1]])` (173), explicit-integer-bit check (181-183), value = mantissa × 2^(e−16383−63) (184); for integer rates ≤ u32::MAX the mantissa has ≤ 32 significant bits, so the `as f64` conversion is exact — no precision loss in the accepted domain. Canonical 44100 bytes `40 0E AC 44 00…` decode to 44100.0 exactly (test :397-398).
— Endianness discipline: every field read is big-endian (`u16be`/`u32be`/`from_be_bytes`), matching IFF; the SSND pair is read but its `blockSize` honestly unused (documented at :330-332).

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/aiff/chunks.rs:29-32, 139-147 (and the liveness hack at crates/demux/src/aiff/tests.rs:536)
Code:
```rust
/// The `MARK` chunk id (marker list — skip-tolerant).
pub const MARK: FourCC = FourCC::tag(b"MARK");
/// The `INST` chunk id (instrument parameters — skip-tolerant).
pub const INST: FourCC = FourCC::tag(b"INST");
```
```rust
/// The parsed SSND facts (absolute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsndChunk {
```
What is wrong: none of this vocabulary is used by the demuxer. The walk (`aiff/mod.rs:310-401`) branches only on `chunks::COMM` and `chunks::SSND`; MARK/INST/COMT/NAME/etc. skip **positionally** via the else-fallthrough (mod.rs:378-380 comment says so). `SsndChunk` (chunks.rs:140-147, 6 lines of doc'd fields `data_start`/`payload_start`/`declared_end`) is never constructed — `parse_ssnd` returns `(u32, u32)` and mod.rs builds its own `SoundRegion`. `INST` has zero references repo-wide; `MARK`'s sole reference is `let _ = chunks::MARK;` inside `error_constructors_keep_their_codes` (tests.rs:536) — a discard statement whose only function is keeping the const alive. Related comment rot: `parse_comm`'s AIFC tail comment (chunks.rs:269-271) says the pstring name is "parsed for the skip math, not kept" — nothing anywhere parses the pstring; the walk skips by ckSize.
Why it matters: dead pub vocabulary in a lib crate evades dead-code lints and misleads readers into believing MARK/INST are recognized ids and SSND facts flow through `SsndChunk`; the `let _ =` in a test is coverage-theater for a constant.
What correct looks like: delete `SsndChunk`, `MARK`, `INST` (the walk's positional skip needs no ids); fix the pstring comment to "skipped positionally by ckSize — never parsed".
Confidence: HIGH — Verification: read + `rg` repo-wide for every symbol (no other references; `rg -n "SsndChunk"` → definition only).

### F-002 [S4]
Where: crates/demux/src/aiff/chunks.rs:184-185 (conversion) and 254 (the integrality gate)
Code:
```rust
    let value = sign * (mantissa as f64) * f64::powi(2.0, i32::from(exponent) - 16_383 - 63);
    if value.is_finite() { Some(value) } else { None }
```
What is wrong: `CommChunk.sample_rate`'s doc (:130-132) and `parse_comm`'s error contract promise "non-integral or non-normalizable rates are typed refusals". The gate `rate.fract() != 0.0` (:254) operates on the f64-rounded value: a rate with more than 53 significant bits — e.g. 44100 + 2^-40, mantissa `0xAC44000000000100`, a shape the 80-bit form encodes exactly — rounds to exactly 44100.0 in `mantissa as f64` (bit 8 is below the f64 ulp 2^11 at that magnitude), so `fract() == 0.0` and the file is **accepted** with rate 44100 instead of refused. Hand-crafted inputs only; error is ~2^-40 Hz.
Why it matters: a documented typed refusal is silently a rounding operation at the edge; anyone relying on "non-integral ⇒ `demux.aiff.bad-structure`" can construct a counterexample.
What correct looks like: do the integrality check in integer space — the value is integral iff the low `s = 16446 − exponent` mantissa bits are all zero (for the accepted range); refuse otherwise, and let the f64 conversion happen only for display.
Confidence: HIGH on the arithmetic (re-derived by hand: value ≤ u32::MAX ⇒ mantissa ≤ 33 sig bits when integral, but non-integral shapes with 54+ sig bits exist and round to integers); UNVERIFIED by execution (no cargo runs allowed; no fixture exercises it).

### F-003 [S4]
Where: crates/demux/src/aiff/chunks.rs:196-220 (`f64_to_extended80`), esp. 205-209
Code:
```rust
    let bits = rate as u64;
    let lead = 64 - bits.leading_zeros(); // 1..=64
    let exponent = i32::try_from(lead).unwrap_or(64) + 16_383 - 1;
    if exponent >= 0x7FFF {
        return Err(error::bad_structure(0));
    }
```
What is wrong: two things. (a) The only callers of this pub fn are `aiff/test_support.rs:17,35` and this file's own tests — `pub mod test_support;` is unconditional (`aiff/mod.rs:69`), so the release library ships a test-only encoder (its own doc calls it "the fixture builder's twin of the parse"). (b) The `exponent >= 0x7FFF` guard is dead: `lead ≤ 64` ⇒ `exponent ≤ 16446`, never near 32767; likewise `u16::try_from(exponent).unwrap_or(0)` (:215) can never fall back. Defensive branches that cannot fire read as if they were load-bearing.
Why it matters: release-surface bloat and guards that pretend to handle cases they can't reach inflate confidence in coverage that doesn't exist.
What correct looks like: cfg-gate the encoder behind a test/fixture feature (or move it to test_support), and either assert the invariant (`debug_assert!(exponent < 0x7FFF)`) or delete the dead arms.
Confidence: HIGH — Verification: read + rg (callers enumerated); baseline build/test logs show the fn only via test binaries (nofailfast :1523-1529 are unit tests; no production run cites it).

### F-004 [S4]
Where: crates/demux/src/aiff/chunks.rs:90-99
Code:
```rust
            AiffCodec::PcmBe { bytes_per_sample } => match bytes_per_sample {
                2 => "pcm_s16be",
                3 => "pcm_s24be",
                _ => "pcm_s32be",
            },
```
What is wrong: the `_` arm maps **every** other `bytes_per_sample` (1, 5, 7, 0…) to `pcm_s32be`. In-crate the only constructor (`resolve_compression`:308-310) produces 2/3/4 from the validated width table, but `AiffCodec` is a pub enum with pub fields — any downstream construction of `PcmBe { bytes_per_sample: 5 }` silently names itself pcm_s32be, corrupting the registry descriptor the decoder layer selects by.
Why it matters: descriptor strings are the demuxer→decoder contract; a catch-all turns contract violations into wrong-codec selection instead of a compile error or panic.
What correct looks like: exhaustive match `2 | 3 | 4 => …` with the impossible arm as `unreachable!()`/debug_assert, or a constructor fn that validates.
Confidence: HIGH — Verification: read (only constructor is resolve_compression; enum fields pub).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `FORM`/`FORM_AIFF`/`FORM_AIFC`/`COMM`/`SSND` | 18-28 | REAL | — | Used by walk/probe (mod.rs:93, 287, 296-297, 328, 353 — verified). |
| consts `MARK`/`INST` | 29-32 | DEAD | S3 | F-001. |
| const `MAX_COMM_BYTES` | 34-37 | REAL | — | Enforced at mod.rs:335; the `[PROPOSAL: 4 KiB]` note is honest about its arbitrariness. |
| consts `COMM_MIN_BYTES`/`COMM_AIFC_MIN_BYTES` | 40-43 | REAL | — | Floors used at mod.rs:331, 339-341 and parse_comm:242. |
| consts `COMPRESSION_*` (5) | 47-57 | REAL | — | All matched in resolve_compression:304-324. |
| `AiffCodec::descriptor` | 88-104 | REAL | S4 | F-004 catch-alls (both Pcm arms). |
| `AiffCodec::bytes_per_sample` | 107-116 | REAL | — | Fixed values per variant; consistent with bytes_per_sample fields. |
| `CommChunk` / `SsndChunk` | 121-136 / 139-147 | REAL / DEAD | S3 | CommChunk flows (mod.rs:169, 218, 620, 629); SsndChunk never constructed (F-001). |
| `u16be` / `u32be` | 151-158 | REAL | — | Bounds are caller's (documented :149-150); all in-module call sites pre-check floors. |
| `extended80_to_f64` | 171-186 | REAL | S4 | Correct 80-bit math (verified by hand + tests); F-002 sub-ulp rounding; extreme exponents: powi overflow→inf caught by is_finite, underflow→0.0 passes through but parse_comm's `rate <= 0.0` (:254) refuses. |
| `f64_to_extended80` | 196-220 | REAL (test-only) | S4 | F-003: ships in release, test-only callers, dead guards at 207/215. Round-trip exact for ≤53-bit rates; >53-bit integers (e.g. 2^64−1) round-trip lossily — irrelevant to the accepted u32 domain. |
| `parse_comm` | 236-289 | REAL | — | Floor check :242; channels 0 and >64 → bad_comm (:259-263); rate ≤0/non-integral/>u32::MAX → bad_structure (:254); AIFC tail read [18..22] guarded by 22-byte floor. Classic path validates NONE widths and discards the codec (:279) — correct refusal for 8-bit classic. |
| `resolve_compression` | 298-327 | REAL | — | Width tables {16,24,32} × {NONE,sowt}, fl32@32, fl64@64, raw@8, else typed refusal — matches tests and the doc vocabulary exactly. |
| `parse_ssnd` | 336-341 | REAL | — | 8-byte floor; returns the raw pair; the walk (mod.rs:366-378) owns the offset math. |
| test helper `comm_bytes` | 347-357 | REAL | — | Faithful encoder twin. |

## 5. Test honesty (7 tests, all executed ok in baseline :1523-1529)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| extended80_roundtrips_the_corpus_rates | YES | 14 rates incl. 1.0 and 65536; a bias/normalization bug breaks equality. |
| extended80_rejects_non_normalized_shapes | YES | Denormal, Apple pseudo-zero, inf, NaN, unnormal, plus the canonical ffmpeg 44100 acceptance — a permissive decoder fails the first five, a broken one fails the sixth. |
| classic_comm_parses | YES | Field-by-field equality. |
| aifc_comm_resolves_the_compression_vocabulary | YES | 5 accepts with exact descriptor strings + 1 refusal with exact error code. |
| unsupported_widths_and_compressions_are_typed | YES | Classic 8-bit, 12-bit NONE, ulaw, ima4 — all exact codes. |
| broken_comms_are_typed | YES | 17-byte floor, zero channels, hand-encoded non-integral rate (0xAC44 80…), 20-byte AIFC tail. GAP: the `channels > 64` ceiling (:262-263) has no test here and none in aiff/tests.rs (rg found no 65/ceiling case) — that branch is executed only by fuzz, if ever. |
| ssnd_header_parses | YES | Happy pair + 7-byte refusal. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "80-bit IEEE 754 extended — self-built, pure integer math" | module doc :1-3 | REAL | extended80_to_f64 :171-186 (read, arithmetic re-derived); round-trip tests baseline-ok. |
| "Only normalized values are accepted … return None" | :166-169 | REAL | :174-183 + rejecting test. |
| "non-integral or non-normalizable rates are typed refusals" | CommChunk doc :130-132 | REAL-UNPROVEN at the edge / falsifiable | F-002: sub-ulp fractions are rounded, not refused; standard fractions (0.5) are refused (test :553-564). |
| "Maximum COMM payload trusted into memory [4 KiB]" | :34-37 | REAL | mod.rs:335 clamp (sed-verified). |
| "the SSND bytes are authority — a disagreement is damage, not a guess" | :125-127 | REAL (mod.rs-side) | Stated here, enforced in the walk's SoundRegion/damage accounting (mod.rs:386-399, `ssnd-extends-past-stream`). |
| "sowt is the little-endian corner [VERIFY-P0: doc 04 §2.2]" | :61-62 | REAL, flag still open | sowt→PcmLe mapping :313-318; the `[VERIFY-P0]` marker remains unresolved in-tree — doc 04 owner should close it. |
| "raw — the ffmpeg AIFC mapping … the committed aifc_raw_u8 fixture pins it" | :321-323 | REAL-UNPROVEN | Code + test pin the mapping (:471-484); I did not open the fixture binary (other agent's asset). |

## 7. Dead / duplicated / unreachable code
F-001 (SsndChunk, MARK, INST, the tests.rs:536 discard). F-003's dead guards (:207, :215). Layering note: this module imports `crate::avi::riff::{FourCC, SourceReader}` — the AIFF family leaning on the AVI module's byte plumbing (documented at :7-10; same crate, acceptable, but the "verbatim reuse" claim makes avi/riff.rs a cross-family chokepoint).

## 8. Dependencies & flags
`media-types` (EngineError), `plugin-api` via crate. No feature flags touch this file. No unsafe. No unwrap in production paths (the `unwrap_or`/`unwrap_or_else` forms are total). Errors all typed via `super::error` with stable codes (`demux.aiff.bad-structure|bad-comm|unsupported-compression` — pinned by tests).

## 9. Open questions for the next agent
- `channels > 64` (`bad-comm` ceiling, :262-263): untested in this file and in aiff/tests.rs — confirm whether any fuzz corpus or later wave covers it; if not, it is an unexecuted branch in shipped code.
- The `[VERIFY-P0: doc 04 §2.2]` marker on the sowt mapping (:62) is still open since card 5.A2 — the doc-04 agent should close or escalate it.
- `pub mod test_support` (mod.rs:69) ships byte-builders in the release lib for aiff AND wav — a repo-wide pattern worth one consolidated architecture finding by whoever owns demux/src/lib.rs.
