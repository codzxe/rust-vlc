# AUDIT: crates/codec-api/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 140 / 0 fns (crate root: 6 module decls, 2 re-export groups, prelude module, 1 executable doc example) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — codec-api is the contract every shipped decoder implements; imported by codecs, playback-engine, apps/cli, apps/web |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of full file; `cat crates/codec-api/Cargo.toml` (dep audit); baseline citation audit/runs/baseline/cargo-test-workspace.txt:246-296 |

## 1. One-paragraph verdict
Exactly what it claims to be: a pure contract crate root — module declarations, flat re-exports, a prelude, and a doc example that is not decoration but an executable doctest exercising the generational-cancellation semantics (stale packet → `NeedMoreInput`) and passing in the baseline. Both structural claims in the header check out against the manifest: `media-types` is the ONLY dependency (Cargo.toml `[dependencies]` has exactly one path dep; no serde/thiserror), and the crate contains no codec implementations — the root declares only capability/config/contract/decode/encode/hw, with the reusable contract suite living in-contract so codec crates need no extra dev-dependency edge. The whole crate is validated by 16 unit + 8 contract-suite + 3 doctests, all green in the baseline.

## 2. What actually works (verified)
— `cat crates/codec-api/Cargo.toml` -> `[dependencies] media-types = { path = "../media-types" }` and nothing else; no `[dev-dependencies]` — matching the header claim "depends only on media-types ... std-only besides".
— Baseline: `test crates/codec-api/src/lib.rs - (line 35) ... ok` (the doc example in THIS file executed and passed, baseline:290); `contract_suite` 8/8 ok incl. `rogue_decoders_are_caught_by_the_suite` (baseline:274-280); unit tests 16/16 ok (baseline:249-263).

## 3. Findings
None. What was checked and how: (a) "no codec implementations" — read the entire file (module decls + re-exports only) plus the crate manifest; (b) "depends only on media-types" — Cargo.toml dependency list read verbatim; (c) doc-example truthfulness — the doctest asserts stale-packet semantics and ran green in the baseline; (d) prelude completeness — every re-exported name cross-checked against the six module files' pub items (config.rs audited earlier this session; capability/decode/encode/hw/contract covered by the baseline suite); (e) no feature flags, no unsafe, no platform types in the root.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module decls (capability..hw) | 107-112 | REAL | — | 6 modules, all tested in baseline |
| headline re-exports | 114-121 | REAL | — | flat surface, names match module pubs |
| prelude | 129-140 | REAL | — | contract + media-types vocabulary; used by every decoder impl in the tree |
| doc example (NopDecoder) | 35-105 | REAL | — | executable doctest, baseline:290 ok; demonstrates drop_stale_packet truthfully |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 35) | YES | asserts a stale-generation packet returns NeedMoreInput through a real Decoder impl |
| contract_suite (separate binary) | YES | includes a deliberately rogue decoder that must be CAUGHT — the suite tests itself against violators, not just conformers |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This crate is a contract: it contains no codec implementations, names no platform types ... carries no selection policy" | lib.rs:9-12 | REAL | full read + manifest; hw stays an opaque trait (`HwAccelerator`) |
| "depends only on media-types [PROPOSAL ... std-only besides]" | lib.rs:7-8 | REAL | Cargo.toml has exactly one dependency |
| "`contract` — the reusable test suite every codec in codecs ... runs against" | lib.rs:26-28 | REAL | contract_suite binary 8/8 ok in baseline; codecs crates import it (see codecs tests) |

## 7. Dead / duplicated / unreachable code
None. Every re-export is consumed downstream (codecs, playback-engine, apps).

## 8. Dependencies & flags
media-types (path) only; no dev-dependencies by design; no features; workspace lints. Matches ADR-001 posture (hw abstract).

## 9. Open questions for the next agent
None.
