# apps/cli/src/main.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 23 (1 function) |
| File verdict | **REAL** |
| Reachable from shipped entry point | YES — it IS the shipped entry point (`lumen` bin target, apps/cli/Cargo.toml `[[bin]] name = "lumen" path = "src/main.rs"`), verified by the orchestrator's executed runs against `./target/release/lumen` (HEAD 727ad6b2) |
| Findings | F-001 [S4], F-002 [S4] |

## 1 One-paragraph verdict

A genuinely thin `main`: seeds the bench wall-clock origin, collects argv, detects terminal-ness of stdin, delegates everything to `lumen_cli::run::run`, flushes, converts the returned `u8` to `ExitCode`. No hidden logic, no argument interpretation of its own, no panics. This is the honest shape the module doc claims ("thin `main` — the whole player is the library"). The only blemishes are the swallowed flush errors, which cost nothing observable here because the process is exiting and the OS flushes on exit.

## 2 What actually works (verified)

- The full argv→open→demux→decode→present chain runs through this entry point: orchestrator run `printf 'advance 512\ntime\nquit\n' | ./target/release/lumen open audit/runs/baseline/sample-h264-aac.mp4` produced `ok: time 3.041814s (100.0%)`, `Playing → Ended`, `EXIT=0` (audit/runs/baseline/mp4-smoke-open-advance.txt).
- `mark_process_start()` (line 13) is the wall origin `decode-bench`'s `wall_s` reads — the orchestrator's bench line shows `wall_s=0.000` because nothing decoded (see bench.rs audit), but the seeding call is real and first-statement as documented.
- Exit-code fidelity: I ran `./target/release/lumen engine-info` → `EXIT=0` with the `ok: engine` envelope; exit 86 (`WEBVIEW_PENDING`) is exclusively apps/desktop (apps/desktop/src/main.rs:21 `pub const WEBVIEW_PENDING: u8 = 86;`), NOT this crate — `lumen` exits only through the sysexits table (exit.rs:44-70: 0/64/65/66/69/70/71/74/77), and the totality is pinned by exit.rs's `every_error_code_maps_to_an_exit_code` test in the 1001-pass suite.

## 3 Findings

### F-001 [S4] main.rs:20-21 — flush errors discarded
Where: apps/cli/src/main.rs:20-21
```rust
    let _ = out.flush();
    let _ = err.flush();
```
Wrong: a failed stdout flush (ENOSPC, closed pipe) is silently dropped and the process still answers the command's own code — a truncated transcript on a full disk reports "success". Why it matters marginally: every other failure in the crate is typed and printed; this one seam is `let _`.
Correct: map a flush failure to `EX_IOERR` (74) the way `run::run`'s writer paths already print `control.internal` on serialization failure.
Confidence: high. Verification: `sed -n '19,22p' apps/cli/src/main.rs`; the code path is 4 lines total. Severity S4 because the process is exiting regardless and the OS performs an exit-time flush of the locked handles.

### F-002 [S4] main.rs:15 — `interactive` decided by stdin terminal-ness only
Where: apps/cli/src/main.rs:15
```rust
    let interactive = std::io::stdin().is_terminal();
```
Wrong: the `lumen> ` prompt is shown iff stdin is a TTY; a user piping a script gets no prompt (correct) but a user running `lumen open f.mp4` inside a terminal session whose stdin is redirected still loses the prompt while `repl_mode` still reads lines. Cosmetic asymmetry with `interactive` otherwise unused (run.rs:219 `let _ = interactive;`).
Confidence: high (code is 23 lines); impact nil — S4.

## 4 Per-function table

| Function (line) | Role | Verdict |
|---|---|---|
| `fn main() -> ExitCode` (9-23) | seed bench origin; argv; stdin-terminal detect; delegate to `run::run`; flush; map code | REAL — verified via executed binary runs above |

## 5 Test honesty

No test targets `main.rs` directly (it is 4 statements of delegation); every mode is covered in-memory through the `run::run` twin (`run.rs:897-908` `run_lines` helper's doc: "the test twin of `main`"). The conformance suite drives the REAL binary via `CARGO_BIN_EXE_lumen` (apps/cli/tests/container_conformance.rs:35), which is `main.rs` itself — the 1001-pass suite (incl. `mp4_h264_family_lane`, container_conformance.rs:426) exercised this entry point end-to-end.

## 6 Doc claims

- docs/feature-matrix.toml:152 (container-mp4 notes): "2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane — mp4_h264_aac.mp4 plays to Ended (exit 0, apps/cli mp4_h264_family_lane)" — CONSISTENT with the executed smoke (same shape: tracks 2 ready, Playing → Ended, exit 0).
- docs/feature-matrix.toml:428 (codec-video-h264-software evidence) cites "engine e2e apps/cli mp4_h264_family_lane (mp4_h264_aac.mp4 plays to Ended, exit 0)" — CONSISTENT.
- No row pins `main.rs` itself to a claim; no lie found.

## 7 Dead/duplicated/unreachable

None. `bench::mark_process_start` is called unconditionally even for non-bench invocations (one `OnceLock::set`, no cost).

## 8 Dependencies & flags

`lumen` (this bin) is built by the release lane with no feature flags (audit/runs/baseline/NOTES.md: "release.yml builds `-p lumen-cli -p desktop` with no feature flags"), so the codecs crate resolves its `default` features — crates/codecs/Cargo.toml:47-53:
```
default = [
    "codec-aac",
    ...
    "codec-h264-sw",
```
`codec-h264-sw` IS in the shipped default set (line 53), and apps/cli/Cargo.toml declares `codecs = { path = "../../crates/codecs" }` with no `default-features = false`. The `H264SwDecoder` path is compiled into the shipped binary — proven empirically by the MP4 smoke decode.

## 9 Open questions

- Would `ExitCode::from(u8)` ever see a code >77? The exit.rs table's totality test pins 18 `ErrorCode`s into the 8-value sysexits set; no path mints anything else. UNVERIFIED beyond the table test claim (I did not re-run the suite).
