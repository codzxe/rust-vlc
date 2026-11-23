# AUDIT: crates/playlists/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 86 / 0 fn (2 enums, 7 variants) |
| File verdict | REAL — transitively unreachable from shipped entry points |
| Reachable from shipped entry point? | NO — only `crates/streaming` depends on playlists, and streaming's playlist-consuming modules (hls, remote_playlist, dash) are not wired into the shipped CLI (which uses only RangedResource + UreqClient) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | for-loop rg over all manifests (exit 0: only streaming); rg ParseError/ParseWarning usage exit 0; sed -n verifications exit 0; baseline citation |

## 1. One-paragraph verdict
A clean, small typed error/warning vocabulary that does exactly what its redaction
rule promises: every `Display` string carries only the kind and a 1-based line number
— no line text, URLs, or paths (all seven variant strings checked by hand). The fatal
pair (LineTooLong with the 64 KiB cap interpolated, InvalidUtf8) is genuinely fatal in
the parser (m3u.rs:157-161), and the five warnings are genuinely non-fatal
skip-and-continue shapes (m3u.rs:180-204 pushes them while parsing continues). No
defects in the file itself. The context defect: the only consumer chain
(playlists → streaming → CLI) dead-ends — the CLI never reaches the playlist parsers,
so this vocabulary is unreachable from every shipped binary.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt L4621-4803 (PACKAGE playlists
exit_code=0): 46 unit + 20 + 20 + 18 + 14 + 14 + 2 integration tests all `ok` — the
parser suites exercising these types.
— verified by read: m3u.rs:157-161 raises LineTooLong at the 64 KiB cap
(MAX_LINE_BYTES, m3u.rs:100); warnings at m3u.rs:180/189/204 are pushed, not thrown.
— verified by manifest scan: `for f in crates/*/Cargo.toml apps/*/Cargo.toml; rg
'^playlists'` -> exactly one hit: crates/streaming/Cargo.toml.

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/error.rs:1-2 (crate scope)
Code:
```
//! Typed parse errors and warnings (card 4.D1).
```
What is wrong: transitively unreachable. playlists' sole dependent is streaming, and
the shipped CLI consumes from streaming only `fetch`/`ranged`/`schemes`
(demux_host.rs:489-496) — never `hls`, `remote_playlist`, or `dash`, the modules that
parse playlists. The error vocabulary of the playlist layer can never be surfaced by a
shipped binary. Confidence: HIGH — Verification: read (manifest scan + CLI usage rg).
F-002 [S4]
Where: crates/playlists/src/error.rs:10-13
Code:
```
/// Fatal parse failures. Dirty input usually degrades to a
/// [`ParseWarning`] instead (D-2); only the D-1 line-cap violation and
```
What is wrong: nothing wrong in the claim (verified against m3u.rs), but the enum
carries no `source`/context chaining — callers cannot attach the input URI without
breaking the redaction rule themselves; the redaction-vs-context tension is left to
the caller. Minor API debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
File is 86 LOC of enum definitions — no functions. Variants:
| Variant | Lines | Verdict | Notes |
|---|---|---|---|
| ParseError::LineTooLong | 16-29 | REAL | Display interpolates MAX_LINE_BYTES (compile-checked); raised at m3u.rs:157 |
| ParseError::InvalidUtf8 | 31-36 | REAL | fatal, no position (honest: cannot tokenize) |
| ParseWarning::BadDuration | 44-50 | REAL | entry survives, duration dropped |
| ParseWarning::ExtinfMissingColon | 52-57 | REAL | line ignored |
| ParseWarning::DirectiveMissingColon | 59-66 | REAL | covers truncated headers (#EXTM) |
| ParseWarning::DanglingEntry | 68-74 | REAL | metadata dropped at EOF |
| ParseWarning::BadExtXTag | 76-85 | REAL | line-level skip (HLS subset) |

## 5. Test honesty
No tests in-file; the parser suites that exercise these variants pass in baseline
(46+20+20+18+14+14+2, cited above). The Display strings are only compile-checked
here — no test pins the redaction rule itself (that no line text ever leaks). A
property test over all variants asserting absence of input substrings would pin the
rule. Not a fraud risk today (strings are literals).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Display of every variant names the kind and the line number only ... never enter an error or warning message" | error.rs:3-6 | REAL | all 7 Display strings hand-checked: only kind + {line}/{length} |
| "only the D-1 line-cap violation and non-UTF-8 input are fatal" | error.rs:11-13 | REAL | m3u.rs:157-161 + InvalidUtf8 path (m3u.rs docs L67) |

## 7. Dead / duplicated / unreachable code
Unreachable vocabulary (F-001). No dead variants — all five warnings and both errors
are constructed in m3u.rs/hls.rs/cue.rs (rg verified).

## 8. Dependencies & flags
thiserror (the crate's one error-macro dep) + crate::m3u::MAX_LINE_BYTES. No
features, no unsafe, no env reads.

## 9. Open questions for the next agent
- If/when a `lumen open <playlist.m3u>` lane lands, this vocabulary becomes reachable
  overnight — re-run the reachability scan then.
