# CLAIM AUDIT: docs/packaging/evidence/README.md
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Claims extracted | 15 |
| REAL: 11 · REAL-UNPROVEN: 2 · SHALLOW: 0 · STUB: 0 · PAPER-ONLY: 0 · FALSE: 0 · UNREACHABLE: 0 · UNVERIFIED: 2 (distinct) |
| Internal contradictions found | 2 |

## 1. Blunt verdict
This is a strong evidence pack: nearly every operational claim (deb fields, BENCH decode line, exit-86 desktop contract, SBOM sizes/digests, SHA256SUMS manifest, r3 negative test, kind-glob bug fix, artifact sizes) is backed byte-for-byte by the two committed logs. The two weak spots are the appimagetool/runtime pin story (pins asserted in prose, never shown as a sha256sum run in either log — and the tool's own banner says "continuous build" while the README calls it a 1.9.1 release binary) and the commit-provenance line, which is unverifiable without git.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "CI-only steps (`wix build`, `signtool`, `codesign`/`notarytool`, `rpmbuild`, `msiexec`) are **not** represented here" | README.md:4-6 | REAL | lane log:58-60 "rpmbuild absent on this host -- spec-only (the binary .rpm build runs in the CI lane where rpm is installed; labeled, never faked)" |
| 2 | "Real .tar.gz flat archive — both binaries + the `[shipped]` docs list … (step 7 cross-checks the member list)" | :23-25 | REAL | lane log:75 "tar.gz members: lumen lumen-desktop LICENSE README.md" |
| 3 | "Real .deb via native `dpkg-deb` … (`Package=lumen`, `Version=0.1.0-dev`, `Architecture=amd64`, `Section=video` from packaging.toml)" | :26-29 | REAL | lane log:68-70 "deb control: Package=lumen / Version=0.1.0-dev / Architecture=amd64"; :6 "(arch=amd64, section=video)"; :5 "dpkg-deb: building package 'lumen'" |
| 4 | "Real .AppImage via the pinned appimagetool + pinned runtime (`--runtime-file`, no rolling runtime fetch)" | :30-31 | **REAL-UNPROVEN** | appimagetool really ran (banner lane log:9); but `--runtime-file`, the runtime, and its pin appear NOWHERE in the lane log (`rg runtime` → rc 1); pins only asserted as prose in w24c-r3.log:15-16 |
| 5 | "rpm .spec generated + structurally validated (`rpm_version=0.1.0~dev`: … tilde)" | :32-34 | REAL | lane log:62 "wrote + validated … (rpm_version=0.1.0~dev)" |
| 6 | "`usr/bin/lumen --version` → `lumen 0.1.0`, exit 0" | :38 | REAL | lane log:65 "CLI --version: lumen 0.1.0"; final :99 "verify-linux-artifacts.sh: PASS —"; r3 log:119 "verify-linux-artifacts.sh exit=0" |
| 7 | decode-bench "exit 0 with a real `BENCH` line (`a_frames=1 a_decoded=1` …)" | :39-41 | REAL | lane log:66 `BENCH file=tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav bytes=35358 v_frames=0 v_decoded=0 a_frames=1 a_decoded=1` |
| 8 | "`usr/bin/lumen-desktop` → exit 86, the documented `WEBVIEW_PENDING` skeleton code" | :42-44 | REAL | lane log:67 "desktop skeleton: exit 86 (as documented)" |
| 9 | "the AppImage itself … → exit 86 (same documented contract, through the AppRun launcher)" | :45-46 | REAL | lane log:73 "AppImage skeleton: exit 86 (as documented)"; step-6 header :72 "=== 6. AppImage launch test (extract-and-run; no FUSE) ===" |
| 10 | "Per-artifact-set SBOMs — `xtask sbom --out` (CycloneDX 1.5, 121,900 bytes each, byte-identical)" | :47-49 | REAL | lane log:79-88 four "(121900 bytes)" lines, step-8 header :76 "(xtask sbom, CycloneDX 1.5)"; four identical digests f0b22c1b… :95-98 |
| 11 | "`SHA256SUMS.txt` written over all eight artifacts (the manifest itself excluded …)" | :50-51 | REAL | lane log:90-98 (8 digest lines; manifest path named :90) |
| 12 | Artifact digest table `3b5eae753466…` / `a39dafa88039…` / `c0b86e2984b1…` / `17bd1068c818…` / `f0b22c1bc2a8…` | :62-68 | REAL | lane log:91-98 — all five full digests match the prefixes |
| 13 | "appimagetool | 1.9.1 x86_64 (`git version 8c8c91f`), SHA-256 `ed4ce84f…84eb0` — the pin recorded in .github/workflows/release.yml" | :17 | **REAL-UNPROVEN + CONTRADICTION** | lane log:9 the tool's own banner: "appimagetool, continuous build (git version 8c8c91f), build 296 built on 2025-12-04 17:55:56 UTC" — "continuous build" with a 2025-12-04 build date vs "1.9.1 release binary"; no sha256sum verification output anywhere in either log; the pin exists at release.yml:817-820 |
| 14 | "type2 runtime `20251108` x86_64, SHA-256 `2fca8b44…260d` — the same pin" | :18 | REAL-UNPROVEN | Only prose: w24c-r3.log:15-16 "type2 runtime 20251108 x86_64 (2fca8b44...260d) — the release.yml pins"; no checksum run recorded |
| 15 | "Binaries … from this worktree (branch `w24/c`, commit `44bd866`)" | :16 | UNVERIFIED | git operations forbidden for this audit; no hash provenance in the logs |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "appimagetool 1.9.1 x86_64 (git version 8c8c91f)" (README:17) | lane log:9 — the executed tool self-reports "continuous build … built on 2025-12-04" | Cannot resolve without network; the repo consistently equates 8c8c91f with 1.9.1 (release.yml:807-808 "Self-reports 'git version 8c8c91f' — the 1.9.1 tag commit"), but the banner's "continuous build" label + 2025 build date contradict the release-binary story. Flagged, not adjudicated. |
| "scripts/packaging/gate.sh … passes all six steps on this host" (README:90-92) | Orchestrator baseline: `scripts/full_gate.sh: GATE RESULT FAIL` with the sbom section among the red ones | Committed evidence supports the README for the 2026-11-10 host: w24c-r3.log:139-140 "gate.sh: PASS (packaging config + scripts + local-provable halves)" / "gate.sh exit=0". The baseline FAIL is a later, different run — the drift (sbom red in baseline vs. local pass) is unresolved. |

## 4. Fabrication / inflation indicators
- The 2026-11-10 digests in the README table match the committed log exactly (claim 12) — the earlier r1 stale-digest incident is disclosed in the README itself (:55-58), which is the opposite of fabrication.
- Honest non-claims: package sizes shown as "—" (:62-68) rather than invented; CI-only steps explicitly excluded (claim 1); AppStream gap kept as a warning (lane log:10-14).
- The one inflation-adjacent pattern: "pinned appimagetool" (README:30, lane log:7) — "pinned" is asserted, but the pinning (SHA verification) has no recorded execution in the lane log; it lives in prose in another log and in release.yml config.

## 5. Matrix rows
N/A — not a matrix chunk.

## 6. What actually works (verified)
- All rg/sed checks cited in the ledger above ran against the two committed logs (exit 0 unless noted rc 1).
- `ls assets/brand` → does not exist — confirms the placeholder-icon honest note (:83-86); lane log:2-3 "gen-icon: wrote … (512x512 RGBA, fill #4f8cff, 1819 bytes)".
- w24c-r3.log backs the README's r3 claims: negative specVersion drift test exit=1 (:121-122), release.yml validation "6 jobs … 63 steps" + "bash -n over all 41 run blocks" (:143-144), kind-glob no-op bug + fix (:169-173), fresh sizes tar.gz 3,054,325 / deb 2,073,444 / AppImage 1,276,408 / spec 1,038 (:180-184), gate.sh pass (:139-140).

## 7. Open questions for next agent
- The appimagetool provenance contradiction (claim 13) is the one item that could unravel the "pinned toolchain" story — needs a network-enabled check of what the release.yml SHA ed4ce84f…84eb0 actually resolves to.
- README:90-92's "gate.sh passes all six steps" IS backed by committed evidence (r3 log:139-140) — but the orchestrator baseline's full_gate sbom section was red on a later run; reconcile that drift.
