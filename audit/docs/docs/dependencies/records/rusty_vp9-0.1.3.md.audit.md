# CLAIM AUDIT: docs/dependencies/records/rusty_vp9-0.1.3.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL: 6 REAL-UNPROVEN: 3 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 0 UNREACHABLE: 0 |
| Internal contradictions found | 0 (one wrong line-number citation) |

## 1. Blunt verdict
This is the VP9 bit-exact admission record, and its evidence quality is the
best in the vta family: the six-lane differential against ffmpeg
`libvpx-vp9` is re-runnable in-tree (`crates/codecs/tests/vp9_differential.rs`
decodes with rusty_vp9 and compares whole-stream MD5 against a LIVE ffmpeg
oracle, falling back to pinned constants only without ffmpeg), and that test
passed in the orchestrator baseline TODAY (4/4, 21.12 s). The record also
discloses its own measurement incidents honestly. Defects: one wrong
line-number citation, several un-reproducible-in-sandbox registry claims.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "six-lane bit-exactness differential against ffmpeg `libvpx-vp9` driven through OUR OWN ivf demuxer" | :9-11 | REAL | baseline cargo-test-workspace.txt:1384-1395 `vp9_differential-422d...` 4 passed/0 failed 21.12s incl. `differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok`; test decodes reference via live `ffmpeg -c:v libvpx-vp9` (vp9_differential.rs:221-244) |
| 2 | "BIT-EXACT — 6/6 lanes, 431/431 frames byte-identical" | :174-176 | REAL | arithmetic 125+90+75+41+50+50=431 ✓; in-tree whole-stream-MD5-vs-live-ffmpeg passes today; per-frame byte-identity implied (same frame counts + concatenated stream MD5 equal) |
| 3 | pin `rusty_vp9 = "=0.1.3"` | :18, :238 | REAL | crates/codecs/Cargo.toml:145 `rusty_vp9 = { version = "=0.1.3", optional = true }` |
| 4 | "crates/codecs/src/video/vp9/** + rusty_vp9" (admission) | :241-243 | REAL | dir exists: decoder.rs, error.rs, mod.rs |
| 5 | "`vp9_differential` — 4/4 pass (21.98 s)" (2026-11-09) | :248 | REAL | baseline re-run today: 4/4 in 21.12 s — reproduces |
| 6 | "`cargo test -p codecs` → 643 cases ... 624 pass, 18 ignored, 0 fail" | :245-247 | REAL-UNPROVEN | past-run claim; no toolchain to re-run; shape consistent with baseline (codecs suites all ok) |
| 7 | matrix row flipped "blocked → implemented" | :250-254 | REAL | docs/feature-matrix.toml:480 `status = { windows = "implemented", linux = "partial", macos = "implemented", web = "not_started" }` + evidence string :481 cites this record |
| 8 | "V_VP9→"vp9" mapping verified in `crates/demux/src/mkv/header.rs:772`" | :219-220 | FALSE-as-cited (line) | actual mapping is header.rs:805 `"V_VP9" => return "vp9".to_string(),` — line 772 is inside an unrelated function; citation off by 33 lines; the mapping itself is REAL |
| 9 | "hostile battery ... Zero panics anywhere" | :196-209 | REAL (part) | in-tree `off_by_one_truncation_sweep_is_panic_free` + `hostile_garbage_is_recovered_and_the_stream_survives` pass today (baseline :1387-1388); truncated-keyframe→full-frame disclosure is honest |
| 10 | "the corpus holds zero VP9 lanes — its single .webm is VP8" | :178-180 | REAL-UNPROVEN | corpus-v4 not present in this sandbox; cannot inspect |
| 11 | "98 unsafe tokens, all SIMD intrinsics" | :37-43 | REAL-UNPROVEN | crate tarball not present in sandbox; cannot recount |
| 12 | OSV "zero advisories", "3,784 downloads", published 2026-10-17 | :112-123 | REAL-UNPROVEN | no network access from sandbox |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| mapping "verified in header.rs:772" | actual `"V_VP9" => "vp9"` at header.rs:805 (rg) | the code; record cites wrong line (harmless, mapping exists) |

## 4. Fabrication / inflation indicators
None material. The record documents its own two harness bugs (:187-194) and
declines to claim unexercised VP9 surface (§G :211-228). The one weak spot:
whole-lane MD5s in the record table are the harness's own measurements —
but they are independently re-derivable via the in-tree test with live
ffmpeg, which passes today; goldens are only the no-ffmpeg fallback
(vp9_differential.rs:292-299).

## 5. Matrix rows
N/A (dependency record; cross-ref matrix row codec-video-vp9 handled in
claim 7 — note the matrix row's own honesty: linux downgraded to `partial`
2026-11-10 because the linux-native compat run refused VP9 with "no decoder
in this build", evidence string feature-matrix.toml:482).

## 6. What actually works (verified)
— baseline cargo-test-workspace.txt:1384-1395 (vp9_differential 4/4, live
ffmpeg present in sandbox) — the bit-exact differential reproduces TODAY.
— rg rusty_vp9 crates/codecs/Cargo.toml:145; ls crates/codecs/src/video/vp9;
rg V_VP9 crates/demux/src/mkv/header.rs:805.
