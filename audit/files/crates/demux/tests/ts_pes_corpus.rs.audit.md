# AUDIT: crates/demux/tests/ts_pes_corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 545 / 4 tests + walker/helpers (corpus_dir, TsPacket, walk_packets, demux_lane) |
| File verdict | REAL-UNPROVEN (assertions are live and non-vacuous — proven by execution — but all 4 tests take the skip path in this tree; every pinned value rests on worklog provenance) |
| Reachable from shipped entry point? | Integration test (`cargo test -p demux --test ts_pes_corpus`); the code under test (`demux::ts::pes::PesAssembler`) is shipped-library surface |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | baseline grep (below); `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 'ts::pes'` -> 20 passed; generated 5 own ffmpeg 7.1.5 `.ts` lanes into /tmp/w04ts/media and ran `LUMEN_CORPUS=/tmp/w04ts/media ./target/debug/deps/ts_pes_corpus-6a1163f465cc2829` per-test (3 differential tests FAILED on value mismatches — pins live; fuzz-lite FAILED its bite meta-assert, no panic — 512 rounds survived) |

## 1. One-paragraph verdict
This is a genuinely differential test file — I proved the pins are live by feeding it freshly generated, equally valid ffmpeg `.ts` lanes: every pinned assert fires (`0000-param-midchange.ts s0 left: 12 right: 81`), so nothing here is vacuous. But in this tree the suite asserts nothing: the corpus dir `/tmp/my-project/corpus-v4/media` does not exist, all four tests take the documented loud-skip path, and the baseline shows `4 passed ... finished in 0.01s` — 0.01 s cannot read and differentially demux five TS lanes, so all four baseline "passes" are skips. The entire ffprobe-truth table, the 53-value B-frame PTS/DTS series, and the cadence pin rest solely on the worklog's claimed 2026-10-24 session (docs/worklog/archive-2026-10.md:9139-9145, whose numbers match the pins exactly). Separately, the fuzz-lite test's "mutations must bite" meta-assert is coupled to the specific corpus instance's file shape: on my own valid h264-only `0091` replacement it fails its own assertion (zero typed refusals in 512 rounds — the assembler correctly refused to panic, and correctly refused to invent errors), meaning the suite cannot be repointed at any regenerated corpus without editing.

## 2. What actually works (verified)
- Pins are non-vacuous: `LUMEN_CORPUS=/tmp/w04ts/media ./target/debug/deps/ts_pes_corpus-6a1163f465cc2829 --exact --nocapture corpus_ts_lanes_match_the_ffprobe_truth` -> panicked at ts_pes_corpus.rs:267 `0000-param-midchange.ts s0 left: 12 right: 81` (my AAC lane has 12 PES vs pinned 81).
- The PES assembler survived 512 deterministic LCG mutation rounds (flips/truncations/wild-AF/bursts) over real h264 TS bytes with **no panic and no hang** (fuzz-lite ran to its final meta-assert in 0.39 s) — the `+FB` no-panic half of the contract is REAL for this input.
- The underlying `ts::pes` unit suite passes: `./target/debug/deps/demux-5e0c7194d741938b --test-threads=2 'ts::pes'` -> `20 passed; 0 failed`.
- Pin/worklog consistency: LANES entries for 0091 (53 pkts, 103,550 bytes, PTS 132,667..302,667, wire DTS 126,000..299,334 on 40 of 53) are copied exactly from archive-2026-10.md:9144-9145.

## 3. Findings

F-001 [S3]
Where: crates/demux/tests/ts_pes_corpus.rs:241-249 (and 123-129), baseline audit/runs/baseline/cargo-test-workspace.txt:2429-2434
Code:
```rust
fn corpus_ts_lanes_match_the_ffprobe_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(ts-pes): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    }
```
What is wrong: the entire differential — 5 lanes of packet counts, PTS/DTS ends, byte totals, monotonicity, the 53-value series, the 5000-tick cadence, and the fuzz-lite — never executes in this tree. Baseline: `test corpus_ts_lanes_match_the_ffprobe_truth ... ok` with `finished in 0.01s` for all 4 tests — the skip path (I verified `/tmp/my-project/corpus-v4/media` does not exist and that a real differential run of five lanes takes far longer than 0.01 s; my single-lane runs took 0.39 s for fuzz-lite alone). Consequence: zero regression protection for the PES layer against the ffprobe truth in CI/baseline; any PTS/DTS/count regression in `PesAssembler` ships green. The skip is documented and loud, and the corpus genuinely cannot live in-tree (~120 MB) — but the honest classification is that this file's evidence is REAL-UNPROVEN, entirely worklog-backed. What correct looks like: a small committed `.ts` fixture lane (even one) wired to the same assert body so the differential shape runs in-tree, with the full-corpus lanes as the LUMEN_CORPUS extension. Confidence: HIGH — Verification: ran (skip path in baseline + live-fire on my own lanes).
Confidence: HIGH — Verification: ran

