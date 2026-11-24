# AUDIT: tools/sweep/src/worker.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 561 / 14 fns + Limb impl + NullSink + 3 tests |
| File verdict | REAL (engine-driving worker; all 3 unit tests passed in the committed baseline) |
| Reachable from shipped entry point? | NO (tools/lumen-sweep supervisor binary — a dev tool, not the release lane; the sweep worker is its core) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |

## 1. One-paragraph verdict
A genuine single-file decode-to-null sweep worker over the real engine crates
(input::FileAccess -> media_probe -> AviDemuxer -> Mjpeg/Pcm decoders behind
codec_api), with typed errors everywhere, a bounded concealment budget, a
deliberate no-catch_unwind design (process death is the census signal), and a
sanitized tab-separated marker protocol. The only real gaps are behavioral
minor: a single shared consecutive-damage counter across both limbs, and the
AVI-only scope (documented).

## 2. What actually works (verified)
- Baseline run (cargo-test-workspace.txt:3316-3318): `test worker::tests::
  from_file_path_percent_encodes_spaces_but_keeps_slashes ... ok /
  marker_line_shape_matches_the_parser ... ok /
  missing_file_is_a_typed_inaccessible_refusal_not_a_crash ... ok` — 3/3 green.
- Read end-to-end: every error path is typed (`status_of`, `kind_tag`,
  `file_uri` typed inaccessibles with codes sweep.path.*); NullSink is an
  honest decode-to-null; `marker_line` round-trips `parse_worker_line` in-test.

## 3. Findings
F-001 [S4] Where: worker.rs:379 and :386-395.
Code: `let mut consecutive: u32 = 0;` … `match (video.as_mut(), audio.as_mut()) { (Some(limb), _) if limb.track == track => Some(limb), (_, Some(limb)) if limb.track == track => Some(limb), _ => None }`
What is wrong: ONE `consecutive` counter is shared by both limbs' `feed`, so
interleaved v/a damage accumulates into a single 1000-consecutive budget —
the engine's `VIDEO_CONCEALMENT_BUDGET` rule it claims to mirror
(:23-29 "exactly like playback-engine's per_frame_damage") is per-stream
there. A mixed stream hits the budget ~2x earlier than the engine would.
What correct looks like: one counter per limb. Confidence: HIGH — read;
engine-side budget scope UNVERIFIED (playback-engine not in this assignment).
F-002 [S4] Where: worker.rs:239-241.
Code: `Ok(DecodeOutcome::Recovered { .. }) => { *consecutive = consecutive.saturating_add(1); }`
What is wrong: `Recovered` outcomes increment the consecutive budget but NOT
the concealed counter — asymmetric with the `Err`-arm which does both — so the
reported `v_concealed`/`a_concealed` columns undercount soft damage while the
budget still trips on it. Confidence: HIGH — read.
F-003 [S4] Where: worker.rs:299-313.
Code: `if container != "avi" { return Err(... "sweep.container.no_demuxer") }`
What is wrong: the sweep's coverage is AVI-only (first vertical); every
non-AVI corpus entry is classified `unsupported` at probe — by design and
honestly refused, but the tool named "sweep" (corpus census) can only ever
census one container. Documented in-line ("until the demuxer matrix grows").
Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| result_skeleton | 81-113 | REAL | — | "-" defaults, typed error fields |
| status_of / kind_tag | 118-139 | REAL | — | discriminant classification |
| per_frame_damage | 144-152 | REAL | — | mirrors engine rule (Corrupt + Decoder{Bitstream}) |
| file_uri | 158-195 | REAL | — | typed not-found/permission/not-file/not-utf8 refusals |
| configure | 200-209 | REAL | — | DecoderConfig with FIRST generation |
| Limb::feed | 227-261 | REAL | S4 | shared budget F-001; Recovered asymmetry F-002 |
| Limb::drain | 264-268 | REAL | — | flush counts frames |
| NullSink | 273-279 | REAL | — | honest no-op sink |
| open | 292-369 | REAL | S4 | AVI-only scope F-003; limb selection by strf |
| run_loop | 372-414 | REAL | S4 | packet routing, Recovered counted, drains |
| walk | 421-477 | REAL | — | all failures are data; process death is the census's |
| probe_format_of | 481-486 | REAL | — | container column from Unsupported info |
| marker_line | 491-510 | REAL | — | 15-field protocol, round-trip tested |
| tests (3) | 516-560 | REAL | — | green in baseline; honest negative-path test |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| from_file_path_percent_encodes | YES | exact URI strings incl. Windows drive form |
| missing_file_is_a_typed_inaccessible_refusal | YES | real walk() on a nonexistent path, asserts stage/kind/limbs |
| marker_line_shape_matches_the_parser | YES | round-trip through the real parser |
## 6. Doc claims: "mirrors tools/mem-audit's walk.rs ... exactly" (:5-11) — UNVERIFIED (walk.rs is 798 lines, another assignment); the sweep's mem-audit baseline tests are green.
## 7. Dead code: none.
## 8. Dependencies: codec_api, demux::avi, media_types, media_probe, input, codecs, plugin_api — all real engine crates.
## 9. Open questions: engine-side concealment budget scope (F-001) for the playback-engine owner.
