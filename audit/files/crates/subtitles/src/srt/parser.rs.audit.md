# AUDIT: crates/subtitles/src/srt/parser.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-21 |
| Lines / functions | 567 (380 production + 187 tests) / 17 production units + 10 tests |
| File verdict | UNREACHABLE (real, differential-tested parser; zero shipped-entry-point callers) |
| Reachable from shipped entry point? | **NO.** `subtitles::srt::parse` is re-exported by `crates/subtitles/src/srt/mod.rs:64` and called only by `crates/subtitles/src/srt/fuzz.rs:41` (`fuzz_sub_srt`) and the crate's own tests (`srt_fuzz_survival.rs`, `srt_canonical_differential.rs`, `fuzz_survival.rs`, `roundtrip_matrix.rs`). `rg -l "subtitles::" crates apps` → nothing outside `crates/subtitles`. The `fuzz/` scaffold that also references it is its own workspace root, unwired (`fuzz/Cargo.toml` header: "NOT wired into any gate"). No demux/CLI/engine seam exists: `sub-srt.md:88-90` itself admits "The plugin-api `SubtitleParser` seam is NOT wired". `lumen engine-info` features list no subtitle lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `cat crates/subtitles/src/srt/parser.rs` (full read); `sed -n '18,24p;34,36p;141,161p;261,268p;356,372p'` (quote verification); `cat crates/subtitles/src/srt/fuzz.rs crates/subtitles/src/srt/mod.rs` (harness bounds claim + re-export map); `rg -n "impl CueInterval" -A 20 crates/subtitles/src/model/time.rs` (`CueInterval::new` errors only on `end < start`; `end == start` legal → parser.rs:252 `unwrap_or(EMPTY)` is infallible as commented); `rg -n "srt::|parse\(" crates/subtitles/tests/*.rs` (caller map: tests only); `sed -n '44,51p;62,82p;84,96p' docs/formats/sub-srt.md` (doc claims); `rg -n "deps/subtitles|test result" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (execution evidence). |

## 1. One-paragraph verdict
A genuinely careful, dirty-world SRT walker with ffmpeg-pinned ms-field semantics, synthetic ids, saturation, run-together resync, and a capped damage ledger — and its unit suite asserts real behavior (µs-exact times, damage kinds, line numbers), backed by a 6/6 ffmpeg differential and fuzz-survival runs (all green in `orchrun-test-workspace-nofailfast.log.txt`). Two lies stand in it: the "one pass / one-pass O(n)" complexity claim is false — the resync loop is quadratic on exactly the malformation it advertises handling (missing blank lines) — and the module doc's millisecond semantics (`,5` = 500 ms) contradicts the code, the tests, and the format record (`,5` = 5 ms). And like every file in this crate: no shipped binary can reach it.

