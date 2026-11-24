# AUDIT: .github/workflows/tauri-launch.yml
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / "functions" (jobs/steps) | 568 / 3 jobs, 29 steps (11+9+9) |
| File verdict | REAL-UNPROVEN (workflow is genuine and every repo-side claim I could check verifies; cannot execute GitHub Actions from sandbox) |
| Reachable from shipped entry point? | N/A — CI lane (workflow_dispatch manual; not in any build path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | sed/rg quotes; python3 yaml.safe_load -> 3 jobs, `on: workflow_dispatch` (exit 0); rg receipt strings in apps/desktop-tauri/src (matches) |

## 1. One-paragraph verdict
This is a competently built, honest-by-design verdict lane, and its load-bearing
claims are TRUE against the repo: the stderr receipt strings it greps are
byte-real (`app.rs:166` `event emit failed on the shell channel`, `main.rs:59`
`lumen-tauri: admission shell failed`), the fixture
`tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` exists (425,516 bytes), the
`windows_subsystem = "windows"` release-subsystem claim matches
`apps/desktop-tauri/src/main.rs:19`, exit faces (0/1/101/143) match main.rs
`std::process::exit(1)` + Rust panic conventions, ci.yml:765 points here, the
`[VERIFY]` marker exists at `ui/src/client/tauri-transport.ts:93`, and the
rc-smoke.sh pattern provenance (set -uo pipefail + `need()`) matches
`scripts/rc-smoke.sh:37,49`. YAML parses (python3). Two claims are false or
unverifiable against the tree: the "ci.yml's link/exit smoke (which accepts
... 86 WEBVIEW_PENDING)" provenance — ci.yml's only smoke (line 744) is an ldd
link check with no bounded run, no exit-code case analysis, and no 86
acceptance — and the cited run IDs. I cannot run the workflow, so the lane's
actual green/red behavior is REAL-UNPROVEN on my authority.

## 2. What actually works (verified)
- `python3 -c yaml.safe_load(...)` -> parses; jobs linux-launch/macos-launch/windows-launch; `on: workflow_dispatch` only (matches "manual by design" header).
- `rg 'event emit failed' apps/desktop-tauri/src` -> app.rs:166 exact string; `rg 'admission shell failed'` -> main.rs:59 exact string; main.rs exits 1 (`std::process::exit(1)` after the receipt).
- `ls tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` -> exists, 425516 bytes.
- `rg 'windows_subsystem' apps/desktop-tauri/src/main.rs` -> line 19 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — the "w28 fix 1" comment claim is real.
- `rg 'tauri-launch' .github/workflows/ci.yml` -> line 765 cross-reference exists.
- `rg 'WEBVIEW_PENDING' apps/desktop/src/main.rs` -> line 21 `pub const WEBVIEW_PENDING: u8 = 86;` (the 86 code itself is real, in the skeleton).

## 3. Findings
F-001 [S3]
Where: .github/workflows/tauri-launch.yml:530
Code:
```
          $receipts = @(Select-String -Path $stderr -Pattern "event emit failed|admission shell failed" -ErrorAction SilentlyContinue)
```
What is wrong: `-ErrorAction SilentlyContinue` swallows file-read failures (e.g.
the redirect handle held by the still-running GUI exe locking $stderr). A
swallowed read error yields `$receipts.Count -eq 0` and prints `receipts: PASS`
— a fail-open verdict on the lane's core assert. The bash lanes use plain
`grep -q` (line 227), whose read failure returns non-zero and would land in the
"FAILED" branch — fail-closed there, fail-open here. Why it matters: the whole
point of the lane is zero-failure-receipts; a silent false PASS corrupts the
verdict. What correct looks like: read with explicit error handling; on read
error set `$status = 1` and print a distinct receipt `UNREADABLE` line.
Confidence: MEDIUM — Verification: read (Windows file-sharing semantics not
exercised here).

