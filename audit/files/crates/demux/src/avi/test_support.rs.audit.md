# AUDIT: crates/demux/src/avi/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 169 / 4 prod fns + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | Compiled into every build (`pub mod test_support;` avi/mod.rs:121) but no shipped player path calls it; used by the in-crate fuzz harness (fuzz.rs:53), the mod.rs doctest (:215), and avi/tests.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 byte reads; rg caller inventory (read/read_into callers across crates/demux/src); baseline citation :1827-1829 |

## 1. One-paragraph verdict
A real in-memory SourceStream with correct file-like semantics (clean
sticky EOF, range-checked seeks with failed-seek-leaves-position, i128
arithmetic for negative Current seeks) and a zero-alloc `read_into`
override. Its three unit tests are strict and pass in baseline. One
genuine robustness hole: `read()` computes `self.pos + len as u64`
unchecked — a `len` near `usize::MAX` with `pos > 0` overflows (debug
panic) or wraps to `end < start` (release slice panic at `data[start..end]`).
No file-input path can reach it (verified: every demuxer read goes through
`read_into` with READ_STEP_MAX-bounded steps; only tests call `read()`),
so it is an API-robustness defect on public test-support surface, not a
player bug — the sibling media-probe `CountingSource::read` handles the
same case safely.

## 2. What actually works (verified)
- All 3 tests pass: baseline cargo-test-workspace.txt:1827-1829
  (`avi::test_support::tests::seek_bounds ... ok` etc.).
- Caller inventory (rg over crates/demux/src): zero parsing-path callers —
  only fuzz.rs:53, the mod.rs doctest :215, and test files; all demuxer
  I/O flows through `SourceReader::read_into` (riff.rs:272,329,362;
  mkv/ebml.rs:317) with 64 KiB steps — so the F-001 hole is unreachable
  from container bytes.
- Seek semantics re-traced: `i128` Current math, out-of-range typed error
  before any mutation (position truly unchanged — asserted by the test at
  :154).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/avi/test_support.rs:60-66
Code:
```
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let end = (self.pos + len as u64).min(self.data.len() as u64);
        let start = (self.pos as usize).min(self.data.len());
        let out = self.data[start..end as usize].to_vec();
```
What is wrong: `self.pos + len as u64` is an unchecked add on a
caller-controlled `usize`. With `pos > 0` and `len > u64::MAX − pos`
(e.g. `pos = 1, len = usize::MAX` on 64-bit): debug builds panic on
overflow; release builds wrap so `end < start` and
`self.data[start..end as usize]` panics (start > end). The trait contract
(plugin-api/src/access.rs:42-51 "Read up to `len` bytes") sets no ceiling
on `len`, and the sibling implementation
media-probe/src/contract.rs:171-174 (`take = want.min(available)`) is safe
for every `len`. Why it matters: public API of the demux crate (compiled
into release) that panics on a legal argument; unreachable from file
input today only because every in-repo caller is bounded. What correct
looks like: `let end = self.pos.saturating_add(len as u64).min(len_u64)`
(or the CountingSource shape). Confidence: HIGH — Verification: read +
caller inventory; not executed (no in-repo caller passes huge len —
crafting one is out of READ-ONLY scope).
F-002 [S4]
Where: crates/demux/src/avi/test_support.rs:8-9
Code:
```
//! pipeline tests will want the same. It is test-support surface, not
//! player surface — nothing in the demuxer itself uses it.
```
What is wrong: two in-crate consumers use it — the fuzz harness
(fuzz.rs:53 `test_support::mem_stream`) and the crate's own doctest
(mod.rs:215 `demux::avi::test_support::mem_stream(bytes)?`). The claim
holds only for the parsing path; as written it is false. Confidence:
HIGH — Verification: read (rg citations).
F-003 [S4]
Where: crates/demux/src/avi/mod.rs:121 (affecting this file)
What is wrong: same as fuzz.rs F-002 — test-support surface compiled
unconditionally into the release lib with no shipped caller; deliberate
per the module doc, but binary mass. Confidence: HIGH — Verification:
read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BytesStream::new | 36-39 | REAL | — | Result shape honest; literal URI |
| BytesStream::bytes | 42-44 | REAL | — | assertion accessor |
| SourceStream::read | 60-66 | REAL | S3 | F-001 unchecked add / wrap → slice panic |
| SourceStream::read_into | 72-79 | REAL | — | zero-alloc copy; same add but out.len() is caller-bounded in practice; `end >= start` holds for in-range buffers |
| SourceStream::seek | 81-98 | REAL | — | i128 math, typed out-of-range, position untouched on failure |
| SourceStream::position/uri/size/supports_seek | 48-58, 100-102 | REAL | — | honest |
| out_of_range | 106-114 | REAL | — | typed error, stable code |
| mem_stream | 121-124 | REAL | — | the doctest/fuzz one-liner |
| 3 tests | 137-168 | REAL | — | strict (EOF stickiness, seek bounds + failed-seek position, size/uri honesty) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| reads_advance_and_clean_eof_is_sticky | YES | exact bytes + repeated empty reads + pinned position |
| seek_bounds | YES | boundary 5/6, negative Current, failed-seek position assert |
| size_and_uri_are_honest | YES | scheme "memory", Some(12) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "reads at the end return empty (clean EOF), seeks land anywhere within the byte range, seeks past the end are typed out-of-range errors and leave the position unchanged" | :19-22 | REAL | implementation + tests :138-160 |
| "nothing in the demuxer itself uses it" | :9 | FALSE (as written) | F-002: fuzz.rs:53 + mod.rs doctest use it |
| "a direct copy out of the backing `Arc<[u8]>` … zero allocation" | :68-71 | REAL | read_into :72-79 has no allocation; the zero-alloc proofs suite passes (baseline :2442-2452 zero_alloc_read_chain 10/10) |

## 7. Dead / duplicated / unreachable code
None dead in-file. Shipped-but-uncalled-by-player status noted (F-003).

## 8. Dependencies & flags
std-only; media-types/plugin-api. No cfg gate (deliberate, documented).
No unsafe. The `lumen-allow-literal` annotation on the synthetic URI is a
lint-registry marker, verified present.

## 9. Open questions for the next agent
- crates/demux/src/wav/test_support.rs and other per-format test_support
  twins: same `read()` arithmetic? A one-line workspace-wide rg for
  `self.pos + len as u64` would catch the family.
