AUDIT: apps/cli/tests/ttf_seek.rs
==================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 789 / 17 (3 measurement tests) |
| File verdict | SHALLOW (real harness, but its lane table documents a dead landscape and its corpus gate silently green-lights absence) |
| Reachable from shipped entry point? | NO — test-only; it *spawns* the shipped binary (`CARGO_BIN_EXE_lumen`, line 99), which is the right shape for a through-the-binary measurement. |
| Findings | S0:0 S1:0 S2:2 S3:1 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (2 chunks)
- `ls /tmp/my-project/corpus-v4/media` → does not exist in this sandbox (corpus gate provably no-ops here)
- `sed -n '45,55p;166,176p;349,355p' apps/cli/tests/ttf_seek.rs` (quote verification, passed)
- Grep of `audit/runs/baseline/cargo-test.log` for `ttf|corpus absent|lumen-cli test binaries` → **no match**: the W00 baseline test log contains no lumen-cli test-binary results at all, so there is NO recorded evidence these measurements ever ran (UNVERIFIED whether the baseline environment had the corpus).
- Cross-read: pipeline_host.rs:346 (h264 dispatch), ts.rs:34-53 + ts.rs:793-797 ("h264 must dispatch the software decoder"), pipeline_host.rs:1470-1488 (the 2026-11-15 four-pair color conversion).

## 1. One-paragraph verdict

This is the CLI's time-to-first-output and in-session-seek measurement harness, and mechanically it is well built: it spawns the real binary, timestamps real stdout lines, and asserts structural truths (receipt ordering, ±10% seek neighborhoods, no-reopen, typed refusals) rather than wall-clock numbers. But its "honest landscape" documentation and its lane table describe the tree as it was BEFORE the 2026-11-15 h264/color landings — the module doc claims "h264 is not dispatched by this host's decoder providers" (line 53-54), which pipeline_host.rs:346 and ts.rs:793-797 flatly contradict — and the mpegts/h264 lane still expects the old `cli.codec_unavailable` refusal. None of this is ever caught, because the harness silently returns PASS whenever the corpus directory is missing (it is missing in this sandbox), and the W00 baseline log contains no evidence the harness ever produced numbers.

## 2. What actually works (verified)

— verified by reading: the harness structure is real — `timed_session` (97-129) spawns `CARGO_BIN_EXE_lumen` with piped stdio and timestamps each line; the three tests assert ordering (`first_output > accepted`, 409-414), typed refusals by exact code (425-430), "a refusing lane must never claim first output" (431-436), ±10% landing windows (519-525), page-granular windows (564-570), single-open no-reopen counts (532-542), and the ts lane's typed exit 65 (692-707).
— UNVERIFIED at runtime: no corpus in this sandbox; no baseline log section for these tests. Everything below is read-verified only.

## 3. Findings

### F-001 [S2] — corpus absence = silent green: the measurement harness cannot fail when its subject is missing
Where: apps/cli/tests/ttf_seek.rs:166-176 (gate), 385-388 / 479-482 / 675-678 (the three no-op arms)
Code:
```rust
fn corpus_media() -> Option<PathBuf> {
    let root = std::env::var("LUMEN_CORPUS")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let default = PathBuf::from("/tmp/my-project/corpus-v4");
            default.is_dir().then_some(default)
        })?;
```
```rust
    let Some(media) = corpus_media() else {
        eprintln!("ttf-seek: corpus absent — measurement no-op");
        return;
    };
```
What is wrong: all three tests `return` (PASS) when the corpus is absent. An eprintln is not a test result. In this sandbox `/tmp/my-project/corpus-v4` does not exist (verified), and the W00 baseline `cargo-test.log` contains **no** lumen-cli test-binary section — so the recorded "1751 pass / 2 fail" total cannot show whether these measurements ran or no-opped anywhere.
Why it matters: a measurement harness whose PASS is indistinguishable from "did nothing" is the §3.4 "tests that never run" pattern. Worse, it is the only through-the-binary ttf/seek evidence surface for the CLI; its absence from the evidence chain is invisible.
What correct looks like: `#[ignore]` + an explicit env var to arm it, or `panic!("corpus required for this measurement")` so absence is loud; at minimum, assert in the baseline gate that the corpus was present.
Confidence: HIGH — Verification: read + `ls` (corpus absent here) + baseline log grep (no section).