F-002 [S4]
Where: .github/workflows/tauri-launch.yml:68-71
Code:
```
# SMOKE PATTERN PROVENANCE: numbered checks + honest fail counter +
# preflight `need()` from scripts/rc-smoke.sh; bounded-run exit-code
# case analysis from ci.yml's link/exit smoke (which accepts the
# apps/desktop skeleton's 86 WEBVIEW_PENDING code on a display-less
```
What is wrong: ci.yml contains no bounded-run exit-code smoke and nothing that
accepts exit 86. ci.yml's only smoke step is "Smoke — the admission binary's
runtime link set resolves (no-DISPLAY lane)" (ci.yml:744) — an `ldd` check whose
own comment (ci.yml:761-763) says the round-14 run died at gtk init with panic
101 and "there is no typed 86 'webview-pending' state in THIS binary". The
provenance claim misdescribes ci.yml; the 86-accepting smoke (if it ever
existed) is not in the tree. Why it matters: a provenance claim used to justify
this lane's assert shape is unverifiable/false against the repo. What correct
looks like: cite the actual step (ldd smoke, ci.yml:744) or the historical
workflow version that accepted 86. Confidence: HIGH — Verification: read + rg.
F-duplicate in docs/packaging/evidence/w26m-t5-launch-verdict.md:55 (same
sentence — that file is another agent's domain).

F-003 [S4]
Where: .github/workflows/tauri-launch.yml:227-233 (linux), 391-397 (macos), 530 (windows)
Code (linux):
```
          if grep -q "event emit failed" "$LOGDIR/stderr.log" \
             || grep -q "admission shell failed" "$LOGDIR/stderr.log"; then
```
What is wrong: the receipts grep runs after the survival poll but BEFORE
termination and `wait` — receipts printed during shutdown (emit failures while
the TERM'd app tears down) are never observed, though the header claims "across
the run". The header's own "Known blind spot" note covers post-traffic web
crashes but not shutdown-time emit failures. Why it matters: minor coverage gap
on the core assert. What correct looks like: re-grep stderr after `wait` and
fold into status. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: .github/workflows/tauri-launch.yml:213-216, 241-242
Code:
```
            # The harness must SURVIVE its own kill: on the round-1 run
            # (43410313414) both the linux and macos verdicts passed
```
What is wrong: run IDs 43410313414 / 43412067856 are cited as evidence of past
behavior but are not verifiable from the repo (no GH API access, agents cannot
run git). The fixes they motivate (`trap '' TERM`, errexit guard on `wait`) are
present and technically sound. Why it matters: claims of executed runs rest on
uncheckable numbers. Confidence: HIGH — Verification: read; run IDs UNVERIFIED.

## 4. Per-job/step table (workflow "functions")
| Job/Step | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header comment block (claims) | 1-81 | mostly TRUE | S4 | 2 claims false/unverifiable (F-002, F-004) |
| `on: workflow_dispatch` + perms `contents: read` | 84-93 | REAL | — | minimal-privilege, manual-only; matches header |
| linux: toolchain pin assert | 101-115 | REAL | — | same assert as ci.yml (byte-equal) |
| linux: apt webkit2gtk + pkg-config assert | 116-131 | REAL | — | assert present |
| linux: xvfb/fonts env | 132-137 | REAL | — | separated from build deps as claimed |
| linux: npm ci + build + dist assert | 138-147 | REAL | — | ordering rule (UI before cargo) is real: frontendDist embedded at compile |
| linux: rust-cache (2 workspaces) | 148-154 | REAL | — | matches "no CARGO_TARGET_DIR override" claim |
| linux: cargo build release locked | 155-156 | REAL | — | subcrate manifest path |
| linux: launch verdict script | 157-262 | REAL-UNPROVEN | S3/S4 | sound bash; F-001 sibling F-003; `set -uo pipefail` + status counter works |
| linux: artifact upload if:always() | 263-270 | REAL | — | `if-no-files-found: error` — fail-closed |
| macos: build + launch attempt | 272-399 | REAL-UNPROVEN | — | same shape, direct exec; honest BEST-EFFORT-PENDING echo |
| macos: artifact upload | 400-407 | REAL | — | same |
| windows: toolchain/build steps | 409-483 | REAL | — | same pins |
| windows: pwsh launch attempt | 484-555 | REAL-UNPROVEN | S3 | F-001 fail-open receipts; pwsh `[VERIFY at first run]` marker honest |
| windows: artifact upload | 556-568 | REAL | — | same |

## 5. Test honesty
No tests in this file; the lane IS a test. Mutation reasoning: killing
`app.rs:166`'s eprintln would flip receipts-PASS to still-PASS (absence
assert) — the lane detects the presence of failure, not absence of success; it
cannot detect a webview that never renders (admitted in header: "What it does
NOT prove"). The 12s survival poll WOULD catch a main.rs exit-1 (died_at set,
status=1). Honest posture overall.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| receipt strings + exit 1 on shell failure | header:13-16 | REAL | app.rs:166, main.rs:59 + exit(1) |
| fixture on argv, file-open seam | header:18-19 | REAL | fixture exists; argv main.rs accepts path |
| 101 = panic face | header:25 | REAL | Rust default panic exit code (convention, not run) |
| windows_subsystem = windows in release | windows job comment | REAL | main.rs:19 |
| ci.yml tauri-check points here | header:8-9 | REAL | ci.yml:765 |
| [VERIFY] marker + clearing procedure | header:43-62 | REAL | ui/src/client/tauri-transport.ts:93 |
| "ci.yml's link/exit smoke ... accepts 86 WEBVIEW_PENDING" | header:68-71 | FALSE (vs current ci.yml) | ci.yml:744 ldd-only; ci.yml:761-763 denies 86 for this binary |
| run IDs 43410313414/43412067856 behavior | 213-216 | UNVERIFIED | no GH access |

## 7. Dead / duplicated / unreachable code
The toolchain-pin assert and the UI-build step are triplicated across the three
jobs (19 near-identical lines ×3) — acceptable workflow idiom, noted for the
record. Nothing dead.

## 8. Dependencies & flags
Pinned actions with comment SHAs: checkout@3d3c42e5 (v7.0.1), dtolnay/rust-toolchain@d124f86f
(matches rust-toolchain.toml channel per ci.yml assert), setup-node@2028fbc5 (v6.0.0),
rust-cache@6323deb (v2.9.2), upload-artifact@cf430e0 (v7.0.2). All versioned-pinned —
no floating tags. Env: CARGO_INCREMENTAL=0, CARGO_TERM_COLOR only — the
"NO CARGO_TARGET_DIR override" claim is TRUE.

## 9. Open questions for the next agent
- Verify with GH run 43410313414/43412067856 (orchestrator has network?) that
  the round-1/2 failure story in the comments is real.
- docs/packaging/evidence/w26m-t5-launch-verdict.md:55 duplicates the false
  "ci.yml's link/exit smoke ... accepts 86" sentence — flag when that file is
  audited.
- The `trap '' TERM` / `wait` dance is correct for bash but untested here;
  first live re-run should confirm exit classification (143 vs 137).
