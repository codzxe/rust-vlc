# AUDIT: scripts/packaging/build-msi.sh
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 135 / 1 flow + die (staging, generation, build, sign) |
| File verdict | REAL-UNPROVEN (logic verified by read + sh -n/bash -n; execution needs Windows + WiX + built exes — CI-only) |
| Reachable from shipped entry point? | YES: release.yml:291 `scripts/packaging/build-msi.sh` (windows x64 job "Package MSI"); WIX_EXE exported at release.yml:233 from the pinned WiX 5.0.2 nupkg (:227-232) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sh -n -> OK; bash -n -> OK; rg wiring cross-checks (release.yml WiX pin/export/invocation, gate.sh self-tests, gen-wxs.sh twin) |

## 1. One-paragraph verdict
A clean, fail-closed POSIX sh MSI builder: 17 required-env guards including
BIN_TAURI (the GUI shell is mandatory — a missing binary is a loud error, and
the exit-86 lumen-desktop stub is deliberately absent from the guards, matching
the w30.8 policy), executable-WIX check, arch mapping with a die on unknown,
staged inputs with existence checks, the ONE shared .wxs generator
(gen-wxs.sh), the generated .wxs kept beside the artifact for auditability,
`wix build` in a subshell with cd (so File/@Source basenames resolve — the
documented WiX working-dir requirement), non-empty-MSI assert, and
sign-windows.sh with its explicit .UNSIGNED.txt fallback. Syntax verified
(`sh -n` + `bash -n` clean, matching :43's claim). The CI wiring is real:
pinned WiX 5.0.2 nupkg with SHA-256 verification, WIX_EXE exported to
GITHUB_ENV, the same-target-dir recipe (CARGO_TARGET_DIR=target) so the tauri
exe lands where this script stages it, and a signing-env sync check right
before invocation. Two S4 notes: the header's "every env assertion ...
executed locally in gate.sh" overstates (gate.sh runs the bridge and the
gen-wxs SELF-TEST, never build-msi.sh's own 17 guards — they cannot run
without exes/wix), and the stub-left-the-MSI comment is corroborated but the
phrase "left the MSI with the twins" is awkward-to-stale phrasing rather than
a defect.

## 2. What actually works (verified)
- `sh -n scripts/packaging/build-msi.sh` -> exit 0; `bash -n` -> exit 0 (claim :43 TRUE).
- Caller chain: release.yml:291 invokes it; :233 `echo "WIX_EXE=..." >> "$GITHUB_ENV"`; :227-230 pinned wix.5.0.2.nupkg + sha256 f30ef0c7...ae173 verified at install — the "pinned WiX 5.0.2 nupkg" claim (:35-36) is byte-real in the workflow.
- Same-target wiring: release.yml:208-213 comment "CARGO_TARGET_DIR=target (workflow env) lands target/release/lumen-tauri in the SAME target dir the packaging scripts below stage from" — matches this script's `release_dir="$TARGET_DIR/release"` (:92).
- The stub ban (:63-64) is consistent with release.yml:12-17 (stub is a CI-only diagnostic, dropped from user payloads; the MSI member list owned by the gen-wxs twins).
- gen-wxs.sh exists and is the documented ONE generator (:120 calls it; its own header at gen-wxs.sh:132 requires PACKAGING_DOCS — the docs-staging loop :110-113 matches the .wxs File-row contract).

