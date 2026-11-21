# AUDIT: crates/demux/src/avi/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 169 / 8 (2 types/fns + 6 trait methods + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — by design (test-support). All non-test callers verified to be `#[test]` fns: apps/cli/src/demux_host.rs:739, containers/mod.rs:123, containers/mp4.rs:637 are test modules; doctests (avi/mod.rs:215), fuzz targets, and sibling test_support modules (asf, wav) are the consumers. The module IS compiled into every non-test build (`pub mod test_support`, avi/mod.rs:121 — not cfg-gated). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg `mem_stream` across crates/ apps/ fuzz/ tests/ (callers enumerated; all shipped-code hits are `#[test]` fns) · rg `BytesStream` (used by asf/wav test_support + avi/tests.rs) · rg `\.bytes\(\)` (the accessor has zero callers) · sed -n verification of quotes |

## 1. One-paragraph verdict
Clean, small, honest test-support: an in-memory seekable `SourceStream` whose arithmetic I traced end-to-end (no panic path: `pos` is provably ≤ `data.len()` at every entry — seek rejects targets past the end and leaves the position pinned, reads clamp; `read_into` is the zero-alloc twin the fuzz/zero-alloc lanes rely on), with three mutation-sensitive in-file tests plus the doctest and 12+ test files that consume it, all executed green in the baseline. The doc's "test-support surface, not player surface" claim is verified — every non-test-module caller is inside `#[test]` functions. Two hygiene findings only: the `bytes()` accessor is dead, and the module ships in release builds by design tradeoff (documented, but it is dead weight in the shipped artifact).

## 2. What actually works (verified)
- Executed: baseline log 1614-1616 — `avi::test_support::tests::reads_advance_and_clean_eof_is_sticky`, `seek_bounds`, `size_and_uri_are_honest` all ok; the avi doctest that motivates the module's publicity (`crates/demux/src/avi/mod.rs - avi::AviDemuxer (line 208)`) ok at log 6374; the fuzz registry that consumes it passed (baseline INDEX #18: 35 targets, diff empty); `zero_alloc_read_chain.rs` and `tests/es_corpus.rs` use `mem_stream`.
- `read_into` zero-alloc claim: direct `copy_from_slice` out of the `Arc<[u8]>` — no allocation (read, verified; the zero_alloc test file exists and ran in the demux suite).
- Seek semantics: failed seek leaves position unchanged (pinned by `seek_bounds`: seek(Start(6)) on 5 bytes errs, position still 5).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/avi/test_support.rs:41-44
Code:
```
    /// The bytes behind the stream (assertions in tests).
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
```
What is wrong: Zero callers. rg `\.bytes\(\)` across the repo: the only hits are unrelated types (`Bits::bytes` in adts/tests.rs:478, iterator `.bytes()` in vorbis comments.rs). Nothing asserts through this accessor despite the doc comment's stated purpose ("assertions in tests").
Why it matters: Dead public API on a public-by-design module — surface that must be maintained and that implies a test pattern nobody uses.
What correct looks like: Delete it, or use it in an assertion as documented.
Confidence: HIGH — Verification: rg.

