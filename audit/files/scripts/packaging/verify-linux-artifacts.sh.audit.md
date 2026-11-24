# AUDIT: scripts/packaging/verify-linux-artifacts.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / functions | 337 / 1 (die) — 9 sequential proof steps |
| File verdict | REAL-UNPROVEN (syntax + structure verified; full lane needs built binaries + pinned appimagetool — not runnable in this sandbox) |
| Reachable from shipped entry point? | YES (release.yml:830-831 x86_64 Linux job, :971-972 aarch64 job: `run: scripts/packaging/verify-linux-artifacts.sh`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | sh -n -> exit 0; bash -n -> exit 0; rg (callers, evidence) -> exit 0 |

## 1. One-paragraph verdict
The Linux packaging PROOF harness: builds tar.gz/.deb/.AppImage (+.rpm where
rpmbuild exists), then asserts per format — packaged CLI really launches (`--version`
exit 0), really decodes a real fixture (`decode-bench` + a `BENCH` output line),
deb control fields equal config, webkit Depends pair present, docs present per
packaging.toml, tar member list, AppImage AppRun content + payload, and the exit-86
desktop stub verifiably ABSENT from every payload. `set -eu`, 23 named `: ${VAR:?}`
env assertions, `trap` cleanup of both mktemp dirs, loud `die()` everywhere, and the
rpmbuild-less half is honestly labeled ("spec-only ... labeled, never faked").
Syntax verified (`sh -n`/`bash -n` exit 0 — the header's "bash -n clean" claim
holds). The full lane cannot run here (needs release binaries + APPIMAGETOOL +
RUNTIME_FILE); committed evidence (docs/packaging/evidence/w26m-t2-linux-payload.md)
records gate.sh PASS + parser proofs but explicitly defers the full
verify-linux-artifacts run to "first CI run" (its L191-192) — so the whole-lane
execution is UNPROVEN in-tree.

## 2. What actually works (verified)
- `sh -n scripts/packaging/verify-linux-artifacts.sh` -> exit 0; `bash -n` -> exit 0.
- All 5 sibling scripts it invokes exist: build-targz.sh, build-deb.sh, build-appimage.sh, build-rpm.sh, gen-sboms.sh (`ls scripts/packaging/`).
- Reachability: `rg verify-linux-artifacts .github/workflows/release.yml` -> run verbatim at release.yml:831 and :972 (both Linux release jobs).
- Assertion design (read): L214-226 launches the extracted packaged CLI twice (version flag + decode-bench) and greps the real `BENCH` line from actual output — this is a launch test, not a presence check.

## 3. Findings
F-001 [S4]
Where: scripts/packaging/verify-linux-artifacts.sh:74-95 vs 135-139
Code (asserted set, excerpt):
```
: "${PACKAGING_LINUX_BINDIR:?verify-linux-artifacts.sh: PACKAGING_LINUX_BINDIR not set}"
...
: "${PACKAGING_CHECKSUM_MANIFEST:?verify-linux-artifacts.sh: PACKAGING_CHECKSUM_MANIFEST not set}"
```
Code (unasserted use, L137-139):
```
        "Release:RELEASE:$PACKAGING_RPM_RELEASE" \
        "Summary:SUMMARY:$PRODUCT_DESCRIPTION" \
        "License:LICENSE:$PACKAGING_RPM_LICENSE" \
```
What is wrong: PACKAGING_RPM_RELEASE and PACKAGING_RPM_LICENSE are consumed in the
rpmbuild branch but are NOT in the named `: ${VAR:?}` assertion block — a missing
value dies as a bare `set -u` "unbound variable" instead of the script's own
named-message discipline, and only on rpmbuild hosts (the local half never touches
them, so the gap is invisible locally). Why it matters: inconsistent input gating
undermines the script's "every failure names the config key" posture, exactly where
the lane differs between local and CI. What correct looks like: add both to the
L74-95 block. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: scripts/packaging/verify-linux-artifacts.sh:326-330
Code:
```
for artifact in $targz $deb $appimage ${rpm_path:-"$dist_abs/${base}.spec"} \
```
What is wrong: unquoted word-split in the checksum loop — a dist path containing a
space would hash wrong filenames; today's paths (dist/, no spaces) are safe. The rest
of the script quotes properly. Confidence: HIGH — read.
F-003 [S4]
Where: scripts/packaging/verify-linux-artifacts.sh:285-288 vs 336
Code:
```
apprun_exec_line="exec \"\$HERE/$PACKAGING_LINUX_BINDIR/$BIN_TAURI\""
grep -Fq "$apprun_exec_line" "$apprun" ||
    die "AppRun does not exec the GUI shell ..."
```
What is wrong: a `grep -F` content match is presented as "AppRun EXECS the GUI
shell" (also in the PASS line, L336) — the check proves AppRun CONTAINS the exec
line; a commented-out line would pass, and nothing executes. The launch boundary is
honestly disclosed at L46-51 (verdicts live in tauri-launch.yml's xvfb lane) — the
overstatement is wording only, but it lands in the final "PASS — ..." summary that
readers cite. Confidence: HIGH — read.

## 4. Per-step table (this file is a linear 9-step pipeline)
| Step | Lines | Verdict | Notes |
|---|---|---|---|
| env assertions | 74-108 | REAL | 23 named `:?` guards; OS/fixture/tool precondition checks |
| 1 tar.gz build | 114-115 | REAL-UNPROVEN | delegates to build-targz.sh |
| 2 deb build | 117-118 | REAL-UNPROVEN | delegates to build-deb.sh |
| 3 AppImage build | 120-121 | REAL-UNPROVEN | delegates to build-appimage.sh |
| 4 rpm lane | 123-183 | REAL-UNPROVEN | header asserts 6 fields incl. the ARCH-vs-ARCHITECTURE lesson; payload rows + stub-absence in both halves |
| 5 deb launch tests | 192-269 | REAL-UNPROVEN | real --version + decode-bench launch, control fields, docs, Depends pair, stub absence |
| 6 AppImage contract | 271-295 | REAL-UNPROVEN | --appimage-extract (no FUSE) + AppRun content + payload + stub absence |
| 7 tar member check | 297-306 | REAL-UNPROVEN | exact `grep -x` per member + stub absence |
| 8 SBOM | 308-317 | REAL-UNPROVEN | gen-sboms.sh over the 4 artifacts |
| 9 checksum manifest | 319-334 | REAL-UNPROVEN | sha256 only wired; F-002 quoting nit |

## 5. Test honesty
N/A (proof script — its assertions ARE the tests). Mutation reasoning: deleting the
`BENCH` grep (L223) would let a CLI that prints nothing pass the smoke; deleting the
stub-absence checks (L210/L161/L180/L292/L303) would let the payload regress silently
— each is an independent, meaningful assert.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "POSIX sh; `bash -n` clean" | L70 | REAL | bash -n exit 0 (and sh -n exit 0) |
| "the locally-PROVEN harness" | release.yml:830 | REAL (2-binary era) / SHALLOW (current shape) | local proof = gate.sh parse/structural halves (evidence doc w26m-t2-linux-payload.md:36-46); a FULL committed run log exists for the earlier 2-binary era: docs/packaging/evidence/linux-lane-20261110.log:99 "PASS — ... packaged binaries launch-tested, SBOMs + checksums written" (distinct artifact hashes at its L92-95); the CURRENT payload shape (CLI+tauri + w30.8 stub-refusal) has no committed full-run log — "first CI run" (w26m-t2 doc L191-192) |
| "AppRun EXECS the GUI shell" | L48, L336 | SHALLOW | content grep, not execution (F-003); launch deferred to tauri-launch.yml |
| "no CI-only divergence" (same script local + CI) | L7-9 | REAL-UNPROVEN | same file both places by rg; CI execution itself UNVERIFIED |

## 7. Dead / duplicated / unreachable code
None — every block feeds an assertion or an artifact.

## 8. Dependencies & flags
- Siblings: build-targz.sh, build-deb.sh, build-appimage.sh, build-rpm.sh, gen-sboms.sh (all exist).
- External: dpkg-deb, rpm/rpmbuild (optional), tar, sha256sum, appimagetool + type2 runtime via env.
- Consumers: release.yml Linux jobs (831, 972); docs/packaging/evidence/w26m-t2-linux-payload.md.

## 9. Open questions for the next agent
- release.yml owner: did any tagged release actually run the Linux verify lane
  (830/971)? If yes, where is the committed run evidence? If no, "locally-PROVEN"
  stays SHALLOW.
- PACKAGING_CHECKSUM_ALGORITHM supports only sha256 (L320-323) — check
  packaging.toml [checksum] does not configure anything else (my file 13).
