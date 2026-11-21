# AUDIT: apps/web/src/io/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 8 items (module docs, 4 module declarations [1 test-gated], 3 re-export groups = 14 symbols) |
| File verdict | UNREACHABLE (the io tree compiles and is mock-proven, but no host/protocol/JS-glue path routes a single byte through it) |
| Reachable from shipped entry point? | NO — and stronger: unreachable even WITHIN the crate's own engine host. `rg "StagedSource\|staged_source\|detect_seekability\|FetchPort" apps/web/src --type rust` outside `src/io/` → 0 hits; `host.rs` imports nothing from io (its use-block at 102-108 lists audio/capabilities/open/protocol/render/source only); the host's open path refuses `http`/`https` with a typed "network streaming is card 7.H4" error (`apps/web/src/open.rs:182-186`) and serves only `blob:` streams from the registry (`open.rs:226-228`). The MessagePort protocol (protocol.rs, my audit) defines no FetchRequest/FetchResponse variants — the lane cannot cross the worker boundary either. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "crate::io::\{\|use crate::io\|crate::io::[A-Z]" apps/web/src` (0 — re-export block unused, F-003); `rg -n "\bio::\|StagedSource\|FetchPort\|detect_seekability" apps/web/src --type rust` (all hits inside src/io/ or std::io); `rg -n "cors\|preflight\|OPTIONS\|Access-Control" apps/web/src/io/{mock_fetch,fetch_port}.rs` (0 — F-002); `ls crates/streaming/src/ranged.rs` + `rg -n "RangedResource"` (mirroring claim's artifact is real); `rg -n "extern \"C\"" apps/web/src/abi.rs` (exports only, :348-399 — "no imports" claim verified); Read of open.rs:140-259 (http/https typed refusal, blob-only wiring); Read of staged_source.rs:1-60 (its own boundary claims); `rg -n "io::.*::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (34 green io test lines, :3125-3158, incl. `fetch_lane_serves_the_real_fixture_to_the_demuxer` :3156). |

## 1. One-paragraph verdict
The module header is a well-written architecture essay about a byte-transport lane that is not connected to anything: fetch_port, seekability, and staged_source are real, carefully bounded (window budgets, eviction rules, typed refusals) and mock-proven — 34 green tests including a full fixture-through-the-demuxer fetch-lane proof — but the engine host never constructs a StagedSource, refuses every http/https URL with "network streaming is card 7.H4" (open.rs:184), imports nothing from io, and the worker MessagePort protocol has no fetch message variants, so bytes cannot cross the wasm↔JS boundary through this lane even in principle today. Three of the header's present-tense claims describe integration that does not exist. Two smaller defects: the CORS paragraph says "the transport (browser **or mock**) enforces CORS" while the mock models no CORS whatsoever, and the 14-symbol re-export block at the bottom has zero consumers (even the io submodules import each other by direct path). The [VERIFY] evidence labels, to the file's credit, are scrupulously honest — and correctly imply the browser rows never ran.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3125-3158` → 34 io test lines green: `io::fetch_port::tests::*` (range header wire form, typed URL validation, error facts), `io::mock_fetch::tests::*` (11 — 206/200/416/redirect/truncation routes, budget enforcement), `io::seekability::tests::*` (10 — the full probe matrix incl. protocol-violation refusals), `io::staged_source::tests::*` (9 — incl. `fetch_lane_serves_the_real_fixture_to_the_demuxer` :3156, the "REAL first-vertical fixture" claim in the header's evidence label, proven against the mock).
— Verified by read: `mock_fetch` is correctly `#[cfg(test)]`-gated (66-67) — the test double cannot leak into the wasm artifact.
— Verified by `rg "extern \"C\"" apps/web/src/abi.rs`: only `pub extern "C" fn` exports (:348-399) — the header's "the wasm module has no imports and no sockets (doc 09 §6.3)" claim is true.
— Verified: `crates/streaming/src/ranged.rs` exists and defines `RangedResource` — the "mirroring" claim references a real, audited-sibling artifact.

## 3. Findings

