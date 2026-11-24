# CLAIM AUDIT: docs/audit/panic-baseline.json
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 13 per-crate counts + grand_total 41 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 3 REAL (executed cross-checks) / 0 / 0 / 0 / 0 / 0 / 10 UNVERIFIED (per-crate method not replicable in-tree) |

## 1. Blunt verdict
The baseline counts the tree's real panic surface — including its own bad news: demux
carries 2 non-test `unreachable!`, and I confirmed them by RUNNING the repo's own gate
(`cargo run -p xtask -- check-panics` → **FAIL, 2 violations, both demux/ts/mod.rs**).
The two sites sit in the TS demux's production codec-descriptor path and are NOT in the
tool's sanctioned CITED_INVARIANTS list. The JSON is honest; the tree it describes ships
a red panic-surface gate with an unsanctioned panic on a demux path.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence (command + output) |
|---|---|---|---|---|
| 1 | grand_total 41 (40 unreachable + 1 unwrap; panic/expect/todo = 0 everywhere) | L4-104 | REAL (sum) | python3 sum over the JSON's per-crate values = 41 ✓ |
| 2 | demux unreachable: 2 | L34-40 | REAL (EXECUTED) | `cargo run --locked -p xtask -- check-panics` → `check-panics: FAIL — 2 violation(s) outside the sanctioned zones:` `crates/demux/src/ts/mod.rs:369: unreachable!` + `crates/demux/src/ts/mod.rs:372: unreachable!` — exact count AND exact crate match |
| 3 | crates/_template unwrap: 1 | L6-12 | REAL | crates/_template/src/lint_canary.rs:30 `maybe.unwrap() // clippy::unwrap_used` — the deliberate lint canary |
| 4 | per-crate unreachable counts for the other 12 crates (input 5, control-protocol 4, tools/xtask 10, playlists 6, …) | L5-103 | UNVERIFIED | the generator of THIS json shape (13-crate scan) is not the in-tree `check_panics.rs` (SCANNED_CRATES = 9 engine crates, different output format); lexical re-count cannot replicate the brace-region + mod-level-cfg method (naive playlists count = 1 vs json 6) — no in-tree tool reproduces it |
| 5 | (context) repo-wide `unreachable!` incl. tests | — | REAL | rg --count-matches over crates+apps+tools = 4124 (baseline loc-patterns.txt:28 = 4126; 2-site drift; ~4,085 of them test-side) |

## 3. Findings
F-001 [S2] — shipped tree FAILS its own panic-surface gate; 2 production unreachable! unsanctioned.
Where: crates/demux/src/ts/mod.rs:366-373.
Code (sed-verified):
```
                return CodecDescriptor::new(&format!("ts-stream-{other:02x}"))
                    .unwrap_or_else(|_| unreachable!("formatted names are valid"));
            }
        };
        CodecDescriptor::new(name).unwrap_or_else(|_| unreachable!("static names are valid"))
```
What is wrong: `CodecDescriptor::new` is fallible; its failure arm is `unreachable!` —
a panic on the TS demux parse path instead of the project's typed-error rule (the gate's
own fix instruction says: return a typed error or CITED_INVARIANTS with an audit
citation). Why it matters: TS demux eats adversarial bytes; the gate that exists to
catch this is red and shipped anyway. Confidence: HIGH — Verification: ran the gate
(exit 1) + read the code.
F-002 [S4] — baseline generator not in-tree: the 13-crate scan's producing tool is not
`tools/xtask/src/cmd/check_panics.rs` (9-crate scope, different output). A baseline you
cannot re-derive cannot be checked for drift. Confidence: HIGH.

## 4. Contradictions
| json says | gate run says | True |
|---|---|---|
| demux unreachable: 2 | check-panics FAIL: 2 violations (ts/mod.rs:369,372) | both agree — record honest, tree red |

## 5. Fabrication / inflation indicators
None: the json records violations, not clean bills. The tree-level red gate is a
code/gate defect, not a record defect.
