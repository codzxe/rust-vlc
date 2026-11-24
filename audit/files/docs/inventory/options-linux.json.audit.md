# AUDIT (DATA/CLAIM LEDGER): docs/inventory/options-linux.json
| Field | Value |
|---|---|
| Auditor | W18-A01 |
| Date | 2026-11-24 |
| Lines / bytes / records | 26,679 / 766,793 / 1,476 option records (15 fields each) |
| File verdict (as data artifact) | REAL — every quantitative self-claim reproduced against raw dumps 12–16 and against the embedded options array |
| Reachable from shipped entry point? | N/A (docs data). Consumers: docs/feature-matrix.toml population (matrix-seed-notes.md:95-103), tools/xtask feature_audit evidence-path resolution (file-existence only, anchor stripped — see F-001). No shipped binary reads it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 json.load + counter scripts (OK); rg over matrix/inventory/dumps (OK); sed -n spot verifications (OK); wc -l / rg -c on raw dumps 12–16 (OK). No git, no builds. |

## 1. One-paragraph verdict
This file is the VLC 3.0.23 Linux options inventory: 1,476 parsed option records with
15 fields each, a metadata header (schema/baseline/scope/structure notes), totals,
parse coverage, reconciliation and leftovers blocks — all derived from raw help dumps
12–16 which are present in-tree. I reproduced **every number the file asserts about
itself** (totals, categories, module sections, vlcrc walk, reconciliation lists) against
the raw dumps and against the JSON's own array — zero numeric mismatches. Its role in
matrix population is exactly what docs/inventory/matrix-seed-notes.md claims: the matrix
seeds 20 config-feature rows and playback/audio/video/playlist rows from it, and all 21
`inventory/options-linux.json#*` anchors in docs/feature-matrix.toml resolve to real
option records (0 misses — I reproduced the claimed verification independently).
The one material defect is enforcement, not truth: the committed tool that "will own
these checks" (tools/xtask feature_audit) strips anchors and never resolves them, so
the "0 misses" property is guaranteed today by nothing in-tree (F-001, S3).

## 2. What actually works (verified)
- `python3 json.load` parses the 766,793-byte file cleanly; structure: 15 top-level keys, `options` = 1,476 records.
- Inventory stats (all reproduced by my scripts over the array):
  - 1,476 records / 1,470 unique names / 6 collisions — collision names exactly `scaletempo-stride, scaletempo-overlap, scaletempo-search, rc-host, glconv, wl-display` (matches totals.vlcrc_duplicate_names).
  - categories: Hot keys 250 · Subpictures 67 · Video 44 · Playlist 35 · Audio 18 · Stream output 16 · Miscellaneous 6 · Input 4 · Decoders 2 · Hacks 1 (inside mp4 module) · null 1,033 (module sections).
  - modules: 179 unique ids (core + 178); core = 442 records (191 basic + 251 advanced) + 1,034 non-core.
  - types: string 519 / integer 420 / bool 267 / enum 162 / float 108; with min: 235 / max: 235 / choices: 162; short_form: 15; negation_form: 267.
  - visibility: longhelp 615 / advanced-only 861 records (855 unique names).
  - in_vlcrc: 1,439 records / 1,433 unique names; in_full_help_not_vlcrc = 37 names and the embedded 37-name list equals the actual set (list == actual: True).
- Raw-dump cross-checks (rg/wc over docs/baselines/raw/linux/):
  - `wc -l`: dump 12 = 1,539 lines, dump 13 = 3,364, dump 14 = 5,509 — exactly `parse_coverage.raw_12/13/14_lines`.
  - dump 16 vlcrc walk: `rg -c '^[#]?[a-zA-Z0-9_.+-]+='` = 1,439 lines; unique names = 1,433; exactly 6 names duplicated ×2 (same six as totals.vlcrc_duplicate_names). The `+` in the name class is required: forward-jump hotkeys `key-jump+extrashort` etc. exist with names containing `+` (8 such records) — the inventory carries them (verified: `key-jump+extrashort` vlcrc_default `Shift+Right`).
  - vlcrc sections: `rg -c '^\['` dump 16 = 191 — matches "191" in options-summary.md §1.
  - module sections: `rg '^ [A-Za-z].+ \([a-z0-9_-]+\)$'` dump 12 = 124 (123 unique ids), dump 13 = 187, dump 14 = 187 (179 unique ids) — matches totals.module_sections_* and unique_module_ids_* exactly.
  - dump 15 `--help` probe: lines 11–417 = 407 lines — matches flag_probes `"--help": "works; 407-line basic dump (raw 15)"`.
  - Hot keys: 250 records = 111 `key-*` + 111 `global-key-*` + 28 non-hotkey options in the category — 111 binary-observed hotkey actions, the same count ui-inventory/matrix-seed-claims.
- Matrix anchor cross-check: extracted all 21 `inventory_ref = "inventory/options-linux.json#<name>"` anchors from docs/feature-matrix.toml (rg); all 21 names exist in the inventory — 0 misses. Bidirectional vlcrc check: every vlcrc name is in the inventory and every in_vlcrc=True inventory name appears in the vlcrc walk (both diffs empty).

