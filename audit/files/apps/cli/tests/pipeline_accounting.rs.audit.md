# AUDIT: apps/cli/tests/pipeline_accounting.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-24 |
| Lines / functions | 201 / 6 (3 helpers + 3 tests) |
| File verdict | REAL (3/3 executed, exact counter pins; strongest suite of the batch alongside native_seek.rs) |
| Reachable from shipped entry point? | N/A (test file) — drives the shipped entry `lumen_dispatch` (apps/cli/src/lib.rs:43) through `CliHost`; counters are real increments in apps/cli/src/pipeline_host.rs, struct at apps/cli/src/host.rs:108-140 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/pipeline_accounting-b2ba9f9722c89ab3` → 3 passed, exit 0, 1.38s; `./target/debug/deps/pipeline_accounting-fc57038aa1821375` → 3 passed, exit 0, 1.55s; `rg -n 'struct HeadlessAccounting' -A 30 apps/cli/src/` → 14 real counter fields |

## 1. One-paragraph verdict
A genuinely strong acceptance suite, executed: all three tests pass on both prebuilt binaries. The whole-media test pins the full pipeline's accounting exactly (120 deliveries = 60 MJPEG + 60 PCM, scheduler 60/0-late, staging bytes = frames × 320×240×4, position exactly 2,000,000 µs, PNG magic); the determinism test compares two complete runs for equality on trail, position, accounting AND final PNG bytes — the strongest determinism pin I have seen in this repo; the seek test re-proves the C8 invariants at the accounting level (90 = 30 decode-ahead + 60 landing, not the 60 a reopen would answer; session epoch survives). The HeadlessAccounting counters are real fields incremented in pipeline_host.rs, not constants. The only defects are three soft assertions, each honestly commented: `contains`-based state-trail checks, one `>=` bound, and a PNG check that never parses the IHDR it names.

## 2. What actually works (verified)
— `./target/debug/deps/pipeline_accounting-b2ba9f9722c89ab3` → `test result: ok. 3 passed; 0 failed` (whole_media_accounting_closes, whole_media_runs_are_deterministic, native_seek_accounting_closes); second binary `pipeline_accounting-fc57038aa1821375` → same 3/3.
— `rg 'struct HeadlessAccounting' apps/cli/src/host.rs:108` → 14 documented counter fields (frames_accepted … scheduler_dropped_stale), incremented in apps/cli/src/pipeline_host.rs — the asserted numbers are produced by the real pipeline.

## 3. Findings
F-001 [S4]
Where: apps/cli/tests/pipeline_accounting.rs:78-90
Code: `// The state trail: a clean open → play → end.\n    assert!(\n        history.contains(&(PlaybackState::Idle, PlaybackState::Opening)),\n        "{history:?}"\n    );\n    assert!(\n        history.contains(&(PlaybackState::Buffering, PlaybackState::Playing)),\n        "{history:?}"\n    );`
What is wrong: The trail is checked with three `contains` probes, not an exact-sequence or forbidden-edge pin. A DETERMINISTIC spurious transition (say a Playing→Paused→Playing blip the engine adds on every run) passes this test AND the determinism test (which only compares run-to-run equality). native_seek.rs pins edges exactly (`teardown_edges == 0`, `open_edges == 1`); this file does not for the whole-media case. What correct looks like: assert the full expected edge multiset. Confidence: HIGH — Verification: read + ran.
F-002 [S4]
Where: apps/cli/tests/pipeline_accounting.rs:122-129
Code: `// The final frame is a real PNG (magic + IHDR + IEND) of the\n    // media's own extent.\n    assert!(\n        png.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),\n        "png head: {:?}",\n        &png[..16]\n    );\n    assert!(png.len() > 8 + 25 + 12, "{png:?}");`
What is wrong: The comment promises "magic + IHDR + IEND … of the media's own extent", but only the 8-byte magic and a minimal length are asserted — no IHDR parse (width/height never checked), no IEND check. The 320×240 extent is proven only indirectly via `staging_bytes == frames * 320*240*4` (L109-113). A PNG of the wrong dimensions (or a truncated-after-IHDR blob longer than 45 bytes) passes. What correct looks like: parse the IHDR (bytes 16..24, big-endian w/h) and assert 320×240, and check the IEND tail. Confidence: HIGH — Verification: read + ran.
F-003 [S4]
Where: apps/cli/tests/pipeline_accounting.rs:101-104
Code: `assert!(\n        accounting.scheduler_presented + accounting.scheduler_repeats >= 60,\n        "at least every frame once: {accounting:?}"\n    );`
What is wrong: The only `>=` bound in the file: presented+repeats could balloon (double-presentation regression) and still pass; the exact neighbors (pushed == 60, dropped_late == 0) constrain it only partially. Honestly labeled "at least every frame once". Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parent` | 15-23 | REAL | — | Same AVI fixture as native_seek.rs (exists, verified). |
| `dispatch_open` | 25-41 | REAL | — | Wire open + settle; `unreachable!` on failure. |
| `run_to_end` | 52-69 | REAL | — | Full run + screenshot_png; returns (trail, position, accounting, PNG). Executed. |
| `whole_media_accounting_closes` | 74-130 | REAL | S4 | Exact pins 120/0/60/0/0, staging identity, 2_000_000; F-001/F-002/F-003 soft spots. |
| `whole_media_runs_are_deterministic` | 134-142 | REAL | — | Full equality across runs incl. PNG bytes — executed; the repo's strongest determinism proof. |
| `native_seek_accounting_closes` | 151-201 | REAL | — | Executed; 90/45/15/1/0/0/0 + epoch survival + exact end position (cross-checked: same pins as native_seek.rs:185-199, both pass). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| whole_media_accounting_closes | YES | Exact delivery/scheduler/staging/position pins; a dropped or duplicated frame breaks 120/60/staging identity. Executed. |
| whole_media_runs_are_deterministic | YES | Byte-equality of final PNG across runs; any decode/schedule/render nondeterminism fails. Executed. |
| native_seek_accounting_closes | YES | 90 ≠ 60 discriminates native vs reopen; epoch + exact end position. Executed. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the whole first vertical … driven by the engine through the ONE command layer, with the headless frame accounting asserted" | pipeline_accounting.rs:1-6 | REAL | 3/3 executed through lumen_dispatch; counters from pipeline_host.rs. |
| "The final frame is a real PNG (magic + IHDR + IEND) of the media's own extent" | pipeline_accounting.rs:122-123 | SHALLOW | Magic + length only; IHDR/IEND never parsed (F-002). |
| "60 MJPEG frames + 60 PCM packets = 120 deliveries" | pipeline_accounting.rs:92-93 | REAL | Executed assert_eq!(frames_accepted, 120). |

## 7. Dead / duplicated / unreachable code
None. DuplicatION note (not a defect): the seek-accounting test duplicates native_seek.rs's counter pins (90/45/15/1) at one fewer level of detail — intentional cross-suite agreement; both pass.

## 8. Dependencies & flags
Test-only deps: commands, lumen_cli (dispatch/host), testkit. No feature gates, no #[ignore], no corpus dependency — runs everywhere (both prebuilt binaries pass).

## 9. Open questions for the next agent
- `screenshot_png()`'s encoding path (host.rs): does the PNG come from the same staging buffer the accounting counts (i.e., is staging_bytes honest evidence for the PNG's extent)? Owner: host.rs auditor.
- The baseline's 2 failing tests (committed_transcripts_match_fresh_runs, final_frame_is_current) are NOT in this file — confirm which suite owns them and whether they contradict the determinism verdict here (UNVERIFIED — out of my file list).
