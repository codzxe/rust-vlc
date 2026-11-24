# CLAIM AUDIT: docs/integration/wave24-merge-plan.md (78 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 0 / 0 / 0 / 0 / 0 / 2 UNVERIFIED (battery run numbers, merge count) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The merge plan's three Lead integration fixes are all verifiable in-tree and all three
landed exactly as described: (1) first-vertical scenarios.rs honors CARGO_TARGET_DIR
(:1338-1342 doc comment explains the wave-24 disk emergency); (2) rebrand_skeleton.rs
derives the README rewrite count case-insensitively instead of pinning it (:168, :445
`source_readme.to_lowercase()`); (3) apps/web/src/host.rs gates the decoder-seam refusal
test under `#[cfg(not(feature = "builtin-codecs"))]` (:417). The battery table
(cargo test 4488/0/0 piecewise at wave-24) is a dated snapshot — the current baseline
shows 4744/2/51 after later waves; no contradiction, only growth.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "scenarios.rs — lumen_binary_path() honors CARGO_TARGET_DIR" | §Integration fixes 1 | REAL — IN TREE | tests/integration/first-vertical/src/scenarios.rs:1338-1342. |
| 2 | "rebrand_skeleton.rs — README rewrite count ... derived from the source README (case-insensitive), not pinned" | fix 2 | REAL — IN TREE | rebrand_skeleton.rs:445 `let lowered = source_readme.to_lowercase();`. |
| 3 | "host.rs — the wasm32 decoder-seam refusal test is cfg(not(builtin-codecs))" | fix 3 | REAL — IN TREE | apps/web/src/host.rs:417 `#[cfg(not(feature = "builtin-codecs"))]`. |
| 4 | "cargo test ... 4488 passed / 0 failed / 0 ignored (piecewise ...)" | battery #3 | UNVERIFIED (dated run) | Current baseline: 4744/2/51 — later waves added tests + the 2 first-vertical failures (committed_transcripts_match_fresh_runs, final_frame_is_current per AGENT-SPEC §13). |
| 5 | "39+2 merges" on w24-integration-rehearsal | header | UNVERIFIED (git) | Lane letters a-y + r1-r16 enumerated in the merge-order list — internally consistent. |
| 6 | "cargo fmt --all --check → CLEAN" | battery #1 | SUPERSEDED | Current baseline cargo-fmt.txt: FAILED (exit 1) — drift after wave 24; documented in AGENT-SPEC §13. |

## 3. Contradictions
None — fmt-clean-then vs fmt-fail-now is post-merge drift, not a false claim at date.
## 4. Fabrication / inflation indicators
None.
