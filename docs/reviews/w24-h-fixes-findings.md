# W24-H fix-verification findings (audit W24-R6)

- **Auditor**: W24-R6 (wave 24 review scout 6), read-only independent check of
  `origin/w24/h` (the W24-H review-fixer lane, 7 commits on `origin/w24/c`).
- **Audited diff**: `git diff origin/w24/c...origin/w24/h` — 11 files
  (9 code/doc + 2 worklog records): `sign-macos.sh`, `sign-windows.sh`,
  `build-msi.sh`, `build-msi.ps1`, `test-msi-upgrade.ps1`, `gen-wxs.sh`,
  `release.yml`, `docs/packaging/README.md`, `docs/packaging/evidence/README.md`.
- **Question**: are all 10 W24-R1 findings (2 P1 / 4 P2 / 4 P3) actually fixed,
  with no regressions and no honesty downgrade?
- **Verdict**: **all 10 fixes VERIFIED — 10/10 fix-verified, 0 incomplete, 0
  regressions, 0 new P1–P3 findings.** The fixer's self-verification claims
  reproduced under independent re-execution, including the two P1s (negative
  tests re-run from scratch, not trusted from the worklog). 4 new P4
  observations (hardening nits, none merge-blocking) listed at the end.

**Severity counts: P0=0, P1=0, P2=0, P3=0, P4=4 (all new observations, all optional).**

---

## P1-1 — sign-macos.sh identity-resolution masking → **FIX VERIFIED**

**The new contract** (`sign-macos.sh:126–200`): `resolve_identity()` runs in
the MAIN shell (no command substitution), behind an explicit return-code
contract — `0` identity resolved (global `resolved_identity`), `2` NO
credentials (the honest R-35 unsigned path), anything else already `die`d.
The caller inspects the rc directly; any third value is an explicit
`exit 1` "refusing to fall back to the unsigned path" (defense in depth —
verified to fire, see below).

**Control-flow trace (creds present + import fails)**: `identity` empty,
`p12_b64` set → skips `return 2` → `command -v codesign/security` `|| die`
→ p12 decode `[ -s ] || die` → create/unlock/import/set-partition-list/
search-list each `|| die` → `die` = `exit 1` in the **main shell** → EXIT
trap `cleanup` sees `work`/`scratch_keychain` (assigned in the main shell
now) and removes the scratch dir / deletes the keychain → **exit 1, no
sidecar** (sidecars are written only at lines 206/242, never reached).

