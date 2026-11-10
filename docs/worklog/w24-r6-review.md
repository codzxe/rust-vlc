# W24-R6 — packaging fix-lane review worklog (read-only audit of w24/h)

Task ID: W24-R6 (wave 24 review scout 6). Branch: `w24/r6` (worktree
`wt/w24r6`, created from `origin/w24/h` @ 45c7136). Report:
`docs/reviews/w24-h-fixes-findings.md`. Read-only on the audited code
(the W24-H fixer lane); no fixes, no merges, no other lanes touched.

Question: are all 10 W24-R1 findings (`docs/reviews/w24-c-packaging-findings.md`
on `w24/r1` @ cfc196c) actually fixed on `origin/w24/h` (7 commits on
`origin/w24/c` @ fb97822), with no regressions and no honesty downgrade?

## What I did (chronological)

1. Fetched `origin/w24/h`, added worktree `wt/w24r6` on new branch
   `w24/r6`. Read the R1 findings doc, the fixer's
   `docs/worklog/w24-h-review-fixes.md` + root worklog entry, then the
   full fix diff `git diff origin/w24/c...origin/w24/h` (11 files,
   +609/−56).
2. **P1-1 (sign-macos.sh)**: traced the new rc contract end to end
   (function runs in the MAIN shell; 0 = resolved, 2 = no creds, die()
   exits the script; caller hard-exits on any third rc). Built an
   independent negative-test harness (fake `codesign`/`security` on
   PATH, real packaging env) and ran it against BOTH the new script and
   the pre-fix script extracted from `origin/w24/c`:
   - new: no-creds → rc 0 + sidecar; creds-present failures (empty
     passphrase, import failure, bad base64) → rc 1, NO sidecar, TMPDIR
     empty (trap cleaned the scratch);
   - injected `return 3` into a /tmp copy → the "refusing to fall back"
     guard fires (rc 1, no sidecar);
   - old: same rc/sidecar outcomes, but the scratch `tmp.*` dir LEAKED
     on failure paths (subshell-local assignments) — the fix's cleanup
     claim reproduced;
   - minimal repro of the old `x=$(f || true)`-with-explicit-exit
     pattern in dash + bash 5 + `bash --posix`: rc 1, parent does NOT
     continue (die's `exit` terminates the subshell before `|| true`
     runs; `set -e` then kills the parent on the failed assignment) —
     the fixer's honest note that R1's literal masking mechanism did not
     reproduce is accurate; the real pre-fix bugs were the scratch leak
     + the fragile implicit contract.
3. **P1-2 (release.yml secrets)**: cross-checked TOML ↔ workflow myself
   (8 `[signing.*]` names; 22 mappings, all `NAME: ${{ secrets.NAME }}`;
   windows MSI step 2/2, both macOS dmg steps 6/6, release visibility
   step 8/8; no other `secrets.*` refs; no literal credentials).
   Simulated the exact guard body under `bash -eo pipefail`: empty
   (absent secret) passes, unset (renamed/stale) errors + exits 1.
   Verified prerequisites: `defaults.run.shell: bash` pre-existing;
   parse step precedes guarded steps in every job; `--github-env` emits
   all 8 bridge vars even in the release job's no-bins mode (49 vars);
   schema pins signing env names to `^[A-Z][A-Z0-9_]*$`; upgrade test
   builds with `-SkipSigning`.
4. **P2-1**: ran gen-wxs.sh functionally — real config → `Doc_1_LICENSE`
   (KeyPath) + `Doc_2_README_md`, XML well-formed; 4-doc list → 4 rows;
   duplicate basenames (two forms) → rc 1, no output; empty/unset →
   loud. Confirmed build-msi.sh now stages docs; build-msi.ps1 asserts
   `PACKAGING_DOCS` and collision-checks before staging; ps1-sanity
   PASS. Noted the bash twin stages before gen-wxs refuses (same loud
   end state) and the unescaped doc-basename XML nit (P4-2).
5. **P2-2**: verified the conversion placement (param default → required
   check → convert → Test-Path), regex form, passthrough for non-drive
   paths, `[VERIFY]` label; `test-msi-upgrade.ps1` consumes WIX_EXE only
   via build-msi.ps1.
6. **P2-3**: grep-verified all 5 table digests in
   `linux-lane-20261110.log` (lines 91–98), both stale digests absent,
   SBOM size 121,900 B ×4 backed (lines 79–88), r3-narrative sizes trace
   to `w24c-r3.log` (lines 180–186).
7. **P2-4**: cross-checked the payload table against the builders
   (stage-linux-payload/build-zip/build-targz/build-deb/build-rpm/
   build-msi = both + docs; build-appimage.sh = BIN_DESKTOP only;
   make-app.sh = desktop + hardcoded doc pair, drift flagged).
8. **P3-1/P3-2/P3-3/P3-4**: literal grep on both .ps1 (only sanctioned
   cargo ids remain); signtool walk replica test on the ACTUAL extracted
   old/new function bodies against a fake SDK tree `{10.0.22621.0/x86/
   signtool.exe, uap10.0.17763.0, x86}` — old gives up, new finds it;
   runbook sandbox-path annotation verified; zip-sidecar copy verified in
   both macOS jobs (+ dmg lands in dist/ natively).
9. **Regression battery**: `bash -n` + `sh -n` on all 14 packaging
   scripts + gen-wxs.sh clean; `yaml.safe_load` OK (6 jobs, 42
   run-blocks — base 41, +1 visibility step); 42/42 run-block `bash -n`
   with `${{ }}` neutralized; `source /home/z/my-project/scripts/
   cargo-env.sh && bash scripts/packaging/gate.sh` → **PASS** (6/6);
   `cargo run -p xtask --locked -- lint-literals` → **PASS, 0 violations,
   452 allowlisted**; ps1-sanity PASS on both .ps1; no `.rs` files in
   the diff; tree clean.
10. **Honesty check**: zero removed `[VERIFY]` lines in the diff
    (build-msi.ps1 gained one); overclaim sweep over added lines found
    only the worklog's within-bounds "fixed and verified" (qualified by
    the unchanged never-executed boundary); the fixer's `|| true`
    non-reproduction note independently confirmed accurate.

## Findings summary

10/10 fix-verified; 0 incomplete; 0 regressions; 0 new P1–P3.
4 new P4 observations (optional hardening): future-proofing the wiring
guards against added (not renamed) secret names; escaping doc basenames
in the .wxs; a stale "41 run-blocks" count in the runbook; the
space-separated PACKAGING_DOCS contract (pre-existing). See the report
for per-finding evidence and the merge posture (merge-ready; first live
run = the documented workflow_dispatch dry-run).

No merges, no pushes to other branches, no code changes on the audited
lane. Only writes: this file + the report. Pushed `w24/r6`.
