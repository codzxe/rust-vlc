# CLAIM AUDIT: docs/dependencies/allowlist-format.md
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Claims extracted | 8 |
| REAL: 2 · REAL-UNPROVEN: 0 · SHALLOW: 0 · STUB: 0 · PAPER-ONLY: 2 · FALSE: 3 · UNREACHABLE: 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
This is the Phase-0 *schema* doc, honest about being format-only at write time, but it has never been updated and now makes three concrete claims that are false against the current repo (a lockfile exists; the allowlist is populated; records/ has 23 files not 1). The G3 enforcement it describes ("a crate in Cargo.lock without a row fails G3") is paper — no checker tool exists anywhere in tools/.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "FORMAT ONLY. No Cargo workspace or lockfile exists in Phase 0, so there is nothing to allowlist yet" | allowlist-format.md:6 | **FALSE (stale)** | `ls docs/dependencies/` shows populated `allowlist.md` (Phase-1 snapshot, task 1.E5-R, L1-6); repo has Cargo.lock at root (`ls` output). |
| 2 | "The single filled example record (`records/wasmi.md`) is a pre-audit record" | :6 | **FALSE (stale)** | `ls docs/dependencies/records/` → 23 files (wasmi.md, rav1d-1.1.0.md, cpal-0.18.2.md, wgpu-30.0.1.md, oxideav-ac3-0.0.11.md, ureq-3.5.0.md, …). "single" is false. |
| 3 | "records/*.md — the records (1 exists: wasmi; the other pre-audited candidates live as table rows …)" | :158-160 | **FALSE (stale)** | Same `ls`: 23 record files; sibling allowlist.md:151-157 itself lists four pre-audit records (wasmi, cpal, wgpu, tauri-wry) contradicting "1 exists". |
| 4 | "A crate present in `Cargo.lock` **without a row fails G3**" | :15-16 | **PAPER-ONLY** | No G3 checker exists: `ls tools/` → bench-vs-vlc, corpus, mem-audit, perf, sweep, vta, xtask — no dependency-audit. allowlist.md:16-17 admits "nothing consumes it until the G3 checker … exists"; :190 lists it as owed queue item 3. |
| 5 | "See `docs/dependencies/records/wasmi.md` — wasmi 2.0.0, filled 2026-10-19 … pre-audit verdict PASS with the proposed pin `>=2.0.0, <2.1`" | :148-153 | **REAL** | `rg -n "2\.0\.0|PASS|403" docs/dependencies/records/wasmi.md` → L1 "wasmi 2.0.0 (pre-audit record, filled 2026-10-19)", L107 `[VERIFY-P0: api.github.com 403 from sandbox]`, L121 "**PASS** (pre-audit)". |
| 6 | "`audit-method.md` — how records are produced (M1–M9)" and ADR-002/preaudit-foundation exist | :157-163 | **REAL** | `ls docs/dependencies/` → audit-method.md, preaudit-foundation.md present; `ls docs/adr/` → ADR-002-evidence.md present. |
| 7 | TOML field rules "testable by the G3 checker" (§1.1, 8 rules) | :53-75 | **PAPER-ONLY** | No code implements any of the 8 rules (no checker binary, no allowlist.toml anywhere: `ls docs/dependencies/*.toml` → none; `ls deps` → no such dir). |
| 8 | "`ree-audit-triggers`" typo'd key in the example TOML (should be `re-audit-triggers` per :107 wording) | :43 | **S4 hygiene note** | `sed -n '43p'` → `ree-audit-triggers = [`. A machine-parsed schema example shipping a misspelled key — harmless today only because nothing parses it (claim 4). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "single filled example record (records/wasmi.md)" (:6) | allowlist.md §5 lists records/wasmi.md, records/cpal-0.18.2.md, records/wgpu-30.0.1.md, records/tauri-wry-2.x.md; records/ dir has 23 files | The repo: 23 files (ls above). allowlist-format.md is stale by an entire project epoch. |

## 4. Fabrication / inflation indicators
- Low. The doc is dated thinking (Phase 0) and does not fake results; its failure is staleness: §4's "Relationship to the other Phase 0 artifacts" describes a world (1 record, no lockfile) that stopped existing when Phase 1 landed. Nobody pruned or annotated it after the fact.
- The schema itself is coherent and the example row's wasmi values match the real record (claim 5).

## 5. Matrix rows
N/A — not a matrix chunk.

## 6. What actually works (verified)
- `ls docs/dependencies/` and `ls docs/dependencies/records/` (exit 0) — inventory claims refuted/confirmed as cited above.
- `rg -n "PASS|403|2\.0\.0" docs/dependencies/records/wasmi.md` (exit 0) — the referenced example record exists and matches the description.

## 7. Open questions for next agent
- If any gate doc cites allowlist-format.md §1.1 rules as *active* enforcement, that is FALSE — cite claim 4.
- Who owns updating this file when the G3 checker lands? Unresolved "[PROPOSAL — Lead confirms at 1.E5]" markers (L7, L13, L75, L82-83, L90) are all still open (no Lead decision recorded in-repo).
