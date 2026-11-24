# AUDIT: scripts/release/gen-wxs.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 506 / 5 (usage, xml_escape, generate_wxs, self_test, dispatch) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — release.yml windows job "Package MSI" -> scripts/packaging/build-msi.sh:120 `"$ROOT/scripts/release/gen-wxs.sh" "$stage/installer.wxs"`; also gate.sh (full_gate step 11) and build-msi.ps1 twin |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |

## 1. One-paragraph verdict
A genuine, config-driven WiX v4/v5 source generator with the strongest self-test in this domain so far: I EXECUTED `sh scripts/release/gen-wxs.sh --self-test` and it passed (exit 0) — hostile identity values and hostile doc basenames round-trip through real XML parsing, the escape set is pinned on raw text, exe rows are KeyPath, Feature ComponentRefs must cover every component, both shortcuts are pinned to the tauri exe with twin-parity registry Keys and uuid5-stable GUIDs, and the exit-86 `lumen-desktop` stub is regression-pinned OUT of the MSI. Every identity value comes from product-env.py env vars with `:?` fail-loud guards. The generator itself cannot produce a silently-broken MSI source: missing env dies, duplicate doc basenames die, empty docs list dies.

## 2. What actually works (verified)
— `sh scripts/release/gen-wxs.sh --self-test` -> exit 0, PASS line printed (executed, output above in run log).
— `bash -n` + `sh -n` -> both OK.
— Payload claim verified in code: only `BinCli` + `BinTauri` components exist in the generated XML; no `BinDesktop`/`lumen-desktop` row (the w30.9 "stub left the MSI" claim is TRUE); self-test asserts `"selftest-desktop.exe" not in sources` and `"BinDesktop" not in comp_ids`.
— Missing-env behavior: `eval "$(python3 scripts/packaging/packaging-env.py --shell)"` (no packages) then invoking gen-wxs -> died loudly: `BIN_CLI: gen-wxs.sh: BIN_CLI not set` (fail-loud confirmed by execution).
— `set -eu` present at L91 (POSIX sh).

## 3. Findings

F-001 [S3]
Where: scripts/release/gen-wxs.sh:227-229 and 262-263
Code (from the heredoc):
```
        <Component Id="BinTauri" Guid="*">
          <File Id="BinTauriExe" Source="${BIN_TAURI}.exe" KeyPath="yes" />
        </Component>
```
What is wrong: `Source="${BIN_TAURI}.exe"` — the two exe basenames are interpolated into XML attributes WITHOUT xml_escape, while every identity field and doc basename goes through xml_escape. A bin name containing `&` or `"` (cargo bin names are restricted, so unreachable in practice) would emit malformed XML. Inconsistent escaping discipline only; cargo bin names are `[A-Za-z0-9_-]` so this cannot fire from real metadata. Confidence: HIGH (read) — impact LOW. What correct looks like: `$(xml_escape "${BIN_CLI}")` for symmetry.

F-002 [S4]
Where: scripts/release/gen-wxs.sh:84-89
Code:
```
#               (space-separated repo-relative
#                               paths — whitespace-free by schema, see
```
(for-doc word splitting) — `for doc in $PACKAGING_DOCS` relies on the schema's whitespace-free pattern; correct as designed, but the script itself does not re-assert it (packaging-env.py does). Defense lives one file away.
What is wrong: nothing functionally; hygiene note only.

F-003 [S4]
Where: scripts/release/gen-wxs.sh:404 area — the self-test's twin-parity comment cites "release run 43729218652, windows job" — a FIFTH distinct live-run ID (with release.yml's 43378757248/43464162696) further proving the release workflow HAS executed, contradicting release.yml L26-27's "NEVER EXECUTED ... dies at 0 steps". Cross-file corroboration for release.yml F-001. Confidence: HIGH (read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| usage | 93-98 | REAL | - | exit 2 on bad argc; `[ $# -eq 1 ] \|\| usage` |
| xml_escape | 103-107 | REAL | - | sed escape of & < > " ' — verified lossless by executed self-test |
| generate_wxs | 109-297 | REAL | - | `:?` guards on 6 identity vars + PACKAGING_DOCS; docs dedup by basename (exit 1); heredoc emission; shortcut GUIDs pinned literals |
| self_test | 299-496 | REAL | - | EXECUTED: PASS; covers well-formedness, round-trip, raw escape-set pins, component coverage, shortcut shape/Key/GUID, placement |
| dispatch | 498-505 | REAL | - | `--self-test` vs output path |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| --self-test (executed) | YES | mutation reasoning: revert `&apos;` escaping -> raw-text pin fails; drop a ComponentRef -> coverage assert fails; re-add BinDesktop -> "must stay GONE" assert fails; heredoc backslash bug -> twin-parity Key pin fails (this is the w28.1 hotfix regression the pin encodes — the comment says it caught a real 1603 upgrade failure, run 43729218652) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the MSI drops the exit-86 skeleton exe" (release.yml L16-18; packaging.toml [shipped] comment) | both | REAL | generated XML has only BinCli/BinTauri components (read + self-test pin) |
| "shortcuts point at the app binary, never the skeleton or the CLI" | gen-wxs.sh comment | REAL | both Shortcut Target=[INSTALLDIR]${BIN_TAURI}.exe; self-test asserts target |
| "MSI ... + Start-Menu AND Desktop shortcuts to BIN_TAURI" | packaging.toml comment | REAL | two Shortcut components present; self-test count==2 |
| "every value ... goes through xml_escape" | gen-wxs.sh L53-54 | PARTIALLY FALSE | exe basenames interpolated raw (F-001); identity + doc basenames escaped (verified) |

## 7. Dead / duplicated / unreachable code
None. BIN_DESKTOP deliberately not consumed (documented, w30.9); self-test explicitly `unset BIN_DESKTOP` to pin that.

## 8. Dependencies & flags
POSIX sh, sed, basename, dirname, printf; python3/python only for --self-test. No cargo/npm. WiX v5 schema emitted; the wix.exe consumer lives in build-msi.sh (pinned nupkg in release.yml).

## 9. Open questions for the next agent
1. build-msi.ps1 twin — does its -SelfTest pin the same GUID literals and Key interpolation? (twin_parity.py is in my queue — answers this.)
2. build-msi.sh stages PACKAGING_DOCS into the staging dir (claim); verify the staging actually copies all docs (else wix build fails loudly — acceptable).
3. Whether `wix build -arch x64` accepts `MediaTemplate EmbedCab="yes"` in v5 — UNVERIFIED (no Windows runner here; the release.yml comment says WiX v5.0.2 nupkg).
