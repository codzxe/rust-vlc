# AUDIT: crates/demux/src/es/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 78 / 6 (mint + 5 pub constructors) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:63-64,66 (`"hevc"/"h264" => es::open_annexb`, `"mp3" => es::open_mpeg_audio`) → es/annexb.rs + es/mp3.rs, which call these constructors at 12 sites (annexb.rs:172,412,448,451,487,500; mp3.rs:255,346,366,373,541,558,570). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg` callers (all 5 constructors have production call sites — no dead refusals here, unlike asf); `rg` es/tests.rs + baseline for per-code coverage (no-sync/not-started/truncated covered; bad-header/au-too-large ZERO); sed of media-types/error.rs:311-324 (display-side 16-byte sniff cap). |

## 1. One-paragraph verdict
Clean, small, real: five typed constructors, all reachable from the shipped
open path, consistent with the adts/asf error pattern (not_started is
Unsupported everywhere — house taxonomy). Two real defects: `no_sync` embeds
up to 64 KiB of stream payload into the error object, contradicting this
file's own contract line ("carry byte counts/offsets only — never paths or
payload content") and inconsistently with its sibling caller (mp3 caps at 16
bytes); and two of the five codes (`demux.es.bad-header`,
`demux.es.au-too-large`) have zero test coverage anywhere — the latter being
the MP §12 memory-ceiling refusal, the exact class of bound a hostile
auditor wants proven.

## 2. What actually works (verified)
- `rg` call sites: every constructor is called from production code — no
  dead minted refusals (contrast: asf's encrypted_stream).
- Baseline cargo-test-workspace.txt:1897,1900,1901 — `no-sync`,
  `not-started`, `truncated` codes are asserted by es tests and pass.
- read: taxonomy mapping is sane (Truncated/InvalidStructure for corrupt,
  ResourceLimit{Memory} for the AU ceiling, Unsupported for no-sync).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/es/error.rs:3-5 vs 20-30 (caller annexb.rs:405-412)
Code:
```
//! media-types' (never redefined here); codes are `demux.es`-scoped,
//! stable, and carry byte counts/offsets only — never paths or
//! payload content.
```
```
pub fn no_sync(format: &'static str, sniffed: &[u8]) -> EngineError {
```
What is wrong: the contract line is violated by this file's own constructor:
the annexb caller passes the whole open-scan window — up to
`MAX_OPEN_SCAN_BYTES` = 64 KiB (annexb.rs:84, 408-412:
`read_prefix_at(MAX_OPEN_SCAN_BYTES.min(len) ...) ... no_sync(..., &win)`) —
and `sniffed.to_vec()` copies it into the EngineError. The mp3 caller caps
at 16 bytes (mp3.rs:412-413). Display is capped at 16 bytes downstream
(media-types/error.rs:321-324), so nothing user-visible leaks, but the
in-memory error carries 64 KiB of payload content the module doc promises it
never carries, and every refused hostile open pays a needless 64 KiB copy.
What correct looks like: cap at construction (`&sniffed[..min(16)]`) and fix
the doc or the code — whichever the contract meant.
Confidence: HIGH — Verification: read + rg (both callers).

F-002 [S3]
Where: crates/demux/src/es/error.rs:56-65, 69-78 (absence of tests)
Code:
```
pub fn au_too_large(attempted: u64) -> EngineError {
    mint(
        ErrorKind::ResourceLimit {
            limit: LimitKind::Memory,
            budget: super::annexb::MAX_AU_BYTES,
```
What is wrong: `rg bad-header|au-too-large crates/demux/src/es/tests.rs` →
zero matches; no test anywhere asserts either code (baseline test list
confirms). `au_too_large` is the 16 MiB access-unit memory ceiling — the
MP §12 bound that protects against unbounded allocation from a hostile
Annex-B stream. Whether a >16 MiB AU actually refuses typed (rather than
allocating, panicking, or misbehaving) is UNVERIFIED — no test drives it, no
committed fixture is that large, and the fuzz sweeps use small inputs.
`bad_header` likewise fires at 3 production sites with no pinned outcome.
Confidence: HIGH (absence verified) — the paths' behavior: UNVERIFIED.
What correct looks like: a builder-forged AU over the ceiling asserting the
typed refusal + sticky shape (the truncation-test pattern at tests.rs:234).

F-003 [S4]
Where: crates/demux/src/es/error.rs:69-71 (callers annexb.rs:172, mp3.rs:255)
Code:
```
pub fn bad_header(offset: u64) -> EngineError {
```
What is wrong: the parameter promises an offset, but two of three call
sites pass a literal `0` (`CodecDescriptor::new(name).map_err(|_|
super::error::bad_header(0))`) — a synthesized offset for a failure that has
no position. The error context then reports offset 0, which points at the
file start, misattributing the failure site.
Confidence: HIGH — Verification: read + rg.
Severity note: S4 because the CodecDescriptor rejection is practically
unreachable (literal names), but the context field is still fabricated.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 12-14 | REAL | — | |
| no_sync | 20-30 | REAL | S3 | F-001 payload-content carry, caller inconsistency |
| not_started | 33-38 | REAL | — | house-consistent kind |
| truncated | 42-52 | REAL | — | tested (tests.rs:241,253) |
| au_too_large | 56-65 | REAL-UNPROVEN | S3 | F-002 no coverage; ceiling behavior unverified |
| bad_header | 69-78 | REAL-UNPROVEN | S4 | F-002/F-003 no coverage, fake offset 0 |

## 5. Test honesty
No tests in this file. Consumers' coverage (es/tests.rs, baseline 1893-1901):
no-sync ✓, not-started ✓, truncated ✓, bad-header ✗, au-too-large ✗.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "codes ... carry byte counts/offsets only — never paths or payload content" | error.rs:3-5 | FALSE | no_sync embeds up to 64 KiB payload (F-001) |
| taxonomy "never redefined here" | error.rs:2-3 | REAL | all kinds from media_types::error |

## 7. Dead / duplicated / unreachable code
None — every constructor called. mint is the only private helper.

## 8. Dependencies & flags
media-types error taxonomy only. No feature gates. No unsafe.

## 9. Open questions for the next agent
- Does a >16 MiB Annex-B access unit actually hit the ceiling refusal, or
  does the PayloadSlots pool (annexb.rs:378, `MAX_AU_BYTES.min(u32::MAX)`)
  grow first? The pool is preallocated at 2×16 MiB per demuxer — check the
  allocation happens lazily enough that a tiny hostile file can't force it.
- es/fuzz.rs's `fuzz_lite_mutations_never_panic_or_hang` (baseline 1895)
  — what input sizes? If < 16 MiB, the ceiling is unreachable there too.
