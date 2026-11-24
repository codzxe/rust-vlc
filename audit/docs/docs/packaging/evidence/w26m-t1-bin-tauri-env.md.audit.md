# CLAIM AUDIT: docs/packaging/evidence/w26m-t1-bin-tauri-env.md
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / UNVERIFIED | 5 REAL / 4 REAL-UNPROVEN / 1 UNVERIFIED |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The evidence doc's code claims are all present in the tree and byte-consistent (workspace exclude, the `--tauri-*` pair plumbing in both python scripts, the exact hard-error strings, `BIN_TAURI` emitted only when the pair is passed). I could not re-execute the two proofs: this sub-agent sandbox has NO cargo anywhere (not on PATH, not in ~/.cargo/bin — the doc's honest note anticipated the PATH problem but assumed ~/.cargo/bin exists; here it does not). Static verification is complete and positive.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "root Cargo.toml `exclude` — the ADR-004 admission crate keeps its own lockfile" | L17-19 | REAL | Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`; apps/desktop-tauri/Cargo.lock exists |
| 2 | "runs a SECOND `cargo metadata --no-deps ... --locked --manifest-path`" | L21-22 | REAL (code) | scripts/release/product-env.py:233-238 (exact flags) |
| 3 | "Half a pair is a hard error, not a silent omission" | L26-27 | REAL (code) | product-env.py:221-223 — die() string matches doc's negative test #1 byte-for-byte |
| 4 | "Wrong manifest path" error | L122-123 | REAL (code) | product-env.py:229-232 — die() string matches doc's negative test #2 |
| 5 | "Omit the pair and `BIN_TAURI` is simply not emitted" | L25-26 | REAL (code) | `env["BIN_TAURI"]` set only inside `if args.tauri_package and args.tauri_manifest:` (product-env.py:225,243) |
| 6 | Proof 1 output `BIN_TAURI=lumen-tauri` | L58 | REAL-UNPROVEN | bin name matches apps/desktop-tauri/Cargo.toml:30-32 (`[[bin]] name = "lumen-tauri"`); execution blocked — no cargo in sandbox (ran: `command -v cargo` -> empty; `ls ~/.cargo/bin/cargo` -> No such file) |
| 7 | Proof 2 diff = one line `> BIN_TAURI=lumen-tauri` | L106-112 | REAL-UNPROVEN | same blocker; logic (claim 5) supports it |
| 8 | Negative tests exit 1 | L118-123 | REAL-UNPROVEN | my runs DID exit 1 but hit the earlier "cargo not found on PATH" guard (product-env.py:204) — proves fail-loud ordering, not the pair message |
| 9 | "`cargo metadata` ONLY — nothing compiled" | L14 | REAL (code) | only subprocess `cargo metadata` invocations in the path |
| 10 | "sandbox's default `PATH` does not carry cargo (it lives in `~/.cargo/bin`)" | L127-129 | half-REAL | PATH claim TRUE here too; ~/.cargo/bin does NOT exist in this sandbox — the proof is not re-runnable in this lane |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none | — | — |

## 4. Fabrication / inflation indicators
None. The negative-test outputs in the doc match the code's die() strings exactly; the doc discloses its own environment limitation (L125-137). Note: negative test #2's example path `/home/z/my-project/rust-vlc/...` is from the authoring checkout, not this tree — cosmetic, message text is environment-dependent by design (path is interpolated).

## 5. Open questions / next agent
- The orchestrator (which has cargo — baseline cargo-build logs exist) can re-run Proof 1/2 verbatim; expected result: exact match with the doc's 52-line output.