F-002 [S3]
Where: crates/demux/tests/ts_pes_corpus.rs:539-544
Code:
```rust
    // The mutations bite (not all rounds are silently clean) —
    // measured: truncation refusals occur on the cut-short rounds.
    assert!(
        rounds_with_typed_error > 0,
        "fuzz-lite never hit a typed refusal — mutations do not bite"
    );
```
What is wrong: the meta-assert is coupled to the corpus instance's file shape, not to a property of the demuxer. Measured: on my own valid ffmpeg h264-only TS (named `0091-bframe-storm.ts`, 75 frames, bf=3), 512 rounds produced ZERO typed errors — the assertion fails verbatim: `fuzz-lite never hit a typed refusal — mutations do not bite` (no panic; the assembler's no-panic contract held). The corpus 0091 carries a bounded-PES audio lane whose cut-short rounds produce sticky truncation refusals; a video-only file (unbounded PES, no declared lengths to violate) legitimately never triggers one. So the test encodes "this specific file must produce errors", not "mutations must be survivable and honest" — repointing at any regenerated corpus breaks it. What correct looks like: feed one bounded-PES stream synthetically (or assert the round outcomes per mode), decoupling the bite-check from the corpus instance. Confidence: HIGH — Verification: ran (failure output above).
Confidence: HIGH — Verification: ran

F-003 [S4]
Where: crates/demux/tests/ts_pes_corpus.rs:164-172 (test-only walker PCR read)
Code:
```rust
                if flags & 0x10 != 0 && 7 + length <= 188 {
                    // PCR: 33-bit base + 6 reserved + 9-bit extension.
                    let base = (u64::from(packet[6]) << 25)
```
What is wrong: the guard bounds the adaptation field, not the PCR itself: with a malformed `length < 6` and the PCR flag set, `packet[6..12]` reads past the adaptation field into payload bytes (still in-bounds of the 188-byte packet — no panic, garbage PCR observed). Test-only walker, superseded by I1a per the module docs; noted for the record, not a shipped defect. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 123-129 | REAL | — | override + default, None = loud skip |
| TsPacket / walk_packets | 131-193 | REAL | S4 | minimal 188-byte scan; bounds-checked; PCR over-read note (F-003); drops non-0x47 without resync (fine for clean lanes, documented) |
| demux_lane | 197-239 | REAL | — | feeds PesAssembler verbatim payloads, collects ledgers |
| corpus_ts_lanes_match_the_ffprobe_truth | 241-353 | REAL-UNPROVEN | S3 | live pins (proven), never runs in-tree (F-001) |
| bframe_reorder_pts_series_matches_ffprobe_exactly | 355-421 | REAL-UNPROVEN | S3 | 53-value series + 13 DTS-absent indices (40-of-53 consistent); never runs in-tree |
| midchange_lane_timestamps_are_uniform_and_events_absent | 423-453 | REAL-UNPROVEN | S3 | 5000-tick cadence + zero-event invariant; never runs in-tree |
| fuzz_lite_mutations_never_panic_or_hang | 458-545 | REAL | S3 | no-panic half VERIFIED by my run; bite meta-assert is instance-coupled (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_ts_lanes... | YES (when the corpus exists) | exact counts/PTS/DTS/bytes/monotonicity per lane — proven live on substitute files |
| bframe_reorder... | YES (when corpus exists) | full 53-entry series equality incl. reorder dips + per-index DTS presence |
| midchange... | YES (when corpus exists) | uniform cadence + all-DTS-absent + zero events |
| fuzz_lite... | NO (panic/hang half) on its own | no-panic is the only demuxer assertion; the bite-assert checks the input file, not the demuxer (F-002) — my run: no panic in 512 rounds, meta-assert failed |
| In-tree reality | ALL FOUR SKIP | baseline 0.01s "ok" = skip path (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compared against the measured ffprobe truth" | ts_pes_corpus.rs:1-7 | REAL-UNPROVEN | pins live (my substitute run fired them) but never executed in-tree; provenance = archive-2026-10.md:9139-9145 only |
| "this suite skips loudly" | ts_pes_corpus.rs:11-13 | REAL | eprintln + baseline 0.01s |
| "measured: no discontinuity_indicator bits set, PCR monotone" | ts_pes_corpus.rs:37-39 | UNVERIFIED | worklog-only; no in-tree run can confirm |
| "outcomes Ok-or-typed only" (fuzz-lite) | ts_pes_corpus.rs:456-457 | REAL (no-panic) / UNVERIFIED (typed half on corpus bytes) | my run: all Ok on video-only input — no typed outcomes observed, none invented |

## 7. Dead / duplicated / unreachable code
None dead. The walker is intentionally test-local (I1a supersedes). LANES/stream tables all consumed.

## 8. Dependencies & flags
`demux::ts::pes` (library surface), `testkit::Must`. LUMEN_CORPUS env override. No features. Binary `ts_pes_corpus-6a1163f465cc2829` is newer than the source (verified mtimes) — my runs reflect current code.

## 9. Open questions for the next agent
- The ts::pes library file (src/ts/pes.rs, another agent's) — my fuzz run shows truncation of unbounded video PES never yields a typed refusal at finish(); confirm finish()'s flush semantics are the intended "emit what completed" (vs corpus audio lanes' sticky truncation).
- If anyone regenerates the corpus, LANES and the 53-entry series must be re-measured — the worklog notes "regenerates with fresh seeds", so the pins are one-way (F-001/F-002 compound).
