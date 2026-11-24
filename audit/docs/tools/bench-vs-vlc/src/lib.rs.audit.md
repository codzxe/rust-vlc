# AUDIT: tools/bench-vs-vlc/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Lines / functions | 531 / 12 (incl. 1 impl-2 methods, 2 enum/struct ctors) |
| File verdict | REAL-UNPROVEN (code fully real; crate is a standalone workspace whose test lane I could not execute — cargo-build-budget) |
| Reachable from shipped entry point? | NO by design (bench tool; driven by tools/bench-vs-vlc/tests/lumen_pcm.rs via the documented cargo-test invocation — it is the producer of docs/corpus/vlc-diff-report-20261023.lumen.json, whose schema and field order this file emits exactly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read full file; rg callers in tests/lumen_pcm.rs; Cargo.toml workspace check; ls md5.rs/fixtures (exit 0) |

## 1. One-paragraph verdict
This is real, disciplined code: a full open-path digest pipeline
(canonicalize → FileAccess → media_probe → AviDemuxer → PcmDecoder → MD5
sink) with a concealment budget, saturating counters, zero unwrap/expect/
panic, and a hand-rolled RFC-1321 MD5 (with test vectors in md5.rs:200-
233). Its JSON emitter produces byte-shape-identical rows to the committed
lumen.json evidence (schema `bench-vs-vlc/lumen-pcm-digests/1`, field
order rel/status/md5/pcm_bytes/… matches the corpus artifact exactly),
which corroborates that artifact's provenance. Nothing here is fake; the
only defects are two dated comments.

## 2. What actually works (verified)
— Field-order/schema equivalence with docs/corpus/vlc-diff-report-
20261023.lumen.json verified by direct comparison of render_batch_json's
format string (lib.rs:464-468,495-500) against the artifact rows.
— Test-harness wiring verified: tests/lumen_pcm.rs imports all 6 public
fns (lines 23-24) and drives in-repo fixtures (generated/valid/
avi_mjpeg_pcm.avi, avi_mjpeg_2pcm.avi). Execution itself UNVERIFIED
(reason: cargo-build-budget — standalone workspace, engine-stack compile).

## 3. Findings
F-001 [S4]
Where: lib.rs:60-61
Code: "the W5-14 branch (pending merge at time of writing) silences
exactly those for declared-but-filtered streams."
What is wrong: dated branch-state comment; unverifiable from this
sandbox (no git). Why it matters: comment rots. Confidence: HIGH —
Verification: read.
F-002 [S4]
Where: lib.rs:29-30
Code: "[VERIFY: VLC default audio track == first, pending the real
oracle run]"
What is wrong: open verification item never closed (oracle still
PENDING in the 2026-10-23 report). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| host_endian | 92-98 | REAL | — | cfg!-based constant answer; correct |
| per_frame_damage | 104-112 | REAL | — | matches! on Corrupt/Decoder{Bitstream} only; honest narrow class |
| DigestSink::new | 134-143 | REAL | — | fresh MD5 state |
| DigestSink::digest_hex | 146-150 | REAL | — | clone-then-finalize (Md5::finalize consumes); correct |
| Default for DigestSink | 153-157 | REAL | — | delegates |
| FrameSink::accept | 160-181 | REAL | — | audio→digest; other kinds counted as wiring-error signal; format-change counter saturating |
| sample_name | 221-231 | REAL | — | exhaustive match over SampleFormat |
| digest_avi_first_pcm_audio | 240-273 | REAL | — | full open path; avi-only gate with typed error (matches corpus error strings verbatim) |
| digest_avi_first_pcm_audio_from | 281-404 | REAL | — | first-PCM-track rule; concealment budget 1000 consecutive; Recovered/skip accounting; flush; no panic paths |
| digest_batch | 420-434 | REAL | — | maps outcomes; collects |
| json_escape | 437-451 | REAL | — | quotes/backslash/C0 controls; correct \u{:04x} |
| render_batch_json | 455-501 | REAL | — | emits the evidence schema; row separator ",\n" |
| parse_fixture_tsv | 508-531 | REAL | — | tab split; duplicate-rel detection; CR-trim |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| md5.rs vectors (RFC 1321 test strings) | YES | assert_eq against published MD5 digests |
| md5 streaming-vs-one-shot (chunk loops) | YES | catches state-restore bugs |
| lumen_pcm digest on in-repo fixtures | YES | real decode path; md5/pcm_bytes/packets asserted per test bodies (read) |
| synth-stream digest tests (:331,:371) | YES | construct AVI bytes in memory; assert report fields |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "differential harness operational … 12 Rust + 30 Python suites green" | docs/gates/phase3-exit.md:35 | REAL-UNPROVEN | suites exist; not re-run here (standalone workspace) |
| lumen.json produced by this code | provenance | REAL | schema + field order byte-match with emitted format string |

## 7. Dead / duplicated / unreachable code
None. All 6 public symbols are consumed by tests/lumen_pcm.rs (:23-24).

## 8. Dependencies & flags
Standalone workspace (own [workspace] table; NOT a root member — root
`cargo test --workspace` does NOT cover it); std-only, zero external deps;
unsafe_code denied locally; path deps on engine crates.

## 9. Open questions for the next agent
The [VERIFY] first-audio-track rule vs VLC default remains open until a
real oracle run exists; the W5-14 comment (F-001) should be re-checked
post-merge.
