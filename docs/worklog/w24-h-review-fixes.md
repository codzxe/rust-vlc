# W24-H — packaging-lane review-fix worklog (W24-R1 findings fixed on w24/h)

Task ID: W24-H (rounds 1–7; this file closes r7). Branch: `w24/h`
(worktree `wt/w24h`, based on `origin/w24/c`). Fixes the 10 findings of
review W24-R1 (`docs/reviews/w24-c-packaging-findings.md`, commit
`cfc196c` on `w24/r1`: 0 P0 / 2 P1 / 4 P2 / 4 P3). This file is the
per-finding before/after evidence record; the standard-format entry is
appended to the repo-root `worklog.md` (AGENTS.md §4). `docs/worklog/`
placement is per this task's explicit scope (same precedent as the
W24-R1 review scout's own entry).

Fix commits on `w24/h` (base `origin/w24/c` = fb97822):

| commit | findings |
|---|---|
| 1f19159 | P1-1 sign-macos.sh subshell masking |
| 1337ca0 | P1-2 release.yml secrets wiring |
| e4725d0 | P2-1, P2-2, P3-1 (MSI lane: wxs docs, WIX_EXE handoff, temp prefixes) |
| 00821d3 | P2-3 evidence digests |
| f6ebd7e | P2-4 payload claims |
| 7f2f894 | P3-2, P3-3, P3-4 (signtool walk, runbook path, zip sidecar) |

Files touched (9): `.github/workflows/release.yml`,
`docs/packaging/README.md`, `docs/packaging/evidence/README.md`,
`scripts/packaging/build-msi.ps1`, `scripts/packaging/build-msi.sh`,
`scripts/packaging/sign-macos.sh`, `scripts/packaging/sign-windows.sh`,
`scripts/packaging/test-msi-upgrade.ps1`, `scripts/release/gen-wxs.sh`.

---

## P1-1 — sign-macos.sh `$(resolve_identity || true)` masks resolution failures

**Before** (`origin/w24/c`): identity resolution ran inside a command
substitution with `|| true`; the no-credentials case was `return 1`
(indistinguishable, by status alone, from a failure) and the scratch
`work`/`scratch_keychain` assignments lived only in the subshell.

**After** (1f19159): `resolve_identity` runs in the MAIN shell behind
an explicit return-code contract — `0` identity resolved (global
`resolved_identity`), `2` NO credentials (the honest R-35 unsigned
path), anything else already `die`d. The caller treats any third value
as a hard exit with an explicit refusal message; the docs-component of
the fix also means `die` exits the script itself, so the EXIT trap sees
the scratch state and cleans it.

**Evidence (r7 re-verification, this worktree):**
- Credentials absent → gate.sh step 6 (run this round): exit 0 +
  `.UNSIGNED.txt` sidecar written — the honest path is unchanged.
- Credential present + step fails (fake `codesign`/`security` on PATH,
  real script): exit 1, NO sidecar, TMPDIR left EMPTY (trap cleaned the
  scratch work dir).
- Same scenario on the pre-fix script (extracted from
  `origin/w24/c`): exit 1 too (see honest note) but the scratch
  `tmp.*` work dir was LEFT BEHIND — the subshell assignment died with
  the subshell and the trap saw `work=""`.
