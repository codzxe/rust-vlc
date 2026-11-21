# AUDIT: crates/demux/src/adts/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 96 / 4 (3 budget consts + `fuzz_demux_adts`) |
| File verdict | REAL (in-crate harness, executed; libfuzzer-side execution UNVERIFIED — no committed run) |
| Reachable from shipped entry point? | Not from the CLI (harness code). Execution paths that DO reach it: (1) in-crate — `adts/tests.rs:641` and `tests/adts_corpus.rs:402-431`, both green in baseline (log 1513, 1961); (2) the cargo-fuzz shim `fuzz/fuzz_targets/fuzz_demux_adts.rs:19` calls it, registered in `fuzz/Cargo.toml:132-133` + `fuzz/README.md:66`, registry gate PASS (INDEX.md row 18, 35 targets, diff empty). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `rg -n "fuzz_demux_adts"` workspace-wide (all callers enumerated above); `sed -n` quote verification; baseline greps (in-crate sweeps 1513, 1957-1964); read of `scripts/fuzz_registry_check.sh` (what the PASS actually checks — registry consistency + seed corpus presence, NOT execution); adts `seek`/`reanchor` traced (mod.rs:1198-1264) to confirm the harness's `reanchor` probe covers the landing-scan surface. |

## 1. One-paragraph verdict
A real, executed-in-crate fuzz harness with an honest contract statement
("absence-of-panic/hang/unbounded-allocation, nothing else" — 6-10): one full
construct/start/drain pass with a 500k event budget, then three data-derived
seek probes (`reanchor`, which internally runs the same frame-header landing
scan `seek` uses — mod.rs:1240-1243 vs 1201-1204, so the probe surface is
genuine). The in-crate sweeps run it over the committed mutation fixtures and
the real corpus (both green in baseline: log 1513 and 1961). Two defects, both
about what the harness claims vs enforces: the "wiring status" doc says the
cargo-fuzz target file hasn't landed when it exists, is registered, and passes
the registry gate; and the event budget silently PASSES a demuxer that emits
500k events for a tiny input — the budget bounds the harness but detects
nothing, so the "never hangs / unbounded" third of the contract is asserted by
doc, not by code. Actual libfuzzer execution has no committed evidence.

## 2. What actually works (verified)
— In-crate execution: `committed_mutation_fixtures_are_typed_or_recover_never_panics ... ok`
   (baseline 1513) and `corpus_adts_survive_the_fuzz_harness ... ok` (1961) —
   the harness runs over 4 mutated fixtures + the full corpus + cuts +
   bit-mutations (adts_corpus.rs:402-431).
— Seek-probe coverage is real: `reanchor` (mod.rs:1235-1253) runs
   `scan_landing_adts`/`landing_adif` — the same damaged-frame scan `seek`
   runs — then commits and drains (88-93), so the post-seek paths are
   exercised on mutated bytes.
