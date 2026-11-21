# AUDIT: crates/demux/src/wav/test_support.rs (lines 1-68, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 68 / 3 items (`minimal_pcm_wav`, `minimal_pcm16_wav`, `mem_stream`) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test-support surface, doc's own claim verified: no production imports); its consumers run in the baseline: unit tests (34 wav::tests ok), fuzz sweeps (`wav::fuzz::tests::*` ok, nofailfast:1922-1926) and the doctest that justifies its pub-ness (`crates/demux/src/wav/mod.rs - wav::WavDemuxer (line 216) ... ok`, nofailfast:6372) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full Read; byte-layout arithmetic of the 44-byte header hand-verified; `grep -rn "minimal_pcm_wav\|minimal_pcm16_wav"` → exactly 3 consumer files (wav/tests.rs ×9, wav/fuzz.rs ×4, wav/mod.rs doctest ×1 — **zero integration suites**); `grep -n` production files (mod.rs/format.rs) for test_support imports → doc-comment only; `sed -n` quote verification; baseline doctest grep |

## 1. One-paragraph verdict
A 68-line test builder that does exactly what it says: the 44-byte minimal-PCM layout is byte-correct (RIFF size patched honestly, WAVEFORMATEX field order right, block-align/avg-rate arithmetic right), the `mem_stream` delegate genuinely reuses the AVI module's single implementation (verified: the ogg and aiff twins delegate to the same seam), and the "nothing in the demuxer itself uses it" claim is grep-true. The single defect is one false consumer class in the pub-ness justification: the header claims "the integration suites … need" these builders, but no integration suite imports them — the actual consumers are the in-crate tests, the fuzz sweeps and one doctest.

## 2. What actually works (verified)
- `minimal_pcm_wav` (21-49) byte layout verified against RIFF/WAVEFORMATEX: RIFF 0-3, size 4-7 (patched to actual), WAVE 8-11, "fmt " 12-15, size 16, tag 20-21, channels 22-23, rate 24-27, avg = rate×block_align 28-31, align 32-33, bits 34-35, "data" 36-39, size 40-43 — this is the ground truth I used to convict the wav/fuzz.rs offset bug (see my aiff/fuzz.rs audit F-001 cross-ref).
- The honest-header discipline: RIFF size is patched to the real length (46-47), so builder output is structurally self-consistent (malformedness is injected only by callers patching bytes — the right pattern).
- `mem_stream` (64-68) delegates to `crate::avi::test_support::mem_stream` — "one SourceStream, not two" verified (ogg/test_support.rs:22 and aiff/test_support.rs:57 delegate to the same function).
- Doctest consumer executed in baseline: `wav::WavDemuxer (line 216) ... ok` (nofailfast:6372) — the pub-ness rationale (rustdoc cannot see cfg(test) items) is real and exercised.
- `#[must_use]` on both builders; no unsafe; no feature gates.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/wav/test_support.rs:3-5
Code:
```rust
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples cannot see `#[cfg(test)]` items, and the integration
//! suites + fuzz harness need deterministic malformed shapes. This is
```
What is wrong: the consumer list overstates. Repo-wide grep for `minimal_pcm_wav|minimal_pcm16_wav` finds exactly three consumer files: `wav/tests.rs` (9 uses), `wav/fuzz.rs` (4), and the `wav/mod.rs` doctest (1). No integration suite (`crates/demux/tests/*`) imports these builders — the integration layer uses its own committed fixtures (`wav_aiff_fixtures.rs` is fixture-file based).
Why it matters: the pub-ness of a test-support surface is an API-surface decision whose justification should name its real consumers; a false "integration suites need it" claim will survive long after the actual rationale (doctests + fuzz) is what matters for review.
What correct looks like: "rustdoc examples cannot see `#[cfg(test)]` items, and the fuzz harness needs deterministic malformed shapes".
Confidence: HIGH — Verification: repo-wide grep (consumer list exhaustive); fixture-based integration suites confirmed by the module docs of `wav_aiff_fixtures.rs` and the corpus suites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `minimal_pcm_wav` | 21-49 | REAL | — | layout byte-verified; u16/u32 multiply overflow only at absurd args (test tool, doc disclaims validation); odd-length `samples` produce unpadded data chunks — documented "no padding" (14), callers pad when needed |
| `minimal_pcm16_wav` | 53-55 | REAL | — | thin delegate for the doctest's tag-1 case |
| `mem_stream` | 64-68 | REAL | — | delegate to the single AVI seam; claim verified |

## 5. Test honesty
No tests live in this file. Its correctness is consumed by: 34 wav unit tests (baseline ok), 4 wav fuzz-sweep tests (baseline ok), 1 doctest (baseline ok) — all executed green in `orchrun-test-workspace-nofailfast.log.txt`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "rustdoc examples cannot see `#[cfg(test)]` items" (rationale) | test_support.rs:3-4 | REAL | doctest at mod.rs:223-235 imports it; ran ok in baseline (:6372) |
| "the integration suites + fuzz harness need deterministic malformed shapes" | test_support.rs:4-5 | SHALLOW (integration half false) | grep: zero integration consumers; fuzz + unit tests + doctest only (F-001) |
| "nothing in the demuxer itself uses it" | test_support.rs:5-7 | REAL | grep of production files: doc-comment references only |
| "`samples` must be whole sample frames … the builder is a test tool, not a validator" | test_support.rs:17-19 | REAL | builder performs no validation — verified by reading; tests that need slivers build them by hand |

## 7. Dead / duplicated / unreachable code
- None: both builders and `mem_stream` have live consumers (grep-exhaustive, §2).

## 8. Dependencies & flags
- `pub` module compiled into the lib unconditionally (not cfg(test)) — the deliberate doctest/fuzz seam per its header; cost is ~68 lines of builder in release binaries, the documented trade for rustdoc visibility. No new deps; `plugin_api::SourceStream` + `std::sync::Arc` only.

## 9. Open questions for the next agent
- None for this file. Cross-ref stands: wav/fuzz.rs `(28, …)` data-size patch bug (A07's file) was proven using this builder's verified layout.
