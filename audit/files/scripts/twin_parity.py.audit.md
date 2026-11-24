# AUDIT: scripts/twin_parity.py
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 367 / 11 (xml_escape, ps_get_filename, docs_rows_ps1, extract_ps1_template, ps_interpolate(+repl), simulate_ps1, run_gen_wxs, strip_xml_comments, canonical, compare, check_guid_derivation, main) |
| File verdict | REAL (executed, PASS) — but NOT WIRED into any CI job or gate (F-001) |
| Reachable from shipped entry point? | NO as automation: no workflow, gate.sh, full_gate or quality script invokes it (grep across non-audit tree: only the two twins' comments tell humans to run it manually). It is a manual discipline tool. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |

## 1. One-paragraph verdict
I EXECUTED `python3 scripts/twin_parity.py` — exit 0 PASS: it ran the real gen-wxs.sh as a subprocess on two fixtures, extracted the real `$wxs = @"..."@` here-string from build-msi.ps1, simulated PowerShell expansion (loudly refusing backticks/`$(...)`/unknown variables), and proved semantic (C14N) identity of both twins' XML, plus re-derived the three stable component GUIDs from product.toml's frozen upgrade code and pinned them in both twins (AppMenuShortcut=44134d88-..., DesktopShortcut=5364ccd7-..., Docs=798918a8-...). The engineering is genuine and the simulator is self-pinned against the real bash output. The defect is operational, not technical: NOTHING runs this proof automatically — the only "enforcement" is a comment in two files telling a future editor to remember to run it by hand.

## 2. What actually works (verified)
— `python3 scripts/twin_parity.py` -> exit 0, PASS with GUID list (executed; output pasted in run log).
— Fail paths exist and are loud: `ParityError` -> stderr + exit 1 (L363-367); template-openers/terminators missing -> error (L176-185); unmodeled interpolation -> error (L201-216).
— GUID derivation genuinely re-derives from product.toml (L307-331): `uuid.uuid5(uuid.UUID(upgrade_code), "component:<name>")` — the values match the literals pinned in gen-wxs.sh and build-msi.ps1 sources (the executed PASS proves it — a changed GUID in either twin fails).
— The real gen-wxs.sh subprocess env is built from scratch (L242-257) — no inherited-env leakage; BIN_DESKTOP deliberately absent.

## 3. Findings

F-001 [S2]
Where: repo-wide wiring (absence), with scripts/twin_parity.py:55 as the claimed usage
Code (twin_parity.py:55):
```
Usage: python3 scripts/twin_parity.py        (exit 0 = PASS)
```
and the only "wiring" (scripts/release/gen-wxs.sh:82-83):
```
# change run BOTH self-tests AND scripts/twin_parity.py (the offline
```
What is wrong: the proof that prevents the w28.1 class of upgrade-1603 regression is invoked by NO automated lane. Verified by grep across the non-audit tree (workflows, scripts/, docs/): the only occurrences of "twin_parity" outside the script itself are the manual-run comments in gen-wxs.sh:82 and build-msi.ps1:47. Neither ci.yml nor release.yml nor scripts/packaging/gate.sh runs it. The file's own docstring history says divergence caused release run 43729218652 to fail — i.e., the divergence class already shipped to a live release once — yet the offline detector still has no automated trigger. The MSI upgrade test in release.yml (test-msi-upgrade.ps1) does exercise the twins against each other at release time, but that is the EXPENSIVE, last-line, Windows-runner-only check this offline proof was built to front-run. Why it matters: a twin edit without a human remembering the comment re-opens a verified production failure class. What correct looks like: one line in gate.sh (full_gate) or the ci.yml windows-lane shell: `python3 scripts/twin_parity.py`. Confidence: HIGH — Verification: read + grep (executed the proof itself successfully, so the tool works; the absence of callers is the finding).

F-002 [S4]
Where: scripts/twin_parity.py:132-136
Code:
```
def ps_get_filename(path: str) -> str:
```
The simulator's `[IO.Path]::GetFileName` mirror splits on both separators; the bash twin uses basename ('/'-only). The divergence is surfaced-not-hidden (comment L133-135) — correct design; noted because it is the one place the twins legitimately differ and the proof would flag, not mask, it.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| xml_escape | 120-129 | REAL | - | five-replacement chain, order pinned to both twins |
| ps_get_filename | 132-136 | REAL | - | both-separator split; documented divergence surface |
| docs_rows_ps1 | 139-163 | REAL | - | mirrored loop; duplicate basename -> ParityError |
| extract_ps1_template | 166-185 | REAL | - | template pulled from REAL .ps1 source — drift reaches proof |
| ps_interpolate/repl | 188-217 | REAL | - | refuses backtick/$(...)/unknown var — no silent mis-model |
| simulate_ps1 | 220-236 | REAL | - | assembles the 9 template variables |
| run_gen_wxs | 241-269 | REAL | - | EXECUTES real gen-wxs.sh subprocess; nonzero -> ParityError |
| strip_xml_comments / canonical | 274-282 | REAL | - | ET.canonicalize, comments dropped |
| compare | 285-302 | REAL | - | unified diff on failure |
| check_guid_derivation | 307-331 | REAL | - | uuid5 re-derivation vs pinned literals, both twins |
| main | 336-359 | REAL | - | missing-file guard; two fixtures; PASS print |
| entrypoint | 362-367 | REAL | - | ParityError -> exit 1 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Executed proof (this run) | YES | mutation reasoning: change one GUID literal in build-msi.ps1 -> check_guid_derivation fails; alter one template attribute -> canonical diff fails; the w28.1 backslash bug -> Key attribute diverges -> diff fails. The simulator is pinned to real .sh output, so a simulator bug that "matches" a .ps1 bug it does not parse cannot occur (template extracted from real source; interpolation subset is loud-refusing). Residual honest gap: PowerShell semantics beyond $var/${var} are unmodeled — loudly refused, so a false PASS requires the .ps1 to use only modeled syntax (verified by reading the template: it does). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "gen-wxs.sh ... and build-msi.ps1 ... generate the SAME installer XML" | twin_parity.py docstring | REAL | executed proof: canonical XML equal on both fixtures |
| "the simulator is itself pinned ... failure mode is a diff, never a false PASS" | docstring L47-53 | REAL | simulator output compared to real bash output; refusal paths |
| "release run 43729218652: the bash heredoc swallowed the registry-key backslashes" | docstring L16-18 | REAL (consistent) | corroborates gen-wxs.sh self-test twin-parity pin; a 6th live-run ID proving release.yml HAS executed (see release.yml F-001) |

## 7. Dead / duplicated / unreachable code
The proof itself is automation-unreachable (F-001). xml_escape is duplicated three ways across twins+simulator — but each duplication is cross-pinned by the executed proof, which is the documented mitigation.

## 8. Dependencies & flags
stdlib only (difflib, re, subprocess, tomllib, uuid, ET). No network, no cargo. Runs in <1 s here.

## 9. Open questions for the next agent
1. Was twin_parity.py ever run in ANY recorded baseline? Check audit/runs/baseline for its output; the full_gate FAIL list (fmt/test/arch/lint-literals/identity/sbom/web-budget) does not name it — consistent with not being wired.
2. build-msi.ps1's own -SelfTest — is IT wired anywhere? (Same wiring question; build-msi.ps1 audit by W17-A09 exists — cross-read it.)
