# CLAIM AUDIT: docs/formats/audio-pcm.md
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A08) |
| Date | 2026-11-24 |
| Claims extracted | 11 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 0 / 0 / 0 / 0 / 1 / 2 |

## 1. Blunt verdict
Substantive record: every artifact path it cites exists, the test count and
smoke claims reconcile exactly against the orchestrator baseline, and the
module is genuinely std-only/no-unsafe. One real defect: it misstates the
matrix row status for linux (says `implemented`; matrix says `partial` since
the 2026-11-10T20:08Z f32-tolerance triage) — the consolidation claim is stale.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "row status `implemented` on win/linux/macos, `not_started` on web" | :8-11 | FALSE (linux) | feature-matrix.toml:636 `status = { windows = "implemented", linux = "partial", ... }` — linux flipped to partial by the 2026-11-10T20:08:26Z f32-tolerance triage in the same row's notes |
| 2 | fuzz_target `fuzz_codec_pcm` + in-crate `crates/codecs/src/audio/pcm/fuzz.rs` | :17-18 | REAL | `ls fuzz/fuzz_targets/` -> fuzz_codec_pcm.rs; `ls crates/codecs/src/audio/pcm/` -> decoder/error/fuzz/mod.rs |
| 3 | "crates = [] — self-built, std-only, no `unsafe`" | :19 | REAL | `rg -c unsafe crates/codecs/src/audio/pcm/` -> 0 hits |
| 4 | "551 src lines in `mod.rs` alone" | :21 | REAL | `wc -l crates/codecs/src/audio/pcm/mod.rs` -> 551 |
| 5 | fixture `avi_mjpeg_pcm.avi` + `.oracle.sha256` sidecar | :43-44 | REAL | `ls tests/fixtures/generated/valid/` -> both present (+structure.json) |
| 6 | `tests/integration/first-vertical/evidence/ledger.json` (2.E1, mock-labeled) | :46 | REAL | `ls` -> exists |
| 7 | "`cargo test ... audio::pcm` — 39 passed / 0 failed" | :53 | REAL | baseline cargo-test-workspace.txt: exactly 39 `test audio::pcm` lines, all ok, 0 FAILED/ignored |
| 8 | "headless-decode ... 60/60 audio frames" | :56-59 | REAL | baseline rc-smoke.txt:131 `a_frames=60 a_decoded=60 ... PASS` on this exact fixture |
| 9 | web: "wasm host ships the engine core decoder-less"; record `apps/web/README.md` | :67-70 | REAL | apps/web/README.md:57-59 "builtin decoders — the `codecs` crate's external decode engines do" |
| 10 | "mock-labeled audio output (null device — ADR-005 ... KNOWN-LIMITS.md §4)" | :64-66 | REAL | docs/KNOWN-LIMITS.md:51 "Audio output: null backend only ... no sound on any OS yet" |
| 11 | "worktree `wt/w24e` @ `c339ee8`"; "run recorded in the `ff001e5` commit message" | :51,59 | UNVERIFIED | no git in sandbox (rulebook §2); commits not re-checkable |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| linux status `implemented` (:9) | matrix row notes: linux `partial` (f32 rows FAIL tolerance, worst 0.22e-1 RMS) | Matrix is true — its own notes field records the 2026-11-10 transition; the record doc predates/misstates it |

## 4. Fabrication / inflation indicators
None — paths, counts, and baselines all reconcile; the one stale claim is
negligence (no re-read of the row it consolidates), not inflation.
F-001 [S3]: consolidation record misstates its own matrix row's linux status.
