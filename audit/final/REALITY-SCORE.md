# REALITY-SCORE — the aggregated matrix-row verdict count (Devil Audit §8.1)

| Field | Value |
|---|---|
| Auditor | W20-A07 (reality scoring) |
| Date | 2026-11-24 |
| Scope | ALL 252 rows of `docs/feature-matrix.toml`, verdicts aggregated from the 20 row ledgers `audit/docs/feature-matrix.toml.rows-*.audit.md` (W18-A05..A10, W19-A01..A10) |
| Headline metric | **REAL% = 49 / 252 = 19.44%** (floor 19% — no rounding up) |
| Row-count reconciliation | 252 / 252 (see §6) |
| Commands run this session | `python3` tomllib parse of feature-matrix.toml (252 rows, 21 categories enumerated); `rg` censuses for rows 251/252 (wallpaper/360, lua); `sed -n` verifications; baseline citations (`audit/runs/baseline/xtask-parity-report.txt`, `cargo-test-workspace.txt`, `rc-smoke.txt`, `ui-test.txt` per ledgers) |

---

## 1. Headline — verdict distribution over all 252 rows

| Verdict | Rows | Share | Meaning (canonical, this file) |
|---|---|---|---|
| **REAL** | **49** | **19.44%** | Row claims implemented/partial AND the claimed feature was executed working (linux lane; orchestrator baseline or ledger-run evidence) |
| ACCURATE-ABSENT | 110 | 43.7% | Row claims not_started/blocked/unsupported and the claim is TRUE — the matrix telling the truth about MISSING features. NOT REAL. |
| SHALLOW | 29 | 11.5% | Claims a working subset; only a happy-path/data/editor shell works; core refused, unexecuted, or unreachable |
| UNDERSTATED | 24 | 9.5% | Row says not_started/"no work yet" while real code has landed (the anti-inflation lie direction) |
| UNREACHABLE | 18 | 7.1% | Real, tested library code that NO shipped entry point links/calls (subtitles, playlists, dnx, colors-adjust) |
| FALSE | 14 | 5.6% | Status/claim contradicts the code (inflated or misclassified; incl. row 163's fabricated evidence citation) |
| REAL-UNPROVEN | 8 | 3.2% | Genuine code, could not execute (TLS handshake, real-device audio, git-hash evidence, sandbox blocks) |
| **TOTAL** | **252** | 100% | buckets sum to 252 exactly |

**Headline: REAL% = 49/252 = 19.44% (do not round up; floor = 19%).**
For contrast: 110 rows (43.7%) are the matrix *correctly documenting absence*. The
matrix's biggest property is not that it lies about what works — it is that 43.7%
of VLC's surface simply does not exist in this repo yet, and another 36 rows
(14 FALSE + 18 UNREACHABLE + 24 UNDERSTATED) mis-state reality in one direction
or the other.

If one insists on a "code-exists-that-works-somewhere" super-metric:
REAL + REAL-UNPROVEN = 57/252 = 22.6%; add UNREACHABLE (real code, no consumer)
= 75/252 = 29.8%. None of these is the headline. The headline is 19.44%.

## 2. Methodology — canonical mapping rules (documented, per task)

The 20 ledgers use heterogeneous vocabularies ("REAL (honest)", "TRUE",
"PAPER-ONLY, accurate", "FALSE (understated)", "SHALLOW underclaim",
"REAL code; UNREACHABLE"). Canonical rules applied:

- **R1 — ledger's own verdict.** Row verdict = the ledger's per-row verdict table
  (§5 of each ledger), primary (first) term. Where a ledger's header roll-up and
  its §5 table disagree, §5 wins; conflicts are listed in §6.
- **R2 — truthful absence is NOT REAL.** Ledger verdicts "TRUE (accurate)",
  "HONEST", "ACCURATE-no-code", "PAPER-ONLY, accurate", "REAL (honest blocked)",
  "REAL (wont-do)", "REAL negative" on rows claiming not_started/blocked/
  unsupported map to **ACCURATE-ABSENT**. Four ledgers label
  honest-absence rows "REAL": this re-mapping moves 21 ledger-"REAL" rows out of
  the REAL bucket (L4 "REAL (honest blocked)": 6; L14 "REAL negative": 6;
  L16 "REAL (wont-do/absence)": 4; L9 "REAL — honest row": 2; L18 "REAL
  (honest)": 2; L19 "REAL (honest blocked)": 1).
- **R3 — real code, no shipped consumer → UNREACHABLE.** Compound verdicts of the
  shape "REAL code + tests; UNREACHABLE from shipped entry points" (subtitle
  rows 057-059/064/065/067/068/072/074, playlist rows 077-079/082, dnxhd 035,
  colors-adjust 129, playlist-feature 136-138) map to **UNREACHABLE**, per the task's
  own bucket definition. A row whose code no shipped binary can reach does not
  count as REAL no matter how green its unit tests are.
- **R4 — understatement bucket.** Ledger verdicts "FALSE (understated)",
  "SHALLOW underclaim", "FALSE-as-stated (no work yet)" on not_started rows where
  code landed map to **UNDERSTATED** (the task's own bucket). Applies to
  084, 095, 099-101, 142, 152, 156, 170-175, 180, 187, 222-226, 228, 232-233.
- **R5 — stale partial claims stay FALSE.** "FALSE (stale-understated)" on
  *partial*-status rows (149-151: "NOT registered" while registered) remain
  **FALSE**, understating direction noted.
- **R6 — honest partial with verified working subset → REAL.** Partial rows whose
  claimed subset was executed working (038 pcm, 041 aac-lc, 027 vp9, 104 rate,
  201 av-tools, 219 menu-open) count as REAL; partial rows whose subset cannot
  work in any shipped host (022, 024, 097, 098, 103, 133, 139, 204, 210, 221,
  229-231, 235-246, 248, 249) count as SHALLOW/FALSE/UNREACHABLE per ledger.
- **R7 — one verdict per row; platforms are notes.** "linux REAL; win/macos
  REAL-UNPROVEN" rows count once as REAL (linux-executed basis, matching the
  ledger-1 convention "row verdicts, linux-executed basis"); win/mac unproven cells
  are recorded in §4, not as extra rows.
- **R8 — re-keying ledger mistakes.** L12 mis-ordered ids for rows 148-152
  (placed config-area-playback at 152; toml has it at 148); verdicts were re-keyed
  BY ROW ID. L20 mislabeled its chunk by +2 (it audited toml rows 246-250 as
  "248-252"); its verdicts were re-keyed by id, and the two rows no ledger covered
  (251 hotkey-wallpaper-360, 252 ext-lua-extensions) were censused by W20-A07
  directly (§6).

Sensitivity notes (judgment calls, each ±1 row, none changes the headline by more
than 0.4 pt): row 012 (adts: "REAL demux / FALSE end-to-end") counted REAL per
ledger §5 primary term; row 035 counted UNREACHABLE though its ledger header
listed it among "5 REAL-rows"; row 141 counted ACCURATE-ABSENT per its ledger §5
("ACCURATE-for-shipped") though the ledger header had listed it under UNREACHABLE
(demo-dead); row 220 counted FALSE (ledger: "Status INFLATED — quit-wired claim
FALSE; row substance = zero") though the ledger header grouped it under SHALLOW;
row 163 counted FALSE per its ledger's official roll-up ("1 FALSE — row 163
evidence citation FALSE") even though the same ledger calls the *status* itself
"defensible" — had it been counted REAL, REAL% would be 50/252 = 19.84%.

## 3. Per-category breakdown (21 categories, rows sum = 252)

| Category (rows) | REAL | REAL-UNPROVEN | SHALLOW | FALSE | UNDERSTATED | UNREACHABLE | ACCURATE-ABSENT | Total |
|---|---|---|---|---|---|---|---|---|
| container (001-021) | 13 | 0 | 0 | 1 | 0 | 0 | 7 | 21 |
| video-codec (022-037) | 4 | 0 | 2 | 0 | 0 | 1 | 9 | 16 |
| audio-codec (038-056) | 9 | 0 | 0 | 0 | 0 | 0 | 10 | 19 |
| subtitle-format (057-073) | 0 | 0 | 0 | 0 | 0 | 8 | 9 | 17 |
| text-render (074-076) | 0 | 0 | 0 | 0 | 0 | 1 | 2 | 3 |
| playlist-format (077-083) | 0 | 0 | 0 | 0 | 0 | 4 | 3 | 7 |
| protocol-access (084-094) | 1 | 0 | 0 | 0 | 1 | 0 | 9 | 11 |
| network-feature (095-101) | 1 | 0 | 2 | 0 | 4 | 0 | 0 | 7 |
| playback-op (102-114) | 3 | 2 | 3 | 4 | 0 | 0 | 1 | 13 |
| audio-feature (115-125) | 0 | 0 | 0 | 2 | 0 | 0 | 9 | 11 |
| video-feature (126-135) | 0 | 0 | 1 | 0 | 0 | 1 | 8 | 10 |
| playlist-feature (136-144) | 0 | 0 | 1 | 0 | 1 | 3 | 4 | 9 |
| config-feature (145-164) | 5 | 5 | 0 | 5 | 2 | 0 | 3 | 20 |
| plugin-feature (165-168) | 0 | 0 | 0 | 0 | 0 | 0 | 4 | 4 |
| platform-feature (169-186) | 0 | 1 | 0 | 0 | 7 | 0 | 10 | 18 |
| services-discovery (187-192) | 0 | 0 | 0 | 1 | 1 | 0 | 4 | 6 |
| ui-screen (193-203) | 10 | 0 | 0 | 0 | 0 | 0 | 1 | 11 |
| ui-dialog (204-218) | 2 | 0 | 2 | 0 | 0 | 0 | 11 | 15 |
| menu-item (219-234) | 1 | 0 | 4 | 1 | 8 | 0 | 2 | 16 |
| hotkey (235-251) | 0 | 0 | 14 | 0 | 0 | 0 | 3 | 17 |
| extension (252) | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 |
| **TOTAL** | **49** | **8** | **29** | **14** | **24** | **18** | **110** | **252** |

Category reality in one line each:
- **container (13/21 REAL)** — the strongest category: mkv/mp4/avi/wav/aiff/ogg/
  mp3/adts/mp2… executed open→decode→Ended on linux. Rot: asf-wmv 007 FALSE
  (implemented on win/mac for a demuxer NO binary can open anywhere); MP4 seek S0
  (002); TS 188-byte panic S1 (004); adts family cannot PLAY (012).
- **video-codec (4/16 REAL)** — h264-sw/av1/vp9/mjpeg real+executed; 022/024
  partial-honest SHALLOW; 035 dnxhd UNREACHABLE (no demuxer mints `dnxhd`).
- **audio-codec (9/19 REAL)** — the most honest quadrant (L4's own words); 10
  honest absences; zero false statuses; note-level fabrications only (adpcm
  symphonia attribution, lewton/alac "oracle" crates that don't exist).
- **subtitle-format (0/17 REAL, 8 UNREACHABLE)** — real, heavily tested crate
  (267 unit tests) that NO shipped binary links. "implemented" cells are
  fiction-by-omission for the entire category.
- **text-render / playlist-format (0 REAL; 5 UNREACHABLE of 10)** — same disease:
  real parsers, zero shipped consumers.
- **protocol-access (1 REAL; 84 UNDERSTATED)** — protocol-file is the matrix's
  most embarrassing row: the first vertical's core feature marked "no work yet".
- **network-feature (1 REAL, 4 UNDERSTATED, 2 SHALLOW)** — ~4,900 lines of
  tested streaming code hidden as "not_started"; hls/dash unreachable lanes.
- **playback-op (3 REAL / 4 FALSE)** — the fraud epicenter: frame-step,
  chapters, bookmarks, snapshot claim partial/implemented while every shipped
  host answers the typed refusal `UnsupportedForHost`.
- **audio-feature (0/11 REAL, 2 FALSE)** — volume/EQ "work" only in the browser
  dev-shell simulation (DemoTransport).
- **video-feature (0/10 REAL)** — all honest absence except colors-adjust 129
  (real adjust.rs, zero callers).
- **playlist-feature (0/9 REAL; 3 UNREACHABLE)** — dead playlist surface;
  142 hides a tested `--start-paused`.
- **config-feature (5 REAL / 5 REAL-UNPROVEN / 5 FALSE / 2 UNDERSTATED)** —
  the stalest category: 3 areas registered-but-claimed-"NOT registered", 2 areas
  landed-but-"not_started", library area inflated, one deleted evidence path.
- **plugin-feature (4/4 ACCURATE-ABSENT)** — honest emptiness; orphan wasm crate.
- **platform-feature (0 REAL; 7 UNDERSTATED)** — wasapi/coreaudio/pulse FFI +
  ~7k LOC hw adapters + shipped wgpu compositor all hidden behind "no work yet".
- **services-discovery (0 REAL; 189 FALSE)** — SSDP core in-tree, "blocked".
- **ui-screen (10/11 REAL)** — the strongest UI category; every implemented
  screen is real with passing vitest suites; the 2 evidence defects (193
  KeyHintStrip.tsx fabricated path, 194 wrong file) do not break the screens.
- **ui-dialog (2 REAL, 2 SHALLOW of 15)** — goto-time/network real; bookmarks
  + open-file overclaim; 11 honest absences.
- **menu-item (1 REAL / 8 UNDERSTATED / 4 SHALLOW / 1 FALSE)** — menus exist in
  PlayerScreen canvas, matrix says "no work yet" or cites a removed menubar.
- **hotkey (0/17 REAL, 14 SHALLOW)** — seed + editor + ~8 live keys of 141
  seeded; every engine half dead or unserved.

## 4. Per-platform notes

- **linux** — the only basis with executions. All 49 REAL verdicts rest on
  linux-lane evidence: orchestrator baseline (`cargo-test-workspace.txt`
  4744 passed/2 failed, `rc-smoke.txt` headless-decode/command-layer-play PASS,
  `ui-test.txt` 1705 vitest) plus ledger-run `lumen open` walks. Linux REAL%
  itself = 49/252 = 19.44%.
- **windows / macos** — 66/252 implemented cells each (live baseline tool run,
  2026-11-23) with ZERO per-OS executions anywhere on record. Rows' own notes
  admit "Linux-only evidence, CI 3-OS PENDING". Worst cells are pure fiction:
  row 007 (asf) claims windows=implemented/macos=implemented for code that
  cannot open a file on ANY platform; rows 057/059/064/065/067/068/078/079/082
  claim implemented on all three desktops for crates no shipped binary links.
  Desktop-tauri never built in sandbox (gdk-sys/GTK missing) — environment
  limit, recorded UNVERIFIED, not a repo defect.
- **web** — 0/252 implemented, honest (wasm is Phase 7); the web build itself
  exists and passes browser-load (row 180 UNDERSTATED for exactly this reason).
- Per-platform implementation share (live tool, `audit/runs/baseline/
  xtask-parity-report.txt`): windows 66/252 (26.2%), linux 61/252 (24.2%),
  macos 66/252 (26.2%), web 0/252. Verified-only: 0/252 on every platform.

## 5. Delta vs PARITY-REPORT (docs/PARITY-REPORT.md)

PARITY-REPORT's own numbers (frozen 2026-11-10 table in the doc):
- Verified parity (MP §1 strict): **0/252 on every platform, every category**.
- Implementation share (`implemented`+`verified`): **64/252 = 25.4% identical on
  windows/linux/macos**, 0/252 web; its machine line: `parity: rows=252
  windows=64/252 linux=64/252 macos=64/252 web=0/252`.
- Cell census: blocked=133, implemented=192, not_started=505, partial=142,
  unsupported=36 (1008 cells).

The live tool run the orchestrator captured (baseline, 2026-11-23) already
disagrees with the doc's frozen table: **windows 66/252 (26.2%), linux 61/252
(24.2%), macos 66/252** — per-platform statuses drifted after the doc froze
(container 14/13/14, audio-codec 9/6/9, video-codec 4/3/4, protocol-access
1/1/1 vs the doc's 0/11). The doc has no refresh mechanism.

**Delta vs this audit:**
- Headline: this audit **REAL 49/252 (19.44%)** vs PARITY implementation share
  **64/252 (25.4%) doc / 66 win / 61 linux live** → the parity metric
  overstates working reality by **12-15 rows (5.0-6.0 points)**, because an
  `implemented` status cell requires no execution, no reachability and no truth.
- Composition of the linux gap (61 linux-implemented rows): 42 are REAL per this
  audit; **9 are UNREACHABLE** (subtitles srt/vtt/pgs/dvb/cc/vobsub + pls/xspf/
  cue — real crates, zero shipped consumers), **4 SHALLOW** (playback-seek with
  executed S0, track-switch, ui-dialog-open-file, ui-dialog-bookmarks),
  **4 REAL-UNPROVEN** (config-performance, config-rebrand,
  config-hotkey-editor, config-localization),
  **2 FALSE** (config-area-library staged shell, config-advanced-folding
  deleted evidence path). The remaining 7 of the 49 REAL rows carry `partial`
  (not implemented) statuses — parity's implementation share also *undercounts*
  partial-but-working rows (pcm, aac-lc, vp9, rate, menu-open, av-tools).
- **Understatement is invisible to the parity tool**: 24 rows (network 095/
  099-101, protocol-file 084, platform 170-175/180, sd-directory-watch 187,
  menu 222-226/228/232-233, plfeat-autostart 142, config-controls/recording
  152/156) hide landed code behind "not_started". Parity credits them 0 —
  wrong in the conservative direction, still wrong (L14's own words:
  "the matrix undercounts progress in the safe direction — no fraud, but
  'not_started' is factually wrong").
- **What PARITY-REPORT gets right (verified here):** verified parity 0/252 is
  the truthful number — no row is CI-verified on any claimed platform; the
  doc explicitly separates implementation share from verified parity and
  refuses to let 64 be read as "works"; web=0 honest; blocked/partial cells
  carry reasons; the tool derives from the matrix, nothing hardcoded.
- **What it gets wrong / misses:** (1) implementation share launders
  UNREACHABLE library code (9 subtitle/playlist rows) and FALSE/SHALLOW rows
  (config 153/163, seek 103, dialogs 204/210) as platform-delivered features;
  (2) its R4 evidence-resolution under-detects — fabricated prose-embedded
  paths (row 193 `KeyHintStrip.tsx`), wrong-file citations (219-221, 244, 204)
  and drifted line anchors (087-093) all pass the tool while its own run flags
  only 2 violations; (3) it has no mechanism to catch status-vs-code drift in
  either direction — the 19 stale cells found here (L12/L13/L14/L15) prove the
  matrix rots between tool runs; (4) the doc's frozen 64/64/64 is already
  stale vs its own live 66/61/66 run.

## 6. Row-count reconciliation (must total 252)

| Source ledger | Chunk claimed | Actual toml rows covered | Status |
|---|---|---|---|
| L1 W18-A07 rows-001-013 | 001-013 | 001-013 | ids verified vs toml: exact |
| L2 W18-A08 rows-014-026 | 014-026 | 014-026 | exact |
| L3 W18-A09 rows-027-039 | 027-039 | 027-039 | exact |
| L4 W18-A10 rows-040-052 | 040-052 | 040-052 | exact |
| L5 W18-A07 rows-053-065 | 053-065 | 053-065 | exact |
| L6 W18-A08 rows-066-078 | 066-078 | 066-078 | exact |
| L7 W18-A09 rows-079-091 | 079-091 | 079-091 | exact |
| L8 W18-A10 rows-092-104 | 092-104 | 092-104 | exact |
| L9 W18-A06 rows-105-117 | 105-117 | 105-117 | exact |
| L10 W18-A05 rows-118-130 | 118-130 | 118-130 | exact |
| L11 W19-A01 rows-131-143 | 131-143 | 131-143 | exact |
| L12 W19-A06 rows-144-156 | 144-156 | 144-156 | **id order mis-mapped** (ledger put config-area-playback at 152; toml has it at 148; video/audio/subtitles/controls shift accordingly). Verdicts re-keyed BY ID — net bucket counts unchanged |
| L13 W19-A05 rows-157-169 | 157-169 | 157-169 | exact |
| L14 W19-A04 rows-170-182 | 170-182 | 170-182 | exact |
| L15 W19-A09 rows-183-195 | 183-195 | 183-195 | exact |
| L16 W19-A10 rows-196-208 | 196-208 | 196-208 | exact |
| L17 W19-A07 rows-209-221 | 209-221 | 209-221 | exact |
| L18 W19-A03 rows-222-234 | 222-234 | 222-234 | exact |
| L19 W19-A08 rows-235-247 | 235-247 | 235-247 | exact |
| L20 W19-A02 rows-248-252 | 248-252 | **246-250** (off-by-two: audited hotkey-navigation/nav-disc/snapshot-record/interface/global under numbers 248-252) | re-keyed by id; rows 246/247 double-covered (L19+L20 verdicts agree: SHALLOW/ACCURATE — no conflict) |
| **W20-A07 (this agent)** | 251-252 | 251-252 | censused directly: 251 hotkey-wallpaper-360 (no engine code — rg `wallpaper|equirect|360.?video` over crates/apps: 0; only a keymap seed marked `disposition not-adopted-v1` at keymaps/default.toml:172 and an i18n hint) → ACCURATE-ABSENT; 252 ext-lua-extensions (rg `\blua\b` crates/apps/ui: 0 code; extensions.tsx:19-26 "NO extensions command exists … wasm-extensions = false … the list seam below is EMPTY") → ACCURATE-ABSENT |

Distinct rows covered: 001-247 (L1-L19) + 246-250 (L20) + 251-252 (W20-A07) =
**252 distinct** (246, 247 double-covered with consistent verdicts). Every row
has exactly one canonical verdict; buckets sum to 252. **Reconciliation: PASS.**

The L20 off-by-two is itself a finding: the ledger wave believed rows 248-252
were hotkey-navigation…hotkey-global, silently dropped the true tail rows 251-252,
and re-audited two rows already owned by L19. Any wave that trusts the ledger
coverage map inherits the hole.

## 7. Sanity check — 5 rows re-verified against feature-matrix.toml directly

Executed this session (`python3 tomllib` + row print); all match the ledgers' quotes:

| Row | id | toml status (verbatim keys) | Ledger's quote | Match |
|---|---|---|---|---|
| 007 | container-asf-wmv | windows=implemented, linux=partial, macos=implemented, web=not_started | L1: "impl / partial / impl / not_started" | ✓ |
| 084 | protocol-file | not_started ×4, evidence="" | L7: "status = not_started on all four platforms; evidence = \"\"; notes: 2.A1 input crate; first vertical" | ✓ |
| 129 | vfeat-colors-adjust | partial ×3, web=not_started, evidence cites `crates/video-engine/src/adjust.rs` | L10: "partial/partial/partial/ns" + adjust.rs evidence | ✓ |
| 163 | config-advanced-folding | implemented ×3, web=not_started, evidence cites `ui/src/screens/settings/areas/advanced.tsx` | L13: "impl/impl/impl/ns" + advanced.tsx citation (file verifiably absent — xtask R4 violation) | ✓ |
| 220 | menu-media-convert-quit | partial ×3, web=not_started, evidence "menu.file.exit (quit wired); Convert/Stream items absent" | L17: "partial/partial/partial/ns" + quit-wired claim (quit item ships `disabled: true`) | ✓ |

---

## Appendix A — per-row verdict table (252 rows: row → id → verdict → ledger source)

Legend: L1..L20 = audit/docs/feature-matrix.toml.rows-XXX-YYY.audit.md (auditors
W18/W19 as listed in §6); W20-A07 = censused by this agent. Verdict key:
REAL / REAL-UNPROVEN / SHALLOW / FALSE / UNDERSTATED / UNREACHABLE / ACCURATE-ABSENT.
| Row | id | Category | Verdict | Source | Note |
|---|---|---|---|---|---|
| 001 | container-matroska-webm | container | REAL | L1 | lacing S1 + keyframe S2 + duration Unknown; win/mac unproven |
| 002 | container-mp4-mov-3gp | container | REAL | L1 | open+decode executed; in-session seek executed-FAIL (S0 reanchor) |
| 003 | container-ogg | container | REAL | L1 | FLAC-in-Ogg lane broken (S1 executed) |
| 004 | container-mpegts | container | REAL | L1 | 188-byte TS panic S1 in shipped open path |
| 005 | container-mpegps | container | REAL | L1 | honest row; zero ps/vob corpus lanes self-admitted |
| 006 | container-avi | container | REAL | L1 | cleanest row; mock-labeled evidence honest |
| 007 | container-asf-wmv | container | FALSE | L1 | probe.no-demuxer exit 65 twice; win/mac "implemented" fiction; asf code unreachable |
| 008 | container-wav | container | REAL | L1 | RF64 >4GiB untested, flagged |
| 009 | container-flac-native | container | ACCURATE-ABSENT | L1 | typed refusal matches not_started; honest |
| 010 | container-aiff | container | REAL | L1 | uLaw breadth unverified |
| 011 | container-au | container | ACCURATE-ABSENT | L1 | honest |
| 012 | container-adts-aac | container | REAL | L1 | demux real+executed; family cannot PLAY (cli.audio_limb_absent) — end-to-end FALSE |
| 013 | container-mp3-raw | container | REAL | L1 | duration Unknown on raw ES; attached-pic divergence recorded |
| 014 | container-mpeg-es-video | container | ACCURATE-ABSENT | L2 | no mpegv wrapper; honest |
| 015 | container-h264-annexb | container | REAL | L2 | baseline corpus lane green |
| 016 | container-hevc-annexb | container | REAL | L2 | HVCC [VERIFY] self-flagged |
| 017 | container-ivf | container | REAL | L2 | fixture-only evidence string STALE-conservative |
| 018 | container-realmedia | container | ACCURATE-ABSENT | L2 | zero hits; honest |
| 019 | container-midi-smf | container | ACCURATE-ABSENT | L2 | honest |
| 020 | container-archive-zip | container | ACCURATE-ABSENT | L2 | only .zip() iterator hits; honest |
| 021 | container-census-long-tail | container | ACCURATE-ABSENT | L2 | honest |
| 022 | codec-video-h264 | video-codec | SHALLOW | L2 | honest partial; hw lane UNREACHABLE from shipped entry; from_setting has zero prod callers |
| 023 | codec-video-h264-software | video-codec | REAL | L2 | release lane; e2e lane to Ended executed |
| 024 | codec-video-hevc | video-codec | SHALLOW | L2 | honest partial; no HEVC decode in any shipped binary (refused exit 65) |
| 025 | codec-video-hevc-software | video-codec | ACCURATE-ABSENT | L2 | blocked honest; no h265 crate |
| 026 | codec-video-av1 | video-codec | REAL | L2 | description clause "UI states active backend" PAPER-ONLY (S3) |
| 027 | codec-video-vp9 | video-codec | REAL | L3 | linux; win/mac REAL-UNPROVEN; compat 2/3 honest partial demotion |
| 028 | codec-video-vp8 | video-codec | ACCURATE-ABSENT | L3 | blocked accurate |
| 029 | codec-video-theora | video-codec | ACCURATE-ABSENT | L3 | blocked accurate |
| 030 | codec-video-mpeg1-2 | video-codec | ACCURATE-ABSENT | L3 | parser real; decode blocked honest |
| 031 | codec-video-mpeg4-asp | video-codec | ACCURATE-ABSENT | L3 | blocked accurate |
| 032 | codec-video-mjpeg | video-codec | REAL | L3 | first-vertical; rc-smoke 60/60; compat 33 tracks |
| 033 | codec-video-raw-fourcc | video-codec | ACCURATE-ABSENT | L3 | honest |
| 034 | codec-video-wmv-vc1 | video-codec | ACCURATE-ABSENT | L3 | descriptors only; honest |
| 035 | codec-video-prores-dnxhd | video-codec | UNREACHABLE | L3 | dnx code+6/6 tests real; NO demuxer mints dnxhd — shipped binary cannot reach it (ledger header listed it among 5 REAL-rows; demoted per canonical R3) |
| 036 | codec-video-ffv1 | video-codec | ACCURATE-ABSENT | L3 | honest |
| 037 | codec-video-legacy-blocked | video-codec | ACCURATE-ABSENT | L3 | honest |
| 038 | codec-audio-pcm | audio-codec | REAL | L3 | f32 tolerance 6/6 FAIL self-reported (S2); linux partial honest |
| 039 | codec-audio-mp3 | audio-codec | REAL | L3 | linux; win/mac REAL-UNPROVEN |
| 040 | codec-audio-mp2 | audio-codec | REAL | L4 | notes count mismatch (±1-LSB "4 rows" vs record) |
| 041 | codec-audio-aac-lc | audio-codec | REAL | L4 | linux partial honest; notes misdiagnose aac.channel_count refusal as "no decoder" |
| 042 | codec-audio-he-aac | audio-codec | ACCURATE-ABSENT | L4 | honest blocked; implicit-ADTS silently half-rate (duty clause false) |
| 043 | codec-audio-flac | audio-codec | REAL | L4 | byte-exact vs ffmpeg+STREAMINFO MD5 |
| 044 | codec-audio-alac | audio-codec | REAL | L4 | linux partial honest; notes misdiagnose alac.config.missing |
| 045 | codec-audio-vorbis | audio-codec | REAL | L4 | "lewton kept as oracle" note FALSE (crate absent) |
| 046 | codec-audio-opus | audio-codec | REAL | L4 | compat clean 4/4 |
| 047 | codec-audio-ac3-eac3 | audio-codec | ACCURATE-ABSENT | L4 | honest blocked |
| 048 | codec-audio-dts | audio-codec | ACCURATE-ABSENT | L4 | honest blocked |
| 049 | codec-audio-truehd-mlp | audio-codec | ACCURATE-ABSENT | L4 | honest blocked; scratch differential doc-corroborated only |
| 050 | codec-audio-adpcm | audio-codec | REAL | L4 | notes cite nonexistent symphonia-codec-adpcm (fabricated attribution) |
| 051 | codec-audio-wma | audio-codec | ACCURATE-ABSENT | L4 | honest blocked |
| 052 | codec-audio-wavpack | audio-codec | ACCURATE-ABSENT | L4 | honest blocked |
| 053 | codec-audio-speex | audio-codec | ACCURATE-ABSENT | L5 | blocked honest; crate-search UNVERIFIED offline |
| 054 | codec-audio-musepack | audio-codec | ACCURATE-ABSENT | L5 | blocked honest |
| 055 | codec-audio-midi-synth | audio-codec | ACCURATE-ABSENT | L5 | blocked honest; external crate facts UNVERIFIED |
| 056 | codec-audio-census-tail | audio-codec | ACCURATE-ABSENT | L5 | not_started honest |
| 057 | subtitle-srt | subtitle-format | UNREACHABLE | L5 | crate unlinked: only fuzz/Cargo.toml:50 depends on subtitles |
| 058 | subtitle-ass-ssa | subtitle-format | UNREACHABLE | L5 | partial honest; same unlinked-crate defect |
| 059 | subtitle-vtt | subtitle-format | UNREACHABLE | L5 | same class |
| 060 | subtitle-microdvd | subtitle-format | ACCURATE-ABSENT | L5 | honest |
| 061 | subtitle-subviewer | subtitle-format | ACCURATE-ABSENT | L5 | honest |
| 062 | subtitle-sami | subtitle-format | ACCURATE-ABSENT | L5 | honest |
| 063 | subtitle-ttml | subtitle-format | ACCURATE-ABSENT | L5 | honest |
| 064 | subtitle-pgs | subtitle-format | UNREACHABLE | L5 | row history admits weeks of staleness |
| 065 | subtitle-dvb-subs | subtitle-format | UNREACHABLE | L5 | honest gaps admitted |
| 066 | subtitle-teletext | subtitle-format | ACCURATE-ABSENT | L6 | blocked honest |
| 067 | subtitle-cc-608-708 | subtitle-format | UNREACHABLE | L6 | 26+11 tests green; no shipped entry point links subtitles |
| 068 | subtitle-vobsub | subtitle-format | UNREACHABLE | L6 | same class |
| 069 | subtitle-kate | subtitle-format | ACCURATE-ABSENT | L6 | honest |
| 070 | subtitle-forced | subtitle-format | ACCURATE-ABSENT | L6 | honest |
| 071 | subtitle-autoload | subtitle-format | ACCURATE-ABSENT | L6 | no sidecar logic; honest |
| 072 | subtitle-encoding-detection | subtitle-format | UNREACHABLE | L6 | honest partial; library only |
| 073 | subtitle-census-tail | subtitle-format | ACCURATE-ABSENT | L6 | honest |
| 074 | textrender-subtitle-rasterizer | text-render | UNREACHABLE | L6 | placeholder block glyphs only, and unwired |
| 075 | textrender-osd | text-render | ACCURATE-ABSENT | L6 | zero OSD code; honest |
| 076 | textrender-subsource-overlays | text-render | ACCURATE-ABSENT | L6 | no marq/rss/logo/mosaic producers; honest |
| 077 | plfmt-m3u-m3u8 | playlist-format | UNREACHABLE | L6 | playlists not linked by any shipped app |
| 078 | plfmt-pls | playlist-format | UNREACHABLE | L6 | library REAL, no shipped .pls consumer |
| 079 | plfmt-xspf | playlist-format | UNREACHABLE | L7 | parse real+executed; write half FALSE (no XSPF writer); crate unwired |
| 080 | plfmt-asx | playlist-format | ACCURATE-ABSENT | L7 | honest |
| 081 | plfmt-b4s | playlist-format | ACCURATE-ABSENT | L7 | honest |
| 082 | plfmt-cue | playlist-format | UNREACHABLE | L7 | cue.rs real, 20 tests green; no shipped playlist loading |
| 083 | plfmt-census-tail | playlist-format | ACCURATE-ABSENT | L7 | honest |
| 084 | protocol-file | protocol-access | UNDERSTATED | L7 | shipped lumen open lane IS input::FileAccess (baseline exit 0); row says not_started |
| 085 | protocol-directory | protocol-access | ACCURATE-ABSENT | L7 | no directory:// access; honest |
| 086 | protocol-http-https | protocol-access | REAL | L7 | loopback e2e through real binary; win/mac REAL-UNPROVEN |
| 087 | protocol-udp | protocol-access | ACCURATE-ABSENT | L7 | refusal real; evidence line anchors drifted |
| 088 | protocol-rtp | protocol-access | ACCURATE-ABSENT | L7 | same |
| 089 | protocol-rtsp | protocol-access | ACCURATE-ABSENT | L7 | same |
| 090 | protocol-smb | protocol-access | ACCURATE-ABSENT | L7 | same (partial-stale anchors) |
| 091 | protocol-nfs | protocol-access | ACCURATE-ABSENT | L7 | same |
| 092 | protocol-ftp-sftp | protocol-access | ACCURATE-ABSENT | L8 | blocked honest; stale line-cites |
| 093 | protocol-mmsh-mms | protocol-access | ACCURATE-ABSENT | L8 | same |
| 094 | protocol-census-tail | protocol-access | ACCURATE-ABSENT | L8 | honest placeholder rollup |
| 095 | network-http-client | network-feature | UNDERSTATED | L8 | client.rs 998L + ureq shipped on open path; row says "no work yet" |
| 096 | network-tls-policy | network-feature | REAL | L8 | rustls always-on verified; TLS handshake never executed (no egress) |
| 097 | network-hls | network-feature | SHALLOW | L8 | 4215+997L real+tested; NO shipped entry opens m3u8 |
| 098 | network-dash | network-feature | SHALLOW | L8 | 2764+2854L real+tested; dash lane has no session/opener |
| 099 | network-reconnect-buffering | network-feature | UNDERSTATED | L8 | reconnect.rs 1560L + buffer.rs 827L, 30 tests green; row says not_started |
| 100 | network-auth | network-feature | UNDERSTATED | L8 | auth.rs 1562L, 21 tests over real sockets; row says not_started |
| 101 | network-remote-playlists | network-feature | UNDERSTATED | L8 | remote_playlist.rs 500L, 9/9 green; row says not_started |
| 102 | playback-play-pause-stop | playback-op | REAL | L8 | command-layer-play PASS; UI consumption real |
| 103 | playback-seek | playback-op | SHALLOW | L8 | engine real; MP4/TS in-session seek executed-FAIL (S0), no caveat in row |
| 104 | playback-rate | playback-op | REAL | L8 | honest partial; engine rate absent by design, typed refusal proven |
| 105 | playback-frame-step | playback-op | FALSE | L9 | partial x4 incl web; nothing works anywhere (command unserved) |
| 106 | playback-track-switch | playback-op | SHALLOW | L9 | engine+UI real; switch refused in every shipped host |
| 107 | playback-av-sync | playback-op | SHALLOW | L9 | catalog+keymap paper surface; no host, no engine |
| 108 | playback-chapter-title | playback-op | FALSE | L9 | chapters.rs zero callers; CLI reports chapters:[] |
| 109 | playback-bookmarks | playback-op | FALSE | L9 | only live handler is DemoTransport simulation |
| 110 | playback-snapshot | playback-op | FALSE | L9 | planning half only; no pixel-readback seam |
| 111 | playback-fullscreen | playback-op | REAL-UNPROVEN | L9 | web Fullscreen API subset real; desktop blocked by sandbox |
| 112 | playback-start-options | playback-op | REAL-UNPROVEN | L9 | --start-time wired in 11+ decoders; notes contain false consumer claim |
| 113 | playback-record | playback-op | ACCURATE-ABSENT | L9 | honest not_started (ledger called it REAL=honest) |
| 114 | playback-format-detection | playback-op | REAL | L9 | executed in baseline smoke; the one matching row |
| 115 | afeat-volume | audio-feature | FALSE | L9 | runtime volume uncontrollable in any shipped host; demo-shell only |
| 116 | afeat-equalizer | audio-feature | FALSE | L9 | Eq3Band no caller; UI sliders inert-with-reason |
| 117 | afeat-compressor | audio-feature | ACCURATE-ABSENT | L9 | honest not_started (ledger called it REAL=honest) |
| 118 | afeat-spatializer | audio-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent (UI staging stubs where noted) |
| 119 | afeat-stereowiden | audio-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent (UI staging stubs where noted) |
| 120 | afeat-resampler | audio-feature | ACCURATE-ABSENT | L10 | honest; adjacent MicroResampler real+unreachable |
| 121 | afeat-stereo-mode | audio-feature | ACCURATE-ABSENT | L10 | honest; channel_map vocab production-unreachable |
| 122 | afeat-visualizations | audio-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent (UI staging stubs where noted) |
| 123 | afeat-replaygain | audio-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent |
| 124 | afeat-passthrough | audio-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent |
| 125 | afeat-audio-device-switch | audio-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent |
| 126 | vfeat-aspect-ratio | video-feature | ACCURATE-ABSENT | L10 | claim accurate; params.rs library surface unreachable |
| 127 | vfeat-crop | video-feature | ACCURATE-ABSENT | L10 | claim accurate; CropTool local-only |
| 128 | vfeat-zoom-scale | video-feature | ACCURATE-ABSENT | L10 | claim accurate; feature absent |
| 129 | vfeat-colors-adjust | video-feature | UNREACHABLE | L10 | adjust.rs real, 48/0 tests; ZERO production callers; no video.set-adjust id |
| 130 | vfeat-sharpen-gradfun | video-feature | ACCURATE-ABSENT | L10 | honest; EssentialTool staging admits absence |
| 131 | vfeat-color-fx | video-feature | ACCURATE-ABSENT | L11 | no code; UI honestly staged |
| 132 | vfeat-hdr | video-feature | ACCURATE-ABSENT | L11 | no code; honest |
| 133 | vfeat-deinterlace | video-feature | SHALLOW | L11 | blocked status honest; blocked_reason settings-UI sub-claim FALSE as shipped (enabled enums) |
| 134 | vfeat-rotate-transform | video-feature | ACCURATE-ABSENT | L11 | honest |
| 135 | vfeat-novelty-filters | video-feature | ACCURATE-ABSENT | L11 | honest; §-pointer drift (§5->§4) |
| 136 | plfeat-queue-model | playlist-feature | UNREACHABLE | L11 | full dead playlist surface; hosts refuse playlist.* |
| 137 | plfeat-repeat-loop-shuffle | playlist-feature | UNREACHABLE | L11 | keymap shuffle dead-end |
| 138 | plfeat-save-load | playlist-feature | UNREACHABLE | L11 | M3U writer + readers, zero non-test callers |
| 139 | plfeat-sort-search | playlist-feature | SHALLOW | L11 | client-local sort/view shipped+user-visible; search absent |
| 140 | plfeat-media-library | playlist-feature | ACCURATE-ABSENT | L11 | staged inert rows; honest |
| 141 | plfeat-recent-items | playlist-feature | ACCURATE-ABSENT | L11 | ACCURATE-for-shipped per ledger §5; header had counted it UNREACHABLE (demo-dead) |
| 142 | plfeat-autostart-behaviors | playlist-feature | UNDERSTATED | L11 | --start-paused shipped+tested (baseline :3218); row says not_started |
| 143 | plfeat-service-discovery-panel | playlist-feature | ACCURATE-ABSENT | L11 | honest seam |
| 144 | plfeat-metadata | playlist-feature | ACCURATE-ABSENT | L12 | honest absence |
| 145 | config-area-global | config-feature | REAL | L12 | registered + LocalePicker real; vitest baseline |
| 146 | config-area-interface | config-feature | REAL | L12 | registered; ThemeModePicker real |
| 147 | config-area-privacy | config-feature | REAL | L12 | registered; privacy-network real |
| 148 | config-area-playback | config-feature | REAL | L12 | registered; tracks/playback sections (ledger listed this row as 152) |
| 149 | config-area-video | config-feature | FALSE | L12 | stale-understated: claims "NOT registered"; SETTINGS_AREA_IDS includes video |
| 150 | config-area-audio | config-feature | FALSE | L12 | stale-understated: audio area IS registered |
| 151 | config-area-subtitles | config-feature | FALSE | L12 | stale-understated: subtitles area IS registered |
| 152 | config-area-controls | config-feature | UNDERSTATED | L12 | controls area registered + schema-generated rows + tests; row says not_started |
| 153 | config-area-library | config-feature | FALSE | L12 | inflated: library area renders staged-inert seam rows (playlist-services retired) |
| 154 | config-area-network | config-feature | REAL | L12 | NetworkArea registered; real |
| 155 | config-area-performance | config-feature | REAL-UNPROVEN | L12 | area registration verified; hw-decode toggle not re-executed |
| 156 | config-area-recording | config-feature | UNDERSTATED | L12 | recording area registered + tests; row says not_started |
| 157 | config-area-extensions | config-feature | ACCURATE-ABSENT | L13 | honest-empty seam |
| 158 | config-rebrand | config-feature | REAL-UNPROVEN | L13 | paths exist; baseline tests green; runtime assertions parked #[ignore] |
| 159 | config-profiles | config-feature | ACCURATE-ABSENT | L13 | no profiles module; honest |
| 160 | config-hotkey-editor | config-feature | REAL-UNPROVEN | L13 | status INFLATED vs description: Global column + search absent; editor mock-store (W15-A05 SHALLOW) |
| 161 | config-keymap-layers | config-feature | REAL-UNPROVEN | L13 | 141 bindings + 9 macOS overrides counted exact; persistence localStorage MOCK |
| 162 | config-localization | config-feature | REAL-UNPROVEN | L13 | 1448-key catalog pinned; baseline-executed |
| 163 | config-advanced-folding | config-feature | FALSE | L13 | evidence cites DELETED advanced.tsx (xtask R4 violation); ledger calls the status itself "defensible" — counted FALSE per ledger roll-up |
| 164 | config-defaults-source | config-feature | ACCURATE-ABSENT | L13 | no default-seeding tooling; honest |
| 165 | plugin-builtin-registry | plugin-feature | ACCURATE-ABSENT | L13 | ids only, zero dispatchers; honest |
| 166 | plugin-codec-api | plugin-feature | ACCURATE-ABSENT | L13 | honest as scoped |
| 167 | plugin-wasm-extensions | plugin-feature | ACCURATE-ABSENT | L13 | orphan plugin-host-wasm crate undisclosed; wasm-extensions=false |
| 168 | plugin-addon-store | plugin-feature | ACCURATE-ABSENT | L13 | no addons code; honest |
| 169 | platform-aout-cpal | platform-feature | REAL-UNPROVEN | L13 | ADR-005 seam real; AC suites green; no real-device evidence anywhere |
| 170 | platform-aout-wasapi | platform-feature | UNDERSTATED | L14 | wasapi.rs 787L FFI in-tree, unwired (SHALLOW underclaim per ledger) |
| 171 | platform-aout-coreaudio | platform-feature | UNDERSTATED | L14 | coreaudio.rs 766L in-tree, unwired |
| 172 | platform-aout-pipewire | platform-feature | UNDERSTATED | L14 | pulse.rs+ffi 1161L real protocol client, unwired |
| 173 | platform-vout-wgpu | platform-feature | UNDERSTATED | L14 | renderer-wgpu SHIPPED as software compositor in apps/cli |
| 174 | platform-hwdecode-h264 | platform-feature | UNDERSTATED | L14 | ~7k LOC VA-API/VT adapters unconditional + settings key; ledger "FALSE-as-stated" |
| 175 | platform-hwdecode-hevc | platform-feature | UNDERSTATED | L14 | hevc/hw.rs 403L packetization in-tree |
| 176 | platform-hwdecode-vp9 | platform-feature | ACCURATE-ABSENT | L14 | truthful negative (sw only) |
| 177 | platform-hwdecode-av1 | platform-feature | ACCURATE-ABSENT | L14 | truthful negative |
| 178 | platform-hwdecode-mpeg-family | platform-feature | ACCURATE-ABSENT | L14 | truthful negative |
| 179 | platform-web-webcodecs | platform-feature | ACCURATE-ABSENT | L14 | typed BlockedCapability matches code |
| 180 | platform-web-target | platform-feature | UNDERSTATED | L14 | web build exists; browser-load PASS in baseline |
| 181 | platform-device-capture | platform-feature | ACCURATE-ABSENT | L14 | blocked honest, no code |
| 182 | platform-disc-dvd | platform-feature | ACCURATE-ABSENT | L14 | blocked honest; μADR verified |
| 183 | platform-disc-bluray | platform-feature | ACCURATE-ABSENT | L15 | blocked honest (PAPER-ONLY-accurate) |
| 184 | platform-disc-cdda | platform-feature | ACCURATE-ABSENT | L15 | honest |
| 185 | platform-zero-copy-interop | platform-feature | ACCURATE-ABSENT | L15 | honest |
| 186 | platform-desktop-integration | platform-feature | ACCURATE-ABSENT | L15 | honest |
| 187 | sd-directory-watch | services-discovery | UNDERSTATED | L15 | device-discovery 2510 LOC landed (seam+monitor) |
| 188 | sd-podcast | services-discovery | ACCURATE-ABSENT | L15 | honest |
| 189 | sd-upnp | services-discovery | FALSE | L15 | ssdp.rs 584L + real-UDP tests; "blocked" misclassifies in-tree work |
| 190 | sd-mdns-renderers | services-discovery | ACCURATE-ABSENT | L15 | honest |
| 191 | sd-sap | services-discovery | ACCURATE-ABSENT | L15 | honest |
| 192 | sd-census-tail | services-discovery | ACCURATE-ABSENT | L15 | honest |
| 193 | ui-screen-player | ui-screen | REAL | L15 | with S2 evidence defect: KeyHintStrip.tsx cited but nonexistent |
| 194 | ui-screen-playlist-library | ui-screen | REAL | L15 | with S3 citation defect: AppShell.tsx wrong file; function real |
| 195 | ui-screen-settings | ui-screen | REAL | L15 | mock persistence disclosed by row itself |
| 196 | ui-screen-media-info | ui-screen | REAL | L16 |  |
| 197 | ui-screen-onboarding | ui-screen | REAL | L16 |  |
| 198 | ui-screen-miniplayer-music | ui-screen | REAL | L16 |  |
| 199 | ui-screen-keyboard-shortcuts | ui-screen | REAL | L16 |  |
| 200 | ui-screen-customize | ui-screen | REAL | L16 |  |
| 201 | ui-screen-av-tools | ui-screen | REAL | L16 | honest partial; inert-with-reason discipline verified |
| 202 | ui-screen-theme | ui-screen | REAL | L16 |  |
| 203 | ui-screen-skins2 | ui-screen | ACCURATE-ABSENT | L16 | wont-do row; ledger "REAL (wont-do)" -> canonical ACCURATE-ABSENT |
| 204 | ui-dialog-open-file | ui-dialog | SHALLOW | L16 | dialog live; description overclaims native pickers/clipboard/DnD |
| 205 | ui-dialog-open-network | ui-dialog | REAL | L16 |  |
| 206 | ui-dialog-open-multiple | ui-dialog | ACCURATE-ABSENT | L16 | honest absence |
| 207 | ui-dialog-open-disc | ui-dialog | ACCURATE-ABSENT | L16 | honest absence |
| 208 | ui-dialog-open-capture | ui-dialog | ACCURATE-ABSENT | L16 | honest blocked |
| 209 | ui-dialog-goto-time | ui-dialog | REAL | L17 | Ctrl+T -> playback.seek IS in SERVED; e2e real |
| 210 | ui-dialog-bookmarks | ui-dialog | SHALLOW | L17 | dialog real; media.*-bookmark unserved in every shipped host; jump-only |
| 211 | ui-dialog-subtitle-add | ui-dialog | ACCURATE-ABSENT | L17 | staged dialog honest-disabled; not_started defensible |
| 212 | ui-dialog-convert | ui-dialog | ACCURATE-ABSENT | L17 | gated screen, disabled-from-data; honest |
| 213 | ui-dialog-stream-output | ui-dialog | ACCURATE-ABSENT | L17 | no sout UI; honest |
| 214 | ui-dialog-messages | ui-dialog | ACCURATE-ABSENT | L17 | no log viewer; honest |
| 215 | ui-dialog-plugins | ui-dialog | ACCURATE-ABSENT | L17 | no plugin manager; honest |
| 216 | ui-dialog-about-update | ui-dialog | ACCURATE-ABSENT | L17 | dead i18n keys only; honest |
| 217 | ui-dialog-vlm | ui-dialog | ACCURATE-ABSENT | L17 | by-design exclusion, reason present |
| 218 | ui-dialog-program-guide | ui-dialog | ACCURATE-ABSENT | L17 | no EPG UI; honest |
| 219 | menu-media-open | menu-item | REAL | L17 | open chain real+executed (rc-smoke); evidence file attribution imprecise (S4) |
| 220 | menu-media-convert-quit | menu-item | FALSE | L17 | quit item disabled:true, dead i18n key, no handler, no native menubar; "quit wired" FALSE; row substance zero |
| 221 | menu-playback-transport | menu-item | SHALLOW | L17 | menu real; play/stop served; 5 of 8 verbs unserved (playlist.*) |
| 222 | menu-playback-navigation | menu-item | UNDERSTATED | L18 | mkv/mp4 chapters + demo handlers exist |
| 223 | menu-playback-bookmarks | menu-item | UNDERSTATED | L18 | dialogs + keymap Ctrl+T/Ctrl+B + 21 tests green |
| 224 | menu-audio | menu-item | UNDERSTATED | L18 | canvas context-menu track selectors live |
| 225 | menu-video-display | menu-item | UNDERSTATED | L18 | useFullscreen + test exist |
| 226 | menu-video-adjust | menu-item | UNDERSTATED | L18 | CropTool + renderer-wgpu aspect/crop params exist |
| 227 | menu-video-snapshot | menu-item | ACCURATE-ABSENT | L18 | no snapshot implementation; honest |
| 228 | menu-subtitle | menu-item | UNDERSTATED | L18 | SubtitlePickList + Add-Subtitle entry exist |
| 229 | menu-tools | menu-item | SHALLOW | L18 | subset real; evidence cites menubar removed in W25/W26 redesign |
| 230 | menu-view | menu-item | SHALLOW | L18 | no View menu exists; rail navigation real |
| 231 | menu-help | menu-item | SHALLOW | L18 | no Help menu exists; about/shortcuts live in ControlBar app menu |
| 232 | menu-context-surface | menu-item | UNDERSTATED | L18 | canvas context menu w/ track selectors exists |
| 233 | menu-context-playlist | menu-item | UNDERSTATED | L18 | real 8-row queue context menu |
| 234 | menu-tray | menu-item | ACCURATE-ABSENT | L18 | no tray code; honest |
| 235 | hotkey-transport | hotkey | SHALLOW | L19 | play/pause branch dormant; ~8 of 141 seeded keys live |
| 236 | hotkey-seek | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 237 | hotkey-rate | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 238 | hotkey-volume | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 239 | hotkey-frame-step | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 240 | hotkey-track | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 241 | hotkey-sync | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 242 | hotkey-video-display | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 243 | hotkey-subtitle-scale | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 244 | hotkey-playlist | hotkey | SHALLOW | L19 | 1 FALSE evidence path (AppShell is not the consumer) |
| 245 | hotkey-bookmarks | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 246 | hotkey-navigation | hotkey | SHALLOW | L19 | seed+editor only; engine half dead or unserved |
| 247 | hotkey-nav-disc | hotkey | ACCURATE-ABSENT | L19 | honest blocked declaration |
| 248 | hotkey-snapshot-record | hotkey | SHALLOW | L20 | plan+catalog only; video.take-screenshot unserved; no capture path (ledger row "250") |
| 249 | hotkey-interface | hotkey | SHALLOW | L20 | "quit wired in AppShell.tsx" evidence element FALSE; show-controls I key handled (ledger row "251") |
| 250 | hotkey-global | hotkey | ACCURATE-ABSENT | L20 | no global-hotkey code; re-verified by W20-A07 (ledger row "252") |
| 251 | hotkey-wallpaper-360 | hotkey | ACCURATE-ABSENT | W20-A07 | NOT covered by any ledger (L20 off-by-two hole). Census: rg wallpaper/equirect/360 over crates+apps = 0; keymaps/default.toml:172 seed marked disposition not-adopted-v1; settings hint string only |
| 252 | ext-lua-extensions | extension | ACCURATE-ABSENT | W20-A07 | NOT covered by any ledger. Census: rg lua over crates/apps/ui = 0 code; extensions.tsx:19-26 — NO extensions command, wasm-extensions=false, seam EMPTY |
