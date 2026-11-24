# AUDIT: scripts/packaging/gen-desktop-entry.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 48 / 0 named (guard block + single heredoc writer) |
| File verdict | REAL (fully executed by me; baseline-executed via the packaging gate) |
| Reachable from shipped entry point? | YES — stage-linux-payload.sh:83-84 invokes it for the deb/rpm/AppImage lanes (which verify-linux-artifacts.sh orchestrates for release.yml's linux-x64/arm64 jobs), and gate.sh:151 executes it directly (full_gate.sh step 11; baseline full_gate.txt:87-88 "gate.sh: PASS ... local-provable halves") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sh -n (0) + bash -n (0); two real executions with full env (output to audit/scratch; exit 0; entry content verified field-by-field); 2 guard runs (exit 2 / exit 2 + usage); rg caller/writer/uniqueness greps; sed quote verifications |

## 1. One-paragraph verdict
This script does exactly what it says and I executed it end-to-end: with the
six config vars set it writes a clean 7-field freedesktop entry (every field
interpolated from product/packaging env — zero literals), chmods 0644, exits
0; with env missing it dies at the ${VAR:?} guards (exit 2) and with no
argument it prints usage and exits 2. It is genuinely wired: the deb/rpm/
AppImage lanes all get their entry through stage-linux-payload.sh:83, and the
packaging gate executes it for real (baseline full_gate.txt:87-88 "gate.sh:
PASS (packaging config + scripts + self-tests + local-provable halves)").
The one real defect is its header's architecture claim: "this helper is the
ONE writer so their entries cannot drift" — two OTHER committed scripts
each embed their own [Desktop Entry] (scripts/release/package-deb.sh:94-100
and scripts/release/make-appimage.sh:78-86), and BOTH have already drifted:
they set Exec=${BIN_DESKTOP} (the w30.8-dropped exit-86 stub) where this
writer sets Exec=${BIN_TAURI}, and make-appimage.sh's Categories is a
hardcoded literal, not config. Both legacy writers have zero workflow callers
today (rg over release.yml), so nothing shipped is broken — but the
anti-drift guarantee the header sells is contradicted by in-tree evidence,
and reviving either legacy script would ship a menu entry that launches the
stub.

## 2. What actually works (verified)
- Real execution (audit/scratch output; second run with the exact
  packaging.toml values): exit 0, file mode 0644, content:
  ```
  [Desktop Entry]
  Type=Application
  Name=Lumen
  Comment=A clean-room, Rust-native media player.
  Exec=lumen-tauri
  Icon=lumen
  Terminal=false
  Categories=AudioVideo;Video;Player;
  ```
- Guards: `env -i sh scripts/packaging/gen-desktop-entry.sh` -> exit 2 with
  "PRODUCT_NAME not set (run packaging-env.py first)"; no-arg -> "usage: ...
  <output-file>" exit 2.
- `sh -n` AND `bash -n` both clean (true POSIX, as the header claims).
- Config wiring: packaging/packaging.toml:69-71 [desktop_entry]
  categories/terminal; packaging-env.py:301-302 exports DESKTOP_CATEGORIES/
  DESKTOP_TERMINAL; BIN_TAURI per the w30.8 payload contract
  (stage-linux-payload.sh:14-24 documents the same flip from BIN_DESKTOP).
- Consumers: stage-linux-payload.sh:83-84 writes
  $dest/$PACKAGING_LINUX_APPLICATIONS_DIR/${PRODUCT_APPSTREAM_ID}.desktop;
  build-deb.sh:65 "shared FHS payload (bins + docs + desktop entry + icon)".

## 3. Findings
F-001 [S3]
Where: scripts/packaging/gen-desktop-entry.sh:5-6 (vs the tree)
Code:
```
# The deb, the rpm and the AppImage all ship the same desktop entry;
# this helper is the ONE writer so their entries cannot drift.
```
What is wrong: it is NOT the one writer — `rg -ln '\[Desktop Entry\]' scripts/` returns THREE files: this one, scripts/release/package-deb.sh:94-100, and scripts/release/make-appimage.sh:78-86. Both other writers have ALREADY drifted: both set `Exec=${BIN_DESKTOP}` (the w30.8-dropped exit-86 stub — the exact regression this script's :10-14 comment describes as fixed), and make-appimage.sh:85 hardcodes `Categories=AudioVideo;Video;Player;` instead of the config env. Why it matters: the header's guarantee ("entries cannot drift") is false as an in-tree property — the drift exists in the two legacy writers; if either is revived (or a copy-paste starts from them), the shipped .deb/.AppImage menu entry would launch the stub — the documented w30.8 user complaint all over again ("v0.1.2 users ... double-clicked the obviously-desktop-named exe and got 'loading, then nothing'"). Both legacy writers currently have 0 workflow callers (rg over .github/workflows/release.yml -> the live lanes go through scripts/packaging/*), so this is architecture-debt, not a live shipping defect. What correct looks like: delete the embedded entries from the two superseded scripts (or make them exec the ONE writer). Confidence: HIGH — Verification: rg + read all three writers side by side.
F-002 [S4]
Where: scripts/packaging/gen-desktop-entry.sh:35-47
Code:
```
cat >"$out" <<EOF
...
Exec=${BIN_TAURI}
```
What is wrong: the unquoted heredoc means every env value is fully interpolated — a description (or any field source) containing `$`/backticks would be command-substituted into the entry; and PACKAGING_DESKTOP_TERMINAL is not validated as true/false here (a typo'd config value lands verbatim, violating the spec's boolean). Mitigation: values come from product.toml/packaging.toml which are schema-validated upstream (packaging-env.py's schema pass), so the trust boundary is the repo's own config. Confidence: MEDIUM — Verification: read + executed with benign values.
F-003 [S4]
Where: scripts/packaging/gen-desktop-entry.sh:44
Code:
```
Icon=${PRODUCT_LOWER}
```
What is wrong: a bare icon name ("lumen") is a cross-lane contract — the deb/rpm/AppImage lanes must install a hicolor icon named exactly that for the entry to resolve; this script neither asserts it nor documents it (the lanes do stage one — build-deb.sh:65 mentions "desktop entry + icon"). A silent broken-icon if a lane regresses. Confidence: HIGH — Verification: read (icon staging is the lanes' business; no assertion anywhere in this script).

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header docs | 1-20 | mostly accurate | S3 | field mapping + w30.8 history accurate; "ONE writer" claim false (F-001) |
| guards | 21-31 | REAL | — | 6 ${VAR:?} + usage check; all executed |
| writer | 33-48 | REAL (executed) | S4 | mkdir -p + heredoc + chmod 0644; F-002/F-003 |

## 5. Test honesty
No in-file tests; the packaging gate's step 5 executes it against a throwaway
tree (gate.sh:125-151) and the orchestrator baseline shows the gate PASSING
(full_gate.txt:87-88) — i.e., the script has a real, green, recorded
execution in-tree. My two sandbox runs corroborate.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ONE writer so their entries cannot drift" | gen-desktop-entry.sh:5-6 | FALSE-in-tree / TRUE-in-live-lane | two other writers exist with drifted entries (F-001); live lanes use only this one |
| "Exec <- BIN_TAURI ... w30.8/w30.12 — was BIN_DESKTOP, the exit-86 stub" | :10-14 | TRUE | stage-linux-payload.sh:20-24 documents the same flip; release.yml:8-18 confirms stub is CI-only diagnostic |
| "The desktop entry comes from gen-desktop-entry.sh (the ONE writer" | stage-linux-payload.sh:24 | TRUE for the live payload lane | :83-84 call verified |

## 7. Dead / duplicated / unreachable code
The script itself is live (three consumer chains). The DUPLICATION is in the
two legacy writers' embedded entries (F-001) — dead code by caller-count but
committed.

## 8. Dependencies & flags
Env: PRODUCT_NAME, PRODUCT_DESCRIPTION, PRODUCT_LOWER, BIN_TAURI,
PACKAGING_DESKTOP_TERMINAL, PACKAGING_DESKTOP_CATEGORIES (all exported by
packaging-env.py — :301-302 for the last two). No tools required. Writes one
file, mode 0644.

## 9. Open questions for the next agent
- Should the two legacy [Desktop Entry] blocks be deleted outright? Both
  writers are superseded (0 workflow callers) and their Exec=stub content is
  the exact w30.8-fixed defect — a deletion PR closes F-001.
- Does any doc still cite the legacy writers as the entry source
  (release.yml:104's stale comment table says "scripts/release/package-deb.sh
  + make-appimage.sh" for linux artifacts — cross-check with the
  packaging/README audit)?
