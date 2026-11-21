# AUDIT: crates/demux/src/ivf/error.rs (215 lines, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 215 / 14 items (mint + 10 constructors + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (same chain as ivf/tests.rs: `apps/cli/src/containers/mod.rs:55` `"ivf" => ivf::open(...)` → IvfDemuxer; every constructor is minted on a live error path traced in the mod.rs/tests.rs audit) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | read whole file; grep nofailfast log → `ivf::error::tests::*` 3/3 ok (lines 1691-1693); grep `fn mint` across demux error modules (wav:23, es:12, ogg:19 — duplication); cross-checked seek_out_of_range's doc against scan_landing (ivf/mod.rs:520-526) and the negative-target test (ivf/tests.rs:396-406) |

## 1. One-paragraph verdict
A clean, small typed-error factory: ten constructors, one shared `mint`, stable
`demux.ivf.*` codes, stage `Demux` everywhere, no secrets (byte counts/offsets/bare
fourcc only), and an exhaustive table test that pins every constructor's code and Display
shape — all three module tests green in the nofailfast baseline. The two honestly-recorded
ffmpeg divergences (header-only refusal vs clean empty stream; frame-beyond-file refusal
vs short packet) are documented at the exact constructors that mint them and pinned by the
walk tests audited in ivf/tests.rs. The residue is three S4s: `no_frames()` fabricates a
`Truncated{expected:1, found:0}` shape for a file that is not truncated, the
`seek_out_of_range` doc names a "target before the first frame" case that the
implementation clamps instead (contradicting the module's own tests), and the `mint`
helper is copy-pasted through at least four sibling error modules.

## 2. What actually works (verified)
— 3/3 module tests ok in baseline (log 1691-1693): `errors_carry_demux_stage_and_ivf_codes`
(10/10 constructors, code + `[demux] ` Display prefix + ` (code)` suffix),
`kinds_map_to_the_documented_taxonomy`, `not_ivf_sniff_capped_by_display` (64×0xC3 →
≤16 "c3" matches: the Display cap asserted).
— No-secrets posture verified by construction: every constructor's payload is a count,
offset or 4-byte tag (read all ten).
— Divergences recorded where they belong: `no_frames` (83-86) and `frame_beyond_file`
(99-102) both name the ffmpeg behavior and the chosen divergence; both pinned by
`start_refuses_every_malformed_shape_typed` and `walk_refuses_frame_beyond_file_sticky`
(baseline ok, ivf/tests.rs audit).

## 3. Findings

### F-001 [S4] `no_frames()` mints a fabricated Truncated shape for a complete file
Where: crates/demux/src/ivf/error.rs:87-96
Code:
```
pub fn no_frames() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 1,
                found: 0,
            },
        },
        "demux.ivf.no-frames",
    )
}
```
What is wrong: a header-only IVF file is complete — nothing was truncated. The
constructor nevertheless reports `Truncated { expected: 1, found: 0 }`, inventing a
1-byte expectation that exists nowhere in the format. The code string is honest; the
kind/detail is a taxonomy misuse that surfaces to any consumer switching on
`CorruptDetail::Truncated` (it will classify a valid empty file as byte-truncation).
What correct looks like: `CorruptDetail::InvalidStructure { offset: Some(32) }` or a
dedicated detail variant; keep the code.
Confidence: HIGH — read.

### F-002 [S4] `seek_out_of_range`'s doc names a case the implementation clamps instead
Where: crates/demux/src/ivf/error.rs:147-154 (doc) vs ivf/mod.rs:520-526 and ivf/tests.rs:396-406
Code:
```
/// A seek the container cannot map (target before the first frame, or
/// past the last frame header the bounded scan can reach).
pub fn seek_out_of_range() -> EngineError {
```
What is wrong: a target before the first frame does NOT produce this error —
`scan_landing` clamps to the first frame (`landing.or(first)`, ivf/mod.rs:526) and the
suite pins it: "Before the stream start: clamps to the first frame … assert_eq!(ts.ticks,
0)" (ivf/tests.rs:396-406, baseline ok). The error actually fires for: no frame header at
all (`seek_out_of_range_only_when_no_frame_exists`, tests.rs:451-468) or the scan budget
exhausted (scan_landing frames > MAX_SEEK_SCAN_FRAMES). The doc's first case is wrong.
What correct looks like: "(no frame header exists to land on, or the bounded scan's
frame budget is exhausted)".
Confidence: HIGH — both sides read and cross-checked with the passing tests.