### F-002 [S4]
Where: crates/demux/src/avi/mod.rs:121 + crates/demux/src/avi/test_support.rs:4-9
Code:
```
//! This module is **public on purpose** (the `input` crate's
//! `SyntheticStream` precedent): rustdoc examples cannot see
//! `#[cfg(test)]` items, the cargo-fuzz target (a separate crate, doc
//! 04 §8.5) needs a `SourceStream` over raw bytes, and the engine's
//! pipeline tests will want the same. It is test-support surface, not
//! player surface — nothing in the demuxer itself uses it.
```
What is wrong: The justification is real (doctests, the separate fuzz crate, and engine tests genuinely cannot see cfg(test) items) — but the consequence is that this module plus its `SourceUri` literal and error machinery compile into every release build of the `demux` crate while being unreachable at runtime there (verified: all shipped-code callers are `#[test]` fns). The same pattern is then replicated per-family (asf/test_support.rs, wav/test_support.rs wrap this one).
Why it matters: Dead code in the shipped artifact; a small attack/maintenance surface (a `memory://` URI parser entry, an out_of_range error constructor) that exists only for tests.
What correct looks like: A `#[doc(hidden)]` + `#[cfg(any(test, doc, feature = "testkit"))]`-style gate (the repo already has a `testkit` crate for exactly this), or consolidating the per-family test_support wrappers on the existing testkit crate.
Confidence: HIGH — Verification: read + rg; the tradeoff is documented in the file, hence S4 hygiene rather than a defect.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BytesStream::new` | 36-39 | REAL | — | Fixed URI parse, never fails (Result shape documented); inline `lumen-allow-literal` marker present (lint-literals gate passed except the known host.rs:1799 site, baseline #13) |
| `BytesStream::bytes` | 42-44 | DEAD | S4 | F-001 |
| `SourceStream::uri` / `size` / `supports_seek` | 48-58 | REAL | — | Honest size (Some(len)) — the demuxer's length-known precondition |
| `SourceStream::read` | 60-66 | REAL | — | Clamp arithmetic traced: `pos ≤ len` invariant holds at every entry; EOF → empty Vec, sticky (test-pinned) |
| `SourceStream::read_into` | 72-79 | REAL | — | Zero-alloc twin; `out[..got]` length matches source slice exactly (traced); EOF → Ok(0) |
| `SourceStream::seek` | 81-98 | REAL | — | i128 math for Current(delta); rejects <0 and >u64::MAX before the u64 cast; past-end → typed error, position unchanged (test-pinned). Note: real POSIX files ALLOW seek-past-EOF — this stream is stricter; the doc documents the actual behavior, so no misleading claim |
| `SourceStream::position` | 100-102 | REAL | — | Trivial |
| `out_of_range` | 106-114 | REAL | — | Typed Inaccessible/NotFound at stage Open, named context code |
| `mem_stream` | 121-124 | REAL | — | The doctest/fuzz one-liner; consumed by 12+ test files + 3 CLI test modules |

## 5. Test honesty (in-file `mod tests`; executed ok in baseline, log 1614-1616)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `reads_advance_and_clean_eof_is_sticky` (138-147) | YES | Read advance, short-final-read clamp, repeated empty EOF reads, position pinned at 10 — breaks on any read-arithmetic mutation |
| `seek_bounds` (150-160) | YES | In-range ok / past-end err / failed-seek-leaves-position / negative Current underflow err / read-after-seek-back — breaks the seek contract |
| `size_and_uri_are_honest` (163-168) | YES | Some(12), supports_seek, scheme "memory" |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Positional semantics mirror a regular file: reads at the end return empty (clean EOF), seeks land anywhere within the byte range, seeks past the end are typed out-of-range errors" | test_support.rs:19-22 | REAL (with one nuance) | Tests pin all three; nuance: POSIX files allow seek-past-EOF — this stream is stricter, but the doc states the stricter behavior explicitly, so nothing is overstated |
| "It is test-support surface, not player surface — nothing in the demuxer itself uses it" | test_support.rs:8-9 | REAL | rg: all non-test callers are `#[test]` fns; the demuxer itself never imports it |
| "read_into … zero allocation" | test_support.rs:68-71 | REAL | Direct copy_from_slice; consumed by `tests/zero_alloc_read_chain.rs` (in the 462-test green suite) |

## 7. Dead / duplicated / unreachable code
- `bytes()` — dead (F-001).
- Per-family wrappers duplicate the one-liner (asf/test_support.rs, wav/test_support.rs both just re-export this module's constructor — acceptable indirection, noted).
- Module unreachable at runtime in shipped builds (F-002 context).

## 8. Dependencies & flags
No feature gates, no unsafe, no external deps beyond `media_types`/`plugin_api`. The `Arc<[u8]>` sharing makes `Clone` cheap for the multi-consumer test pattern (demux_host.rs:739's `started` closure clones per call).

## 9. Open questions for the next agent
- The testkit crate exists for shared test infrastructure — why did demux grow its own per-family test_support instead? (Layering: testkit may not depend on plugin_api's SourceStream; whoever audits testkit should confirm.)
- If a wave adds a `testkit` feature gate, this module and its asf/wav twins are the migration candidates (F-002).