### F-001 [S3]
Where: apps/web/src/io/mod.rs:22-29 (with open.rs:182-186, host.rs:102-108, protocol.rs:52-225)
Code:
```
//! - [`staged_source`] — the [`SourceStream`] the engine reads
//!   network media through: fetch responses are **staged** into
//!   bounded windows (the message-transport adaptation — the browser
//!   `fetch()` is async and lives in the Worker's JS glue, so bytes
//!   cross the boundary as `FetchResponse` messages correlated by
//!   request id; the pump gates engine polls on the source's
//!   readiness so a synchronous demuxer read never sees an unstaged
//!   byte).
```
What is wrong: Every present-tense integration claim in this bullet is false in the current tree. (1) "the SourceStream the engine reads network media through" — zero references to `StagedSource` outside `src/io/`; the host's `open` reads only registry blob streams (open.rs:226-228) and refuses http/https: `open.rs:182-186` — `"network streaming is card 7.H4 (fetch + Range seekability detection)"` — a refusal message that names THIS VERY CARD while this very card's implementation sits one module away, unwired. (2) "bytes cross the boundary as FetchResponse messages correlated by request id" — the worker protocol (protocol.rs `HostToWorker`/`WorkerToHost`, 17 variants, my audit) defines NO fetch message types; there is no boundary lane. (3) "the pump gates engine polls on the source's readiness" — host.rs's pump (its import block at 102-108) touches no io type. The lane exists as three self-consistent, mock-tested modules and nothing else.
Why it matters: A reader (or the next card) tracing "how does the web engine read network media" is told a working lane exists; the actual answer is: it doesn't — the web host opens blob: only, and the fetch components are an island. The ~1,200-line staged_source + fetch_port + seekability tree is dead weight in the wasm artifact until a wiring card lands, and the header misrepresents that state. (The components' own quality is NOT in question — 34 green tests; the defect is the claimed integration.)
What correct looks like: Header phrased in the file's own honest style: "the fetch lane components (card 7.H4): staged_source is the SourceStream the engine WILL read network media through once the host wiring card lands; today the host refuses http/https (open.rs) and the protocol carries no fetch messages" — plus an open-tracking reference to the wiring card.
Confidence: HIGH — Verification: read + rg census (0 external references; 0 protocol fetch variants; host import block enumerated; open.rs refusal quoted).

### F-002 [S4]
Where: apps/web/src/io/mod.rs:50-54
Code:
```
//! `Range` is not a CORS-safelisted header: every ranged fetch on
//! cross-origin media triggers a preflight, and the server must
//! answer `OPTIONS` with `Access-Control-Allow-Headers: Range`. The
//! transport (browser or mock) enforces CORS; this layer only
//! records it in the docs, because the wasm side never sees origins.
```
What is wrong: "The transport (browser or mock) enforces CORS" — the mock enforces nothing CORS-shaped: `rg "cors|preflight|OPTIONS|Access-Control"` over mock_fetch.rs and fetch_port.rs returns zero hits. The mock's routes model statuses, ranges, redirects, truncation — never preflights or origin checks. Only a real browser enforces CORS.
Why it matters: The parenthetical invites the reader to believe the mock doubles as a CORS harness; it does not. When 7.I1's browser matrix runs, preflight behavior will be exercised for the first time with zero prior art.
What correct looks like: "The browser enforces CORS (preflight on cross-origin Range); the mock does not model it — browser rows only (card 7.I1)."
Confidence: HIGH — Verification: read + rg (0 CORS references in the mock).

### F-003 [S4]
Where: apps/web/src/io/mod.rs:71-76
Code:
```
pub use fetch_port::{
    DEFAULT_CHUNK_BYTES, DEFAULT_WINDOW_BYTES, FetchError, FetchPort, FetchRequest, FetchResponse,
    RangeRequest,
};
pub use seekability::{Seekability, detect_seekability};
pub use staged_source::{MAX_READ_BYTES, StagedConfig, StagedSource, StagedStats, WindowRequest};
```
What is wrong: The 14-symbol re-export block has zero consumers. `rg "use crate::io::|crate::io::[A-Z]"` over apps/web/src finds no importer — host.rs, open.rs, source.rs, lib.rs use none of it, and the io submodules cross-import via direct paths (`crate::io::fetch_port::{…}`, mock_fetch.rs:54). The re-exports are a public API for the wiring card that has not landed.
Why it matters: Dead indirection that will rot: a rename in a submodule leaves the facade stale with no test noticing (nothing imports it). Minor — consistent with F-001's not-yet-wired state.
What correct looks like: Land the re-exports with the host-wiring card, or add a prelude test (the testkit_wiring.rs pattern plugin-api uses) that forces every re-export to compile-import.
Confidence: HIGH — Verification: read + rg import census.

