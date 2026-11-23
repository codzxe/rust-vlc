# AUDIT: crates/subtitles/src/vtt/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen; audit session 2025-07-14 clock) |
| Lines / functions | 63 / 1 harness fn + 2 tests |
| File verdict | REAL (harness is a 2-line honest shim; wiring target exists and calls it; sweeps pass) |
| Reachable from shipped entry point? | NO — `subtitles` is not linked by lumen-cli/desktop/web (pgs/mod.rs audit F-002). Reachable from the separate `fuzz/` workspace target `fuzz_sub_vtt` (verified: fuzz/fuzz_targets/fuzz_sub_vtt.rs:15 calls `subtitles::vtt::fuzz::fuzz_sub_vtt`) and the crate's test lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/subtitles-70f89de07410626f vtt::fuzz` → 2 passed, 0 failed (exit 0); `./target/debug/deps/vtt_fuzz-7b8ccb82eebade1a` → 3 passed (exit 0); cat fuzz/fuzz_targets/fuzz_sub_vtt.rs; rg cap verification |

## 1. One-paragraph verdict
The harness does exactly what it says and nothing more: lossy-decode the
bytes, call `vtt::parse`, discard the result — the contract is explicitly
absence-of-panic only, and the doc says so in bold. The cargo-fuzz target it
points at genuinely exists and calls this function (many "wiring pending"
claims in this repo are vapor; this one is real). The one false note is in
the Bounds section: "the damage ledger is capped" — the VTT damage ledger
(`VttDocument.damage`) is an uncapped `Vec` with no MAX constant anywhere in
the vtt module (contrast SRT's 256-event and PGS/DVB's 64-event caps); it is
input-proportional (one event per malformed block), which still satisfies
the no-unbounded-allocation fuzz contract but is not "capped".

## 2. What actually works (verified)
— `./target/debug/deps/subtitles-70f89de07410626f vtt::fuzz` → `test result:
ok. 2 passed` (empty/marker inputs; 4096×0xff, 4096×0x00, 64-cue timing
flood).
— `./target/debug/deps/vtt_fuzz-7b8ccb82eebade1a` → 3 passed (random
buffers, mutated documents, <200 KB large document — the real sweeps).
— Wiring: fuzz/fuzz_targets/fuzz_sub_vtt.rs:15 `subtitles::vtt::fuzz::fuzz_sub_vtt(data);`
— verified by cat; fuzz/Cargo.toml lists `subtitles` as a dependency.
— Corroborating execution record: docs/security/sweep-parsers.md:101-109
lists `fuzz_sub_vtt` at 8.33 M execs in the stable random-mutation run
(W20's doc to audit, cited here as cross-evidence only).

## 3. Findings

F-001 [S4]
Where: crates/subtitles/src/vtt/fuzz.rs:20-23, vtt/mod.rs:92-99, vtt/damage.rs (rg MAX: no cap constant)
Code:
```
//! # Bounds (MP §12 / D-2)
//!
//! Parse: one pass, output ≤ O(input); the damage ledger is capped
//! and the cue count is bounded by the line count.
```
What is wrong: the VTT damage ledger is NOT capped. `VttDocument.damage` is
`pub damage: Vec<DamageEvent>` (vtt/mod.rs:99), pushed per tolerated
malformation with no length check (e.g. vtt/mod.rs:186-215), and `rg MAX
crates/subtitles/src/vtt/*.rs` finds no damage cap (only i64::MAX saturation
in timestamp.rs). The true bound is one event per malformed block —
input-proportional, fine for the fuzz contract, but "capped" (which for this
codebase means a constant ceiling: SRT 256, PGS/DVB 64) is false and
misleads a reader auditing memory behavior. Each event's name string IS
truncated (damage.rs:75 `truncate_name`), so per-event memory is bounded.
What correct looks like: either add the cap constant like the sibling
formats, or reword the doc to "input-proportional (one event per malformed
block), event payload length-capped".
Confidence: HIGH — Verification: read + rg over the whole vtt module.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_vtt` | 28-31 | REAL | — | lossy decode + `let _ = parse` — contract-scoped shim, no swallowed-panic risk of its own |
| tests::empty_and_marker_inputs_survive | 37-42 | REAL | — | 3 smoke inputs |
| tests::hostile_wide_inputs_survive | 44-62 | REAL | — | 0xff/0x00 floods + 64 hostile timing lines |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES (panic only) | a panic in parse fails the test; nothing else asserted — honestly scoped |
| hostile_wide_inputs_survive | YES (panic only) | same; the meaningful assertions live in tests/vtt_fuzz.rs (which pins parse outcomes on mutations — ran, 3 pass) |
The in-crate tests are smoke-only by design; the module doc correctly
defers outcome pinning to the AC suites. Not a fake suite — it never claims
more than "survive".

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cargo-fuzz target is fuzz/fuzz_targets/fuzz_sub_vtt.rs" | L15-16 | REAL | file exists, calls this fn (cat) |
| "harness logic is … proven on stable by the deterministic sweeps below and the crate's VTT suites" | L16-18 | REAL | ran both (2 + 3 pass) |
| "the damage ledger is capped" | L22 | FALSE | F-001: uncapped Vec, no MAX in module |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | L7-8 | REAL | body matches (L28-31) |
| "output ≤ O(input)" | L22 | REAL | cues per block, damage per block (read) |

## 7. Dead / duplicated / unreachable code
None in-file. (Crate-level unreachability noted in the header row.)

## 8. Dependencies & flags
None at file level (std only; the harness allocates one lossy String per
call — input-proportional). No unsafe, no features.

## 9. Open questions for the next agent
- vtt/mod.rs auditor: consider whether the uncapped damage ledger (F-001)
  should get the sibling-format cap before the crate is ever wired into a
  player (a 100 MB hostile VTT → millions of DamageEvents retained on the
  document).
- W20 (sweep-parsers.md): verify the 8.33 M-exec claim's provenance — this
  file's doc defers all proof to those suites.
