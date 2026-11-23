# AUDIT: crates/demux/src/aiff/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 58 / 3 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO by design — test-support surface ("nothing in the demuxer itself uses it", line 3, rg-verified: users are aiff/tests.rs (7 call sites) and the mod.rs doctest). Compiled into the shipped crate but never called by production paths; exercised by the executed 32-test aiff run. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | part of `target/debug/deps/demux-036dd705535b2fb3 aiff::` → 32 passed (builders drive those tests); `~/.cargo/bin/cargo test -p demux --doc aiff` → 1 passed (doctest uses `minimal_aiff`); rg for users |

## 1. One-paragraph verdict
A correct minimal AIFF builder: 18-byte COMM with the 80-bit extended rate encoded through the
production encoder, SSND with its 8-byte pair, patched FORM size, big-endian throughout. Two
hygiene defects: an unrepresentable rate (e.g. `rate = 0`) silently falls back to writing
44 100 Hz bytes instead of failing, and `channels = 0` divides by zero (panic — fail-loud but
crude). Both are test-support-only blast radius; the builders drove 32 green tests plus the
doctest.

## 2. What actually works (verified)
- `target/debug/deps/demux-036dd705535b2fb3 aiff::` → 32 passed — the fixtures these builders
  produce (COMM/SSND walks, AIFC vocabulary, truncations, seeks) all execute against them.
- `~/.cargo/bin/cargo test -p demux --doc aiff` → 1 passed — the mod.rs doctest builds its file
  via `minimal_aiff(1, 44_100, 16, &[0x12, 0x34])` (frames = 2/(1×2) = 1, correct).
- Layout arithmetic verified by read: 12 (FORM head) + 8+18 (COMM) + 8+8+samples (SSND) =
  54 + len — matches `Vec::with_capacity(54 + samples.len())`; SSND declared size
  `samples.len() + 8` correctly includes the offset/blockSize pair; FORM size patched to
  `len − 8`.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/aiff/test_support.rs:35-38
Code:
```rust
    match crate::aiff::chunks::f64_to_extended80(f64::from(rate)) {
        Ok(ext) => b.extend_from_slice(&ext),
        Err(_) => b.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]), // 44100
    }
```
What is wrong: on an unrepresentable rate the builder SILENTLY writes the canonical 44 100 Hz
encoding — `minimal_aiff(1, 0, 16, …)` produces a file that claims 44 100 Hz, not 0. A test
asking for a degenerate rate gets a different fixture than it asked for, with no signal. What
correct looks like: panic (test-support convention: literals must be representable) or return
Result. Confidence: HIGH — Verification: read (f64_to_extended80 refuses rate ≤ 0 per
chunks.rs:196-203).
F-002 [S4]
Where: crates/demux/src/aiff/test_support.rs:25
Code:
```rust
    let frames = (samples.len() / (channels as usize * usize::from(bits).div_ceil(8))) as u32;
```
What is wrong: `channels = 0` (or `bits = 0`, making the divisor 0) panics with divide-by-zero
— fail-loud, but as an integer panic in builder code rather than a message; `bits` values not
divisible into the accepted vocabulary are not validated either (the demuxer will refuse them,
so tests get a typed refusal fixture — arguably a feature). Test-support blast radius only.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 LOC — table optional, included for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| extended80 | 16-18 | REAL | — | thin re-export of the production encoder; used by aiff/tests.rs:58 |
| minimal_aiff | 24-47 | REAL | S4 | silent 44100 fallback (F-001), div-by-0 on channels=0 (F-002); layout arithmetic verified |
| mem_stream | 54-58 | REAL | — | delegates to the avi shared seam |

## 5. Test honesty
Not a test file; the builders are test infrastructure. The tests they drive (32) are honest
(see the aiff/mod.rs audit's §5).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "public test-support surface, nothing in the demuxer itself uses it" | test_support.rs:2-3 | REAL | rg: users are aiff/tests.rs + the mod.rs doctest only |
| "`samples` must be whole sample frames" (caller's business) | test_support.rs:22 | REAL | frames computed by integer division; partial frames under-count silently — documented contract |

## 7. Dead / duplicated / unreachable code
None (all three functions have users).

## 8. Dependencies & flags
- `plugin_api`, `crate::aiff::chunks`, `crate::avi::test_support`. No features. Public-by-
  design (rustdoc examples cannot see #[cfg(test)] — the WAV twin's documented rationale).

## 9. Open questions for the next agent
- None material. (If a test ever intends a rate-0 or huge-rate AIFF fixture, it currently gets
  44 100 Hz instead — F-001 would silently mask that intent.)
