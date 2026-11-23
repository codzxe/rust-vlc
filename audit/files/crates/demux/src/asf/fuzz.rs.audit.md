# AUDIT: crates/demux/src/asf/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 107 / 2 pub fns + 1 test |
| File verdict | REAL (harness) with a FALSE wiring claim |
| Reachable from shipped entry point? | Compiled into the lib; exercised by its test (baseline :1775) and nothing shipped |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | ls fuzz/fuzz_targets/ (36 targets, NO asf); rg fuzz/Cargo.toml bins (35 `[[bin]]`, none asf); python3 byte reads; baseline citation :1775 |

## 1. One-paragraph verdict
The in-crate ASF harness is real and correctly scoped to the landed
ASF-a surface (probe gate → construct → seekability → start → the typed
not-yet refusals), and its deterministic sweep (2 templates × per-byte
mutations ×3 values × all truncations ≈ 530 cases) passes in baseline.
But the module's central wiring claim is FALSE: "The cargo-fuzz target
lives at `fuzz/fuzz_targets/fuzz_demux_asf.rs`" — that file does not
exist (36 target files, none asf) and fuzz/Cargo.toml registers no asf
bin (35 bins enumerated). The format doc doubles down
(docs/formats/container-asf.md:127 "fuzz_demux_asf target registered") —
also FALSE. An untrusted-input container parser whose fuzzing
infrastructure is claimed but absent is an inflated security posture,
mitigated only by the honest "Lead-owned nightly wiring" hedge and the
real in-crate sweep.

## 2. What actually works (verified)
- Test executed ok: baseline cargo-test-workspace.txt:1775
  `asf::fuzz::tests::hostile_sweeps_survive_without_panics ... ok`.
- Harness scope matches the landed surface: asf/mod.rs `next_event`
  exists with typed refusals (:651-656 verified by read); fuzz.rs:50-51
  exercises exactly one next_event + one seek — the ASF-b/c not-yet
  refusals (doc :48-49). Sweep math re-traced: template lens 64/68 →
  64×3+65 + 68×3+69 = 530 cases, all under the 8192 budget.
- Wiring claim DISPROVEN by enumeration: `ls fuzz/fuzz_targets/` shows
  36 files (fuzz_demux_{adts,aiff,avi,es,flac,ivf,mkv,mp3stream,mp4,
  mpd,ogg,ps,ts,wav} + codec/sub/playlist/http targets) — no asf;
  fuzz/Cargo.toml declares 35 `[[bin]]` names — no asf.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/asf/fuzz.rs:12-17
Code:
```
//! # Wiring status (honest)
//!
//! The cargo-fuzz target lives at
//! `fuzz/fuzz_targets/fuzz_demux_asf.rs` (Lead-owned nightly wiring,
//! see `fuzz/README.md`); the harness logic is meanwhile proven on
//! stable by the deterministic sweeps below.
```
What is wrong: the target file does not exist and is not registered.
`fuzz/fuzz_targets/` contains no fuzz_demux_asf.rs (verified by
enumeration); fuzz/Cargo.toml's 35 `[[bin]]` entries contain no asf
target; fuzz/README.md never mentions asf. The section titled "Wiring
status (honest)" asserts the exact thing that is missing. The plan
promised it (docs/plans/asf-selfbuild.md:321 "`fuzz/fuzz_targets/
fuzz_demux_asf.rs` + seeds") and the format doc claims it landed
(docs/formats/container-asf.md:127 "fuzz_demux_asf target registered.")
— both contradicted by the tree. Why it matters: ASF/WMV/WMA is an
untrusted-input parser; the claimed-but-absent libFuzzer target inflates
the fuzzing posture (the AVI twin has the real target; ASF does not);
any registry/parity evidence citing "fuzz_demux_asf registered" is
false. What correct looks like: the target file + Cargo.toml bin, or
the doc saying "target owed" like it actually is. Confidence: HIGH —
Verification: ran (ls + rg enumerations). Cross-ref for the docs agent:
container-asf.md:127 is a FALSE claim.
F-002 [S4]
Where: crates/demux/src/asf/fuzz.rs:78
Code:
```
    let mut budget = 8_192usize;
```
What is wrong: the sweep's budget can never bind — worst case is 530
iterations (2 templates × (len×3 + len+1)); the "hard cap" is dead
headroom that suggests a bound that does nothing. Harmless (the sweep
is structurally finite) but misleading. Confidence: HIGH —
Verification: read + arithmetic.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_demux_asf | 32-52 | REAL | — | probe gate + full landed trait surface; refusals swallowed by contract |
| stable_loop_survival | 60-99 | REAL | S4 | 2 templates (valid header prefix, broadcast prefix) × byte mutations ×3 values + all truncations; budget never binds (F-002) |
| tests::hostile_sweeps_survive_without_panics | 103-106 | REAL | — | passes (baseline :1775) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| hostile_sweeps_survive_without_panics | YES for panics/hangs on the 530 deterministic shapes | panic propagates; nothing else asserted BY CONTRACT (doc :7-10) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The cargo-fuzz target lives at `fuzz/fuzz_targets/fuzz_demux_asf.rs`" | :14-15 | FALSE | F-001 enumeration |
| "`fuzz_demux_asf` target registered." | docs/formats/container-asf.md:127 (cross-ref) | FALSE | same enumeration |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below" | :16-17 | REAL | baseline :1775 |
| "The walk's own ceilings apply (header memory ceiling, object count guards, field caps — see header.rs)" | :21-22 | REAL-UNPROVEN | header.rs is another lane's file; the ceilings' existence not re-verified here |

## 7. Dead / duplicated / unreachable code
None dead. The pub fns are consumed by the test and (per the false doc)
were meant for a cargo-fuzz target that does not exist — as shipped,
`fuzz_demux_asf`/`stable_loop_survival` have no caller outside the
crate's own tests.

## 8. Dependencies & flags
std-only; `super::{AsfDemuxer, probe}` + shared test_support (the avi
BytesStream reuse). No cfg gate. No unsafe.

## 9. Open questions for the next agent
- docs/formats/container-asf.md and docs/04-codec-format-roadmap.md both
  cite `fuzz_demux_asf` — the docs-lane agent should mark those rows
  FALSE (registered ⇒ nonexistent).
- crates/demux/src/asf/header.rs ceilings (1 MiB/32 MiB per
  container-asf.md:126) — verify in the asf lane.
- The wmv fixture (my next file) is the ASF parser's only committed
  real-world byte lane — check its audit note.