— Registry integrity: the shim→harness fn linkage is gate-checked
   (`fuzz_registry_check.sh:30-49` greps for `fn fuzz_demux_adts` in
   crates/demux/src) and PASSED in baseline (INDEX.md row 18).

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/adts/fuzz.rs:12-18
Code:
```
//! The cargo-fuzz target file is a Lead-owned dependency decision
//! (`libfuzzer-sys`), exactly as the AVI/WAV/Ogg cards recorded; until
//! it lands the harness logic is proven by the deterministic in-crate
//! sweeps (bit-flip/truncation families over the committed fixtures
//! and the real corpus, run by the unit + integration suites).
```
What is wrong: The cargo-fuzz target file HAS landed:
`fuzz/fuzz_targets/fuzz_demux_adts.rs` exists (its line 19 calls this exact
harness), it is registered as a `[[bin]]` in `fuzz/Cargo.toml:132-133`, listed
in `fuzz/README.md:66`, and the registry gate passed in the baseline (INDEX.md
row 18: "35 targets, diff empty"). What has NOT landed is the nightly
libfuzzer build wiring (the shim's own comment: "Build (once nightly is
wired)"). The "until it lands" provenance note is stale.
Why it matters: the fuzz provenance chain is exactly what an audit (and the
security sweep that claims "165M fuzz execs clean" in commit messages — not
committed evidence) must rely on; a stale wiring-status note misstates which
guarantees exist. A reader concludes no target exists and none can be run;
in fact the target exists and only the runner wiring is missing.
What correct looks like: "The cargo-fuzz target shim is registered
(fuzz/fuzz_targets/fuzz_demux_adts.rs); the nightly libfuzzer build wiring is
the Lead-owned decision still pending — until it lands the harness logic is
proven by the deterministic in-crate sweeps."
Confidence: HIGH — Verification: read + file/registry/gate existence verified; baseline row 18 cited.
### F-002 [S4]
Where: crates/demux/src/adts/fuzz.rs:70-74
Code:
```
        drained += 1;
        if drained >= FUZZ_EVENT_BUDGET {
            break;
        }
    }
```
What is wrong: When the budget is hit the harness `break`s and the pass
RETURNS SUCCESS. A demuxer that emits 500,000 events for a 4 KiB input —
pathological event amplification, the exact "unbounded" class the module doc
names in its contract — sails through green. The budget bounds the harness's
own loop; it detects nothing. Likewise a `next_event` that never returns (a
true hang) is beyond the harness entirely — the in-crate sweeps run without
any timeout, so the "never hangs" third of the documented contract is
enforced by nothing in this file.
Why it matters: fuzz harnesses are the instrument that catches resource bugs;
this one is structurally blind to the resource class it claims to cover. The
doc is half honest ("by construction of the demuxer and the budgets above") but a
budget-exceeded break is a signal the harness throws away.
What correct looks like: `if drained >= FUZZ_EVENT_BUDGET { panic!("event amplification: {drained} events for {} bytes", data.len()); }` — or return a failure the sweep tests assert on; plus a wall-clock budget for the hang case.
Confidence: HIGH — Verification: read; both break paths traced (61, 68, 71-73).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` | 34-36 | REAL | — | 500k with [PROPOSAL] rationale; adequate (largest corpus lane ~376 packets). |
| `FUZZ_POST_SEEK_DRAIN` / `FUZZ_SEEK_PROBES` | 38-42 | REAL | — | 64 events × 3 probes; adequate. |
| `fuzz_demux_adts` | 47-96 | REAL | S4 | Start-error early return (54-56) correctly gates on the format's own first-frame rule; drain loop swallows all outcomes per the documented no-panic contract; `data.to_vec()` (48) one alloc per pass — acceptable. Seek probes stamp targets in a hard-coded 48 kHz base (77) while `last_ticks` is in the stream's own base — targets mis-scale by den/48000 for non-48k streams (harmless: any target is a valid probe, but the constant misleads); `i64::try_from(FUZZ_SEEK_PROBES).unwrap_or(3)` (80) is dead fallback noise (const 3 always converts). F-002 budget-blindness. |

## 5. Test honesty
| Caller | Would it fail if the harness were broken? | Reason |
|---|---|---|
| adts/tests.rs::committed_mutation_fixtures (baseline 1513) | NO (harness-level) | The harness cannot fail except by panic — see tests.rs audit F-001 (S3, this wave): the "typed or recover" half is unobservable. |
| adts_corpus.rs::corpus_adts_survive_the_fuzz_harness (baseline 1961) | NO (harness-level) | Same: full corpus + cuts + bit-mutations pass unless a panic occurs. The PANIC protection is real and mutation-sensitive (bit-flips over real files that exercise deep parse paths). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-container pass … probe seeks at data-derived targets" | fuzz.rs:4-6 | REAL | Code traced: construct→start→drain→3 reanchor probes; reanchor runs the landing scan (mod.rs:1240-1243). |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:6-10 | REAL (panic) / UNENFORCED (hang, unbounded) | Panic: enforced (any panic fails the sweep tests — baseline green means none on the swept inputs). Hang/unbounded: F-002 — no timeout, budget-break passes. |
| "until it lands the harness logic is proven by the deterministic in-crate sweeps" | fuzz.rs:15-18 | FALSE (wiring status) | F-001: target file landed; sweeps proven (baseline 1513, 1961). |
| "165M fuzz execs clean" (issue #52 sweep) | commit message a05516c5 (NOT a doc) | UNVERIFIED | No committed fuzz-run evidence in audit/runs/baseline; registry gate (row 18) checks consistency, not execution. |

## 7. Dead / duplicated / unreachable code
- Not dead: three execution lanes (unit sweep, corpus sweep, cargo-fuzz shim).
- `unwrap_or(3)` (line 80) is dead-fallback noise on an infallible const
  conversion (noted in the table).

## 8. Dependencies & flags
No feature gates; std + plugin_api/media_types only. The `fuzz/` workspace
(member `fuzz/Cargo.toml`) is separate from the shipped crates. No unsafe.

## 9. Open questions for the next agent
- W16-A10 (assigned `fuzz/fuzz_targets/fuzz_demux_adts.rs`): confirm the shim
  matches this harness's signature and check whether ANY libfuzzer run output
  is committed anywhere (I found none in audit/runs/baseline).
- Whoever wires nightly fuzzing: F-002's amplification panic should land with
  it — the first real fuzz campaign will otherwise pass pathological demuxers.
