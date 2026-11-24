# CLAIM AUDIT: docs/formats/video-h264-software.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 6 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 0 / 1 / 0 / 0 / 0 / 5 FALSE (all stale vs 2026-11-15 admission) / 0 |

## 1. Blunt verdict
This gap record is itself the gap: it says the h264-software row is `blocked` ×4 with
"nothing implemented" and "no decoder crate is a workspace dependency" — every one of
those claims is FALSE in the current tree. The matrix row flipped to implemented ×3 on
2026-11-15 (rust_h264 0.6.1 Lead-admitted), sw code, dependency, admission record and
fuzz target all exist. The doc was accurate for exactly five days (2026-11-10 → 11-15)
and was never updated when its subject changed.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "row status `blocked` ×4, `phase_target = 5` … the matrix row keeps `blocked` ×4 until then" | L10-11, L49-50 | FALSE | feature-matrix.toml:427 `status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }` (sed-verified) |
| 2 | "Nothing implemented for software decode" | L36 | FALSE | crates/codecs/src/video/h264/sw.rs exists (ls verified); matrix evidence :428 "crates/codecs/src/video/h264/sw/** (codec-h264-sw, rust_h264 =0.6.1 Lead-admitted 2026-11-15 …)" |
| 3 | "crates = [] — none admitted; **no decoder crate is a workspace dependency**" | L16-17 | FALSE | crates/codecs/Cargo.toml:73 `codec-h264-sw = ["dep:rust_h264"]` + :162 rust_h264 pin; default feature list includes codec-h264-sw (:53) |
| 4 | "fuzz_target = none (no decoder admitted)" | L14 | FALSE | fuzz/fuzz_targets/fuzz_codec_h264.rs exists; crates/codecs/src/video/h264/fuzz.rs exists |
| 5 | "No corpus results — the audits' numbers … are upstream claims" | L55 | REAL | honest for the audits record itself; but contradicted by matrix :428 "differential crates/codecs/tests/h264_sw_differential.rs: 6 libx264 lanes x 2 … bit-exact vs ffmpeg 7.1.5" — file exists (ls codecs/tests/) |
| 6 | "the admission evidence landed 2026-11-09 … `docs/dependencies/records/h264-software-decoder-audits.md`" | L7-9 | REAL | file exists (ls docs/dependencies/records/) |

## 3. Findings
F-001 [S2] — the entire record contradicts the tree it audits.
Where: docs/formats/video-h264-software.md:10-17, 36, 49-50 vs feature-matrix.toml:427-428, crates/codecs/Cargo.toml:73, crates/codecs/src/video/h264/sw.rs.
What is wrong: a "records-consolidation" doc whose sole job is to mirror one matrix row
states the exact opposite of the row (blocked vs implemented). Why it matters: the doc
chain is the anti-drift mechanism; a five-day-stale mirror that no gate caught is drift
by construction. What correct looks like: the `audio-pcm.md` consolidation shape it
itself cites, or deletion on admission. Confidence: HIGH.
F-002 [S3] — related cross-check: baseline engine-info (audit/runs/baseline/mp4-smoke.md)
lists `decoders: mjpeg, pcm_s16le` while the MP4 smoke PLAYS H.264+AAC exit 0 and the
HEVC error text claims 12 landed decoders — the engine-info registry understates the
actually-registered set (wave-0 open question; this doc's flip compounds the confusion).
Confidence: MEDIUM — registry wiring not traced to source here.

## 4. Contradictions
| This doc says | Matrix/code says | True |
|---|---|---|
| blocked ×4, nothing implemented | implemented ×3, sw.rs + rust_h264 dep + admission record | matrix/code |
| "no decoder crate is a workspace dependency" | codec-h264-sw default feature with dep:rust_h264 | Cargo.toml |

## 5. Fabrication / inflation indicators
Staleness, not fabrication: every claim was verifiable on 2026-11-10 (its own last_audit).
The doc even predicted its own obsolescence shape ("this record gains the landed scope
(the audio-pcm.md consolidation shape)") without anyone executing that update.
