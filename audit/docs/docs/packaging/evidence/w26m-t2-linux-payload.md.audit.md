# CLAIM AUDIT: docs/packaging/evidence/w26m-t2-linux-payload.md
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Claims extracted | 14 |
| REAL | 9 |
| REAL-UNPROVEN | 3 (gate PASS, stand-in staging proofs, integration rehearsal) |
| SHALLOW | 0 |
| FALSE | 0 (1 SUPERSEDED payload state — see contradictions) |
| UNVERIFIED | 2 (first-CI-run items: rpm -qlp, appimage extract row) |
| Internal contradictions found | 1 (era supersession vs current tree) |

## 1. Blunt verdict
Honest, well-scoped task evidence: every script change it claims is present in the
current tree (Depends/Requires rows, BIN_TAURI guards, the 7 verify rows, the gate.sh
bridge machinery, packaging-env.py's --tauri-package pair). Proof 1 re-verified live
(all 8 scripts parse under bash -n + sh -n). Its one structural weakness: the payload
it proves is a THREE-binary deb/rpm/tar incl. `lumen-desktop` — a state later
REVERSED by w30.8/w30.12 (stub dropped from every payload; desktop entry Exec
re-pointed to BIN_TAURI). The doc is honest for its date (2026-11-13) and nothing
cites it as current — but its payload tables must not be read as today's contract.
gate.sh itself could NOT be re-run in THIS audit sandbox (no cargo on PATH —
environment), so its PASS verdict stays REAL-UNPROVEN here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "all `bash -n` + `sh -n` clean, gate step 2 green" (8 scripts) | L18 | REAL | I ran bash -n + sh -n on all 8 named scripts -> PARSE OK x8 (exit 0 each) |
| 2 | "build-deb.sh: **`Depends: libgtk-3-0, libwebkit2gtk-4.1-0`**" | L24 | REAL | scripts/packaging/build-deb.sh:93 `Depends: libgtk-3-0, libwebkit2gtk-4.1-0` |
| 3 | "build-rpm.sh: **`Requires: gtk3` + `Requires: webkit2gtk4.1`** in the generated spec" | L26 | REAL | scripts/packaging/build-rpm.sh:149 `Requires:       gtk3` (webkit2gtk4.1 row alongside) |
| 4 | "verify-linux-artifacts.sh: `BIN_TAURI` guard + 7 new assertion rows" | L27 | REAL | verify script L82 `: ${BIN_TAURI:?...}`; rows at L207 (deb presence), L265-268 (Depends pair), L299 (tar member), L156/L175 (rpm payload/spec rows) |
| 5 | "gate.sh: bridge-pair probe + pair passed at all 3 `packaging-env.py` call sites" | L28 | REAL | gate.sh:89-90 `*BIN_TAURI*) ... step "   bridge exports BIN_TAURI (third shipped bin wired end to end)"` |
| 6 | "packaging-env.py ... `--tauri-package`" (T1 bridge) | L58-59, L180 | REAL | scripts/packaging/packaging-env.py:41,199,220-225 (--tauri-package pair, "pass together with --tauri-package") |
| 7 | "Proof 3 ... `755 usr/bin/lumen` `755 usr/bin/lumen-desktop` `755 usr/bin/lumen-tauri`" | L83-89 | REAL-UNPROVEN + SUPERSEDED | historical staging proof (zero-byte stand-ins, stated); current stage-linux-payload.sh:41,61,72 stages ONLY BIN_CLI + BIN_TAURI — the lumen-desktop row no longer exists (w30.8) |
| 8 | "`sh scripts/packaging/gate.sh` → **PASS**, exit 0" | L46 | REAL-UNPROVEN | re-run in this audit sandbox FAILED at step 1: `product-env: error: cargo not found on PATH` — environment (no cargo in audit sandbox), not a repo defect; steps 2/3/4/7 DID pass in the same run (parse clean, gen-wxs --self-test PASS) |
| 9 | "Proof 5 ... tar members `lumen` `lumen-desktop` `lumen-tauri` + docs; deb Package/Version/Architecture/Depends" | L102-127 | REAL-UNPROVEN + SUPERSEDED | stand-in archives, historical; current tar contract = CLI + tauri + docs, stub refused (verify script L303-305) |
| 10 | "Proof 6 ... `/usr/bin/lumen-tauri` ... `validate_spec` now asserts BOTH Requires rows" | L129-152 | REAL | build-rpm.sh carries the rows (#3); validate_spec assertions in current source |
| 11 | "Proof 8 ... gate PASS on top of T1; `packaging-env.py: 52 variables resolved`" | L168-185 | REAL-UNPROVEN | rehearsal on a throwaway worktree of a pushed branch — not re-runnable (no git, no cargo here); gate machinery for both labels exists at gate.sh:89-93 |
| 12 | "gen-desktop-entry.sh ... `Exec=${BIN_DESKTOP}` stays" | L29 | REAL (then) / SUPERSEDED (now) | current scripts/packaging/gen-desktop-entry.sh:43 `Exec=${BIN_TAURI}` with comment L10-11 "w30.8/w30.12 — was BIN_DESKTOP, the exit-86 ..." |
| 13 | "Real binaries ... are the release lane's proof (verify-linux-artifacts.sh, first CI run)" | L189-192 | UNVERIFIED | a full-run log DOES exist for the earlier 2-binary era (docs/packaging/evidence/linux-lane-20261110.log:99 "PASS ... launch-tested"), but no committed run log exists for the CURRENT payload shape (CLI+tauri, stub-refusal) |
| 14 | "rpmbuild absent ... rpm binary build + `rpm -qlp` payload row are CI-only [VERIFY]" | L199-200 | UNVERIFIED | no rpm build evidence in-tree |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Payload = `lumen` + `lumen-desktop` + `lumen-tauri` (3 bins, L83-89, L109-111, L145-147); `Exec=${BIN_DESKTOP}` (L29) | verify-linux-artifacts.sh:210-211/303-305 dies if the stub appears in any payload; stage-linux-payload.sh:8,13,41 stages CLI+tauri only; gen-desktop-entry.sh:43 `Exec=${BIN_TAURI}` | TODAY the stub is refused everywhere (w30.8) and the entry execs the tauri shell. This doc records the 2026-11-13 intermediate state — honest history, superseded contract. Not cited by any other file (rg), so no live doc is misled by it. |

## 4. Fabrication / inflation indicators
None. The doc's best property is negative honesty: it labels its stand-in binaries
("zero-byte stand-ins ... labeled as such"), its env-bridge gap, its un-run CI
halves ([VERIFY] tags), and its deliberate untouched files. The supersession is a
matter of tree evolution after the doc's date, not deception by the doc.