**Independent negative tests** (fake `codesign`/`security` on PATH, real
script, real packaging env — my own harness, not the fixer's):

| scenario | rc | sidecar | TMPDIR after |
|---|---|---|---|
| no creds | 0 | PRESENT (R-35 prose) | empty |
| p12 set + password empty | 1 | none | empty |
| p12 set, import step fails | 1 | none | empty (trap cleaned) |
| p12 not valid base64 | 1 | none | empty |
| injected `return 3` (copy of script) | 1 | none | — "refusing to fall back" message |

Same battery on the **pre-fix** script (extracted from `origin/w24/c`):
same rc/sidecar outcomes, **but the scratch `tmp.*` work dir was left
behind** (1 entry) on the failure paths — the subshell-local
`work`/`scratch_keychain` assignments died with the subshell, so the trap
saw `work=""`. The fix removes that leak (proven above).

**On the fixer's honest note** (the `|| true` mechanism "did not
reproduce"): **accurate — independently confirmed.** Minimal repro of the
old pattern (`x=$(f || true)` with `die(){ …; exit 1; }` inside `f`) in
dash, bash 5, and `bash --posix`: rc 1, parent does NOT continue — because
`die`'s explicit `exit` terminates the command-substitution subshell
*before* the `||` operand can run, the substitution fails, and `set -e`
kills the parent on the failed assignment. So W24-R1's literal mechanism
narrative ("`|| true` swallows the status → unsigned sidecar with false
reason") was wrong for die()-based failures; what was really broken
pre-fix was (a) the scratch/keychain cleanup (leak proven above — on a
real runner that is the p12 scratch dir + keychain-db per failed build)
and (b) an implicit, refactor-fragile contract. W24-R1's *fix direction*
(explicit rc contract) is exactly what shipped. The reframing is honest
and the new contract is locally proven instead of accidental.

**Residual nuances (checked, not findings)**: the `|| die` on the
find-identity substitution can only catch `head`'s status (POSIX sh, no
pipefail) — the real failure mode (empty result) is caught by the
`[ -n "$resolved_identity" ] || die` guard one line later; both die.
`work=$(mktemp -d)` / `keychain_pw="$(uuidgen)"` are unguarded, and
`set -e` is suppressed inside a function invoked in a `||` context — but
their failure modes fall through to the `[ -s cert.p12 ] || die` /
`security … || die` guards. No silent path found.

## P1-2 — release.yml zero secrets mappings → **FIX VERIFIED**

**TOML ↔ workflow cross-check (mine)**: `packaging.toml [signing.*]`
declares exactly 8 env names (2 windows + 6 macOS). The workflow contains
**22 mappings, all in `NAME: ${{ secrets.NAME }}` identity form** —
windows MSI step (2/2), both macOS dmg steps (6/6 each), release-job
"Signing-credentials visibility" step (all 8). No other `secrets.*`
references exist; **no secret values inlined** (literal-credential sweep
clean). The visibility step maps all 8 a second time at release assembly:
UNSET = wiring gap (`::error::` + exit 1), EMPTY = absent credential
(`::warning::` naming the blockers-table owners).

**Guard semantics (simulated with the exact guard body under
`bash -eo pipefail`)**: mapped-but-absent secret (empty string) → passes
(honest-unsigned path unchanged); renamed-in-config/stale mapping (unset)
→ `::error::` + exit 1. Both directions verified.

**Prerequisites verified**: (a) `defaults.run.shell: bash` is workflow-wide
and pre-existing (not touched by the fix) — so the `${!cfg}` /
`${!name+x}` bash indirection is legal on every runner including git-bash
on windows-latest; (b) every job's parse step (`packaging-env.py
--github-env` → `$GITHUB_ENV`) runs **before** the guarded steps —
verified `--github-env` emits all 8 `PACKAGING_SIGNING_*_ENV` bridge vars
even in the release job's no-bin-names mode (49 vars); (c) the schema
pins signing env names to `^[A-Z][A-Z0-9_]*$`, so the indirection target
is always a legal variable name; (d) the only signing consumers are
covered: MSI step → `build-msi.sh` → `sign-windows.sh`; dmg steps →
`build-dmg.sh` → `sign-macos.sh` ×2; the upgrade-test step builds with
`-SkipSigning` (no secrets owed).

## P2-1 — .wxs hardcoded docs list → **FIX VERIFIED**

`gen-wxs.sh` now emits one `<File>` row per `PACKAGING_DOCS` entry (first
doc = component KeyPath; ids positionally prefixed + sanitized;
`PACKAGING_DOCS` required via `:?`). Functional tests (mine, real script
+ real config): 2-doc config → `Doc_1_LICENSE` (KeyPath) +
`Doc_2_README_md`, XML well-formed (minidom); 4-doc list → 4 rows;
duplicate basenames (`LICENSE docs/other/LICENSE` and
`README.md docs/packaging/README.md`) → loud refusal rc 1, **no output
file**; empty list → rc 2 loud; unset → rc 2 loud. `build-msi.sh` now
stages the docs list into the MSI staging dir (the latent
wix-build-breaks-at-first-live-run bug the fixer found while fixing —
real catch, real fix). `build-msi.ps1` generates the same rows with the
collision check *before* staging, and `PACKAGING_DOCS` joined the
required-env assertion list; ps1-sanity PASS, `[VERIFY at first live run]`
labeled (no pwsh in this sandbox — honestly stated).

Minor asymmetry (not a defect): in `build-msi.sh` the staging loop runs
before `gen-wxs.sh`, so a duplicate-basename list clobbers one file in
staging first and is then refused by gen-wxs — end state is the same loud
failure, one step later than the .ps1 twin.

## P2-2 — WIX_EXE POSIX path vs pwsh Test-Path → **FIX VERIFIED (structural; [VERIFY] labeled)**

`build-msi.ps1` converts the git-bash drive-letter form
(`/d/a/.../wix.exe` → `D:\a\...\wix.exe`) after the param/env resolution
and required-check, before `Test-Path`; other forms pass through
untouched; the conversion prints what it did; marked
`[VERIFY at first live run]` in-file (no pwsh exists in this sandbox —
boundary honestly stated, unchanged). The workflow still exports
`WIX_EXE=${PWD}/...` from bash (correct for the bash consumers);
`test-msi-upgrade.ps1` consumes it only via `build-msi.ps1`, which now
converts — the consumer-side fix covers every pwsh path. Bash consumers
unaffected.

## P2-3 — evidence table digests vs the log → **FIX VERIFIED**

All five table digests (`3b5eae753466`, `a39dafa88039`, `c0b86e2984b1`,
`17bd1068c818`, `f0b22c1bc2a8`) grep-FOUND in
`docs/packaging/evidence/linux-lane-20261110.log` (step-9 manifest,
lines 91–98); both stale digests (`b3735fdc84d2`, `5bd0ff0dd309`)
confirmed ABSENT from it. The four unsupported package byte sizes were
honestly dropped to `—`; the retained SBOM size (121,900 B ×4) is backed
by log lines 79–88. The r3-narrative sizes at README line 117 trace to
the *other* committed log (`w24c-r3.log` `ls -l` lines 180–186) — every
remaining size/digest claim in the evidence README now traces to a
committed log.

## P2-4 — "every artifact packages BOTH binaries + docs" → **FIX VERIFIED**

Replaced by a per-artifact payload table (README) + matching release.yml
header. Cross-checked against the builders: `stage-linux-payload.sh` /
`build-zip.sh` / `build-targz.sh` / `build-deb.sh` / `build-rpm.sh` /
`build-msi.sh` = both bins + docs (the MSI row is now *actually true*
thanks to the P2-1 fix, not just documented); `build-appimage.sh`
requires/stages only `BIN_DESKTOP` (lines 36/64/72 — desktop-only row
correct); `scripts/release/make-app.sh` = desktop + hardcoded
LICENSE/README, and the table **flags that hardcode as drift** vs
`[shipped]` instead of hiding it. No overclaim remains.

## P3-1 — product-name literals in scratch paths → **FIX VERIFIED**

`"$($env:PRODUCT_LOWER)-msi-"` / `"$($env:PRODUCT_LOWER)-upgrade-test-"`
in the two .ps1 files; `rg -i lumen` on both now hits only the two
sanctioned cargo package ids (`lumen-cli`, `desktop`) — same documented
exception class as release.yml. lint-literals green (below).

## P3-2 — signtool locator probed only lexically-newest SDK dir → **FIX VERIFIED**

New logic walks ALL version dirs newest-first (`sort -Vr`), x64 then x86
per version, across both SDK bases; `SIGNTOOL` override retained.
**Replica test on the actual extracted function bodies** (old vs new,
only the base path parameterized) against a fake SDK bin tree
`{10.0.22621.0/x86/signtool.exe, uap10.0.17763.0, x86}`: OLD → rc 1,
gives up (bug reproduced — `uap10.0.17763.0` sorts "newest"); NEW →
finds `10.0.22621.0/x86/signtool.exe`.

## P3-3 — runbook hardcoded sandbox path → **FIX VERIFIED**

The `source /home/z/my-project/scripts/cargo-env.sh` line is now
explicitly annotated as a build-sandbox-only helper (NOT a repo file,
pins this sandbox's shared CARGO_TARGET_DIR) with "skip this line
anywhere else" guidance — one of the two resolutions W24-R1 offered
(explicit sandbox-convention marker). The rest of the runbook stays
repo-relative.

## P3-4 — app-zip shipped without unsigned sidecar → **FIX VERIFIED**

Both macOS jobs' zip steps now copy
`bundle/${PRODUCT_NAME}.app.UNSIGNED.txt` → `dist/${base}.zip.UNSIGNED.txt`
when it exists (signed `.app` has no sidecar and needs none — correct
asymmetry). The dmg sidecar already ships (build-dmg.sh writes the dmg
directly into `dist/`, so `sign-macos.sh --dmg`'s sidecar lands in the
uploaded `dist/**`).

---

## Regression battery (all re-run by me, in this worktree)

| check | result |
|---|---|
| `bash -n` + `sh -n` on all 14 `scripts/packaging/*.sh` + `gen-wxs.sh` | clean |
| `python3 yaml.safe_load(release.yml)` | OK — 6 jobs, **42 run-blocks** (base 41; +1 = the visibility step, matches the fixer's count) |
| per-run-block `bash -n` with `${{ }}` neutralized | 42/42 clean |
| `source …/cargo-env.sh && bash scripts/packaging/gate.sh` (full_gate step 11 body) | **PASS** — 51 vars, 14 scripts, python ast, ps1 sanity, stagers, both unsigned-fallback contracts executed for real |
| `cargo run -p xtask --locked -- lint-literals` | **PASS — 0 violations, 452 allowlisted** |
| `ps1-sanity.py` on `build-msi.ps1` + `test-msi-upgrade.ps1` | PASS |
| files touched outside scripts/docs/.github/worklog | none; no `.rs` files (fmt/clippy not owed) |
| working tree | clean; no stray artifacts committed |

## Honesty check

- **No `[VERIFY at first live run]` label was removed or upgraded** — the
  diff contains zero removed VERIFY lines; `build-msi.ps1` *gained* one
  (the WIX_EXE conversion). Counts per touched file old→new: sign-macos
  1→1, sign-windows 1→1, build-msi.ps1 1→2, build-msi.sh 1→1,
  test-msi-upgrade 1→1, release.yml 7→7, README 1→1.
- **Overclaim sweep over all added lines**: one hit — the root worklog's
  "All 10 W24-R1 findings fixed and verified on w24/h" — immediately
  qualified by "the release workflow has still never executed (CI billing
  blocked); wix/signtool/codesign/notarytool/msiexec paths remain
  [VERIFY at first live run], labeled in-file". Within-bounds claim,
  acceptable.
- **The fixer's honest note on the `|| true` non-reproduction is accurate**
  (independently reproduced three ways — see P1-1). The worklog correctly
  separates what was proven locally from what remains CI-only, and the
  P2-1 commit message flags the superseded wave-22 `package-windows.sh`
  flow contract change to the Lead.
- The workflow's new comments correctly describe GitHub secrets semantics
  (absent secret → empty string; no auto-injection of repo secrets as env
  vars).

## New observations (P4 — optional hardening, none merge-blocking)

1. **P4-1 (P1-2 hardening)**: the wiring guards enumerate the 8 known
   bridge vars per job. A *future* ninth `[signing.*]` name would be
   exported by packaging-env.py but checked by no guard (the guards
   verify the names they know, which protects against renames of the
   current 8 — the R1 scenario — not additions). A
   `${!PACKAGING_SIGNING@}`-style enumeration filtered to `*_ENV` would
   make the guard future-proof. Not a defect for the current config.
2. **P4-2 (P2-1 hardening)**: doc *basenames* are inserted into the .wxs
   `Source` attribute unescaped (identity fields go through `xml_escape`;
   ids are sanitized). A doc named `a&b.md` yields malformed XML —
   **loud** failure (verified: gen-wxs exits 0 but the output fails any
   XML parse, so `wix build` dies), never silent. Escaping `$base` would
   match the identity-field treatment; same property in the .ps1 twin.
3. **P4-3 (doc drift)**: `docs/packaging/README.md` still says "every
   run-block parses (`bash -n` over all 41 steps)" — now 42 after the
   visibility step. The sentence cites the r3 evidence (historical), but
   a present-tense reader sees a stale count.
4. **P4-4 (pre-existing, not introduced here)**: `PACKAGING_DOCS` is a
   space-separated env contract, so doc paths containing spaces cannot be
   represented (they'd word-split; downstream staging fails loudly).
   A W24-C bridge property, unchanged by this fix — noting for the
   config-plane backlog.

## Merge posture

**w24/h is merge-ready from this audit's perspective**: both P1s fixed and
independently re-proven (including the negative paths the gate cannot
cover), all P2/P3 fixes verified against the artifacts they claim to fix,
no regressions in the local-provable battery, honesty labels intact. The
4 P4s are follow-up-commit material at leisure. Standing boundary
(unchanged, correctly labeled everywhere): the workflow itself has never
executed; wix/signtool/codesign/notarytool/msiexec/ditto remain
`[VERIFY at first live run]` — first live run should be the documented
`workflow_dispatch` dry-run.