- **Honest note on the review's mechanism claim**: the literal
  "`|| true` swallows the die() status → unsigned sidecar" path did
  NOT reproduce in this sandbox's `sh` (dash) or bash 5 / `bash
  --posix` — for `die()` exits, `set -e` on the failed assignment
  still killed the script (rc 1, no sidecar, on both old and new
  code). What was reproducibly broken before the fix: (a) the scratch
  cleanup was dead (leak proven above; on a real runner that is the
  p12 scratch dir + keychain-db per failed build), and (b) the
  no-creds vs broken-creds distinction rested entirely on
  set -e-through-assignment semantics with no explicit contract — any
  refactor of the call site into a checked context (`if`, `&&`, `||`)
  would have made the masking real. The fix makes the behavior
  explicit and locally proven instead of accidental.

## P1-2 — release.yml had zero `secrets.*` mappings

**Before** (review W24-R1): `rg 'secrets\.' .github/workflows/` → no
matches; the runbook's "set the repository secrets" next-step changed
nothing in CI — releases stayed unsigned forever, silently.

**After** (1337ca0): all eight configured names mapped in the steps
that sign — windows MSI step (2), both macOS dmg steps (6 each) — plus
a release-job "Signing-credentials visibility" step mapping all 8 once
more: UNSET = wiring gap (::error::, hard fail), EMPTY = absent
credential (::warning:: naming the blockers-table owners).

**Evidence (r7):**
- TOML↔workflow cross-check script: the 8 `[signing.*] *_env` names in
  `packaging/packaging.toml` = exactly the 8 `NAME: ${{ secrets.NAME }}`
  mappings; no missing, no extras.
- Guard-logic simulation (exact guard body, bash as the workflow's
  `defaults.run.shell: bash` guarantees): mapped-but-absent (empty
  string) → passes (honest-unsigned unchanged); renamed-in-config,
  stale mapping (unset) → `::error::` + exit 1.
- `packaging-env.py --print` confirms all 8 `PACKAGING_SIGNING_*_ENV`
  bridge vars the guards read are exported by the parse step that
  precedes them in every job.
- 42/42 `run:` blocks parse under `bash -n` with `${{ }}` neutralized
  (41 before this lane; the visibility step is the +1).

## P2-1 — .wxs hardcoded the docs list vs config-driven `[shipped]` docs

**Before**: `build-msi.ps1`'s WiX template pinned
`<File Source="LICENSE">`/`<File Source="README.md">`; `gen-wxs.sh`
(the bash twin) the same. A docs-list change in packaging.toml reached
tar.gz/zip/deb/rpm/AppImage but silently not the MSI.

**After** (e4725d0): both generators emit one `<File>` row per
`PACKAGING_DOCS` entry (first doc = component KeyPath; ids sanitized +
positionally prefixed); both MSI builders stage the same list into the
flat staging dir and REFUSE duplicate basenames loudly; empty list =
hard error; `PACKAGING_DOCS` is a required var (`:?`) everywhere.

**Evidence (r7):** real generation with the repo config →
`Doc_1_LICENSE` (KeyPath) + `Doc_2_README_md`; with a 4-doc list → 4
rows; `PACKAGING_DOCS="LICENSE docs/x/LICENSE"` → loud duplicate-
basename refusal (also accidentally re-proven by
`"LICENSE README.md docs/packaging/README.md"` → refusal, as
designed); empty list → hard error. `.ps1` half: ps1-sanity PASS
(structural; no pwsh in this sandbox — real parse is a Windows-runner
[VERIFY], marked in the file).

## P2-2 — WIX_EXE POSIX path breaks the pwsh upgrade test

**Before**: the bash WiX-install step exported `WIX_EXE=/d/a/...`;
`Test-Path` in `build-msi.ps1` resolves a leading `/` against the
current drive → `Fail "WixExe not found"` at the first live
upgrade-test run.

**After** (e4725d0): `build-msi.ps1` converts the git-bash drive-letter
form (`/d/...` → `D:\...`) before `Test-Path`; other forms pass
through untouched; the conversion prints what it did. `[VERIFY at first
live run]` marked in-file.

**Evidence (r7):** ps1-sanity PASS on the changed file (ASCII-only,
delimiters/here-strings balanced, `param(` first). No pwsh exists in
this sandbox — the conversion itself is a Windows-runner [VERIFY], and
the file says so. Bash consumers were and are unaffected
(`build-msi.sh` takes the same env var natively).

## P2-3 — evidence table digests contradicted the log

**Before**: `docs/packaging/evidence/README.md` listed `.deb` →
`b3735fdc84d2…` and `.AppImage` → `5bd0ff0dd309…` (stale, from an
intermediate run); the committed log records `a39dafa88039…` /
`c0b86e2984b1…`.

**After** (00821d3): table regenerated from the committed log's step-9
manifest; package byte sizes replaced with `—` (the log records
digests + SBOM sizes only — no size is claimed that the log cannot
support).

**Evidence (r7)**: all five table digests (`3b5eae753466`,
`a39dafa88039`, `c0b86e2984b1`, `17bd1068c818`, `f0b22c1bc2a8`) grep
FOUND in `linux-lane-20261110.log`; both stale digests confirmed
absent from it.

## P2-4 — "every artifact packages BOTH binaries + docs" was false

**Before**: the runbook's artifact-matrix preamble and the workflow
header claimed every artifact carries both binaries + docs; the
AppImage ships the desktop binary ONLY (4 files: AppRun + entry + icon
+ binary) and the macOS `.app` ships desktop + hardcoded
LICENSE/README.

**After** (f6ebd7e): a per-artifact payload table in
`docs/packaging/README.md` (tar.gz/zip/deb/rpm/msi = both + docs;
AppImage = desktop-only GUI-shell channel; macOS .app/.dmg/app-zip =
desktop + doc pair, with the make-app.sh hardcode flagged as drift vs
`[shipped]`); the release.yml header states the same.

**Evidence (r7)**: `build-appimage.sh` requires/stages only
`BIN_DESKTOP` (lines 36/64/72) — the table's desktop-only claim
cross-checked against the builder.

## P3-1 — product-name literals in scratch paths

**Before**: `"lumen-msi-"` / `"lumen-upgrade-test-"` temp prefixes
hardcoded in the two .ps1 files under a "ZERO IDENTITY LITERALS"
banner.

**After** (e4725d0): `"$($env:PRODUCT_LOWER)-msi-"` /
`-upgrade-test-` (the env var is guaranteed by the assertion block
above each use).

**Evidence (r7)**: grep for `lumen` in both .ps1 files → only the two
sanctioned cargo package ids (`lumen-cli`, `desktop` — the documented
exception, same as release.yml).

## P3-2 — signtool locator probed only the lexically-newest SDK dir

**Before**: `ls | sort -V | tail -1` picks one directory; non-version
dirs (`x86`, `uap10.0.*`) can sort "newest", so the probe could give
up while an older version dir still holds `signtool.exe`.

**After** (7f2f894): walks ALL version dirs newest-first (`sort -Vr`),
x64 then x86 in each; `SIGNTOOL` env override unchanged.

**Evidence (r7)**: side-by-side replica test (same loop bodies,
parameterized base) against a fake SDK bin tree
{`10.0.19041.0`, `10.0.22621.0/x86/signtool.exe`, `uap10.0.17763.0`,
`x86`}: OLD logic → gives up (rc 1); NEW logic → finds
`10.0.22621.0/x86/signtool.exe`. (The real script's bases are
Windows-runner paths — the loop logic is what is locally provable, and
it is what changed.)

## P3-3 — runbook hardcoded the sandbox absolute path

**Before**: bare `source /home/z/my-project/scripts/cargo-env.sh` in
the repo-level runbook.

**After** (7f2f894): the line is explicitly marked a build-sandbox-only
helper (shared CARGO_TARGET_DIR pin for parallel worktrees) with a
"skip this line anywhere else" note.

## P3-4 — macOS app-zip shipped without an unsigned sidecar

**Before**: `sign-macos.sh --app` writes the sidecar beside the bundle
in `bundle/`, but the workflow uploads `dist/**` only — the ditto'd
app-zip was the one asset whose unsigned status was not self-describing.

**After** (7f2f894): both macOS jobs' zip steps copy
`bundle/<Name>.app.UNSIGNED.txt` → `dist/<base>.zip.UNSIGNED.txt` when
it exists (a signed .app has no sidecar and needs none).

---

## Verification battery (r7, all in `wt/w24h` at HEAD 7f2f894)

| check | result |
|---|---|
| `bash -n` on every touched shell script (sign-macos.sh, sign-windows.sh, build-msi.sh, gen-wxs.sh) | clean; `sh -n` also clean on the POSIX `#!/bin/sh` files; sweep of all 14 `scripts/packaging/*.sh` under `bash -n` clean |
| `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` | OK (42 run-blocks, 6 jobs) |
| per-run-block `bash -n` with `${{ }}` neutralized | 42/42 clean |
| `cargo run -p xtask --locked -- lint-literals` | PASS — 0 violations, 452 allowlisted |
| `bash scripts/packaging/gate.sh` (= full_gate step 11 body) | **PASS** — 51 env vars, 14 scripts, python ast, ps1 sanity, stagers, both unsigned-fallback contracts executed for real |
| `ps1-sanity.py` on `build-msi.ps1` + `test-msi-upgrade.ps1` (and all `scripts/packaging/*.ps1`) | PASS (ASCII-only, delimiters balanced, `param(` first) |
| TOML↔workflow secret-name cross-check | 8/8 mapped, no missing, no extras |
| P1-1 behavior tests (absent creds / broken creds, old vs new script) | contract holds; leak fixed (evidence above) |
| P2-1 gen-wxs functional + negative tests | config-driven rows; duplicate/empty refused loudly |
| P2-3 digest cross-check | 5/5 table digests in the log; stale pair absent |
| P3-2 walk-logic replica, old vs new | old gives up, new finds |

## Honest limits (unchanged boundary, restated)

- No pwsh, no Windows/macOS, no CI execution in this sandbox: the wix
  build, signtool, codesign/notarytool/stapler, msiexec, ditto and the
  workflow itself remain `[VERIFY at first live run]` — every such
  site is labeled in-file. Nothing here claims otherwise.
- The workflow has still never executed (CI billing blocked,
  documented in the runbook blockers table).
- No Rust code touched on this branch (delta vs origin/w24/c is
  scripts/workflow/docs only), so fmt/clippy were not owed.
