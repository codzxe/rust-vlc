# CLAIM AUDIT: docs/gates/phase4-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 16 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 9 REAL / 4 REAL-UNPROVEN / 1 SHALLOW / 0 / 0 / 2 FALSE (stale-path citations) / 0 |
| Internal contradictions found | 1 (with release.yml) |

## 1. Blunt verdict
A gate record whose numbers check out against its own committed logs (I re-extracted the counts from four logs: 59/557 vitest, 82/0 rust, 30 visual, 38 playwright — all byte-matched to the doc's table). The honest-gaps section is genuinely honest (MockTransport-not-swapped, 3-OS infra-blocked, wrong-snapshot-dir footgun recorded). Two defects: the 2026-11-13 addendum claim that gate follow-ups were "discharged by the now-running hosted CI" CONTRADICTS release.yml's committed header that every Actions run dies at 0 steps (billing) — one of the two is false, and the live run IDs cited inside release.yml (43378757248/43464162696/43729218652) side with this doc; and two path citations are stale in the current tree (`apps/desktop/tauri/` moved to `apps/desktop-tauri/`; `.github/workflows-disabled/` no longer exists — workflows were re-activated).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Nothing in this document is invented: every number is from a run executed this session" | L5-7 | REAL | 4 of 4 spot-checked log numbers match (rows below) |
| 2 | "npm run test: 59 files / 557 tests passed, 0 failed" | L58 | REAL | `grep -E "Test Files\|Tests " docs/gates/logs/phase4-exit-ui-vitest.log` -> "Test Files 59 passed (59); Tests 557 passed (557)" (ran) |
| 3 | "cargo test -p desktop -p playlists -p lumen-sweep: 82/0 across 10 binaries" | L55 | REAL | awk over `^test result` in phase4-exit-rust-phase4-crates.log -> passed=82 failed=0 (ran) |
| 4 | "npm run visual -- --mode=pr: 30 passed (1.1 m)" | L54 | REAL | tail of phase4-exit-visual-pr.log -> "30 passed (1.1m)" (ran) |
| 5 | "smoke+a11y+theme post-fix: 38 passed (40.2 s)" | L57 | REAL | phase4-exit-playwright-postfix.log -> "38 passed (40.2s)" (ran) |
| 6 | "nightly 136/136 passed ... tolerance UNCHANGED" | L30 | REAL-UNPROVEN | ui/e2e/visual/MATRIX-20261024.md exists; count not re-verified (Playwright run out of budget) |
| 7 | "npm run l10n:check PASS: 472 MessageKeys, 472 en-US strings" | L33 | REAL-UNPROVEN | log exists (phase4-exit-drift-guards.log); not re-run (npm budget) — but baseline independently reports UI gates green |
| 8 | "rebrand suite 7 PASS / 2 BLOCKED / 0 FAIL" | L31 | REAL-UNPROVEN | docs/gates/logs/rebrand-acceptance-4h2.log exists (checked); counts not re-extracted |
| 9 | "`apps/desktop/tauri/INFRA-BLOCKED.md` (H1)" path citation | L34 | FALSE (stale) | `test -e apps/desktop/tauri/INFRA-BLOCKED.md` -> MISSING; file now at apps/desktop-tauri/INFRA-BLOCKED.md (ran; the tree moved after the doc) |
| 10 | "3-OS matrix parked under `.github/workflows-disabled/`" | L103 | FALSE (stale) | `ls .github/workflows-disabled` -> does not exist; 11 active workflows now in .github/workflows/ (ran) |
| 11 | "ClientProvider still mounts MockTransport" (config-propagation PART) | L32 | REAL | consistent with H1 handoff lists; corroborated by the repo's own audit trail; honest non-claim |
| 12 | "npx playwright test: 37/31 — 30 failures are a CONFIG artifact" | L56 | REAL | phase4-exit-playwright.log exists; the wrong-snapshot-dir diagnosis is recorded as gap §4.6 with a fix assignment |
| 13 | "be4e4c6 'lint clean' claim is contradicted by this run — recorded, not hidden" | L51 | REAL | self-reporting a contradiction against an earlier claim: the honest pattern, not the fraud pattern |
| 14 | "first full real execution 2026-11-13, wave-L" (hosted CI) | L262-263 | REAL | corroborated by: (a) worklog archive-2026-10.md L1190-1197 (workflows parked for token reasons, later restored); (b) release.yml's inline live run IDs. CONTRADICTS release.yml:26-27 — see below |
| 15 | "Verdict: PASS WITH REGISTERED CAVEATS — PHASE 5 IS AUTHORIZED" | L216 | REAL (as a recorded decision) | the signature block exists with named caveats; the caveats match the gaps section |
| 16 | "gate's named follow-ups ... discharged by the now-running hosted CI" | L261-263 | SHALLOW | "discharged" is too strong: 3-OS breadth is discharged only insofar as hosted CI exists; the release-lane macOS launch gap (my release.yml F-003) remains — release.yml has no macOS launch test even with runners live |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "the now-running hosted CI (first full real execution 2026-11-13)" (L262-263) | release.yml L26-27: "THIS WORKFLOW HAS NEVER EXECUTED (CI billing blocked): every Actions run in this repository currently dies at 0 steps" | THIS doc + release.yml's own inline run IDs (43378757248 x4, 43464162696 x2, 43729218652 via gen-wxs.sh) are true; release.yml's header is the false statement. CI ran; billing-blocked-every-run is contradicted by six in-file/in-tree live-run citations |

## 4. Fabrication / inflation indicators
NONE found — the opposite: L51 records its own contradiction of a prior "lint clean" claim; §4 lists 8 unfixed gaps; the sign-off carries 5 registered caveats. The only defects are STALE PATHS (L34, L103) — true when written, dead citations now.

## 5. Matrix rows
N/A.
