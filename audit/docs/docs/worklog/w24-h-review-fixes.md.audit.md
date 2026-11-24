# CLAIM AUDIT: docs/worklog/w24-h-review-fixes.md (248 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 REAL / 0 / 0 / 0 / 0 / 0 / 2 UNVERIFIED (git commits, sandbox behavior tests) |
| Internal contradictions found | 1 (review's mechanism claim not reproduced — the doc itself flags it) |

## 1. Blunt verdict
The best worklog in my assignment: a per-finding before/after evidence record that fixes a
REVIEW's overclaim in the process. All 9 files it lists as touched exist;
scripts/packaging/ holds exactly the 14 .sh scripts the doc's sweep claims; the P1-1 fix
(resolve_identity return-code contract) is verifiably in the tree at sign-macos.sh:139,
:189-196 with the exact 0/2 contract described; the source review
(w24-c-packaging-findings.md) and the gate driver (scripts/packaging/gate.sh, full_gate
step 11) exist. Its most honest passage: "Honest note on the review's mechanism claim: the
literal `|| true` swallows the die() status path did NOT reproduce in this sandbox's sh
(dash) or bash 5 — for die() exits, set -e still killed the script (rc 1, no sidecar, on
both old and new)" — the fix worklog corrects its own source review's mechanism while
still fixing the real defect (subshell state loss leaving scratch dirs behind, which its
old-vs-new test DID reproduce).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Files touched (9): release.yml, packaging READMEs, build-msi.{ps1,sh}, sign-macos.sh, sign-windows.sh, test-msi-upgrade.ps1, gen-wxs.sh" | :18-22 | REAL — 9/9 EXIST | ls of all 9 paths -> present. |
| 2 | "P1-1 ... resolve_identity runs in the MAIN shell behind an explicit return-code contract — 0 identity resolved, 2 NO credentials" | P1-1 § | REAL — IN TREE | sign-macos.sh:139 `resolve_identity()`, :189-196 `resolve_identity_rc` contract with the 0-or-2 guard. |
| 3 | "sweep of all 14 scripts/packaging/*.sh under bash -n clean" | battery | REAL — count EXACT | ls scripts/packaging/*.sh -> 14 files. |
| 4 | "bash scripts/packaging/gate.sh ... PASS — 51 env vars, 14 scripts, python ast, ps1 sanity, stagers, both unsigned-fallback contracts executed for real" | battery | REAL (driver exists; run UNVERIFIED) | scripts/packaging/gate.sh exists (full_gate.sh step 11 references it verbatim). |
| 5 | "Honest note ... the literal `|| true` swallows the die() status path did NOT reproduce" | P1-1 § | REAL (self-correction of the review's mechanism) | The review w24-c-packaging-findings.md exists; the worklog documents the non-reproduction with the sandbox shells tried. |
| 6 | "The workflow has still never executed (CI billing blocked)" + honest-limits section | limits § | REAL disclosure | Consistent with w24-e review (never-executed workflow rows) and RELEASE.md known-limit 2. |
| 7 | Fix commits 1f19159/1337ca0/e4725d0/00821d3/f6ebd7e/7f2f894 on w24/h | :11-17 | UNVERIFIED (git) | Commit hashes not inspectable in sandbox. |

## 3. Contradictions
| This doc says | Other source says | Truth |
|---|---|---|
| The review's P1-1 mechanism ("|| true swallows the die() status → unsigned sidecar") did not reproduce | w24-c-packaging-findings.md asserted that mechanism | The worklog's old-vs-new tests show rc 1 + no sidecar on BOTH versions; the reproduced defect is the subshell-state leak. The worklog fixes the real bug and documents the review's error — the correction is the truth (verified only by the doc's recorded tests, the script fix itself is in-tree). |

## 4. Fabrication / inflation indicators
None — the worklog's "Honest limits" section pre-declares every un-executable item
("No pwsh, no Windows/macOS, no CI execution in this sandbox ... every such site is
labeled in-file. Nothing here claims otherwise.").