### F-002 [S2] — stale lane table asserts a dead landscape: "h264 is not dispatched" is FALSE in the current tree
Where: apps/cli/tests/ttf_seek.rs:53-54 (doc), 349-355 (mpegts/h264 lane)
Code:
```rust
//! identification, or OpusHead), so ADTS/AAC and MKA/FLAC-ALAC
//! lanes open limb-less and refuse on the first audio frame; h264
//! is not dispatched by this host's decoder providers. The
```
```rust
        Lane {
            family: "mpegts/h264",
            path: corpus("0018-ts-video.ts"),
            plays: false,
            refusal: Some("cli.codec_unavailable"),
```
What is wrong: pipeline_host.rs:346 dispatches `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (its comment dates the landing 2026-11-15), and ts.rs:39-41 documents "h264 video dispatches the software decoder (Annex-B framing … no `avcC` seam in TS)" with ts.rs:793-797 asserting exactly that. The lane here still encodes `plays: false, refusal: Some("cli.codec_unavailable")` — the pre-landing behavior. If the corpus existed, this lane would either play (assertion at 431-436 "a refusing lane must never claim first output" would fire) or refuse differently (425-430 would fire on the wrong code).
Why it matters: the file's own header calls this "The honest landscape this pins (measured this session)" — the landscape is not the current tree's. Any future corpus-armed run fails confusingly, and any reader of this file (or of a worklog quoting it) is told h264 is undispatchable, feeding the false "no decoder" narrative the W00 bench message also pushes (bench.rs:334).
What correct looks like: re-measure the lane table after the 2026-11-15 landings; make the module doc carry the measurement date and the tree state it describes.
Confidence: HIGH — Verification: read (three files cross-checked; no run possible).
### F-003 [S3] — "present limb accepts exactly BT.601 full-range i420" is stale against the four-pair converter; vp9 refusal expectations suspect
Where: apps/cli/tests/ttf_seek.rs:48-50 (doc), 357-369 (ivf/vp9 + matroska/vp9 lanes expecting `cli.i420_color_unsupported`)
Code:
```rust
//! the pipeline-layer evidence: the CLI host's present limb accepts
//! exactly BT.601 full-range i420 (the MJPEG case), so every
//! vp9/av1 lane decodes but refuses at the renderer; the audio limb
```
What is wrong: pipeline_host.rs:1470-1488 extended `convert_i420_to_rgba` to all four BT.601/BT.709 × full/limited pairs ("the h264 software lane's landing … the four landed combinations convert now"), naming vp9's default as BT.709 limited (1464-1465). If rusty_vp9 tags its frames BT.709-limited, these vp9 lanes convert and play, and `cli.i420_color_unsupported` (line 360, 367) never fires.
Why it matters: same stale-landscape class as F-002; two more lanes whose expectations may be dead. UNVERIFIED which color tag the vp9 decoder actually stamps (crates/codecs is another wave's file; no run possible).
What correct looks like: re-measure; assert what the binary does today.
Confidence: MEDIUM — Verification: read (cross-file contradiction is certain; the actual vp9 tag is UNVERIFIED).
### F-004 [S4] — machine-specific hard-coded corpus path
Where: apps/cli/tests/ttf_seek.rs:171
Code:
```rust
            let default = PathBuf::from("/tmp/my-project/corpus-v4");
