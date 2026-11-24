# AUDIT: packaging/packaging.toml
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / config keys | 157 / 24 leaf keys across 10 tables |
| File verdict | REAL (every key checked against schema + consumers; identity-free claim verified) |
| Reachable from shipped entry point? | YES (release lane: packaging-env.py reads it at every packaging script + release.yml use) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg/sed/ls — exit 0 (schema enums, consumer wiring, anchor files); packaging-env.py itself not runnable here (needs cargo) |

## 1. One-paragraph verdict
The packaging config is real, consistent, and completely wired: every value I could
trace lands in a consumer — checksum algorithm/manifest (verify-linux-artifacts.sh
L320-334), SBOM suffix, shipped docs list (staged by every lane), desktop-entry
categories/terminal (gen-desktop-entry.sh:45-46), all five Linux FHS dirs
(stage-linux-payload.sh:46-91), deb section + rpm release/license (build-deb.sh:43,
build-rpm.sh:60,137), windows bindir + upgrade_test (release.yml:395 runs
test-msi-upgrade.ps1), launch-test flag/verb/fixture (verify-linux-artifacts.sh
L107-226, and the fixture EXISTS), macos universal (build-dmg.sh:62,80-81 refuses
universal=true with a typed error), and all six signing env names (release.yml:250,
511-515, 657-661, 1029-1035). The header's "Product identity NEVER appears here"
claim is true — not one product name/version string is in the file. Two dead rows:
[macos].targets is exported and consumed by nothing; launch_test.desktop_pending_exit
is exported and consumed by nothing (the file itself documents the retirement).

## 2. What actually works (verified)
- `ls LICENSE README.md tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav` -> all exist (smoke fixture present).
- `rg '"sha256"' packaging.schema.json` -> `"enum": ["sha256"]` — schema enforces the only algorithm the scripts wire (sha256sum), no lie possible.
- `rg WEBVIEW_PENDING apps/desktop/src/main.rs` -> `pub const WEBVIEW_PENDING: u8 = 86;` — desktop_pending_exit=86 matches the code constant.
- `rg packaging-env.py` semantic pass: launch_test.smoke_fixture existence is validated at env resolution (packaging-env.py:269-271 "not ..." die) — the "a violation is a hard error" claim has a real mechanism.
- Schema required-keys for launch_test (packaging.schema.json:160) match the table exactly.
- Payload honesty block (L45-56: stub absent from tar.gz/zip/deb/rpm; AppImage/macOS = BIN_TAURI only) — cross-verified against verify-linux-artifacts.sh stub-refusals (L210/303) and build-msi.ps1 w30.9 comments from my earlier audits: CONSISTENT.
- LICENSE is MIT -> rpm_license = "MIT" is the true SPDX id.

## 3. Findings
F-001 [S4]
Where: packaging/packaging.toml:97 vs scripts/packaging/packaging-env.py:313
Code:
```
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
```
Code (export, packaging-env.py:313):
```
    put("MACOS_TARGETS", " ".join(macos["targets"]))
```
What is wrong: PACKAGING_MACOS_TARGETS is exported and consumed by NOTHING
(rg across scripts/ + .github/ = 0 consumers). Dead config row, kept as
documentation of the universal2 future flip. Confidence: HIGH — Verification: rg.
F-002 [S4]
Where: packaging/packaging.toml:124 + 114-119
Code:
```
desktop_pending_exit = 86
```
What is wrong: LAUNCH_DESKTOP_PENDING_EXIT is exported (packaging-env.py:321) with no
consumer anywhere (only a historical evidence doc mentions it). The file honestly
says so ("no release-lane assert consumes this row anymore") — recorded here so
the ledger notes the dead row against its schema-required status (schema forces it:
required-key). Confidence: HIGH — Verification: rg.

