# AUDIT: crates/input/src/file.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 280 / 18 rows |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/demux_host.rs:394` `input::FileAccess::new().open(&parsed)` (file lane of the shipped open path) → `FileAccess::open` (file.rs:186) → `FileSource::open` (file.rs:55) → `std::fs::File::open`). Verified by rg over apps/ (demux_host.rs:394, :422) and prior W-audit of crates/input/Cargo.toml. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | `rg -n "FileAccess|FileStream|FileSource"` (caller map: CLI demux_host, tools, many test crates); read `apps/cli/src/demux_host.rs` http/file lane split; read `crates/input/tests/file_stream.rs` (open-path coverage) + `truncation.rs` header; `rg -n "nix|libc|O_NONBLOCK"` over crates/input (absent — std blocking open only); grep nofailfast + all-features baselines for input test binaries (:2603–2672 nofailfast; :2471–2481 all-features fail-fast stop); `sed -n` on every quoted range. |

## 1. One-paragraph verdict
The file source is real, wired into the shipped CLI, and its open path is properly tested (9/9 `file_stream.rs` green in the no-fail-fast baseline :2653–2666: missing file, directory, `/dev/null`, wrong scheme, permission-denied, growing file, empty file, oversized read, read_into). The seek bounds and window behavior are covered by `bounded_reads.rs` 5/5 (:2642–2651) and the lib's 33/33 (:2603–2641). Two defects: (1) the module doc promises "FIFOs … are typed rejections at open time", but `std::fs::File::open` on a FIFO with no writer BLOCKS INDEFINITELY before the metadata check ever runs — a named-pipe MPEG-TS open (a standard media-player use case) hangs the CLI's synchronous open path; the test suite has a device-node and a directory test but conspicuously no FIFO test. (2) The crate's card-2.A1 truncation acceptance suite (`tests/truncation.rs`, 6 real-file `set_len` tests) is gated behind the non-default `contract` feature and has executed in ZERO recorded baseline runs — the default runs compile it to a green "running 0 tests" binary, and the only all-features run on record is fail-fast and died before reaching the input crate's binaries.

## 2. What actually works (verified)
— Typed open rejections for missing/permission/directory/device/wrong-scheme: `file_stream.rs` 9/9 green (nofailfast :2653–2666) — exact error codes asserted (`input.file.not-a-regular-file`, `plugin.access.scheme-not-served`, …).
— Handle-metadata regular-file check (no open/stat race): read (L56–62), exercised by the directory + /dev/null tests.
— Seek bounds over a real file: `bounded_reads.rs::seek_positions_and_bounds_over_a_real_file` green (:2646); `Seekable::seek` refuses `to > len` typed (L87–92).
— Growing-file semantics (live `size()`, open-time snapshot as truncation reference): `growing_file_reads_past_the_open_time_length` green (:2660); synthetic-source truncation typing green every run (`contract::tests::shrink_mid_read_is_a_typed_truncation_error`, :2618).
— Read-ahead budget join (product `cache_demux_mib` → window under ceiling, zero refused): the file's own test green (:2627), plus `read_ahead_budget_opens_and_serves` (:2664).
— `read_into` zero-alloc override: `real_file_passes_the_read_into_contract` green (:2663); demux crate's `zero_alloc_read_chain` 10/10 green over this stack (all-features :2237–2251).

