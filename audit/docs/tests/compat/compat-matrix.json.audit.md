# CLAIM AUDIT: tests/compat/compat-matrix.json
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Claims extracted | 35 rows × 4 platform cells (140) + 4 structural claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | linux cells 35 REAL (lane ran, decode counts recorded) · web cells 35 REAL-honest (recorded build steps / not_started) · windows+macos cells 70 PAPER-ONLY (no run ever) · 4 linux rows FALSE-stale (5 measured transitions: implemented→partial) |
| Internal contradictions found | 3 |

## 1. Blunt verdict
This config drives the compat runner (tests/compat/src/main.rs:33 "Everything
user-visible is config-driven (`compat-matrix.json`") and CI derivation
(tests/compat/ci/derive_config.py:60). Its committed claims snapshot is
factually stale against the run it references: 5 rows claim `implemented` on
linux that the recorded 2026-11-10 run measured as partial, including
container-asf-wmv whose corpus the probe flatly REFUSES. The 70 windows/macos
cells say `implemented` while the same file's own lanes block those platforms
("no result inferred") — statuses with zero runs behind them. The Linux lane
cells and web-lane honesty are real; the cross-platform half of this matrix is
assertion, not evidence.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | `container-asf-wmv: linux "implemented"` | compat-matrix.json:709-713 | FALSE (vs recorded run) | docs/corpus/compat-matrix-20261110.json transitions: `"row":"container-asf-wmv","from":"implemented","to":"partial"` reason: probe REFUSES asf corpus (probe.no-demuxer, sniffed GUID 30 26 b2 75…) |
| 2 | `codec-audio-aac-lc: linux "implemented"` | :554-560 | FALSE (vs recorded run) | same transitions: "142 concealed (damaged) packets on VALID corpus files … the decode is not clean" + 1 track REFUSED vs 4 decoded |
| 3 | `codec-audio-alac: linux "implemented"` | :568-574 | FALSE (vs recorded run) | transitions: "1 corpus tracks of codec `alac` REFUSED (no decoder in this build) vs 1 decoded" |
| 4 | `codec-video-vp9: linux "implemented"` | :680-686 | FALSE (vs recorded run) | transitions: "1 corpus tracks of codec `vp9` REFUSED (no decoder in this build) vs 2 decoded" |
| 5 | `windows/macos cells = "implemented" (26 rows)` | :553-799 vs lanes :168-188 | PAPER-ONLY | lane json:171 `"reason": "3-OS runner machines do not exist…"`; live notes: "Windows/macOS cells are blocked-lane citations … NO result is inferred for platforms that did not run" |
| 6 | `web-wasm lane: build + lane-proof steps, module ships decoder-less` | :133-165 | REAL | live matrix lanes web-wasm steps: `cargo build … wasm32-unknown-unknown` outcome exit 0 wall 19.85s; `cargo test -p lumen-web` exit 0 |
| 7 | `linux-native is "the lane that actually ran"` | :125-131 | REAL | live matrix counts: files_total 166, decoded 84, decoded_partial 22, malformed_typed 54 |
| 8 | `oracle pinned: VLC 3.0.23` | :71-84 | REAL-UNPROVEN here | bin `/tmp/vlc-3.0.23/usr/bin/vlc` is machine-local; records exist (docs/corpus/vlc-oracle-run-20261109.json) but the pin is not portable |
| 9 | `f32 tolerance rms_max 0.0001 peak_max 0.001` | :67-70 | REAL (ratified) but unenforced in claims | live counts: `oracle_tolerance_fail: 6` — six failures recorded, no transition, no claim change |
| 10 | `container-flac-native: not_started (all platforms)` | :722-728 | REAL (pessimistic-honest) | rg: crates/demux/src has no `flac` dir; only FLAC-in-WAV/OGG paths exist |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| asf-wmv linux implemented (:709-713) | recorded run: open path REFUSES asf corpus (transitions) | the run: probe.no-demuxer on sniffed ASF GUID; demux/src/asf exists but the open path refuses these files |
| lane label "no result inferred" for windows (:176) | claims windows="implemented" ×26 | the lane statement is the true one; the cells are hand-asserted, not run-derived |
| claims == live matrix rows (0 diffs, python compare) | w24-d review P3-8 says 5 rows stale | the staleness persists — review docs/reviews/w24-p-final-findings.md:89 records P3-8 DEFERRED, unrecorded; my python diff confirms claims still match the stale live rows, not the measured transitions |

## 4. Fabrication / inflation indicators
- Committed claims block (553-799) is a stale snapshot the repo's own review
  tagged DEFERRED (docs/reviews/w24-d-compat-findings.md:228-233 "the five
  flipped rows still say `implemented`; live matrix says `partial`").
- windows/macos "implemented" cells inflate cross-platform coverage without
  a single run on those platforms; the runner mitigates (STALE check
  rejects proposals all-or-nothing) but the published matrix still shows them.
- `oracle_tolerance_fail: 6` in the referenced report never propagates to any
  claim — a failure class recorded but not reflected in statuses.

## 5. Matrix rows
Config maps 13 probe formats + 23 codec names onto feature-matrix rows
(`container-*`, `codec-*`). Per-row: linux cells carry run evidence (REAL);
windows/macos cells are unevidenced (PAPER-ONLY); web cells are honest
not_started (module ships decoder-less — verified by lane steps + baseline
rc-smoke wasm build pass). MP2 rides the `container-mp3-raw` row (:326-334),
so per-codec granularity for MP2 is lost — S4 data-model quirk.