## 4. Per-function table (76 lines, whole file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module docs | 1-63 | MISLEADING (integration claims) | F-001, F-002 | Component descriptions accurate (verified against submodule tests); integration claims false; [VERIFY] labels honest. |
| `pub mod fetch_port` | 65 | REAL | — | Compiles into the wasm module; mock-tested (3+ tests green); zero external callers. |
| `#[cfg(test)] pub mod mock_fetch` | 66-67 | REAL | — | Correctly test-gated; 11 tests green. |
| `pub mod seekability` | 68 | REAL | — | 10 tests green (probe matrix incl. protocol-violation refusals). |
| `pub mod staged_source` | 69 | REAL (unwired) | F-001 | 9 tests green incl. real-fixture fetch lane; zero host/protocol consumers. |
| Re-export block (3 groups, 14 symbols) | 71-76 | REAL (dead) | F-003 | Zero importers. |

## 5. Test honesty (no tests in this file; submodule evidence)
| Test group | Would it fail if the code were broken? | Reason |
|---|---|---|
| io submodule suites (34 green, :3125-3158) | YES — for the components | Each submodule's tests exercise real logic against the mock double (the honest pattern: the mock implements the same FetchPort vocabulary; `fetch_lane_serves_the_real_fixture_to_the_demuxer` runs the actual AVI fixture through demux+staged windows). What they cannot catch: the modules' disconnection from the host (F-001) — no test opens a URL through the host. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the browser byte-transport lane" (what this tree is) | mod.rs:4-9 | REAL (components) / PAPER-ONLY (lane) | F-001 — components proven; lane not connected to host, protocol, or glue. |
| "mirroring `crates/streaming`'s … `RangedResource`, `crates/streaming/src/ranged.rs`" | mod.rs:6-8 | REAL | ranged.rs exists, defines RangedResource (verified); seekability probe semantics mirrored per its own 10 green tests. |
| "staged_source — the SourceStream the engine reads network media through" | mod.rs:22-29 | FALSE (today) | F-001 — host refuses http/https (open.rs:182-186); 0 external references. |
| "bytes cross the boundary as FetchResponse messages correlated by request id" | mod.rs:25-27 | FALSE (today) | protocol.rs has no fetch variants (17-variant census, my protocol.rs audit). |
| "the wasm module has no imports and no sockets" | mod.rs:39-40 | REAL | abi.rs declares only `pub extern "C"` exports (:348-399); no import statements. |
| "The transport (browser or mock) enforces CORS" | mod.rs:50-54 | FALSE (mock half) | F-002 — zero CORS/preflight modeling in mock_fetch. |
| "Everything here is proven headless-native against the mock double … the fetch-lane playback proof over the REAL first-vertical fixture" | mod.rs:56-60 | REAL | 34 green io tests; `fetch_lane_serves_the_real_fixture_to_the_demuxer` :3156. |
| "Real-`fetch()` behavior … is `[VERIFY]` (no browser in this sandbox; card 7.I1's matrix owns the browser rows)" | mod.rs:61-63 | REAL (honest) — and the debt is OPEN | No browser run exists in baseline (rc-smoke step 8 SKIPPED; web-matrix lane absent — my protocol.rs audit). |

## 7. Dead / duplicated / unreachable code
- UNREACHABLE + unwired: the entire production io tree (fetch_port, seekability, staged_source) — compiled into the wasm module, called by nothing (F-001).
- DEAD: the 14-symbol re-export facade (F-003).
- No duplication found beyond the intended mirroring of ranged.rs (cross-crate pattern, documented).

## 8. Dependencies & flags
- mod.rs itself adds no deps; the submodules stay within the crate's declared set (media-types, plugin_api SourceStream). mock_fetch correctly cfg(test). No feature gates. `[lints] workspace = true` inherited.

## 9. Open questions for the next agent
- The host-wiring card owner: open.rs's refusal text "network streaming is card 7.H4 (fetch + Range seekability detection)" is now STALE relative to the io tree's existence — either 7.H4 has an unfinished wiring half or the follow-up card needs naming here.
- protocol.rs lane: when fetch wiring lands, the MessagePort contract needs FetchRequest/FetchResponse variants (or the raw ABI word lane) — none exist today (my F-001 there + F-001 here are the two halves of the same gap).
- staged_source part1/part2 auditors: your per-function audits inherit this file's F-001 context — the modules are real; check them as components, not as a wired lane.
