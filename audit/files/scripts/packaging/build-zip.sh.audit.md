# AUDIT: scripts/packaging/build-zip.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 88 / 1 (die) |
| File verdict | REAL (full happy path EXECUTED by me with synthetic binaries; 4 guard paths EXECUTED) |
| Reachable from shipped entry point? | YES — .github/workflows/release.yml:235-236 (windows job, "Package flat zip" step) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sh -n (0); bash -n (0); full run with staged fake exes (exit 0, zip verified via python zipfile); env-i guard runs x4 (2/1/1/1); python3 ZipFile inspection; rg greps; sed quote verifications |

## 1. One-paragraph verdict
This script is REAL and I executed its entire happy path in a sandboxed
staging area: with two synthetic 1 MiB exes and the real config doc list it
wrote `lumen-0.1.3-x86_64-windows.zip` (exit 0) whose members are exactly
the four flat entries — lumen.exe, lumen-tauri.exe, LICENSE, README.md
(stub deliberately absent, the w30.8 posture) — all DEFLATED
(compress_types [8,8,8,8]; zeros 1,048,576 → 1,034 bytes). Every guard
fires with an actionable message (missing env exit 2, wrong OS exit 1,
missing binary exit 1, missing doc exit 1), syntax is clean under both sh -n
and bash -n, staging uses mktemp + trap cleanup, and the config-driven doc
list matches packaging.toml [shipped]. One S3 defect: the header's claim
that "the zip creation path is EXECUTED for real in the local gate
context" is false — the local packaging gate (gate.sh, full_gate step 11)
only parse-checks this script and never runs it. One S4: unquoted
$PACKAGING_DOCS word-splitting.

## 2. What actually works (verified)
- Full happy path: `env -i ... TARGET_DIR=<scratch> DIST_DIR=<scratch> sh
  scripts/packaging/build-zip.sh` -> "build-zip.sh: wrote .../lumen-0.1.3-
  x86_64-windows.zip", exit 0; ZipFile inspection -> members
  ['lumen.exe','lumen-tauri.exe','LICENSE','README.md'], compress_types
  [8,8,8,8] (ZIP_DEFLATED), sizes [(1048576,1048896),(1048576,1034),
  (1088,640),(8586,4191)].
- Guards (real runs, outputs pasted):
  - `env -i sh scripts/packaging/build-zip.sh` -> "build-zip.sh: 28: PRODUCT_LOWER: ... not set (run packaging-env.py first)", exit 2.
  - OS_ID=linux -> "build-zip.sh: error: build-zip.sh is the Windows lane (OS_ID=linux)", exit 1.
  - TARGET_DIR with no binaries -> "missing packaging input: .../release/lumen.exe (build first: cargo build --release --locked)", exit 1.
  - PACKAGING_DOCS="NOPE.md" -> "shipped doc not found: NOPE.md (packaging.toml shipped.docs)", exit 1.
- Corroboration: docs/packaging/evidence/w26m-t3-win-mac-payload.md:83-94
  records the same BIN_TAURI guard firing in-tree.
- Config wiring: packaging.toml:63-64 `docs = ["LICENSE", "README.md"]`;
  packaging-env.py:265-267 asserts each doc exists, :300 exports
  PACKAGING_DOCS.