### F-003 [S4] `mint` duplicated verbatim across the demux crate's error modules
Where: crates/demux/src/ivf/error.rs:20-22 (wav/error.rs:23, es/error.rs:12, ogg/error.rs:19, + crate-root error.rs and adts/asf modules)
Code:
```
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}
```
What is wrong: the same three-line helper (modulo the STAGE constant) is copy-pasted in
every demuxer's error module — five-plus instances counted by grep. The module doc
defers it honestly ("the crate-root `error.rs` is the AVI card's file — scope
discipline, R3 consolidates when the registry merges", lines 5-7), but until R3 the
code+stage minting convention must be fixed in N places.
Why it matters: a change to error minting (e.g., adding a required context field) becomes
an N-file sweep; one module drifting is invisible until a table test fails.
What correct looks like: one `demux::error::mint` (or a macro) parameterized by the
scope prefix, adopted by all modules in the same PR.
Confidence: HIGH — grep counted the duplicates.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` | 20-22 | REAL | S4 | duplication F-003 |
| `not_ivf` | 27-37 | REAL | — | Unsupported + FormatInfo sniffed; Display cap tested |
| `truncated` | 41-48 | REAL | — | honest expected/found |
| `bad_header` | 55-64 | REAL | — | doc cites ffmpeg "Invalid frame rate" parity |
| `unsupported_fourcc` | 71-81 | REAL | — | fourcc in `sniffed` (documented reuse); scope refusal AV1/VP9 honest |
| `no_frames` | 87-97 | REAL | S4 | fabricated Truncated shape (F-001); divergence honestly documented |
| `frame_beyond_file` | 103-110 | REAL | — | divergence documented; sticky pinned in tests.rs audit |
| `frame_too_large` | 115-124 | REAL | — | ResourceLimit/Memory with budget+attempted — the MP §12 refusal shape |
| `pts_overflow` | 128-137 | REAL | — | InvalidStructure with offset |
| `not_started` | 140-145 | REAL | — | Unsupported{format: None} — same shape across demuxers |
| `seek_out_of_range` | 149-154 | REAL | S4 | doc/code contradiction (F-002) |
| `errors_carry_demux_stage_and_ivf_codes` | 160-182 | REAL | — | exhaustive 10/10 table + Display shape |
| `not_ivf_sniff_capped_by_display` | 184-189 | REAL | — | proves the 16-byte cap |
| `kinds_map_to_the_documented_taxonomy` | 191-214 | REAL | — | kind-shape spot checks incl. full detail equality |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_ivf_codes | YES | every constructor, code and Display prefix/suffix asserted |
| not_ivf_sniff_capped_by_display | YES | 64 sniffed bytes → ≤16 rendered |
| kinds_map_to_the_documented_taxonomy | YES | structural equality on kinds (would catch a kind swap) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The sniffed bytes … are capped by the taxonomy's own Display (16)" | error.rs:12-13 | REAL | tested (184-189), baseline ok |
| "ffmpeg's own demuxer opens it and yields a clean empty stream — the honestly-recorded divergence" | error.rs:84-86 | REAL (divergence recorded) | pinned as `demux.ivf.no-frames` in tests (baseline ok); ffmpeg behavior itself UNVERIFIED in-sandbox (no ffmpeg run permitted — recorded as the crate's corpus-derived claim) |
| "target before the first frame" produces seek_out_of_range | error.rs:147-148 | FALSE | implementation clamps (scan_landing `landing.or(first)`; tests.rs:396-406 asserts Ok landing tick 0) — F-002 |
| "R3 consolidates when the registry merges" (mint duplication deferral) | error.rs:5-7 | REAL (deferred) | 5+ copies counted — F-003 |

## 7. Dead / duplicated / unreachable code
`mint` ×5+ (F-003). Nothing dead; all constructors have call sites (traced in the
mod.rs/tests.rs audit of this wave).

## 8. Dependencies & flags
media_types only; no unsafe; no feature gates; the cfg(test) block is the only test
surface (integration via the walk tests).

## 9. Open questions for the next agent
1. If R3 (registry merge) ever lands, the F-003 consolidation should also revisit
   `no_frames`'s kind (F-001) — a dedicated "empty container" detail variant would serve
   avi (no-movi) and adif (no-data) too.
2. `unsupported_fourcc` reusing `FormatInfo.sniffed` for the fourcc — check whether any
   UI/log consumer renders sniffed bytes as a prefix hint (it would print a 4-byte
   binary tag where a magic was expected); cosmetic but user-visible.