## 2. What actually works (verified)
— Full block walk, tolerances, damage kinds: verified by reading, and by baseline `orchrun-test-workspace-nofailfast.log.txt` → `subtitles` lib `test result: ok. 267 passed; 0 failed` (includes this file's 10 tests) + `srt_canonical_differential` `6 passed` + `srt_fuzz_survival` `4 passed`.
— `CueInterval::new(...).unwrap_or(CueInterval::EMPTY)` (line 252) is infallible as claimed: the reversal is rejected at :221-224 and `CueInterval::new` errors only on `end < start` (model/time.rs:48-56, `end == start` is legal).

## 3. Findings

### F-001 [S2] Quadratic resync: a blank-line-stripped SRT (the documented "classic malformation") parses in O(N²) — "one pass"/"one-pass O(n)" claims are false
Where: crates/subtitles/src/srt/parser.rs:141-161 + :261-268 (with the false claims at :34-36)
Code:
```rust
    let mut offset = 0usize;
    while offset < owned.len() {
        match parse_block(&owned[offset..], cues, synthetic, sink) {
            Resync::Done => break,
            Resync::At(at) => {
                sink.record(SrtDamageKind::StrayLinesBeforeTiming, owned[offset + at].0);
                offset += at;
            }
        }
    }
```
```rust
fn resync_after(lines: &[(u32, &str)], from: usize) -> (usize, Resync) {
    for (q, (_, line)) in lines.iter().enumerate().skip(from + 1) {
        if line.contains("-->") && looks_like_timing(line.trim()) {
```
What is wrong: `finish_block` advances one cue at a time, and every `parse_block` call runs `resync_after`, which scans forward **to the end of the block** (`lines.len()`), re-probing every remaining line with `contains("-->")` plus a full `looks_like_timing` re-parse. A document with no blank lines — one giant block — costs Σ(N−k) probes = O(N²·L). This is not exotic: the module doc itself calls the missing-blank-line shape "the classic 'missing blank line' malformation" (:136-137), i.e. real-world stripped/joined SRT files. A 100k-line (≈2.5 MB) stripped SRT performs ~5×10⁹ line probes (each a throwaway `DamageSink` + full timing parse) — minutes to hours; 30k cues ≈ 10–60 s. The damage ledger cap does not bound the loop.
Why it matters: three separate docs assert the opposite of reality: parser.rs:34-36 "Everything is one pass over `str::split('\n')` slices", `srt/fuzz.rs:27-29` "Parse: one pass, output ≤ O(input)" (part of the never-hangs harness contract), and `docs/formats/sub-srt.md:92` "parser is one-pass O(n)". A user dropping a large damaged SRT on any future wiring freezes the parse. The fuzz corpus (37 committed mutants ≤4.7 KB + 2000 LCG iterations) is orders of magnitude too small to notice.
What correct looks like: one global scan that pre-identifies all valid timing lines (a `Vec<bool>` of resync points built in a single pass), so `finish_block` consumes block slices without re-scanning; or memoize `looks_like_timing` per line index.
Confidence: HIGH — Verification: read (loop structure traced; per-call scan range verified). Wall-clock extrapolated, not executed (no cargo runs permitted): REQUEST-ORCH-RUN at the bottom.

### F-002 [S3] Module doc contradicts the code (and its own test) on millisecond semantics
Where: crates/subtitles/src/srt/parser.rs:20-21 (doc) vs :358-360 (code) vs :448-452 (test)
Code (doc, :18-23):
```rust
//!   `.` is accepted for the `,` millisecond separator; the
//!   millisecond field may be 1–3 digits (decimal-fraction scaling:
//!   `,5` = 500 ms) or absent; whitespace around `-->` is optional;
```
Code (implementation, :358-360):
```rust
    // Field semantics, ffmpeg/VLC `sscanf("%3d")` style: the digits
    // ARE the milliseconds (`,5` = 5 ms), 1–3 digits accepted; more
    // than 3 truncate to the first 3 with damage.
```
What is wrong: the module doc says `,5` = 500 ms (decimal-fraction scaling); the implementation parses the digits as a literal millisecond field (`,5` = 5 ms), the test `timing_tolerances` pins 5 ms (`assert_eq!(doc.cues[0].interval.start_us(), 1_005_000)` for `00:00:01.5`), and the format record agrees ("1–3 digit ms FIELD semantics (verified against ffmpeg 7.1.5 output)", sub-srt.md:46-47). The code is right; the doc comment is false.
Why it matters: the doc is the first thing a maintainer reads; anyone "fixing" the code to match the doc would regress away from the ffmpeg-verified behavior the differential pins.
What correct looks like: change the doc line to "field semantics: `,5` = 5 ms (ffmpeg/VLC `sscanf` behavior)".
Confidence: HIGH — Verification: read all three sites; test green in baseline no-fail-fast run.

### F-003 [S4] Hours field beyond u64 drops the cue (BadTimestamp) instead of the documented saturation
Where: crates/subtitles/src/srt/parser.rs:355 + :374-385, doc claim :23-24
Code:
```rust
    let h: u64 = h.parse().ok()?;
```
What is wrong: the doc tolerance list says "values beyond the i64 microsecond domain saturate with damage" (:23-24), and the code saturates — but only for values whose fields fit `u64`. A hours field with ≥20 digits (e.g. `99999999999999999999:00:00,000`) fails `h.parse::<u64>()` → `None` → the whole timing line is unparseable → `BadTimestamp` and the cue is dropped, not clamped.
Why it matters: trivial inconsistency between documented and actual tolerance on hostile input; the saturation path exists only for the u64-representable subset.
What correct looks like: saturate `h` at `u64::MAX/3_600_000` on parse failure rather than failing the line (or document the u64 domain limit).
Confidence: HIGH — Verification: read (`parse().ok()?` propagates None; no u64 fallback).
### F-004 [S4] Duplicated near-identical predicates; numeric-text-theft heuristic undocumented outside a code comment
Where: crates/subtitles/src/srt/parser.rs:277-288 (the pair), :231-239 (the heuristic)
Code:
```rust
/// `true` for a plain unsigned decimal (an index-line candidate).
fn is_index_like(line: &str) bool {
    let t = line.trim();
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// `Some(n)` when the line is a plain unsigned decimal that fits u64.
fn parse_index(line: &str) -> Option<u64> {
```
What is wrong: (a) `is_index_like` and `parse_index` encode the same predicate twice with a silent divergence (u64 fit), inviting drift; (b) the resync adoption rule (:231-239) silently reinterprets a legitimate numeric TEXT line (a subtitle that is just "42") as the next run-together cue's index — the text is dropped, leaving only `EmptyCueText`/`StrayLinesBeforeTiming` damage. Real-content loss, but only on already-malformed (run-together) input, damage-recorded, and hinted at in the :226-229 comment. Also: `TrailingJunk` (:305-307) is recorded for LEADING junk on the start side — the name lies.
What correct looks like: one `is_index_like` returning `Option<u64>`; the heuristic documented in the module doc's tolerance list; rename the start-side damage or record a distinct kind.
Confidence: HIGH — Verification: read (heuristic traced on the `stray_text_before_timing_is_kept_as_text` shape).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct SrtDocument` | 50-59 | REAL | — | Plain data: cues + damages + dropped count + encoding. |
| `SrtDocument::is_clean` | 63-66 | REAL | — | Correct: empty ledger AND zero dropped. Tested. |
| `parse` | 76-88 | REAL | — | Bytes → decode → normalize → walk. `# Panics Never` doc holds (no unwrap/index panic path found; all indexing guarded by `Option`/bounds). Entry point for a seam that does not exist. |
| `parse_str` | 93-104 | REAL | — | Pre-decoded variant; encoding hardcoded `Utf8` (honest: caller decoded). |
| `type Block` | 107 | REAL | — | Vec of (line_no, &str); grows per block — one giant block = N entries (16 B each, fine). |
| `walk` | 110-124 | REAL | — | Split on `\n`, blank-trim separator, final flush. `i as u32 + 1` overflows at 4.29e9 lines (unrealistic). |
| `next_synthetic` | 128-132 | REAL | — | u64::MAX descending, saturating — collision with real ids only if a real index is near u64::MAX (documented pathological). |
| `finish_block` | 141-161 | REAL | S2 (F-001) | The resync loop; quadratic on blank-less documents. Damage line number points at the resync target, not the stray content (cosmetic). |
| `enum Resync` | 164-169 | REAL | — | Two-state control flow, fine. |
| `parse_block` | 173-255 | REAL | S4 (F-004) | Core slice parse. Double `StrayLinesBeforeTiming` convention (once here at `lines[0].0` for tpos>1, once in finish_block at the resync target); numeric-index adoption heuristic :231-239; `unwrap_or(CueInterval::EMPTY)` at :252 verified infallible. |
| `resync_after` | 261-268 | REAL | S2 (F-001) | Scans to block end every time — the quadratic driver. |
| `looks_like_timing` | 272-274 | REAL | — | Probe with a throwaway sink; damage recorded exactly once by the later real parse (verified: the probe's sink is dropped). |
| `is_index_like` | 277-280 | REAL | S4 (F-004) | Duplicate of `parse_index` minus u64 fit. |
| `parse_index` | 283-288 | REAL | — | Correct; empty/all-digits/fit checks. |
| `struct Timing` | 291-294 | REAL | — | Plain data. |
| `parse_timing` | 299-332 | REAL | — | First-`-->` split; last-token start (leading junk tolerated as misnamed `TrailingJunk`); first-token end + positional `X1:`/`Y1:` discrimination. Sound. |
| `parse_time_token` | 336-386 | REAL | S3/S4 (F-002, F-003) | Field-ms semantics (code right, doc wrong); checked arithmetic throughout; saturation only inside the u64 domain. `rfind([',','.'])` separator choice sound (a '.' inside HH:MM:SS fails the digit check anyway). |

## 5. Test honesty (10 tests, lines 400-566)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `canonical_two_cue_document_parses_clean` | Yes | Exact ids, µs times, text equality, clean ledger. |
| `crlf_and_blank_whitespace_lines_separate_blocks` | Yes | CRLF + whitespace-only separators + missing final newline flush. |
| `index_line_is_optional_and_multiline_text_is_verbatim` | Yes | Synthetic ids (MAX, MAX−1) + verbatim leading-space text. |
| `timing_tolerances` | Yes | µs-exact asserts for `.`-separator, glued `-->`, 1-digit ms, missing ms, >23 h, rolled minutes. Would catch a decimal-fraction "fix" (the doc-vs-code conflict is one bad edit away). |
| `trailing_junk_and_position_payloads_are_tolerated` | Yes | Damage-kind + line-number asserts. |
| `malformed_blocks_are_skipped_without_aborting` | Yes | Garbage block + stray lines; surviving cues asserted. |
| `reversed_and_empty_and_bad_indexes` | Yes | Four damage kinds pinned with counts. |
| `saturation_and_long_milliseconds` | Yes | i64::MAX clamp + LongMilliseconds kind. Does NOT cover u64-overflow hours (F-003 gap). |
| `byte_entry_decodes_utf16_and_reports_encoding` | Yes | BOM UTF-16LE roundtrip + empty/whitespace-only inputs. |
| `stray_text_before_timing_is_kept_as_text` | Yes | Resync + index adoption + literal `-->` in text staying text. Would fail if resync broke. |
| Coverage gaps (mutation reasoning) | — | No test: (a) any performance/size property — a quadratic walker passes everything (F-001); (b) u64-overflow hours (F-003); (c) damage-ledger cap behavior from the parser side (owned by damage.rs tests). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "status = implemented (win/linux/macos)" | matrix row `subtitle-srt` (feature-matrix.toml:902); sub-srt.md:3 | UNREACHABLE (implemented-at-crate, unreachable-in-product) | Code+tests real (this audit + no-fail-fast baseline: lib 267/0, differential 6/0); zero shipped callers (Reachability row); sub-srt.md:88-90 admits the `SubtitleParser` seam is not wired; "Phase-6 sidecar autoload … consumes this parser" is a next step, not a present fact. |
| "parser is one-pass O(n)" / "one pass" / "Parse: one pass, output ≤ O(input)" | sub-srt.md:92; parser.rs:34-36; srt/fuzz.rs:27-29 | FALSE | F-001: quadratic resync on blank-less documents. |
| "1–3 digit ms FIELD semantics (verified against ffmpeg 7.1.5 output)" | sub-srt.md:46-47 | REAL | Code :358-371 + test :446-452; the record is honest — parser.rs's own module doc (:20-21) is the outlier (F-002). |
| "run-together blocks RESYNC at next valid timing line (iterative — stack bounded, D-2)" | sub-srt.md:50-51 | REAL (stack) / FALSE (cost) | Stack bounded yes (explicit loop); time quadratic (F-001). |
| "garbage blocks skipped to next boundary; `X1:`/`Y1:` payloads junk-ignored-with-damage" | sub-srt.md:48-50 | REAL | parse_block :180-183, parse_timing :314-330; tested. |
| "It **never** returns an error, … **never** panics" | srt/mod.rs:27-31 | REAL | No unwrap/expect/panic/index-out-of-range path found in the walk (all `?`-based); fuzz-survival green (no-fail-fast baseline). |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable from shipped entry points (see Reachability).
- `is_index_like`/`parse_index` duplication (F-004).

## 8. Dependencies & flags
- No external deps (crate is `media-types`-only; this file uses only std + crate-internal modules). No feature gates, no unsafe, no `#[allow]`. Clippy-clean per baseline #5.

## 9. Open questions for the next agent
- To the `encoding.rs` owner: the parser trusts `normalize_line_endings` for CR handling — if CR-only files normalize wrong, `walk`'s `\n` split breaks blocks; the CRLF test covers `\r\n` but I found no CR-only test here (may live in encoding.rs's suite).
- REQUEST-ORCH-RUN (perf proof for F-001): generate a blank-line-stripped SRT of 100k cues (`python3 -c` one-liner producing "1\n00:00:01,000 --> 00:00:02,000\ntext\n" ×100000 with newlines-only separators) and time `cargo test -p subtitles --release` won't show it — better: a 5-line `examples/`-style timing via `cargo run --release -p subtitles --example ...` is not permitted to add; ask orchestrator to time `fuzz_sub_srt` on that byte string. Expect superlinear scaling vs a 10k-cue control.
