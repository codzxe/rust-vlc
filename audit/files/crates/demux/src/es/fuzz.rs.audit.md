# AUDIT: crates/demux/src/es/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 130 / 7 (2 harness fns, 1 shared drain fn, 1 const, 2 tests) |
| File verdict | UNREACHABLE (real, executed harnesses; callers are the in-crate tests and two cargo-fuzz shims no gate builds) |
| Reachable from shipped entry point? | NO — `fuzz_demux_es`/`fuzz_demux_mp3stream` are called only by the tests below and by `fuzz/fuzz_targets/fuzz_demux_es.rs` / `fuzz_demux_mp3stream.rs` (both exist on disk; both "not built by any gate" — nightly wiring outstanding, fuzz/README.md). The shipped lanes themselves (h264/hevc/aac/mp3 `open`) ARE reachable via the ES container host (`apps/cli/src/containers/mod.rs:63-68` `h264`/`hevc`/`mp3`/`mp2` arms) — this file is the fuzz surface over them. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full Read; `rg "pub fn probe\|pub fn open\|parse_head\|impl RawMp3Demuxer"` over es/*.rs (all lane fns exist with matching signatures); `ls fuzz/fuzz_targets/` (both shims exist); baseline `orchrun-test-workspace-nofailfast.log.txt:1678-1679` (2/2 file tests ok), `:2019-2028` (es_corpus 4/4 ok) |

## 1. One-paragraph verdict
A correct, tightly-scoped survival harness: budgeted drain (≤200k events, budget checked *before* each call — at most BUDGET `next_event` invocations), bounded probe offsets (`at ≤ len−4`, re-derived safe), 16-byte head-window probes, errors swallowed by contract, and an accurate wiring-status doc (unlike its avi sibling, this one correctly states the targets exist and only nightly wiring is owed). The weakness is depth: the "deterministic sweeps" it claims prove the harness on stable are two tests over ~70 fixed inputs with zero structured-input mutation — no scrambles, no adversarial size patches, no boundary truncations, and no integration survival suite sibling (avi got `fuzz_loop_stable` + `fuzz_survival`; ES got only `es_corpus`, which is well-formed-file *parity*, not hostile survival). Both shims have never been built by any gate, so no libFuzzer execution has ever exercised these lanes.

## 2. What actually works (verified)
— Both tests executed and passed: `orchrun-test-workspace-nofailfast.log.txt:1678-1679` (`es::fuzz::tests::* ... ok`).
— The ES lanes' real outcomes are pinned by `tests/es_corpus.rs` — 4/4 ok at baseline :2019-2028 (`corpus_h264/hevc_lane_matches_ffprobe_packet_geometry`, `corpus_aac/mp3_lanes_match_ffprobe_packet_counts`) — ffprobe-differential, not self-generated goldens.
— Budget arithmetic re-derived: `drain_lane` checks `drained >= FUZZ_EVENT_BUDGET` before incrementing and calling — exactly BUDGET calls max; both break arms (`EndOfStream`, `Err`) terminate.
— Probe-offset arithmetic re-derived: guard `data.len() >= 4`; `at = (i*97).min(len−4)` for i ∈ 0..8 → `data[at..at+4]` always in bounds; `usize::try_from(i*97).unwrap_or(0)` cannot hit its fallback (max 679).
- Lane surface verified: `h264::probe/open` (h264.rs:15/25), `hevc::probe/open` (hevc.rs:16/26), `aac::probe/open` (aac.rs:20/30), `mp3::parse_head` (mp3.rs:180), `RawMp3Demuxer::new` (mp3.rs:293) — all exist, signatures match usage.

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/es/fuzz.rs:26-28 (claim) vs 104-129 (the entire hostile suite)
Code:
```rust
//! (Lead-owned nightly
//! wiring, see `fuzz/README.md`); the harness logic is meanwhile
//! proven on stable by the deterministic sweeps below and the crate's
//! ES corpus run.
```
with the full hostile inventory at 108-129:
```rust
    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_demux_es(&[]);
        fuzz_demux_es(&[0x00, 0x00, 0x00, 0x01]);
        fuzz_demux_mp3stream(&[]);
        fuzz_demux_mp3stream(&[0xFF, 0xFB]);
```
What is wrong: "proven … by the deterministic sweeps below" overstates ~70 fixed inputs (2 marker inputs, 2×4096 hostile fills, 64 short ramps) with no mutation of a *structured* lane input. The avi twin (avi/fuzz.rs) has 64-flip scrambles of a plausible AVI prefix, targeted u32::MAX/0/1 size patches at 8 structural offsets, boundary truncations inside the newest parse surface, plus two integration survival suites (`fuzz_loop_stable`, `fuzz_survival`). The ES lanes — byte-header grammars (Annex-B start codes, MP3 frame headers) with their own size arithmetic — have none of that, and `es_corpus` pins parity on well-formed files, not survival on hostile ones. No ES integration survival suite exists (checked `ls crates/demux/tests/`: only `ts_fuzz_survival.rs`).
Why it matters: the claimed stable-toolchain proof of "never panics, never hangs, never allocates without a ceiling" rests on the thinnest fixture set of any demux fuzz harness, while the actual campaign (which would close the gap) has never run.
What correct looks like: port the avi pattern — a plausible H.264/HEVC/MP3 prefix scrambled at increasing severity, targeted patches of the length/rate/channel fields, truncations at AU boundaries; or an `es_fuzz_survival.rs` integration sibling.
Confidence: HIGH — Verification: read + tests-directory listing + baseline log (test counts exact).

### F-002 [S4]
Where: crates/demux/src/es/fuzz.rs:74-85
Code:
```rust
    // The three open-constructed lanes, each a full drain (one
    // shared stream clone per lane — the mem stream is seekable and
    // length-known, exactly the lanes' contract).
    if let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) {
```
What is wrong: each lane re-copies the whole input (`data.to_vec()` ×3 per `fuzz_demux_es` pass) — the comment calls these "one shared stream clone per lane", but nothing is shared: each is an independent full copy. The avi harness manages one copy; this one triples memory churn per pass for no benefit (each lane's stream is consumed independently, but a single `Arc<[u8]>` cloned per lane would do — `mem_stream` accepts `Arc<[u8]>` per its aiff usage).
Why it matters: fuzz throughput (three copies per exec eats budget in a campaign) and a comment that misdescribes the code it sits on.
What correct looks like: `let bytes: Arc<[u8]> = data.into(); … mem_stream(Arc::clone(&bytes))` per lane, comment fixed to "one stream per lane over a shared buffer".
Confidence: HIGH — Verification: read (mem_stream signature cross-checked via aiff/test_support usage).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FUZZ_EVENT_BUDGET` | 38-39 | REAL | — | 200k, same value/proposal marker as the avi twin. |
| `drain_lane` | 43-63 | REAL | — | Generic over `impl Demuxer`; budget checked pre-call (≤ BUDGET calls exactly); `Err` and `EndOfStream` both break; no panics (no unwrap/index — verified line by line). |
| `fuzz_demux_es` | 68-86 | REAL | S4 | F-002 (3 copies). Probes: 16-byte head on all three families; `let _ =` discards bool probes (h264/hevc/aac::probe return bool — verified) — contract-compliant. |
| `fuzz_demux_mp3stream` | 90-102 | REAL | — | 8 clamped `parse_head` probes + one lane drain; bounds re-derived safe. |
| tests ×2 | 108-129 | REAL | S4 | F-001 (depth). Survival-only contract, honest; would fail on any panic in any lane. |

## 5. Test honesty (2 tests, baseline :1678-1679 both ok)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES | Empty/2-byte/4-byte-startcode/2-byte-sync inputs — a construction or probe panic fails it. |
| hostile_wide_inputs_survive | YES | 4096×0x00 (start-code floods), 4096×0xFF (maximal sync false positives), 64 ramping lengths — a lane panic on these shapes fails it. GAP (F-001): no mutated structured input, no size-field patches, no truncation sweep — the mutation classes that historically find demuxer bugs. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | :17-20 | REAL | Read; tests execute exactly that. |
| "The cargo-fuzz targets live at fuzz/fuzz_targets/fuzz_demux_es.rs and …mp3stream.rs (Lead-owned nightly wiring)" | :24-26 | REAL (accurate, unlike avi/fuzz.rs's stale twin) | Both shims exist on disk (ls); README confirms nightly outstanding. |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below and the crate's ES corpus run" | :26-28 | REAL-UNPROVEN at the claimed strength | 2 tests / ~70 inputs executed (baseline-cited) + es_corpus parity (4/4) — "proven" overstates the hostile coverage (F-001). |
| "the walks are monotone" | :32 | REAL (cross-check) | Same walk family as the avi lanes (es/annexb.rs + mp3.rs walk — read during lane-signature verification). |

## 7. Dead / duplicated / unreachable code
None dead in-file. Pattern note (same as files 2/5/6): `pub mod fuzz` + `crate::avi::test_support::mem_stream` ship in the release lib for test/fuzz-only use — the ES family reaching into the AVI module's test_support is the same cross-family layering noted in the aiff audit.

## 8. Dependencies & flags
None beyond crate-internal (plugin_api Demux/DemuxEvent, es lanes, avi::test_support). No unsafe, no unwrap, no panics. Budgets named with `[PROPOSAL]` markers, consistent house style.

## 9. Open questions for the next agent
- If the es wave adds a survival integration suite (F-001's fix), mirror `fuzz_loop_stable.rs`'s accounting determinism checks so a regression is exactly reproducible.
- The mp3 lane's `parse_head` probes cover only 8 offsets of the first ~679 bytes — under a real campaign libFuzzer will find the rest; under the stable-only regime they are the entire hostile coverage of the header parser (cross-check with the mp3.rs owner's unit tests).