## 4. Per-key table
| Key | Value | Consumer | Verdict |
|---|---|---|---|
| checksum.algorithm | sha256 | verify-linux-artifacts.sh:320-322 (only wired hasher) | REAL (schema-enum-forced) |
| checksum.manifest_name | SHA256SUMS.txt | verify script L324 | REAL |
| sbom.spec_version | 1.5 | xtask sbom generator | REAL (phase7 log: "lockfile format v4"/CycloneDX — consistent with SBOM content 121900 B) |
| sbom.artifact_suffix | sbom.cdx.json | verify script L327-330, gen-sboms.sh | REAL |
| shipped.docs | LICENSE, README.md | all staging lanes (PACKAGING_DOCS) | REAL (both files exist; whitespace-free constraint documented + schema pattern) |
| desktop_entry.categories/terminal | AudioVideo;Video;Player; / false | gen-desktop-entry.sh:45-46, gate.sh:156 | REAL |
| linux.install_bindir / docs_dir / applications_dir / icons_dir | usr/bin / usr/share/doc / usr/share/applications / usr/share/icons/hicolor/512x512/apps | stage-linux-payload.sh:46-91 | REAL |
| linux.deb_section | video | build-deb.sh:43 | REAL |
| linux.rpm_release / rpm_license | 1 / MIT | build-rpm.sh:60,137; verify script header asserts | REAL (LICENSE is MIT) |
| macos.universal | false | build-dmg.sh:62,80-81 (typed refusal if true) | REAL |
| macos.targets | 2 darwin targets | NOTHING | DEAD (F-001) |
| windows.install_bindir | bin | gen-wxs.sh/build-msi (MSI layout) | REAL |
| windows.upgrade_test | true | release.yml:395 (runs test-msi-upgrade.ps1) | REAL |
| launch_test.version_flag | --version | verify script L214 | REAL |
| launch_test.smoke_verb | decode-bench | verify script L220 | REAL |
| launch_test.smoke_fixture | tests/fixtures/.../wav_pcm_s16le_44k1_st.wav | verify script L107-108 | REAL (file exists) |
| launch_test.desktop_pending_exit | 86 | NOTHING (retired; matches WEBVIEW_PENDING=86) | DEAD-documented (F-002) |
| signing.windows.* | timestamp_url + 2 env names | sign-windows.sh via release.yml:250,1029 | REAL |
| signing.macos.* | 5 env names | sign-macos.sh via release.yml:511-515,657-661,1031-1035 | REAL |

## 5. Test honesty
N/A (config). The schema (packaging.schema.json) enforces structure; the semantic
pass enforces cross-file rules (docs exist, fixture exists, no identity duplication
per its $comment L5) — mechanisms verified by reading both sides.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Product identity NEVER appears here" | L16-18 | REAL | full read: only env-var names, paths, SPDX ids, flags — no product name/version |
| "Validated against packaging.schema.json by packaging-env.py at every use; a violation is a hard error" | L12-14 | REAL-UNPROVEN | schema + die-mechanisms verified in packaging-env.py (semantic pass L269-271); full env resolution needs cargo (absent in audit sandbox — gate.sh run failed at product-env.py "cargo not found") |
| "SHA-256 is the only wired algorithm (schema enum)" | L24-25 | REAL | schema enum ["sha256"]; script has no other hasher |
| "the exit-86 lumen-desktop stub left these payloads" (w30.8) | L45-53 | REAL | cross-verified against verify-linux-artifacts.sh + build-msi.ps1 + stage-linux-payload.sh in my earlier audits |
| "doc paths must be whitespace-free ... packaging-env.py refuses it at validation time" | L57-62 | REAL-UNPROVEN | schema pattern exists (per the $comment); the refusal path not executed here (no cargo) |

## 7. Dead / duplicated / unreachable code
F-001 (macos.targets) and F-002 (desktop_pending_exit) — both exported, both
unconsumed; both documented in-file as deliberate. Everything else is live.

## 8. Dependencies & flags
Read by scripts/packaging/packaging-env.py (structural + semantic validation) and
consumed by the entire scripts/packaging/* family + release.yml. schema_version=1.

## 9. Open questions for the next agent
- packaging-env.py owner (W17-A08): confirm the semantic pass rejects whitespace
  paths in shipped.docs exactly as L57-62 promises (their file, my cross-check only
  saw the fixture check).
- Cosmetic: build-dmg.sh comments say "packaging.toml acos] universal" (mangled
  "[macos]") at L28/L81 — typos, no behavior impact.
