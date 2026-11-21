# AUDIT: crates/codecs/examples/huffman_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 323 / 6 (NullSink, FrameSpan, split_frames, loadavg, configured, main) |
| File verdict | REAL-UNPROVEN (compiles in the baseline workspace test build; never executed anywhere — no recorded output) |
| Reachable from shipped entry point? | NO — an `examples/` binary: release.yml builds `-p lumen-cli -p desktop` + the tauri manifest (release.yml:193,219); examples are dev tools with no install path. Compiled as part of `cargo test --workspace` (baseline run #3 compiled clean — the suite cannot run otherwise). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Read tool full file; `rg -n "huffman_bench\|HUFFMANBENCH"` over docs/, scripts/, .github/, tools/, baseline runs → only the merge note (worklog archive-2026-10.md:8128, commit 393ca75 "W6-02 … + huffman_bench", "cut-off session WITHOUT worklog entries"); `rg -n "0\.16"` worklog → the 0.16x numbers come from `lumen decode-bench` + speed_bench.py (:7808, :7826), NOT from this tool; baseline INDEX grep → no huffman_bench run |

## 1. One-paragraph verdict
A competently written micro-benchmark harness: correct arg parsing with honest exit codes (0/64/66/70, all four implemented exactly as the docs promise), packet shells built once and re-fed by Arc clone so the measured loop contains no frame-byte copies, 2 warm-up + N≥3 measured passes, sorted median/p95/min stats with a machine-parseable summary line, and load-average sampling bracketing the measured window. The splitter's bounds arithmetic is panic-free on every path I traced. But the file's whole reason to exist — "the honest-numbers instrument for the heavy-mjpeg 0.16x-vs-ffmpeg target" — has never fired: no run of this binary exists anywhere in the repo's docs, worklog, or the baseline runs; the 0.16x figures it claims to instrument were measured by a different tool. The output format also has a misleading cumulative-count print, the splitter duplicates the parser's marker walk, and the name promises a Huffman-only scope the code does not have.

## 2. What actually works (verified)
— Compiles in the baseline: `cargo test --workspace` (baseline run #3, test-workspace.log.txt) compiles examples; a compile failure would have failed the suite — 1681 tests passed, so this binary built.
— Bounds/exit-code audit (read, every branch): all four documented exit codes implemented (64 at :192,:198,:206,:214,:218; 66 at :225,:231; 70 at :245,:273,:285); every `data[p]`/`data[p+1]`/`data[p+2]`/`data[p+3]` access traced to a preceding loop guard (`p + 3 < data.len()`, `p + 1 < data.len()`) — no panic path; `pos`/`p` strictly advance (no infinite loop); segment `len < 2` abandons the SOI (resync) instead of wrapping.
— Stats arithmetic: median index `passes/2` (true median at 21), p95 index `(passes*0.95).min(passes-1)` correct at the boundaries (3→max, 21→index 19); `partial_cmp().unwrap_or(Equal)` is NaN-safe; MB/s uses decimal 1e6 while the stderr display says MiB — units labeled distinctly, no lie.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/examples/huffman_bench.rs:1-2
Code:
```
//! MJPEG decode-core benchmark (card W6-02) — the honest-numbers
//! instrument for the heavy-mjpeg 0.16x-vs-ffmpeg target.
```
What is wrong: the instrument has never produced a number. `rg "huffman_bench|HUFFMANBENCH"` across docs/, scripts/, .github/, tools/ and audit/runs/baseline/ finds only the merge note (worklog archive-2026-10.md:8128: commit 393ca75, made by "a cut-off session WITHOUT worklog entries"). The 0.16x heavy-mjpeg figure this tool exists to chase was measured by `lumen decode-bench` + `scripts speed_bench.py` (worklog :7808, :7826) — a different runner over a different surface (the CLI's demux→decode path, not this example's direct contract path).
Why it matters: the optimization target ("heavy-mjpeg 0.16x → SIMD/color card", worklog :7834) will be judged against numbers this tool never emitted; two instruments measuring different surfaces (CLI decode-bench vs example direct-decode) will not agree, and there is no baseline run of either this tool or a documented invocation for the future optimization card to compare against. A benchmark that has never run is a claim, not an instrument.
What correct looks like: one committed run transcript (input file recipe from the doc's own ffmpeg line, `--passes 21`, the summary line) in docs/ or the worklog, establishing the pre-optimization baseline this tool was created to pin.
Confidence: HIGH — Verification: read + repo-wide grep (no run evidence anywhere); UNVERIFIED only in the sense that I did not execute it (no cargo per rules) — REQUEST-ORCH-RUN: `cargo run -p codecs --release --example huffman_bench -- <a generated heavy-mjpeg fixture> --passes 21` and commit the summary line.

### F-002 [S4]
Where: crates/codecs/examples/huffman_bench.rs:290-295 (and doc :19)
Code:
```
        eprintln!(
            "pass {pass:>2}: {:8.1} ms ({} frames, {} emitted)",
            durations[pass],
            frames.len(),
            sink.frames
        );
```
What is wrong: `sink.frames` is CUMULATIVE across warm-up + all measured passes (never reset), but the parenthetical reads as per-pass ("N frames, M emitted") — a reader comparing 300 frames with 6,900 emitted must do arithmetic to discover the pass count. The final summary at :307-312 does state the cumulative semantics honestly; the per-pass line does not. Related: module doc :19 says "The 300 packet shells are built once" — the code builds `frames.len()` shells; 300 is the doc's example-recipe count, not a code fact.
Why it matters: measurement-tool output that invites misreading undermines the tool's stated purpose ("honest numbers").
What correct looks like: print the per-pass delta (`sink.frames - frames_before_pass`), and drop the hard-coded 300 from the doc.
Confidence: HIGH — Verification: read (sink never reset; counters traced).

### F-003 [S4]
Where: crates/codecs/examples/huffman_bench.rs:74-152 (esp. 108-129)
Code:
```
                    while p + 1 < data.len() {
                        let b = data[p];
                        if b == 0xff {
```
What is wrong: (a) the splitter re-implements the mjpeg parser's marker walk (~78 lines: segment lengths, FF00/FFF/RST entropy skips, EOI) instead of reusing `parser.rs` — documented as "test-side … the same walk shape", but it is a second walker to maintain and it already differs from the parser (e.g., the parser rejects non-baseline SOFs typed; this splitter silently skips any segment, so a progressive-JPEG frame in the stream becomes a span the decoder then refuses at exit 70 — behavior differs, honestly fatal). (b) a final frame truncated at EOF (entropy loop exits on `p + 1 >= data.len()`) still gets `end = Some(p)` — the splitter hands the decoder an incomplete frame as if complete, contra the doc's "must not fabricate frames from garbage" (:72-73); the bench then fails exit 70 rather than reporting a split error.
Why it matters: (a) drift between the two walkers will silently change what the bench measures vs what the player decodes; (b) a truncated input yields "decode failure" instead of "input truncated" — misdiagnosed exit reason.
What correct looks like: reuse `parser.rs`'s walk for the header portion (it is `pub(crate)`; the example cannot see it — either expose a test-support split or accept the duplication with a diff-reminder comment); and distinguish EOF-truncation in the splitter (resync/abandon, exit 66 with "truncated final frame").
Confidence: HIGH — Verification: read (both walkers compared line by line; EOF path traced).

### F-004 [S4]
Where: crates/codecs/examples/huffman_bench.rs:45 (name), 155-159 (loadavg)
Code:
```
const USAGE: &str = "usage: huffman_bench <file.mjpeg> [--passes N]";
```
What is wrong: the name promises a Huffman benchmark; the binary times the FULL decode path (marker walk + Huffman + IDCT + color conversion + frame handoff) — the doc's own first line says "MJPEG decode-core benchmark". Also `loadavg()` reads `/proc/loadavg` (Linux-only): on macOS/Windows the summary prints `load_before=? load_after=?` — the "load honesty" protocol silently degrades on two of the three dev platforms.
Why it matters: misleading names are on the audit checklist; a dev on macOS gets a weaker honesty protocol with no warning printed.
What correct looks like: rename (e.g., `mjpeg_decode_bench`) or scope the docs; print a one-line warning when loadavg is unavailable on a non-Linux host.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-33 | REAL | S3/S4 | protocol claims all verified in code EXCEPT "300 packet shells" (F-002) and the instrument's never-run status (F-001); exit-code table exact |
| `NullSink` + impl | 49-58 | REAL | — | counts and drops; cumulative counter is the F-002 print bug's root |
| `FrameSpan` | 61-64 | REAL | — | trivial |
| `split_frames` | 74-152 | REAL | S4 | F-003: duplicate walk + EOF-truncated frame accepted; otherwise bounds-clean, resync advances, no panics |
| `loadavg` | 155-159 | REAL | S4 | F-004 portability; `.ok()?` chain correct |
| `configured` | 162-180 | REAL | — | real contract-path configure; errors as strings (fine for an example) |
| `main` | 182-323 | REAL | S4 | arg parse exact (no `--flag=value` form — acceptable); stats arithmetic verified; F-002 print; exit codes match docs |

## 5. Test honesty
No tests exist for or in this file (it is an example). Nothing asserts the splitter's frame extraction, the stats arithmetic, or the exit codes — the example's own correctness is verified only by this read audit and by compilation. A mutation in `split_frames` (e.g., the EOF-truncation path) would ship silently. (Examples are conventionally untested in this repo — walk_bench.rs is the same shape — but for the named optimization instrument, a golden-input split test would be cheap.)

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the honest-numbers instrument for the heavy-mjpeg 0.16x-vs-ffmpeg target" | huffman_bench.rs:1-2 | REAL-UNPROVEN | code is real and compiles (baseline #3); zero recorded runs (F-001); the 0.16x number came from lumen decode-bench + speed_bench.py (worklog :7808) |
| "per-pass wall time on stderr, one machine-parseable summary line on stdout (median / p95 / min, fps, MB/s)" | :14-18 | REAL | code 280-321 implements exactly this |
| "after 2 warm-up passes, N measured passes" | :15-16 | REAL | 269-276 (2 warm-up), 280 (N) — `--passes` floor of 3 enforced at :195 |
| "each measured pass re-feeds them by `Arc` clone, so the bench's own per-pass overhead is refcount bumps" | :19-22 | REAL | 252-264 (built once), 270/282 (`packet.clone()`); allocation counting delegated to zero-alloc suites (verified: `tests/zero_alloc_decode.rs` passed in baseline) |
| "Exit codes: 0 ok, 64 usage, 66 unreadable input, 70 decode failure" | :27-29 | REAL | all four sites verified (§2) |
| "the splitter must not fabricate frames from garbage" | :72-73 | SHALLOW | the EOF-truncated tail IS handed over as a frame (F-003b) — it fails decode (exit 70) rather than fabricating a measurement, so the honesty outcome holds by accident of the decoder's refusal |

## 7. Dead / duplicated / unreachable code
Not shipped (example). Duplication: the marker walk vs parser.rs (F-003a). No dead code inside the file — every function is on main's path.

## 8. Dependencies & flags
codec-api (prelude), codecs (mjpeg decoder), media-types — same graph as the crate itself; no new deps, no unsafe, no feature gates. Built by `cargo test --workspace` (examples compile) and `cargo build --examples`; NOT built by release.yml's artifact lanes.

## 9. Open questions for the next agent
- Whoever owns the heavy-mjpeg SIMD/color optimization card: this tool needs one committed baseline run (F-001) before optimization work starts, or the card will compare against the CLI decode-bench numbers measured through a different surface.
- mjpeg/decoder.rs owners (W06-A04/A05): the pooled-slot allocation discipline this bench measures (`FRAME_SLOTS`) is the same one whose test I found tautological in dnx (my dnx part2 F-001) — the mjpeg twin of that test deserves the same scrutiny.
