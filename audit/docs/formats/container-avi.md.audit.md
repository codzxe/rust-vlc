# CLAIM AUDIT: docs/formats/container-avi.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 7 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 REAL / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Every gate number re-executed by this auditor and matched exactly (85/0, 6/0, 3/0, 4/0,
11/0). Module, fixtures, sidecars, matrix row, wasm wiring, and the no-unsafe claim all
verify. First-vertical decode is additionally proven by the orchestrator baseline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "row status `implemented` on win/linux/macos, `not_started` on web, with evidence `crates/demux/src/avi/**`" | L9-12 | REAL | feature-matrix.toml:194-203 `id = "container-avi"` … implemented×3, web not_started; evidence line quotes `crates/demux/src/avi/**` (sed-verified) |
| 2 | "self_built = [`demux::avi` (mod/header/index/riff/tests/test_support/fuzz)" | L21-22 | REAL | ls crates/demux/src/avi/ → exactly those 7 files |
| 3 | "`cargo test --locked -p demux --lib avi::` — 85 passed / 0 failed" | L43-44 | REAL (EXECUTED) | I ran it: `test result: ok. 85 passed; 0 failed; 0 ignored; … 377 filtered out; finished in 0.01s` |
| 4 | "fixtures_parent — 6/0; idx1_stream_scope — 3/0; strl_advisory — 4/0; fixtures_class_b — 11/0" | L45-46 | REAL (EXECUTED) | I ran all four: `11 passed` / `6 passed` / `3 passed` / `4 passed`, 0 failed each (cargo output pasted in run log) |
| 5 | "fixture corpus … `avi_mjpeg_pcm.avi` (+ `.oracle.sha256` + `.structure.json`) and the class-B variants (`avi_mjpeg_2pcm.avi`, the `noidx0` … shape)" | L31-35 | REAL | ls tests/fixtures/generated/valid/ → all three files + sidecars; noidx0 at generated/malformed/avi_mjpeg_pcm__noidx0.avi (CORPUS.md:89) |
| 6 | "no AVI writer exists in-tree … no `unsafe` … std-only" | L25, L19 | REAL | rg 'unsafe' over avi/*.rs → 0 matches (exit 1); no avi writer (rg write/mux in avi → none) |
| 7 | "the wasm host wires AVI open (`apps/web/src/open.rs`)" | L58 | REAL | apps/web/src/open.rs:25 `use demux::avi::AviDemuxer;` + :16 "v1 wires exactly the first vertical's family (`avi`)" |

## 3. Contradictions
None. First-vertical decode claim backed by baseline rc-smoke.txt (headless-decode
AVI/MJPEG/PCM PASS) and `apps/cli/tests/session_script.rs` exists (open→play→pause→seek→stop
scripted session). Matrix note's "ALL evidence … Linux-only, mock-labeled" is the honest
caveat this doc repeats at L52-53.

## 4. Fabrication / inflation indicators
None. The "60/60 video + 60/60 audio frames (run recorded in the `ff001e5` commit message)"
claim is UNVERIFIED at the frame-count level (git forbidden here; baseline proves the
headless run green, not the exact 60/60 counts — those rest on the commit message).
