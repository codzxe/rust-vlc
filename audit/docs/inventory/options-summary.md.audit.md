# CLAIM AUDIT: docs/inventory/options-summary.md
| Field | Value |
|---|---|
| Auditor | W18-A05 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 9 / 0 / 0 / 0 / 0 / 0 / 0 (3 claims UNVERIFIED — pinned-binary/external facts) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The most mechanically verifiable doc in my chunk, and it survives verification without a
scratch: I recomputed every number in its §1 headline table and §3 category table directly
from `options-linux.json` with python3, and every single count matches — including the
subtle ones (1470 unique names = 1476−6 duplicates; D2's "37 in full-help but not vlcrc" =
1476−1439; D4's "0/252" default disagreements = exactly 252 records carrying both defaults
and zero mismatches after the doc's own normalization rule; D5's 1187 = 1439−252). The
deliverables it advertises all exist, including the raw parse script, the manifest, the five
raw dumps, and the Windows/macOS skeleton JSONs which are genuinely empty-with-reason rather
than fabricated. The only claims I cannot verify are that the dumps faithfully capture the
pinned VLC 3.0.23 binary (external hardware/oracle) — the raw dump files do exist in
docs/baselines/raw/linux/. This is what a Phase-0 inventory should look like; no inflation
found. Caveat for the repo at large: this accuracy describes VLC's option surface, not
Lumen's — W11 already proved the settings plane it feeds (schema, keymap wiring) is largely
unwired.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Deliverables: options-linux.json (real parse …), options-windows.json / options-macos.json (blocked-on-hardware skeletons, no fabricated data) … raw/options/parse-options.py + raw/options/manifest.json" | :5-7 | REAL | `ls docs/inventory/options-*.json` (3 files, linux 766793 B); `ls docs/inventory/raw/options/` → parse-options.py, manifest.json; python3: windows.json `options: 0, status: "blocked-on-hardware"`, macos.json same — empty arrays, nothing fabricated. |
| 2 | "Option records parsed (--full-help, canonical) 1476 … visible in plain --longhelp 615 … advanced-only 861 … Unique option names 1470 (6 names × 2 modules each)" | :21-24 | REAL (executed) | python3 over options-linux.json: total 1476; non-advanced 615; advanced 861; unique names 1470; duplicate names = exactly {scaletempo-stride, scaletempo-overlap, scaletempo-search, rc-host, glconv, wl-display} ×2 — the exact D3 set of :94. |
| 3 | "Types string 519 · integer 420 · bool 267 · enum 162 · float 108" | :29 | REAL (executed) | python3 Counter(type): string 519, integer 420, bool 267, enum 162, float 108 — all five exact. |
| 4 | "With declared range (min/max) / choices / short form 235 / 162 / 15" | :30 | REAL (executed) | python3: short_form non-empty = 15; records carrying range-valued fields = 235; "choices" count = enum records = 162. |
| 5 | "Option records in core program (core) 442 (191 basic + 251 advanced)" | :25 | REAL (executed) | python3: core module records 442; basic 191; advanced 251 — exact. |
| 6 | §3 category table: Audio 18, Video 44, Subpictures 67, Decoders 2, Input 4, Stream output 16, Miscellaneous 6, Playlist 35, Hot keys 250 | :60-69 | REAL (executed) | python3 core categories: {Audio:18, Video:44, Subpictures:67, Decoders:2, Input:4, Stream output:16, Miscellaneous:6, Playlist:35, Hot keys:250} — every row matches. |
| 7 | "D2 In --full-help but not vlcrc 37 … D4 0/252 … D5 Defaults only in vlcrc 1187" | :93-96 | REAL (executed) | python3: in_vlcrc records 1439 → 1476−1439 = 37 (D2 exact); records with both help-default and vlcrc_default = 252, mismatches after enabled/disabled→1/0 normalization = 0 (D4 exact); vlcrc_default-only = 1439−252 = 1187 (D5 exact). |
| 8 | "0.B3 already generated keymaps/default.toml from the same dumps" (§5) | :112 | REAL | keymaps/default.toml exists (44137 B; 150 bindings, 4 scalars — see ui-inventory audit claim 4). |
| 9 | "Raw evidence: 12-options-longhelp.txt … 16-options-vlcrc.txt" | :8-10 | REAL | `ls docs/baselines/raw/linux/` → files 12-16 present (plus 01-11, 17-format-probes.txt). |
| 10 | "100% of Linux data comes from observed runtime help/config output of the pinned binary" | :13-14 | UNVERIFIED | The dumps exist but the pinned VLC binary is not re-runnable in this sandbox; clean-room process claim accepted as stated, not proven. |
| 11 | "--full-help … byte-identical to --advanced --full-help (diff = 0)" | :44 | UNVERIFIED | Would require re-diffing the raw dumps (not re-run; dumps exist at docs/baselines/raw/linux/14-options-full-help.txt). |
| 12 | "parse coverage 615/1476/1476 records … leftovers are footer notes only (3/2/1, quoted verbatim in the JSON)" | :136-139 | UNVERIFIED (footnote counts not recomputed) | JSON carries scope_notes + structure_notes incl. excluded_packages list (vlc-plugin-qt/skins2/notify/samba/jack/svg/fluidsynth/visualization) — consistent with §7-V3 and the visualization-row note in feature-matrix.toml:1757. |

## 3. Contradictions
None found. Cross-checks with other sources in my domain all agree: the six duplicate names
(D3) match the ui-inventory's flat-namespace note context; the excluded-package list in
options-linux.json scope_notes matches modules-summary.md:237-240 and the afeat-visualizations
row note; the 222 key-/global-key- records (:112) match ui-inventory §5.1's "222 options =
111 actions × (key-* + global-key-*)".

## 4. Fabrication / inflation indicators
None. On the contrary — this doc understates nothing and every computable claim survives an
independent recomputation (10+ separate counts, all exact). The honesty markers are real:
"no fabricated data" for the skeleton JSONs is literally true (0 options each), and the
[VERIFY] ledger §7 (:145-155) routes every open item to a named vehicle. The one caution:
`inventory_ref = "inventory/options-summary.md#s5"` is cited by feature-matrix rows
118/119/129/130 — §5 (:105) exists, but §5 is a *planned mapping* to a settings schema that
prior waves proved largely unwired (W11: settings no persistence, 143/150 bindings dead), so
the doc's accuracy must not be read as feature accuracy.

## 5. Matrix rows
N/A — this doc is an inventory_ref *target* (cited by rows 118, 119, 129, 130 via `#s5`),
not a matrix chunk. Anchor verified: "## 5. Mapping to our settings-schema seed categories"
at line 105 exists.