## 3. Findings

F-001 [S3]
Where: docs/inventory/matrix-seed-notes.md:206-215 + tools/xtask/src/cmd/feature_audit.rs:457-460
Code (matrix-seed-notes.md:212-215, verbatim):
```
`verified`/`implemented` (no CI); every blocked/unsupported row has a
`blocked_reason`; **P0 ⇒ ph ≤ 6 holds (0 violations)**; all
`inventory/options-linux.json#*` anchors exist in options-linux.json (0 misses);
```
Code (tools/xtask/src/cmd/feature_audit.rs:457-460, verbatim):
```
fn check_token(root: &Path, token: &str) -> Option<String> {
    // Strip a trailing/inline anchor (`docs/x.md#section`).
    let token = token.split('#').next().unwrap_or(token);
```
What is wrong: the "0 misses" anchor claim was produced by `tmp/d2/gen_matrix.py`
(gitignored scratch, absent from the tree — `ls tmp/d2/` fails), and the committed
successor tool explicitly STRIPS the anchor before checking, so it validates only that
`docs/inventory/options-linux.json` exists, never that `#rate`/`#osd`/`#crop` resolve
to real records. Why it matters: a future matrix edit can typo an anchor and CI stays
green; the claimed R8 "inventory completeness" check does not exist in code. What
correct looks like: feature_audit loads the inventory JSON and asserts each
`options-linux.json#<name>` anchor against the 1,476 `option_name` values. Confidence:
HIGH — Verification: ran (my reproduction found 21/21 anchors resolve TODAY; the tool
code read confirms no anchor resolution).

F-002 [S4]
Where: docs/feature-matrix.toml:1134 area (textrender-osd notes) vs docs/inventory/options-linux.json:20180
Code (inventory record, options-linux.json:20180-20196 excerpt, verbatim):
```
   "option_name": "osd",
   "short_form": null,
   "negation_form": "--no-osd",
   "type": "bool",
   "help_text": "On Screen Display",
```
What is wrong: the matrix note "Core --osd* options in inventory" implies a family; the
core family is exactly ONE option (`--osd`, bool). The only other osd-prefixed records
are the 8 `rmtosd-*` options of the remoteosd module (options-linux.json:2457-2580).
Blast radius trivial (wording). Confidence: HIGH — Verification: ran (rg over names).

F-003 [S4]
Where: docs/feature-matrix.toml:1894-1898 (vfeat-deinterlace description) vs inventory deinterlace-mode record
Code (inventory `deinterlace-mode` choices, verbatim from parsed JSON):
```
choices = ['auto,discard,blend,mean,bob,linear,x,yadif,yadif2x,phosphor,ivtc']
```
What is wrong: the row lists 8 of the 11 modes ("Discard, Blend, Mean, Bob, Linear, X,
Yadif, Yadif(2x)"), omitting auto/phosphor/ivtc — i.e. it hides the default `auto` mode
a user actually sees. The anchored options (`deinterlace`, `deinterlace-mode`) exist as
claimed, so this is a description-quality defect only. Confidence: HIGH — Verification:
ran (json field read).

No S0/S1/S2 findings: no fabricated numbers, no invented options, no stale anchors. The
`rate-step` absence that the playback-rate row flags as "[VERIFY V-08]" is honest — VLC
3.0.23's dump 14 has no `--rate-step` and neither does the inventory (verified by name
search).

## 4. Per-record spot-check vs feature-matrix.toml rows (10 entries, all sed-verified)
| # | Option (JSON line) | Matrix row citing it | Matrix claim checked | Verdict |
|---|---|---|---|---|
| 1 | rate (L20640) | playback-rate (L1528) | "+/−/=/]/[ hotkeys" — inventory: key-faster `+`, key-slower `-`, key-rate-normal `=`, key-rate-faster-fine `]`, key-rate-slower-fine `[`; rate float default 1.000000 | MATCH (5/5 hotkeys + option) |
| 2 | key-jump-extrashort (L22968) | playback-seek (L1516) | "4 jump magnitudes"; "3/10/60/300 s" — extrashort/short/medium/long-jump-size defaults 3/10/60/300; backward `Shift+Left` + forward `key-jump+extrashort` `Shift+Right` | MATCH exact |
| 3 | volume-step (L19075) | afeat-volume (L1665) | "--volume-step/--gain options" — both present; volume-step float [1.000000..256.000000] default 12.800000 | MATCH |
| 4 | subsdec-encoding (L3656) | subtitle-encoding-detection (L1091) | option exists, enum with 40+ charset choices | MATCH |
| 5 | sub-autodetect-fuzzy (L20251) | subtitle-autoload (L1079) | fuzziness semantics 0–4 — long_description enumerates exactly 0/1/2/3/4 | MATCH |
| 6 | osd (L20180) | textrender-osd (L1134) | "Core --osd* options" — see F-002: exactly one core osd option | PARTIAL (wording) |
| 7 | audio-desync (L19177) | playback-av-sync (L1564) | "--audio-desync option" — integer (ms) default 0, advanced-only | MATCH |
| 8 | start-time (L20572) | playback-start-options (L1624) | "start-time, stop-time, run-time, start-paused" — all 4 present in Playback control/Playlist | MATCH |
| 9 | random (L21835) | plfeat-random (L1954) | "--loop/--repeat/--random" — all three, short forms L/R/Z | MATCH |
| 10 | snapshot-path (L19679) | playback-snapshot (L1600) | "Snapshot subsection (7 options)"; "vlcsnap- prefix" — exactly 7 records in subsection Snapshot; snapshot-prefix default `vlcsnap-`; snapshot-format enum default `png` | MATCH exact |

Additional traced claims (bonus, not counted in the 10): playback-bookmarks "20 hotkeys
(Ctrl+F1–F10 / F1–F10)" — inventory has key-set-bookmark1..10 (`Ctrl+F1..F10`) and
key-play-bookmark1..10 (`F1..F10`), all 20 verified; plfeat-random row's matrix
comment "Playlist category, 35 options" (feature-matrix.toml:1932) — Playlist = 35
records exactly; afeat-replaygain "--replay-gain-* cluster" — 4 records
(audio-replay-gain-{default,mode,peak-protection,preamp}).

## 5. Coverage-claim vs inventory size (the ledger-A question)
- matrix-seed-notes.md:95 claims "1,476 options" — actual: 1,476. MATCH.
- matrix-seed-notes.md:95 claims "20 config-feature rows" — actual: exactly 20 `config-*` ids, 13 of which are `config-area-*` seed areas. MATCH.
- matrix-seed-notes.md:97 claims "442 core options map via the 13 seed areas" — actual: 442 core records, 13 config-area rows. MATCH.
- Direct anchor coverage: 21 distinct options anchored out of 1,476 (1.42%). This is NOT a shortfall claim — the frozen scoping rule is "individual options are never rows" (matrix-seed-notes.md:96), so the matrix never claims per-option row coverage; clusters (Snapshot subsection, replay-gain family, Playback control subsection = 9 options) are grouped. No inflation found.
- Worklog claim (docs/worklog/archive-2026-10.md:503) "615 longhelp-visible (41.7%) vs 861 advanced-only (58.3%)" — reproduced: 615/1,476 = 41.66%, 861/1,476 = 58.33%. MATCH.
- options-summary.md §1 headline table — every row I tested reproduced (1476/615/861+855/1470+6/442+191+251/1034/124-123-187-179/191-1439-1433/types/235-162-15).

## 6. Role in matrix population (verified chain)
docs/feature-matrix.toml:27-29 (verbatim, sed-verified) lists options-linux.json among
the population sources; matrix-seed-notes.md §3.2 documents the mapping; the 21
`inventory_ref` anchors are the live, checkable residue of that seeding; tools/xtask
feature_audit.rs:597 exercises the `inventory/…` shorthand for evidence-path mapping
(test only, file-existence semantics — see F-001). Rows citing options spans:
playback-op (play-and-pause, key-jump-extrashort, rate, audio-track, audio-desync,
key-chapter-next, bookmarks, snapshot-path, key-toggle-fullscreen, start-time,
input-record-path), afeat-volume (#volume-step), afeat-replaygain
(#audio-replay-gain-mode), vfeat rows (#aspect-ratio, #crop, #zoom, #deinterlace),
plfeat-random (#random), subtitle rows (#sub-autodetect-fuzzy, #subsdec-encoding),
textrender-osd (#osd).

## 7. Dead / duplicated / unreachable content
- Duplicated names (6) are VLC's own cross-module collisions (scaletempo×2 modules, rc-host in rc+rc legacy, glconv, wl-display) — faithfully recorded, not parser noise.
- The 37 not-in-vlcrc options are annotated in `reconciliation.in_full_help_not_vlcrc_annotation` (CLI/meta flags, handle/callback options, remainder for [VERIFY] review) — consistent with dump reality.
- Scope honesty: `scope_notes.consequence` declares the qt/skins2/jack/... families ABSENT because the pinned headless install excluded those Debian packages; options-windows/macos skeletons reference this limitation rather than fabricating counts. No fabricated cross-platform data.

## 8. Dependencies & flags
None. Pure JSON data; no build inputs. Raw derivation inputs present in-tree:
docs/inventory/raw/options/parse-options.py + manifest.json; docs/baselines/raw/linux/12–16.
Regeneration NOT re-runnable here (pinned VLC binary `tmp/pinning/extract/usr/bin/vlc` absent
from the sandbox — clean-room statement says no binaries committed, so this is by design,
not a defect). Regenerability: REAL-UNPROVEN in this sandbox.

## 9. Open questions for the next agent
1. F-001 fix: make tools/xtask feature_audit resolve `#anchor` suffixes against
   options-linux.json/formats.json content (the R8 rule the header promises).
2. The `key-jump+*` forward-jump names use `+` — any consumer regex that excludes `+`
   will under-count (my first regex missed 8 lines; the parser did not).
3. Windows/macOS options inventories remain [VERIFY-blocked-on-hardware] skeletons —
   cross-check them when hardware dumps land.