## 3. Findings
F-001 [S3]
Where: scripts/packaging/build-zip.sh:23-25
Code:
```
# POSIX sh (git-bash on the Windows runner); `bash -n` clean; the zip
# creation path is EXECUTED for real in the local gate context (same
# host tools) whenever the release binaries are present.
```
What is wrong: the "local gate context" never executes this script. The local packaging gate is scripts/packaging/gate.sh (full_gate.sh step 11); gate.sh only parse-checks scripts/packaging/*.sh (gate.sh:102-107 bash -n + sh -n) and executes gen-desktop-entry.sh/stage-linux-payload.sh (:125-151) — rg 'build-zip' gate.sh -> no hits, and the gate stages no release binaries, so the "whenever the release binaries are present" condition is unsatisfiable there. Why it matters: the header sells gate coverage that does not exist — the same false-coverage disease as build-rpm.sh's gate claim (cross-ref W17-A08 build-rpm.sh F-002). The only real executors are release.yml:236 (CI windows job) and manual runs (mine). What correct looks like: either gate.sh runs the zip path against throwaway fake binaries (like it already does for stage-linux-payload.sh), or the header says "CI-executed; parse-checked by the gate". Confidence: HIGH — Verification: read gate.sh + rg + executed the script myself.
F-002 [S4]
Where: scripts/packaging/build-zip.sh:60, :71, :82
Code:
```
for doc in $PACKAGING_DOCS; do
    [ -f "$doc" ] || die "shipped doc not found: $doc (packaging.toml shipped.docs)"
done
```
What is wrong: unquoted $PACKAGING_DOCS relies on IFS word-splitting in three loops; a doc path containing whitespace splits into phantom members (each fragment then fails the -f check, so it dies loudly — good — but only after partial staging), and an empty PACKAGING_DOCS would silently produce a zip with no docs at all (the :36 guard accepts empty-but-set; packaging-env.py:265-267's per-doc existence check is vacuous on an empty list). Why it matters: currently zero blast radius (docs = ["LICENSE", "README.md"], space-free), but it is the standard shell foot-gun the repo's own quality bar forbids. What correct looks like: emit the list as a delimiter-separated env (packaging-env.py already joins with spaces at :300 — switch to \x1f and split explicitly), or assert non-empty at :36. Confidence: HIGH — Verification: read + executed loops.

## 4. Per-section table (88 LOC script; single function die())
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header docs | 1-25 | mostly accurate | S3 | pipeline, payload decision (stub not a member — CONFIRMED by my run), python-zipfile rationale all correct; the gate-execution claim is F-001 |
| set -eu + 7 guards | 26-38 | REAL | — | 2 executed (:28 guard, :33 per w26m-t3:83-84) |
| ROOT cd + OS guard + PY resolve | 39-48 | REAL | — | OS guard executed; python3/python fallback is honest |
| input existence checks | 50-62 | REAL | S4 | binary guards executed (:53-56); doc loop F-002 |
| staging + zip creation | 64-86 | REAL (executed) | — | mktemp + trap EXIT/INT/TERM; flat root via cd stage; DEFLATED confirmed |
| verify + echo | 87-88 | REAL | — | [ -s ] double guard; wrote message matched my run |

## 5. Test honesty
No in-file tests. Mutation reasoning: if the zip creation step regressed
(wrong members, missing exe), the [ -s ] check catches emptiness only —
member correctness is unasserted locally (CI's verify-linux/windows upgrade
lane and the release checksum manifest are the outer checks). My run proves
current member correctness.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Package flat zip (scripts/packaging/build-zip.sh — exes + [shipped] docs from config)" | release.yml:235-236 | TRUE | my execution: 2 exes + 2 docs, flat, config-driven |
| "the shipped exes (the CLI and the tauri admission crate's bin)" / stub not a member since w30.8 | build-zip.sh:4-13; release.yml:8-18 | TRUE | members list of my run has no lumen-desktop |
| "Made with python -m zipfile ... One tool, three images" | build-zip.sh:13-17 | TRUE | executed via python3 -m zipfile; DEFLATED output verified |

## 7. Dead / duplicated / unreachable code
None dead. The naming convention is owned HERE (base= line 75) and not
duplicated in a callee (better than build-dmg.sh's F-002 twin).

## 8. Dependencies & flags
Runtime: python3-or-python (stdlib zipfile). Env: PRODUCT_LOWER/VERSION,
BIN_CLI, BIN_TAURI, OS_ID, ARCH_ID, PACKAGING_DOCS, TARGET_DIR, DIST_DIR
(defaults target/dist). Callee-free (no other repo script invoked).

## 9. Open questions for the next agent
- Does packaging.schema.json force minItems>=1 on shipped.docs (would close
  F-002's empty-list half)? Check packaging/packaging.schema.json.
- gate.sh could cheaply add the fake-binary zip run (it already fakes trees
  for stage-linux-payload.sh) to make the header's claim true.
