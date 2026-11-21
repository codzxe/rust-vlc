# AUDIT: crates/input/src/lib.rs (whole file, 76 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 0 functions — 6 module declarations, 6 re-export groups, 2 cfg gates, 1 module doc (61 lines) |
| File verdict | REAL (reachable facade of the shipped open path; executed in baseline) — with one S1 at the composed system boundary that this file's doc understates (F-001) |
| Reachable from shipped entry point? | YES. Call chain: `lumen` main.rs:14 → `lumen_cli::run::run(argv)` → open verb → `apps/cli/src/demux_host.rs:394` `input::FileAccess::new()`, `:419` `input::resolve_file(&parsed)`, `:422` `input::FileAccess::new()` → `crates/input/src/file.rs:181` `impl Access for FileAccess`. Executed: the baseline MP4/AVI open sessions (INDEX.md CORRECTION v2: sample-420.mp4 Buffering→Playing→Stopped→Idle; AVI first-vertical lifecycle) run through this exact lane; input lib unittests 33/33 `ok` (`orchrun-test-workspace-nofailfast.log.txt:2603-2640`); release build compiles input (`build-release-workspace.log.txt:213`). |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 0 |
| Commands run | Read tool over the whole file (verbatim quotes); `rg -n "input" apps/cli/Cargo.toml crates/demux/Cargo.toml` (consumers: CLI dep :54, demux dev-dep :39); `rg -n "input::\|FileAccess\|resolve_file" apps/cli/src/` (call sites demux_host.rs:394/419/422); `rg -n "impl.*Access for FileAccess" crates/input/src/file.rs` (:181); `rg -n "DEFAULT_BUFFER_BYTES\|MAX_BUFFER_BYTES\|MAX_READ_BYTES\|Truncated" src/buffered.rs` (constants 64 KiB / 16 MiB / 64 MiB + truncation doc); `sed -n '85,130p' src/buffered.rs` (`buf: vec![0u8; capacity]` — allocated once); `rg -n "fn register" crates/input/src/` (none — "no registry hook" true); `rg -n "pub fn" src/error.rs` (constructors take no strings — secrets claim); `rg -n "scheme" src/uri.rs` + `:58-59` (`scheme_not_served` typed rejection); existence grep for all 6 re-export targets (all found); `rg -rn "features.*contract"` outside the crate (no external enabler); `rg -n "F-2" docs/audits/error-surface-20261023.md` (:236, :285 — citation resolves); **EXECUTED the prebuilt baseline binary** `/home/z/my-project/target/release/lumen` with a non-UTF-8 argument and a UTF-8 control (F-001's runtime proof, pasted below) |

## 1. One-paragraph verdict
The rare facade that tells the truth: every module it names exists with the role described, every load-bearing behavioral claim (fixed never-growing buffer, large-read window bypass, typed ResourceLimit ceiling, typed truncation, typed non-file-scheme rejection, no registry hook, secrets-free error constructors, cfg-gated conformance vocabulary) checks out against the module sources and against the 33/33 baseline lib tests that pin them — and the whole lane is live in the shipped `lumen` open path. The one defect is at the composed boundary this file's own doc gestures at: "Non-UTF-8 filesystem names are a recorded v0 limitation" — in the shipped binary that limitation is not a typed rejection but a **panic in the first line of main** (`std::env::args()` unwraps non-UTF-8 argv), which I executed for proof: the binary dies with exit 101 before the crate's beautifully typed `not_utf8_path` / demux_host "the path is not valid UTF-8" refusals can ever run, and it contradicts main.rs's own "Std-only, no panics: every failure is a typed CommandError" header. The crate did its part; the CLI entry point voids it.

## 2. What actually works (verified)
— Facade completeness — verified: all 6 declared modules exist (`buffered`, `contract`, `error`, `file`, `seekable`, `uri`); all 6 re-export targets exist (`BufferedReader`/`DEFAULT_BUFFER_BYTES`/`MAX_BUFFER_BYTES`/`MAX_READ_BYTES` buffered.rs:43/49/55, `ScriptSource`/`SyntheticStream` contract.rs:69/163, `FileAccess`/`FileSource`/`FileStream` file.rs:39/202…, `RawRead`/`Seekable` seekable.rs:65/37, `resolve_file` uri.rs:49). The crate compiles as a dependency of the shipped CLI (baseline build :213).
— Shipped-lane execution — verified: `input::FileAccess::new()` + `input::resolve_file` at demux_host.rs:394/419/422; the baseline open sessions (INDEX.md CORRECTION v2) run this lane end to end.
— "buffer … allocated once and never grows" — verified: `BufferedReader::new` (buffered.rs:97): `buf: vec![0u8; capacity]` once at construction; `buffer_capacity()` documents "never changes after construction"; capacity bounds are typed errors (85-89).
— "requests at or above the capacity bypass the window — leftover read-ahead drains first … one transfer" — verified: baseline tests `bypass_reads_serve_leftover_read_ahead_first`, `large_reads_bypass_the_window_in_one_source_call` `ok` (nofailfast log 2605/2609).
— "requests above a hard ceiling are refused with a typed ResourceLimit error" — verified: `MAX_READ_BYTES = 64 MiB` (buffered.rs:55); `read_requests_over_the_ceiling_are_typed_limit_errors` `ok` (2617).
— "Truncation is a typed error, never a silent EOF" — verified: `shrink_mid_read_surfaces_typed_truncation` `ok` (2618).
— "Network protocols … rejected here with typed errors" — verified: uri.rs:58-59 `if uri.scheme().as_str() != "file" { return Err(scheme_not_served(uri)); }`.
— "No registry hook yet" — verified: `rg "fn register"` over the crate → no hits.
— "Secrets never enter errors: constructors take no path or URI text" — verified: error.rs `pub fn` signatures are `missing()`, `permission()`, `not_a_regular_file()`, `truncated(u64, u64)`, `seek_*()`, `remote_host()` — no string parameters.
— contract gating — verified: `#[cfg(any(test, feature = "contract"))]` on both the module (64-65) and its re-exports (72-73); the `contract = []` feature exists (Cargo.toml); no crate/script/CI outside input enables it; the release baseline build (default features) carries none of it; the F-2 citation resolves (docs/audits/error-surface-20261023.md:236 "Residual observation (rank F-2)").

## 3. Findings

### F-001 [S1]
Where: crates/input/src/lib.rs:55-58 (the claim) + apps/cli/src/main.rs:14 (the defect)
Code (the claim):
```rust
//! - **Paths are UTF-8**: `file:` URI paths are percent-decoded and then
//!   required to be UTF-8, keeping the crate free of platform-specific
//!   `OsStr` byte APIs (ARCH-6). Non-UTF-8 filesystem names are a
//!   recorded v0 limitation.
```
Code (the shipped boundary):
```rust
    let argv: Vec<String> = std::env::args().skip(1).collect();
```
against main.rs's own header (main.rs:3-4): "Std-only, no panics: every failure is a typed `CommandError` printed by the one honest printer".
What is wrong: `std::env::args()` **panics** on any argument that is not valid Unicode (std-documented behavior; the unwrap lives at std env.rs:878). On Linux, filenames are arbitrary bytes — `lumen open $'\xff\xffbadname.mp4'` never reaches this crate's typed `not_utf8_path()` (uri.rs:92) or demux_host's typed refusal ("the path is not valid UTF-8", demux_host.rs:330-336); the process dies in the first line of main. I executed the prebuilt baseline binary to prove it:
```
$ /home/z/my-project/target/release/lumen open $'\xff\xffbadname.mp4'
thread 'main' (5413) panicked at /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library/std/src/env.rs:878:51:
called `Result::unwrap()` on an `Err` value: "\xFF\xFFbadname.mp4"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
EXIT=101
```
Control (UTF-8 path, same binary): `error: [input.inaccessible]: cannot resolve …/does-not-exist.mp4: No such file or directory (os error 2)` — the typed lane works and proves the contrast.
Why it matters: A media player must open files the OS and shell accept; legacy-encoded filenames are routine in real libraries. The shipped binary crashes (exit 101) on them — a panic on valid input, exactly the S1 class — and the crash voids three documents at once: main.rs's "no panics" header, this file's "recorded v0 limitation" framing (a limitation implies a typed refusal, and the typed refusal EXISTS but is unreachable from the real argv), and worklog archive-2026-10.md:10623(c)'s "bare-path arm passes OS strings natively" (false at the binary boundary: the bare-path arm receives `Vec<String>`; non-UTF-8 OS strings panic before it is called). In-process tests (`run(&argv)` with `Vec<String>`) cannot ever exercise this — the bug is only reachable through the real process entry.
What correct looks like: `main.rs` collects `std::env::args_os()`, and each non-UTF-8 `OsString` becomes the typed `CommandError` (Stage::Input, ErrorCode::Unsupported, "argument is not valid UTF-8") — or, better, the `OsString` flows into the bare-path arm, which already handles non-UTF-8 canonical paths typed at demux_host.rs:330-336; plus a spawned-binary integration test with a non-UTF-8 argv, and corrections to the three documents.
Confidence: HIGH — Verification: EXECUTED (prebuilt baseline binary, output pasted above; exit 101 captured) + read (both cited sites; std env.rs unwrap frame in the panic message).

## 4. Per-function table (structural units — the file has no functions)
| Unit | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `pub mod buffered` + re-exports | 63, 71 | REAL | — | Constants re-exported (64 KiB default / 16 MiB buffer ceiling / 64 MiB read ceiling); `BufferedReader` itself has no code consumer outside the crate (demux/src/avi/riff.rs:409 mentions it in a doc comment only; demux dev-tests consume `FileStream`/`SyntheticStream`) — library API staged for the pending registry, see §7. |
| cfg-gated `pub mod contract` + re-exports | 64-65, 72-73 | REAL | — | Both the module and its re-exports share the identical `#[cfg(any(test, feature = "contract"))]` — no way to get the module without the feature or vice versa; no external enabler exists; release binaries carry none of it (claim verified). |
| `pub mod error` | 66 | REAL | — | Constructors mint media-types errors with input context codes; no string params (secrets claim). |
| `pub mod file` + re-exports | 67, 74 | REAL | — | `impl Access for FileAccess` (file.rs:181); `FileAccess`/`resolve_file` are the names the shipped CLI consumes (demux_host.rs:394/419/422). |
| `pub mod seekable` + re-exports | 68, 75 | REAL | — | `Seekable`/`RawRead` traits; `resolve_origin_math_is_checked` test ok in baseline. |
| `pub mod uri` + re-export | 69, 76 | REAL | — | `resolve_file` — the CLI's file-URI lane; typed scheme rejection verified. |
| Module doc | 1-61 | REAL (11/12 claims verified; 1 understated) | S1 (F-001) | Claims ledger in §6. |

## 5. Test honesty (no tests in this file; the facade is compile-pinned by its consumers)
| Check | Would it fail if the facade were broken? | Reason |
|---|---|---|
| Removing/mis-naming any re-export | YES (compile) | `apps/cli` fails to compile (`input::FileAccess`, `input::resolve_file` at demux_host.rs:394/419/422); demux dev-tests fail (`FileStream`). The facade is pinned by consumer compilation, which the release baseline build (exit 0) proves end-to-end. |
| Flipping the contract cfg wrong (e.g. `feature = "contract"` without gate) | YES (compile, all-features) | Baseline #4 (`--all-features`) compiles the contract module for the crate's own tests — the gate's two halves (module + re-export) are syntactically identical, drift is impossible without editing both. |
| The 33 lib unittests | YES | They pin the module behaviors the doc claims (bypass, ceiling, truncation, origin math, URI shapes) — see §2 commands; all `ok` at nofailfast log 2603-2640. |
| Coverage hole | — | Nothing anywhere tests the real process argv boundary — F-001's panic is invisible to every in-process test by construction (`run(&argv)` takes `Vec<String>`). Needs a spawned-binary test. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "card 2.A1; doc 03 §4.1, doc 01 §2.4" (crate identity) | lib.rs:1 | REAL | Registry docs; assignments.tsv row structure; the crate implements what the card describes. |
| "Turns a validated SourceUri into a positioned, seekable byte stream over local sources: file: URIs opened through std::fs only" | lib.rs:3-5 | REAL | file.rs:181 `impl Access for FileAccess`; std::fs only (Cargo.toml: no external deps). |
| "Network protocols … rejected here with typed errors, never silently mis-opened" | lib.rs:5-7 | REAL | uri.rs:58-59 `scheme_not_served` typed rejection. |
| "Its buffer is allocated once and never grows" | lib.rs:17-18 | REAL | buffered.rs:97 `buf: vec![0u8; capacity]`; `buffer_capacity` "never changes after construction". |
| "requests at or above the capacity … bypass the window — leftover read-ahead drains first … one transfer" | lib.rs:20-23 | REAL | Baseline tests `bypass_reads_serve_leftover_read_ahead_first`, `large_reads_bypass_the_window_in_one_source_call` ok. |
| "requests above a hard ceiling are refused with a typed ResourceLimit error (MP §12)" | lib.rs:24-25 | REAL | MAX_READ_BYTES 64 MiB; `read_requests_over_the_ceiling_are_typed_limit_errors` ok. |
| "Truncation is a typed error, never a silent EOF (CorruptDetail::Truncated)" | lib.rs:48-51 | REAL | `shrink_mid_read_surfaces_typed_truncation` ok; buffered.rs:14 doc. |
| "No registry hook yet … waits on the registry itself" | lib.rs:52-54 | REAL | No `fn register` in the crate. |
| "Non-UTF-8 filesystem names are a recorded v0 limitation" | lib.rs:55-58 | SHALLOW (understates) | The crate-side lane IS typed (uri.rs:92 `not_utf8_path`; demux_host.rs:330-336 typed Unsupported) — but the shipped binary PANICS on non-UTF-8 argv before any of it runs (F-001, EXECUTED, exit 101). "Limitation" is not "panic". |
| "Secrets never enter errors: constructors take no path or URI text" | lib.rs:59-61 | REAL | error.rs constructor signatures — zero string parameters. |
| "contract … **Test/conformance-only** (audit F-2) … release binaries carry none of its panicky conformance vocabulary" | lib.rs:31-34 | REAL | cfg gates verified; F-2 citation resolves (error-surface-20261023.md:236); no external feature enabler; default release build excludes it. |
| "cancellation by generation staleness (not tokens)" (landed plugin_api traits) | lib.rs:45-46 | REAL-UNPROVEN (from this seat) | The traits exist and the impl compiles (file.rs:181); staleness semantics are plugin-api's audit to execute. |

## 7. Dead / duplicated / unreachable code
- Nothing dead in the facade; every module is consumed (file/uri by the shipped CLI; buffered/seekable/contract by the crate's own layers and tests; error by all).
- `BufferedReader` + the three buffer constants are re-exported public API with no external code consumer today (riff.rs:409 is a doc-comment mention; demux dev-tests use `FileStream`) — staged for the pending plugin-registry/conformance composition (documented at lib.rs:52-54); noted, not a finding.
- The 7 production `unreachable!` outside the gated contract (uri.rs ×1, error.rs ×2, buffered.rs ×4 — counted by `rg -c`) are the module auditors' (A01-A03/A07/A08 this wave) to classify; the facade's gating claim only covers contract.rs and is true as scoped.

## 8. Dependencies & flags
`media-types` + `plugin-api` only (the ARCH-2 set the Cargo.toml header declares); dev-dep `testkit` (dev-only by MP §3.7 — verified in `[dev-dependencies]`). One feature (`contract = []`), consumed only by the crate's own cfg. std-only, zero unsafe, `[lints] workspace = true`. The crate is the cleanest dependency graph in my domain: no feature-flag traps, no cfg(target_os) games.

## 9. Open questions for the next agent
1. **CLI owner (urgent, S1):** main.rs:14 → `args_os()` + typed refusal (or OsString through to the existing typed arm at demux_host.rs:330-336); fix main.rs:3-4 "no panics" header; add a spawned-binary non-UTF-8 argv test. My executed repro is above — cite it, don't re-derive.
2. Worklog correction: archive-2026-10.md:10623(c) "bare-path arm passes OS strings natively" is false at the binary boundary (it receives `Vec<String>`; the panic precedes it).
3. Module auditors this wave: classify the 7 production `unreachable!` (uri 1 / error 2 / buffered 4) vs the Cargo.toml comment "no unwrap/expect/panic" (that manifest was W01-A10's file).
4. media-types auditor: `SourceUri::from_file_path(&str)` (ids.rs:191) makes the UTF-8 requirement structural — if the CLI ever adopts `args_os`, the typed path needs an `OsStr`-accepting constructor or the demux_host bare-path arm stays the only non-UTF-8-aware door.