## 3. Findings
### F-001 [S2]
Where: crates/input/src/file.rs:7-10
Code:
```
//! Regular files only: directories, FIFOs and device nodes are typed
//! rejections at open time — a device like `/dev/zero` would be an
//! unbounded read (MP §12 bounds memory; doc 01 §2.4 "no unbounded
//! read-ahead").
```
What is wrong: The claim is false for FIFOs. `FileSource::open` reaches the typed rejection only AFTER `std::fs::File::open` returns (L56→L59–62), but a POSIX `open(O_RDONLY)` on a FIFO with no writer **blocks indefinitely** — the metadata check at L60 never runs. Opening `file:///tmp/somepipe` (a named pipe feeding MPEG-TS, a standard VLC-style use case) hangs the CLI's synchronous open path (CLI → demux_host.rs:394 → FileAccess::open → FileSource::open; the playback engine is "synchronous and thread-free" per pipeline.rs module docs L32–33, so nothing time-boxes the open). The typed rejection fires only in the case where a writer already holds the FIFO — and then the file is promptly rejected as "not a regular file", meaning the ONLY FIFO behavior this code can produce is: hang, or refuse. Never read.
Why it matters: A user-visible indefinite hang on a valid local path, where the docs promise a typed error. The test suite's omission is structural: a FIFO test without a writer would hang the test itself (there is a device-node test and a directory test — no FIFO test; rg over tests/ confirms).
What correct looks like: Open with `O_NONBLOCK` (via `OpenOptionsExt` on unix), check `metadata().file_type().is_fifo()` → typed rejection, then (if FIFO reading is ever wanted) revert to blocking mode with a connect timeout. At minimum, fix the doc and add an `is_fifo` rejection test using a writer-held FIFO.
Confidence: HIGH on the POSIX semantics and the code path (read); MEDIUM on the exact CLI hang UX (single-threaded open traced through demux_host/pipeline docs, not executed — no builds allowed).
### F-002 [S3]
Where: crates/input/tests/truncation.rs:15-16
Code:
```
//! Feature-gated (audit F-2): the suite drives the shared executable
//! contract in `input::contract`, which is test/conformance-only —
//! run with `--features contract` (CI's `--all-features` always
//! does).
#![cfg(feature = "contract")]
```
What is wrong: The card-2.A1 acceptance suite for this crate's headline fault behavior — a file that shrinks below its open-time length mid-read must surface typed `input.file.truncated` (the exact contract file.rs's module docs reference at L20–22: "the truncation reference stays the open-time snapshot inside the reader") — has executed in ZERO recorded baseline runs. Default workspace runs do not enable `contract`: the no-fail-fast log shows the binary compiling to nothing (`Running tests/truncation.rs … running 0 tests … ok`, :2668–2672) — a green line that tests nothing. The all-features run that would enable it is FAIL-FAST and died at `first-vertical evidence.rs` (:2471–2481) before ever reaching the input crate's test binaries (rg of that log: no input binary, no `truncation_below_the_cursor`/`file_truncated_mid_read` names). The doc's fallback claim ("CI's --all-features always does") is unverifiable on record — the only recorded all-features run provably did not.
Why it matters: The 6 real-file `set_len` tests (truncation.rs:52,75,121,140,161,193) are the only coverage of truncation landing at several positions relative to the reader cursor and window; everything green on record is either a synthetic source (`contract::tests::shrink_mid_read_is_a_typed_truncation_error`) or engine-level AVI fixtures (`class_b_truncated_open/midplay`, all-features :2463–2464). A regression in the real-file truncation path would pass every recorded gate.
What correct looks like: Run `cargo test -p input --features contract` in the gate (REQUEST-ORCH-RUN below), or drop the feature gate (the lib's `contract` module is already `cfg(any(test, feature))`, so the integration file costs nothing ungated in test builds).
Confidence: HIGH — Verification: read both baseline logs at the cited lines; test names enumerated from source.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1–22 | REAL | S2 | F-001: FIFO claim false (blocking open). |
| `FileSource` (struct) | 39–44 | REAL | — | file + tracked pos + fallback_len (open-time snapshot). |
| `FileSource::open` | 55–68 | REAL | S2 | F-001: blocks on unwritten FIFO before the regular-file check; handle metadata (no TOCTOU) correct. |
| `impl Seekable for FileSource::position` | 72–74 | REAL | — | Tracked, not polled (documented why). |
| `impl Seekable for FileSource::length` | 76–85 | REAL | — | Live stat with open-time fallback; stat-per-seek is negligible cost. |
| `impl Seekable for FileSource::seek` | 87–98 | REAL | — | `to > len` typed refusal; seek-to-EOF allowed; pos updated only on success. |
| `impl RawRead for FileSource::read` | 101–108 | REAL | — | Saturating pos bookkeeping; short reads propagate (0 = EOF). |
| `FileAccess` (struct) + `new`/`Default` | 116–129, 175–179 | REAL | — | Default window. |
| `FileAccess::with_buffer_capacity` | 136–144 | REAL | — | Zero/over-ceiling typed errors (tested in bounded_reads::access_capacity_bounds_are_typed). |
| `FileAccess::with_read_ahead_budget` | 161–167 | REAL | — | Budget→ceiling clamp, observable; zero refused; pinned by the file's own test. |
| `FileAccess::buffer_capacity` | 170–172 | REAL | — | Observability accessor. |
| `impl Access for FileAccess::accepts` | 182–184 | REAL | — | scheme == "file". |
| `impl Access for FileAccess::open` | 186–198 | REAL | — | scheme check → resolve_file → FileSource::open → BufferedReader (bounded window). |
| `FileStream` (struct) | 202–205 | REAL | — | Reader + uri. |
| `impl SourceStream::uri`/`size`/`supports_seek` | 207–218 | REAL | — | size from reader's source_length (live truth). |
| `impl SourceStream::read` | 220–222 | REAL | — | Allocating generic path (per-call Vec) — the non-hot lane. |
| `impl SourceStream::read_into` | 224–228 | REAL | — | Zero-alloc override (card 3.B3); contract-tested green. |
| `impl SourceStream::seek`/`position` | 230–238 | REAL | — | resolve_origin math (checked, seekable.rs tests green) + reader seek. |
| `tests::read_ahead_budget_maps_to_the_window_under_the_ceiling` | 252–279 | REAL | — | Budget join exact (64K/ceiling/32MiB-over/zero-refused). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| file_stream.rs::real_file_passes_the_stream_contract / …_read_into_contract | YES | Full contract over a real temp file (positions, EOF, sizes). |
| file_stream.rs::missing_file_is_typed_not_found | YES | Exact code + kind. |
| file_stream.rs::directory_is_a_typed_rejection | YES | Linux dir open succeeds → metadata rejection path proven. |
| file_stream.rs::device_node_is_a_typed_rejection | YES | /dev/null opens instantly → rejection path. |
| file_stream.rs::non_file_scheme_is_refused_typed | YES | Three schemes, exact code. |
| file_stream.rs::unreadable_file_is_typed_permission_denied | YES | Real chmod 000 (skipped as root — honest guard). |
| file_stream.rs::growing_file_reads_past_the_open_time_length | YES | The live-size promise. |
| file_stream.rs::empty_file_is_a_clean_immediate_eof | YES | EOF-at-0 corner. |
| file_stream.rs::oversized_read_requests_are_typed_limit_errors | YES | MAX_READ_BYTES enforcement. |
| file.rs::read_ahead_budget_maps_to_the_window_under_the_ceiling | YES | Exact window sizes incl. over-ceiling clamp. |
| (gap) FIFO open | NO TEST | F-001 — untestable synchronously without a writer; omitted entirely. |
| (gap) real-file truncation AC | NO TEST (gated off in every recorded run) | F-002. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "directories, FIFOs and device nodes are typed rejections at open time" | file.rs L7–10 | FALSE for FIFOs | F-001: blocking `File::open` precedes the check; no FIFO test. |
| "no open/stat race" (metadata from the handle) | file.rs L17–18 | REAL | L56–62 read; directory/device tests green. |
| "growing files report their current truth; the truncation reference stays the open-time snapshot" | file.rs L20–22 | REAL for the live-size half (test green); the real-file truncation half UNVERIFIED on record | F-002. |
| "never a silent surprise" (budget clamp observable) | file.rs L151–153 | REAL | buffer_capacity accessor + test. |

## 7. Dead / duplicated / unreachable code
None in this file. (`FileSource`/`FileStream` exported and used by CLI + 9 workspace dependents' tests/tools.)

## 8. Dependencies & flags
media-types, plugin-api (workspace); std only for I/O — no nix/libc (which is WHY F-001 exists: no `O_NONBLOCK` available without `std::os::unix::fs::OpenOptionsExt`, which is std and unused). The `contract` feature (`Cargo.toml:30`, empty) gates only the test/conformance surface (lib.rs:64–65,72–73). Input lib 33/33 + bounded_reads 5/5 + file_stream 9/9 green in nofailfast (:2603–2666); truncation.rs 0 tests (:2668–2672).

## 9. Open questions for the next agent
- REQUEST-ORCH-RUN: `cargo test -p input --features contract --release --locked` (executes the 6 real-file truncation AC tests that no recorded run has ever run — F-002).
- To the input crate's other auditors (buffered.rs/seekable.rs owners): `BufferedReader`'s truncation detection compares against the open-time snapshot — confirm the mid-refill and below-cursor shapes once the gated suite actually runs.
- To the CLI wave: whether any UX timeout wraps the synchronous open (F-001's blast radius is the whole CLI thread until then).