```
What is wrong: a developer-machine absolute path baked in as the default; portable only via the `LUMEN_CORPUS` escape hatch.
Why it matters: guarantees F-001's silent no-op on every other machine/CI.
What correct looks like: repo-relative corpus path or env-var-required.
Confidence: HIGH — Verification: read.

### F-005 [S4] — transcript parsing couples to Debug formatting of the commands crate
Where: apps/cli/tests/ttf_seek.rs:150-161
Code:
```rust
    fn first_int_after(&self, needle: &str, key: &str) -> Option<i64> {
        self.lines.iter().find_map(|(line, _)| {
            let at = line.find(needle)? + needle.len();
            let tail = &line[at..];
            let open = tail.find(key)? + key.len();
```
What is wrong: needles like `"position: Timestamp("` / `"duration: Known"` / `"time: Timestamp("` (513-517, 553-557) are Rust `Debug` output shapes of `media_types` types; any refactor of those types' Debug derive silently breaks the measurement parse (returns None → `must_or` panic with context, at least loud).
Why it matters: brittleness across crate boundaries in the evidence chain.
What correct looks like: machine-readable `--json` events (run.rs already supports them) instead of Debug-prose scraping.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Timed` (struct) | 85-89 | REAL | — | data. |
| `timed_session` | 97-129 | REAL | — | spawns real binary; stdin pre-buffered; line timestamps. NOTE: relies on Rust line-buffered stdout (comment 92-93) — true for `println!`/`writeln!` on piped stdout? Rust stdout is line-buffered ONLY when terminal; piped it is block-buffered... UNLESS the binary flushes per line. run.rs REPL flushes only the prompt (run.rs:223); `writeln!(out…)` on a locked stdout handle flushes per write since `StdoutLock` is line-buffered by `LineWriter` — Rust's std stdout IS LineWriter-backed regardless of tty. So the assumption holds. (Verified: std docs, UNVERIFIED by run.) |
| `Timed::contains` | 133-135 | REAL | — | substring scan. |
| `Timed::first_ms` | 139-144 | REAL | — | first-match offset. |
| `Timed::first_int_after` | 150-161 | REAL | S4 | F-005 Debug coupling; ASCII-digit scan only (negative payloads unparsable — all uses are non-negative). |
| `corpus_media` | 166-176 | REAL | S2/S4 | F-001/F-004. |
| `fixture` | 179-184 | REAL | — | CARGO_MANIFEST_DIR-relative. |
| `codec_fixture` | 188-193 | REAL | — | codecs-crate fixture tree (documented 6.OPW-ivf gap). |
| `SeekClass` (enum) | 200-215 | REAL | — | 4 classes. |
| `Lane` (struct) | 219-228 | REAL | — | data. |
| `lanes` | 231-371 | SHALLOW | S2/S3 | the stale lane table (F-002/F-003); 16 lanes, 3 of them encode pre-landing behavior. |
| `ttf_first_output_per_family_through_the_binary` | 383-457 | REAL-UNPROVEN | S2 | asserts receipt ordering + typed refusals; would fail on stale lanes IF the corpus existed (F-001). |
| `in_session_seek_per_family_class_through_the_binary` | 477-638 | REAL-UNPROVEN | S2 | ±10% / page-window / no-reopen asserts; same corpus gate. |
| `START_TIME_LANES` (const) | 647-663 | REAL | — | 10 lanes. |
| `start_time_pure_seek_open_path_watermarks` | 673-763 | REAL-UNPROVEN | — | landing window `target-0.05..target+1.0` (734-735); ts exit-65 pin (704-707); same gate. |
| `transcript` | 766-773 | REAL | — | message helper. |
| `MustOr` trait/impl | 777-788 | REAL | — | panic-with-context bridge (test-acceptable). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ttf_first_output_per_family_through_the_binary | ONLY IF CORPUS PRESENT | absent corpus → `return` PASS (F-001). With corpus: yes — ordering, refusal-code, and no-first-output asserts are real. |
| in_session_seek_per_family_class_through_the_binary | ONLY IF CORPUS PRESENT | ±10% window, no-reopen count, refusal-code asserts are real; stale lanes would misfire (F-002/F-003). |
| start_time_pure_seek_open_path_watermarks | ONLY IF CORPUS PRESENT | landing-window + exit-code asserts are real. |

The three tests also carry real mutation-killing power for the seek/ttf code paths they cover (host settle, seek composition, start-time skip) — the failure is the gate, not the assertions.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "h264 is not dispatched by this host's decoder providers" | ttf_seek.rs:53-54 | FALSE | pipeline_host.rs:346; ts.rs:39-41, 793-797. |
| "the CLI host's present limb accepts exactly BT.601 full-range i420" | ttf_seek.rs:48-50 | FALSE (stale) | pipeline_host.rs:1475-1488 converts four (matrix,range) pairs since 2026-11-15. |
| "MP4/MOV/M4A refuse typed at demux reanchor (the recorded 6.OPW-mp4 gap)" | ttf_seek.rs:66-67 | REAL-UNPROVEN | mp4.rs:173-178 forwards the trait-default `reanchor` refusal ("the moving half of in-session seek is its honest gap"); not run here. |
| "AVI/AIFF land; the WAV lanes land … OGG refuses typed at decode" | ttf_seek.rs:55-69 | REAL-UNPROVEN | consistent with mp4.rs/ogg.rs docs and the 6.FIX comments in pipeline_host; not run here. |
| "every number this harness prints is a WALL-CLOCK measurement of ONE `lumen open` process in the DEV profile" | ttf_seek.rs:9-14 | UNVERIFIED | no numbers exist in the baseline evidence (F-001). |

## 7. Dead / duplicated / unreachable code

- The lane entries for mpegts/h264, ivf/vp9, matroska/vp9 (349-369): expectations contradicted by or suspect against the current tree (F-002/F-003) — dead assertions as long as the corpus is absent.
- No duplication found.

## 8. Dependencies & flags

- Test target of lumen-cli; needs the built binary (`CARGO_BIN_EXE_lumen`) and the external corpus (`LUMEN_CORPUS` or `/tmp/my-project/corpus-v4`) plus codecs-crate fixtures. No feature gates. W00 baseline release build PASS does not cover this file's runtime behavior (no evidence section).

## 9. Open questions for the next agent

1. crates/codecs vp9 decoder — what (matrix, range) does it stamp? Decides whether F-003's vp9 lanes are dead expectations or live refusals.
2. tests/integration/first-vertical/tests/evidence.rs:47/162 — the W00 baseline's 2 failures live there, NOT in apps/cli; whoever owns that file should check whether the committed transcripts encode the same stale pre-2026-11-15 landscape.
3. Does any CI/workflow set `LUMEN_CORPUS`? (scripts/ wave should answer; if not, F-001 means these measurements have never run in CI.)
