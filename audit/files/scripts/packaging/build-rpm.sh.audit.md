# AUDIT: scripts/packaging/build-rpm.sh
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 283 / 2 (main flow + validate_spec) |
| File verdict | REAL-UNPROVEN (logic verified by read + syntax check + committed historical evidence; not executed here — needs built binaries + rpmbuild) |
| Reachable from shipped entry point? | YES: release.yml:831 (verify-linux-artifacts.sh) -> build-rpm.sh full build (rpmbuild installed at release.yml:828-829; aarch64 lane :969-970); spec-only at verify-linux-artifacts.sh:169 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | sh -n -> OK; bash -n -> OK; rg for callers; sed quotes; read verify-linux-artifacts.sh rpm section; read committed evidence log |

## 1. One-paragraph verdict
A carefully written POSIX-sh spec generator with a genuine structural validator
(required tags, no-dash Version, both Requires rows asserted, every %files row
checked against the staged tree — lines 201-239). Syntax verified (`sh -n` and
`bash -n` both clean, matching its own claim at :41). The commit trail of
evidence logs shows the --spec-only half ran for real (docs/packaging/
evidence/linux-lane-20261110.log:62-63 "wrote + validated ... spec
(rpm_version=0.1.0~dev)"). Two real defects: (1) the generated %description
prose claims THREE shipped binaries ("the engine-driven CLI, the desktop
composition-root binary and the tauri admission shell") while the payload and
%files list ship TWO (CLI + tauri) — since w30.8 the desktop stub is banned from
user payloads (release.yml:5-16) and the rpm consumer verify-linux-artifacts.sh
actively FAILS if the stub appears — so the package's own `rpm -qi` metadata
misdescribes the payload to every user; (2) the header claims the --spec-only
proof "is what scripts/packaging/gate.sh and full_gate step 11 exercise" —
gate.sh never invokes build-rpm.sh (rg 'rpm' in gate.sh: only comments); the
real exerciser is verify-linux-artifacts.sh. Coverage exists, but the citation
is false and would leave a build-rpm regression invisible to the local gate.

## 2. What actually works (verified)
- `sh -n scripts/packaging/build-rpm.sh` -> exit 0; `bash -n` -> exit 0 (the "bash -n clean" claim at :41 is TRUE).
- Committed run evidence: docs/packaging/evidence/linux-lane-20261110.log:62-63 shows the real --spec-only output, including the tilde mapping (`rpm_version=0.1.0~dev`) — the dash->tilde logic ran for real.
- Caller chain verified: release.yml:828-831 (install rpm; run verify-linux-artifacts.sh) -> verify-linux-artifacts.sh:127 `scripts/packaging/build-rpm.sh` (full build when rpmbuild present) and :169 `--spec-only` fallback.
- 24 env-var `:?"required"` guards (lines 45-65) — fail-closed on every missing input, including BIN_TAURI (a missing GUI shell is a loud error, not a silently-one-binary package — matches stage-linux-payload.sh's contract).
- validate_spec's %files-vs-staging check (:230-237) is a real existence proof, not a tautology.

## 3. Findings
F-001 [S3]
Where: scripts/packaging/build-rpm.sh:152-157
Code:
```
%description
${PRODUCT_DESCRIPTION}
The shipped binaries (the engine-driven CLI, the desktop
composition-root binary and the tauri admission shell) install under
/${PACKAGING_LINUX_BINDIR}; this package ships files only, no
scriptlets.
```
What is wrong: the shipped rpm payload contains exactly TWO binaries
(BIN_CLI, BIN_TAURI — %files rows at :111-112; stage-linux-payload.sh stages
exactly those two and release.yml:12-16 says the desktop skeleton was "dropped
from every user payload this workflow's scripts own (zip/AppImage/.app/
tar.gz/deb/rpm)" in w30.8). The %description prose still sells a third,
"the desktop composition-root binary". The rpm consumer verifies the payload
ban (verify-linux-artifacts.sh:160-162 dies if the stub is in the payload) —
but nobody checks the prose, so every user running `rpm -qi lumen` reads a
false description of the package contents. Why it matters: user-visible
package metadata is wrong; it is the exact class (doc says X, payload does
not do X) this audit hunts. What correct looks like: "the engine-driven CLI
and the tauri admission shell". Confidence: HIGH — Verification: read +
release.yml + stage-linux-payload.sh cross-check.
F-002 [S3]
Where: scripts/packaging/build-rpm.sh:10-12
Code:
```
#       entry exists in the staging tree. Exits 0 — this is the
#       locally-provable half (runs on any POSIX host; it is what
#       scripts/packaging/gate.sh and full_gate step 11 exercise).
```
What is wrong: gate.sh does not run build-rpm.sh in any mode — `rg -i rpm
scripts/packaging/gate.sh` hits only comments; gate.sh's payload exercise is
stage-linux-payload.sh (:139), the shared stager, not this script's spec
generation or validate_spec. full_gate step 11 calls gate.sh (full_gate.sh:80),
so the chain never reaches build-rpm.sh either. A regression in build-rpm.sh
(spec generation, Requires rows, %files drift) passes the "packaging: PASS"
gate. The claim is false; actual coverage is verify-linux-artifacts.sh:169
(release lane) — which needs built binaries, i.e. never a quick local gate.
Why it matters: false coverage claims are how broken packagers ship. What
correct looks like: gate.sh gains a `build-rpm.sh --spec-only` step, or the
header names verify-linux-artifacts.sh. Confidence: HIGH — Verification: rg
on gate.sh + full_gate.sh.
F-003 [S4]
Where: scripts/packaging/build-rpm.sh:131-192 (unquoted heredoc)
Code:
```
Summary:        ${PRODUCT_DESCRIPTION}
```
What is wrong: the spec heredoc is unquoted by design (config-driven), so any
`%` or backtick in PRODUCT_DESCRIPTION (or other config values) would be
expanded by rpm at build time or executed by the shell at generation time —
the file's own comments (:167-191) document both hazards from real dry-run
incidents, but no escaping/validation of the config values exists; the current
values ("A clean-room, Rust-native media player.") are safe by luck, not by
check. What correct looks like: validate config values for `%`/backtick at
generation (die loudly) — same posture the file already applies to dashes.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: scripts/packaging/build-rpm.sh:279
Code:
```
built=$(find "$stage/RPMS" -name "${PRODUCT_LOWER}-${rpm_version}-*.${rpm_arch}.rpm" -print -quit)
```
What is wrong: `-quit` is a GNU/BSD extension (not POSIX); acceptable because
the full-build mode is Linux-CI-only, but it silently narrows the portability
the "runs on any POSIX host" header suggests (that phrase applies only to
--spec-only; the split is documented, so this is a hygiene note). Also a
wrong-arch rpm match is possible if rpmbuild emits compat arches — mitigated
by the exact `.${rpm_arch}.rpm` suffix. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main flow (env guards, arg parse, staging, spec gen) | 43-130, 244-283 | REAL-UNPROVEN | S3 (F-002 header claim), S4 (F-003 heredoc exposure) | env guards genuine; arg case handles --spec-only/""/garbage; tilde mapping correct; trap cleanup correct |
| files_list builder | 110-122 | REAL | — | docs rows iterate the same PACKAGING_DOCS the stager installs (single source) |
| validate_spec | 201-239 | REAL-UNPROVEN | — | tags/dash/Requires/%files-existence checks all genuine; historical log shows it ran (evidence log:62) |
| full-build block | 252-283 | REAL-UNPROVEN | S4 (F-004) | rpmbuild guard, arch map, tar --transform trick, topdir layout — cannot execute here |

## 5. Test honesty
No in-file tests. Consumers test it for real: verify-linux-artifacts.sh
asserts rpm header fields (Name/Version/Release/Summary/License/ARCH vs
config), payload rows for both bins, and stub absence (:137-168). Mutation
reasoning: drop a Requires row -> validate_spec fails (grep asserts); drop a
%files row -> staging check fails; rename BIN_TAURI -> env guard fails.
Those mutations would be caught — the validator is honest. The prose defect
(F-001) is NOT caught by any check — no consumer asserts the %description
text.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exits 0 — this is the locally-provable half ... it is what scripts/packaging/gate.sh and full_gate step 11 exercise" | :10-12 | FALSE | rg rpm gate.sh -> comments only; gate.sh runs stage-linux-payload.sh, never build-rpm.sh |
| "POSIX sh; `bash -n` clean" | :41 | REAL | sh -n + bash -n both exit 0 |
| "the lumen-desktop stub left the payload" (header, w30.8) | :31-32 | REAL | %files has BIN_CLI+BIN_TAURI only; stage-linux-payload.sh stages 2 bins |
| "%description ... The shipped binaries (CLI, desktop composition-root, tauri shell)" | :152-157 | FALSE (payload has 2, not 3) | verify-linux-artifacts.sh:160-162 dies if stub in payload; release.yml:12-16 |
| "requires rpmbuild ... CI lane" | :36-39 | REAL | release.yml:828-829 installs rpm; sandbox has no rpmbuild (verified: command -v rpmbuild empty) |

## 7. Dead / duplicated / unreachable code
None dead. Requires rows are script-local by documented decision (:25-28,
same as deb Depends) — duplication across deb/rpm is acknowledged and
cross-referenced, not accidental.

## 8. Dependencies & flags
Requires 24 env vars from packaging-env.py; needs stage-linux-payload.sh,
gen-desktop-entry.sh (via stager), tar+rpmbuild for full mode. No literals
leak (paths from PACKAGING_LINUX_*; name from PRODUCT_LOWER) — the
"100% config-driven" claim (:18) holds for every field I checked except the
hard-coded Fedora Requires pair (documented as script-local).

## 9. Open questions for the next agent
- Has any CI run executed the full rpmbuild path yet (release.yml lane)?
  Header says "[VERIFY at first live run]" — check the release evidence docs
  for a first-run record; UNVERIFIED from this sandbox.
- build-deb.sh (same family) — check whether its Description field has the
  same stale three-binary prose (F-001 sibling); that file is not in my list.
