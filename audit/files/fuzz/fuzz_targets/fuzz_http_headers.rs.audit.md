# AUDIT: fuzz/fuzz_targets/fuzz_http_headers.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 |
| Date | 2026-06-14 |
| Lines / functions | 50 / 1 fuzz_target! + 1 #[path] module |
| File verdict | REAL (two-seam shim, both harnesses verified) — UNREACHABLE as a running fuzz target (campaign not wired) |
| Reachable from shipped entry point? | NO — cargo-fuzz binary; campaign never run; the twin module IS exercised on stable by netfuzz_sweep (baseline) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg both harness fns; baseline netfuzz_sweep lines |

## 1. One-paragraph verdict
The most interesting shim of the four: it feeds each input to TWO seams —
`streaming::mockhttp::fuzz_response_head` (the pure parsing half of the
socket path's read_response_head, bounded by the transport's own 64 KiB head
budget per the comment) and the `#[path]`-included
`crates/streaming/tests/netfuzz/fuzz.rs` twin — so the libFuzzer campaign and
the stable sweep share one body. The `#![allow(unused_must_use)]` is
explained honestly in-file: the discarded Result IS the point (survival, not
verdict) and the registry gate's line grammar would not see through a
`let _ =`. Both harness functions exist; the twin's stable sweep ran in the
baseline with the 60 s variants explicitly ignored-and-named.

## 2. What actually works (verified)
— `rg 'fn fuzz_response_head'` → crates/streaming/src/mockhttp.rs:1088
`pub fn fuzz_response_head(data: &[u8]) -> Result<FuzzResponseHead, NetworkError>`.
— `rg 'fn fuzz_http_headers'` → crates/streaming/tests/netfuzz/fuzz.rs:127
`pub fn fuzz_http_headers(data: &[u8])`.
— Baseline: cargo-test-workspace.txt:5708 `Running tests/netfuzz_sweep.rs` …
:5711-5712 `netfuzz_sweep_hls_playlist_60s ... ignored, long fuzz run` /
`netfuzz_sweep_http_headers_60s ... ignored, long fuzz run` — the ignored
long runs are NAMED with their reason in the runner output (honest ignore
pattern), and the non-ignored sweep tests passed (baseline overall).
— Registry row: fuzz/README.md:90 matches the two-seam description.

## 3. Findings
F-001 [S4]
Where: fuzz/fuzz_targets/fuzz_http_headers.rs:35
Code:
#![allow(unused_must_use)]
What is wrong: a crate-level lint allow in a shim — the comment justifies it
(discarded Result = survival contract; gate line grammar vs `let _ =`), and
the allow is scoped to this 50-line shim only. Acceptable, but it is the
pattern spec §6.5 flags: an `#[allow]` hiding a lint that exists for a
reason; a `let _ = streaming::mockhttp::fuzz_response_head(data);` with the
gate taught the grammar would not need the allow.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: fuzz/fuzz_targets/fuzz_http_headers.rs:9-10
Code:
// (issue #14 M2, card 6.F3): hostile bytes from the wire into the transport's
// response-head grammar (status line + `Name: value` lines).
What is wrong: nothing functional — noting the in-file self-acknowledged
naming drift (target plural `fuzz_http_headers` vs the mockhttp fn singular
`fuzz_response_head`) is documented rather than fixed; consistent with the
registry gate reading the call line.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| netfuzz module include | 42-43 | REAL | #[path] twin shared with the stable sweep |
| fuzz_target! body | 45-48 | REAL | both seams called; Result discarded by contract |

## 5. Test honesty
The shim's logic is deliberately empty (survival semantics); the honesty
lives in the shared twin body + the named-ignored 60 s sweep variants
(baseline-visible).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "input capped at the transport's own 64 KiB head budget … never an unbounded parse" | file L13-15 | REAL-UNPROVEN | mockhttp.rs read shows the fn returns NetworkError (typed); the 64 KiB cap constant not re-verified in this audit (UNVERIFIED: line cite in mockhttp not pulled) |
| "the campaign and the sweep share one body" | file L22-24 | REAL | #[path] include + baseline netfuzz_sweep run |

## 7. Dead / duplicated / unreachable code
None; the shim is minimal by design.

## 8. Dependencies & flags
libfuzzer-sys only; streaming crate + #[path] tests module; no_main;
no feature gates. `#![allow(unused_must_use)]` (F-001).

## 9. Open questions for the next agent
- Pull the 64 KiB head-budget constant cite in mockhttp.rs (F-002's sibling
  claim) when auditing crates/streaming.
