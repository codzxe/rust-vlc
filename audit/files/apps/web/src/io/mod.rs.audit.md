# AUDIT: apps/web/src/io/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 76 / module declaration + 4 re-export groups (no functions) |
| File verdict | PAPER-ONLY as wiring — the declared pillar is real, tested code that NO production path uses, and its evidence label cites a test that does not exist |
| Reachable from shipped entry point? | NO — zero references to any io item outside io/ itself (grep sweep); lib.rs:65 declares `pub mod io;` and re-exports nothing from it |
| Findings | S0: 1 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `cargo test -p lumen-web --lib 'io::'` -> 49 passed (34 io + 15 audio substring-matches, verified via `-- --list`); grep sweeps for StagedSource/detect_seekability/FetchPort/FetchRequest/seekability/staged/fetch_port/mock_fetch outside io/ -> 0 production hits; sed verification of quotes |

## 1. One-paragraph verdict
This 76-line declaration file fronts a four-module "web I/O pillar" that is
genuinely implemented and unit-tested (34 tests pass: seekability matrix,
window aging, URL validation, mock transport) — and wired to NOTHING. No file
outside io/ references StagedSource, detect_seekability, FetchPort, or
FetchRequest; lib.rs re-exports none of it; the open path refuses http/https
with a stale "network streaming is card 7.H4" string while this — card 7.H4's
machinery — sits dead in the same crate. Worse, the module's evidence label
claims "the fetch-lane playback proof over the REAL first-vertical fixture";
no such test exists anywhere in the repository (the only real-fixture test is
the BLOB-path test at open.rs:410, which never touches the fetch lane). Every
io test runs on synthetic mock bytes.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p lumen-web --lib 'io::'` -> all pass
(34 io tests: staged_source 9, fetch_port 4, seekability 11, mock_fetch 10 —
the 49 total includes 15 `audio::` tests matched by the substring).
— verified: mock_fetch is genuinely test-only (`#[cfg(test)] pub mod
mock_fetch;`, mod.rs:66-67); the re-exported names all exist (crate compiles).

## 3. Findings
F-001 [S0]
Where: apps/web/src/io/mod.rs:56-63
Code:
```
//! # Evidence labels
//!
//! Everything here is proven headless-native against the mock double
//! (the seekability matrix, the bounded-memory proofs, and the
//! fetch-lane playback proof over the REAL first-vertical fixture).
```
What is wrong: The first two proofs exist (seekability's 11 tests; window
aging/bounded-memory tests). The third — "the fetch-lane playback proof over
the REAL first-vertical fixture" — does not exist. Exhaustive sweep: no file
in apps/web/src/io reads any real fixture (no include_bytes, no fixture path;
all test bodies use mock_fetch's synthetic `&[u8]` routes); the only
"REAL first-vertical fixture" test in the crate is
open.rs:410 `open_first_vertical_fixture_harvests_facts`, which exercises the
BLOB open path and never touches the fetch lane; no other crate references
StagedSource at all. The label cites a fabricated test as evidence. Why it
matters: an evidence string that names a nonexistent proof is exactly the
class of claim this audit exists to catch — a reader (or the parity/matrix
machinery) would believe fetch-lane playback is proven; nothing proves it,
and the lane is not even wired (F-002). What correct looks like: the label
naming the real tests, or a real integration test playing the fixture through
StagedSource once the lane is wired.
Confidence: HIGH — Verification: ran (grep sweeps + --list + test run)

F-002 [S2]
Where: apps/web/src/io/mod.rs:65-69; apps/web/src/lib.rs:65; apps/web/src/open.rs:183-186
Code:
```
pub mod fetch_port;
#[cfg(test)]
pub mod mock_fetch;
pub mod seekability;
pub mod staged_source;
```
and (open.rs:183-186, the http refusal):
```
                "http" | "https" => Err(unsupported_for_host(
                    scheme,
                    "network streaming is card 7.H4 (fetch + Range seekability detection)",
                )
```
What is wrong: the whole pillar is dead code. grep for
StagedSource/detect_seekability/FetchPort/FetchRequest across every non-io
file in apps/web/src returns nothing; host.rs, source.rs, open.rs never touch
it; lib.rs declares the module but re-exports nothing (contrast lines 71-76,
which re-export audio/capabilities/host/protocol/render/source). Meanwhile
open.rs refuses http/https with "network streaming is card 7.H4" — a refusal
reason that describes the very code sitting unwired one directory over, and
capabilities.rs:272-277 reports http-streaming `NotStarted` with the same
stale reason. Why it matters: a complete, tested network-streaming
implementation ships as dead weight while the user-visible behavior is
"unsupported"; the refusal strings and capability rows lie about what exists.
What correct looks like: open.rs routing http/https through StagedSource (the
mod doc already describes exactly that wiring), or the pillar marked
unwired/future and the refusal strings kept honest.
Confidence: HIGH — Verification: ran (grep sweeps)
UNVERIFIED: whether the dead tree survives wasm DCE into the shipped
lumen_web.wasm (would need a built wasm + symbol inspection; cargo-build not
run for wasm target here — rc-smoke baseline only checks size/magic).

F-003 [S3]
Where: apps/web/src/io/mod.rs:22-29
Code:
```
//! - [`staged_source`] — the [`SourceStream`] the engine reads
//!   network media through: fetch responses are **staged** into
//!   bounded windows (the message-transport adaptation — the browser
```
(including lines 27-29: "the pump gates engine polls on the source's readiness so a synchronous demuxer read never sees an unstaged byte")
What is wrong: present-tense wiring claims. No engine path reads network media
through StagedSource and no pump gates anything on its readiness (host.rs has
zero io references). The doc describes integration that does not exist.
Confidence: HIGH — Verification: read + grep sweeps

F-004 [S4]
Where: apps/web/src/io/mod.rs:49-54
Code:
```
//! `Range` is not a CORS-safelisted header: every ranged fetch on
//! cross-origin media triggers a preflight, and the server must
//! answer `OPTIONS` with `Access-Control-Allow-Headers: Range`. The
```
What is wrong: outdated browser fact. The Fetch standard safelists `Range`
(valid single byte-range form) — shipped in Chromium and Firefox since
2022/2023 — so in 2026 browsers a conforming single-range GET does NOT
trigger a preflight. The note concludes "this layer only records it in the
docs", so no code depends on the falsehood; doc staleness only. Doc-only
impact, S4.
Confidence: MEDIUM — Verification: read (spec knowledge; no browser in
sandbox to demonstrate — UNVERIFIED empirically)

## 4. Per-function table (structure, no functions in this file)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-63 | MIXED | S0/S3/S4 | architecture description accurate; evidence label fabricated (F-001); wiring claims false (F-003); CORS note stale (F-004) |
| mod declarations | 65-69 | REAL | S2 | modules compile and are tested; unwired (F-002) |
| re-exports | 71-76 | REAL | — | all names exist; consumed only by io's own tests |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| io's 34 tests (in child files) | YES (unit level) | seekability matrix covers 206/200/416/302; window aging asserts progress; mock routes include truncating/redirecting behaviors — all executed, all pass |
The tests are honest about the MODULES. The dishonesty is at the mod.rs level:
the evidence label (F-001) claims an integration proof that was never written.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the fetch-lane playback proof over the REAL first-vertical fixture" | mod.rs:58-60 | FALSE | no such test exists (sweep); only open.rs:410 tests the real fixture, on the blob path |
| "[`staged_source`] — the [`SourceStream`] the engine reads network media through" | mod.rs:22-23 | FALSE | zero consumers |
| "the pump gates engine polls on the source's readiness" | mod.rs:27-29 | FALSE | host.rs pump has no io references |
| "the wasm module has no imports and no sockets" | mod.rs:39-40 | REAL | worker.js:151,154 instantiates with an empty import object `{}` |
| "mirroring crates/streaming's ... ranged.rs" | mod.rs:6-8 | REAL-UNPROVEN | streaming's ranged.rs exists (another agent's file); semantic mirroring not verified here |
| "`Range` is not a CORS-safelisted header" | mod.rs:50-51 | OUTDATED | Fetch spec safelists single byte-range (F-004) |

## 7. Dead / duplicated / unreachable code
The entire declared pillar is unreachable from production: fetch_port.rs,
seekability.rs, staged_source.rs (mock_fetch is test-only by design). No
duplication found in this file.

## 8. Dependencies & flags
No new deps; child modules use plugin_api (SourceStream/SeekOrigin),
media_types. `#[cfg(test)]` correctly gates the mock. Crate is cdylib+rlib;
wasm lane only.

## 9. Open questions for the next agent
- open.rs auditor: the stale refusal strings (open.rs:184, capabilities.rs
  "NotStarted" for http-streaming) — if the matrix/parity rows cite them as
  evidence, this escalates F-002.
- Whoever owns worker.js/ABI: does the JS glue even have a fetch-serving path
  for FetchRequest messages (the io doc says bytes cross as FetchResponse
  messages)? worker.js's only `fetch(` is the module self-load (line 134) —
  there is no FetchRequest-serving loop, so the pillar is dead on BOTH sides
  of the wasm/JS boundary.