## 3. Findings
F-001 [S4]
Where: scripts/packaging/build-msi.sh:45-49
Code:
```
# HONEST LOCAL-PROOF BOUNDARY: this sandbox is Linux — the .wxs
# GENERATION and every env assertion are executed locally in
# scripts/packaging/gate.sh (spec generation + XML well-formedness via
# python), but the `wix build` invocation needs Windows + .NET and is
# CI-only [VERIFY at first live run].
```
What is wrong: gate.sh never runs build-msi.sh, so build-msi.sh's OWN 17
`:?` env assertions (:53-70) are not "executed locally in gate.sh" — gate.sh
runs the packaging-env.py bridge (which asserts the config side) and the
gen-wxs.sh --self-test (hostile-fixture generation, not the real installer.wxs
from this lane's env). The sentence conflates the bridge's assertions with
this script's. Why it matters: mild provenance overstatement, same family as
build-rpm.sh's gate claim (there it was false outright; here the underlying
halves ARE tested, just not by this script's guards). What correct looks like:
"the config bridge + .wxs generator self-test run locally in gate.sh; this
script's own guards run on the Windows runner." Confidence: HIGH —
Verification: read + rg gate.sh.
F-002 [S4]
Where: scripts/packaging/build-msi.sh:63-64
Code:
```
# w30.8/w30.12: the exit-86 lumen-desktop stub left the MSI with the
# twins — it is a CI-only diagnostic now, never staged here.
```
What is wrong: "the stub left the MSI with the twins" — reading is ambiguous
(the stub left the MSI **in the care of** the gen-wxs/build-msi twins, i.e. the
twins' member list now owns the drop). The staging loop (:95-105) stages only
cli+tauri, consistent with the policy; only the sentence is cryptic.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| env guards + arch map | 51-97 | REAL-UNPROVEN | — | 17 guards; unknown arch dies; exes existence-checked |
| staging + docs | 99-113 | REAL-UNPROVEN | — | mktemp + trap; docs staged with existence check; word-split PACKAGING_DOCS by design |
| generate + keep .wxs | 115-125 | REAL-UNPROVEN | — | provenance copy to dist — good auditability practice |
| wix build + assert | 127-131 | REAL-UNPROVEN | — | subshell cd for basename resolution (documented WiX requirement); -s empty check |
| sign handoff | 133-135 | REAL-UNPROVEN | — | sign-windows.sh with documented unsigned fallback |

## 5. Test honesty
No in-file tests. Real coverage exists in three layers: gate.sh runs gen-wxs.sh
--self-test + ps1 structural checks (local); release.yml:264-289 asserts the
signing env sync before invoking this script; and test-msi-upgrade.ps1 exists
(the MSI upgrade test named in the skeleton). Mutation reasoning: a dropped
BIN_TAURI guard would be caught by the .wxs File row (gen-wxs requires the
staged exe? — UNVERIFIED whether gen-wxs.sh itself asserts the exe exists in
its working dir; the staging check :95-97 here is the guard). An empty MSI is
caught by :130. Fail-closed throughout.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "POSIX sh (git-bash on the Windows runner); `bash -n` clean" | :43 | REAL | sh -n + bash -n exit 0 |
| "the workflow installs the pinned WiX 5.0.2 nupkg and exports this [WIX_EXE]" | :35-36 | REAL | release.yml:227-233 |
| "every env assertion ... executed locally in scripts/packaging/gate.sh" | :46-47 | SHALLOW | gate.sh runs the bridge + self-tests, not this script's guards (F-001) |
| "the exit-86 lumen-desktop stub left the MSI ... never staged here" | :63-64 | REAL | no BIN_DESKTOP guard; staging loop stages cli+tauri only; release.yml policy |
| "hands the finished MSI to sign-windows.sh ... explicit .UNSIGNED.txt fallback" | :133-135 | REAL-UNPROVEN | sign-windows.sh exists (gate.sh:184 exercises it on a stand-in msi locally) |
| "wix build needs Windows + .NET and is CI-only [VERIFY at first live run]" | :48-49 | REAL-UNPROVEN | honest marker; first live run UNVERIFIED from sandbox |

## 7. Dead / duplicated / unreachable code
None. The .ps1 twin (build-msi.ps1) is a parallel implementation, not
duplication of this file's logic (self-test contract shared — gen-wxs.sh:308
notes the twin's interpreter requirements).

## 8. Dependencies & flags
Requires 17 env vars via the bridge; WIX_EXE path (pinned binary); gen-wxs.sh;
sign-windows.sh. No literals — every identity field from the env; the only
hard-codes are the WiX arch mapping (x64/arm64 — inherent to WiX's flag set).

## 9. Open questions for the next agent
- Has the Windows lane ever run green (wix build)? The [VERIFY at first
  live run] markers (this file + release.yml WebView2 comment) suggest the
  first live run was recent; check release evidence docs for the MSI artifact
  + upgrade-test result. UNVERIFIED here.
- gen-wxs.sh:240's Desktop shortcut (w30.9) — the skeleton-v0.1.3 shortcut
  claim rides on it; verified in that ledger.
