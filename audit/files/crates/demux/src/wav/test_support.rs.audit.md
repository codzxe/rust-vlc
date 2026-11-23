# AUDIT: crates/demux/src/wav/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 69 / 3 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (by design and by doc): test-support surface — consumed by wav/tests.rs, wav/fuzz.rs, and the wav/mod.rs doctest; "nothing in the demuxer itself uses it" verified (mod.rs references are doc-example lines 223/235 only). The builders are compiled into the release crate (pub, not cfg(test)) — see F-002. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg usage sweep (wav/tests.rs, wav/fuzz.rs, wav/mod.rs doctest; no non-wav consumers of minimal_pcm_*); baseline cross-check: demux Doc-tests "wav::WavDemuxer (line 216) ... ok" (cargo-test-workspace.txt:2476); sed + od quote verifications |

## 1. One-paragraph verdict
A small, honest test-builder file: `minimal_pcm_wav` emits a correct minimal
RIFF/WAVE layout (fmt chunk exactly 16 bytes little-endian, data size and
patched RIFF size verified), `minimal_pcm16_wav` is a thin tag-1 wrapper,
and `mem_stream` is a pure delegation to the AVI crate seam (verified at
avi/test_support.rs:121-124 — one implementation, not two, as claimed).
The builder's output is proven end-to-end by the passing wav/mod.rs doctest
(WavDemuxer parses it, codec pcm_s16le, packet emitted — baseline line
2476). Only hygiene defects: an argument-order trap between the wrapper and
the wrapped builder, and unchecked arithmetic in code that ships in release
builds.

## 2. What actually works (verified)
— verified by baseline doctest run: "test crates/demux/src/wav/mod.rs -
wav::WavDemuxer (line 216) ... ok" (audit/runs/baseline/cargo-test-workspace.txt:2476),
which builds a file via `minimal_pcm16_wav(&[0x34, 0x12], 44_100, 1)` and
drives the real demuxer to a packet. Delegation target read
(avi/test_support.rs:121-124). Layout arithmetic checked by hand: 4+4+4 +
4+4+16 + 4+4+len, riff_size patched to b.len()−8 — correct.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/wav/test_support.rs:53-55 vs :21-27
Code:
```
pub fn minimal_pcm16_wav(samples: &[u8], rate: u32, channels: u16) -> Arc<[u8]> {
    minimal_pcm_wav(1, channels, rate, 16, samples)
}
```
What is wrong: the wrapper puts `samples` FIRST while the wrapped builder
takes it LAST (`minimal_pcm_wav(fmt, channels, rate, bits, samples)`).
Distinct types stop the worst misuse from compiling, but the mirror-image
order is a reading trap in test code that mixes both calls. What correct
looks like: `minimal_pcm16_wav(rate, channels, samples)` to mirror the
parent, or drop the wrapper.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/src/wav/test_support.rs:28,39,44,46
Code:
```
    let block_align = channels * (bits / 8);
    ...
    let avg = rate * u32::from(block_align);
    ...
    b.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    ...
    let riff_size = (b.len() - 8) as u32;
```
What is wrong: four unchecked operations — `bits/8` floors to 0 for bits<8
(block_align 0), `rate × block_align` can overflow u32, `samples.len() as
u32` truncates above 4 GiB, and `as u32` on b.len() likewise. All are
acceptable for a test tool whose callers pass sane values (the module doc
explicitly disclaims validation), but the file is `pub` and compiled into
the RELEASE crate (not cfg(test)), so the unchecked surface ships and any
future in-crate misuse would inherit silent truncation. What correct looks
like: debug_assert! the invariants (bits % 8 == 0, len < u32::MAX) — they
fire in tests, cost nothing in release.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| minimal_pcm_wav | 21-49 | REAL | S4 (F-002) | correct minimal layout; verified by passing doctest |
| minimal_pcm16_wav | 52-55 | REAL | S4 (F-001) | arg order mirrors nothing |
| mem_stream | 64-68 | REAL | — | pure delegation to avi::test_support (verified :121-124) |

## 5. Test honesty
No tests in this file (by design — the builders are the test surface).
Their output is validated by wav/tests.rs, wav/fuzz.rs and the doctest
(baseline green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Public like crate::avi::test_support on purpose: rustdoc examples cannot see #[cfg(test)] items" | test_support.rs:3-5 | REAL | doctest at wav/mod.rs:223-236 consumes it and passes in baseline |
| "nothing in the demuxer itself uses it" | test_support.rs:6-7 | REAL | rg: mod.rs references are doc-example lines only |
| "the AVI module's implementation, reused verbatim: one SourceStream, not two" | test_support.rs:58-59 | REAL | avi/test_support.rs:121-124 read; delegation is literal |

## 7. Dead / duplicated / unreachable code
None. All three functions consumed (tests.rs, fuzz.rs, doctests).

## 8. Dependencies & flags
plugin_api::SourceStream, std Arc. No unsafe, no features. Compiled
unconditionally (test-support-in-release, the documented pattern).

## 9. Open questions for the next agent
- wav/mod.rs and wav/tests.rs (not assigned to me): the doctest asserts
  `p.keyframe` for a PCM packet — confirm the keyframe semantics for audio
  are intentional crate-wide when auditing the packet layer.
