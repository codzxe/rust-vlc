# W24-R1 — packaging-lane review worklog (read-only audit of w24/c)

Task ID: W24-R1 (wave 24 review scout 1). Branch: `w24/r1` (worktree
`wt/w24r1`, created from `origin/w24/c` @ fb97822). Report:
`docs/reviews/w24-c-packaging-findings.md`. Read-only on the audited
code; no fixes, no merges, no other lanes touched.

## What I did (chronological)

1. Fetched `origin/w24/c`, added worktree `wt/w24r1` on new branch
   `w24/r1`. Audited `git diff origin/main...HEAD` (28 files, +3968/-91).
2. Read every new file end to end: `packaging/packaging.toml`,
   `packaging.schema.json`, all 14 `scripts/packaging/*.sh`,
   `packaging-env.py`, `ps1-sanity.py`, `build-msi.ps1`,
   `test-msi-upgrade.ps1`, `release.yml` (611 lines), `gate.sh`,
   `verify-linux-artifacts.sh`, `full_gate.sh` step-11 diff, both
   evidence logs + evidence README + runbook, `.gitignore` diff,
   worklog entry. Cross-read the reused wave-22 scripts
   (`product-env.py`, `gen-wxs.sh`, `make-app.sh`, `make-dmg.sh`).
3. Executed the checks (all in the worktree):
   - `yaml.safe_load` on release.yml → OK, 6 jobs / 63 steps,
     needs-graph correct.
   - `bash -n` + `sh -n` on all 14 lane scripts → clean.
   - Per-run-block `bash -n` with `${{ }}` neutralized → 41/41 clean.
   - Script-reference existence check → all 8 referenced paths exist.
   - `bash scripts/packaging/gate.sh` → PASS (6/6 steps, 51 env vars).
   - `cargo run -p xtask --locked -- lint-literals` → PASS, 0
     violations / 452 allowlisted.
   - Schema-validator negative test (tamper `algorithm = "md5"`) →
     exit 1 with actionable message; restored → exit 0. Validator is
     real, not decorative.
   - Live-verified all three x86_64 tool pins by download + sha256:
     appimagetool 1.9.1 ✓, type2-runtime 20251108 ✓, WiX 5.0.2 nupkg ✓
     (nupkg layout + `rollForward: Major` claim also verified).
   - shellcheck: not installed in this sandbox (noted; not run).
   - `rg 'secrets\.' .github/workflows/` → zero matches (→ P1-2).
4. Findings triage: 0 P0 / 2 P1 / 4 P2 / 4 P3 — see the report for
   file:line + evidence + suggested fix for each.

## Findings summary

- P1-1 `sign-macos.sh:175` — `$(resolve_identity || true)` masks every
  hard-fail (empty passphrase, import failure, …) into the unsigned
  fallback: exit 0 + sidecar that misattributes the cause to R-35.
- P1-2 `release.yml` — no `secrets.*` → env wiring anywhere; signing
  can never happen in CI even after the user provisions the secrets the
  runbook tells them to set.
- P2-1 `build-msi.ps1:153-156` — .wxs hardcodes LICENSE/README.md vs
  config-driven `[shipped]` docs (silent MSI-only drift).
- P2-2 `release.yml:164` WIX_EXE POSIX path breaks `Test-Path` in the
  pwsh upgrade test at first live run.
- P2-3 evidence README table digests ≠ the log's digests (deb +
  AppImage rows).
- P2-4 "every artifact packages BOTH binaries + docs" is false for the
  AppImage (desktop-only) and the macOS .app.
- P3: `lumen-` temp-prefix literals in the two .ps1 files; signtool
  locator single-SDK-dir probe; sandbox absolute path in the runbook;
  app-zip artifact ships without an unsigned sidecar.

## What held up green

Config plane (schema + semantic pass, negative-tested), literal lint,
gate, workflow YAML + run-block syntax + script refs, tool pins
(live-verified), evidence logs (internally consistent, reproducible
claims, honest [VERIFY] labels, no fake Windows/macOS verification
claims), no stubs/fake signing/checksums, POSIX-sh discipline
(`set -eu`, guarded traps), repo hygiene.

## Notes / deviations

- Disk pressure hit during the audit (rootfs 100%): I removed my own
  downloaded pin-verification files from /tmp and a stray `target/`
  in this worktree; no other worktree or shared state touched.
- Per the task rules my only writes are this worklog and the findings
  report; the repo-root `worklog.md` (live shared log) was NOT
  appended to, since the task scoped my writes to `docs/worklog/`.
- Suggested merge posture (in the report): fix P1-1 + P1-2 first (both
  small); P2s as an immediate follow-up commit; P3s at leisure.
