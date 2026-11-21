# AUDIT: apps/web/src/io/fetch_port.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-21 |
| Lines / functions | 381 / 14 (+4 tests) |
| File verdict | UNREACHABLE (real, mock-tested seam of an entirely unwired lane) |
| Reachable from shipped entry point? | NO — and not even from the crate's own entry: `rg "fetch_port\|seekability\|staged_source\|io::"` outside `src/io/` → zero hits in host.rs/abi.rs/protocol.rs. `HostToWorker` (protocol.rs:55-107) has variants Boot/LoadBlob/Command/Pump/SurfaceLost/AudioContext/Shutdown — **no fetch request/response message exists**; open.rs:162-184 accepts only `blob:` URLs and refuses network ("network streaming is card 7.H4"); worker.js's only `fetch()` is the wasm module self-load (worker.js:134). Sole consumers: staged_source.rs (same dead subtree) + mock_fetch.rs (`#[cfg(test)]`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "validate_http_url\|FetchRequest\|FetchPort\|detect_seekability\|StagedSource" apps/web/src` excluding src/io (zero external callers); `grep -n "Fetch\|fetch" apps/web/src/protocol.rs apps/web/src/abi.rs` (zero); `grep -n "fetch" apps/web/worker.js` (module self-load only, l.134); `sed -n '95,130p' apps/web/src/protocol.rs` (enum complete, 7 variants); quotes verified with `sed -n` + `od -c` (a display artifact renders `#[must_use]` as `#ust_use]` in some tool outputs — files are clean); baseline: `orchrun-test-workspace-nofailfast.log.txt` lines 3129-3132 (4/4 fetch_port tests `ok`) |

## 1. One-paragraph verdict
The typed vocabulary for browser network I/O — request/response/range/error types plus early URL validation — is cleanly designed, correctly implemented at the function level, and pinned by four exact tests that passed in the orchestrator's no-fail-fast baseline run. But the lane it vocabulary-izes does not exist: no protocol message can carry a `FetchRequest` across the Worker boundary (the `HostToWorker`/`WorkerToHost` enums have no fetch variants), the open path refuses network URLs outright, and the JS glue's only `fetch()` fetches the wasm module itself. The module doc's "Requests ride the Worker MessagePort round trip (the glue serves them asynchronously)" describes plumbing nobody built — the same phantom-glue pattern as canvas.rs's `player.js`. Within the dead subtree the code holds up, with one real defect: `FetchError::as_engine_error` throws away its variant's own facts and reports a hard-coded 8 MiB budget with `attempted: 0` no matter what limit was actually exceeded (the default request budget is 256 KiB).

## 2. What actually works (verified)
— `io::fetch_port::tests::{range_header_is_inclusive_wire_form, url_validation_is_early_and_typed, response_header_lookup_is_case_insensitive, fetch_errors_carry_their_facts}` → all `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lumen_web 111/0).
— End-exclusive → inclusive wire form re-verified by hand: `{0, 262144}` → `bytes=0-262143` ✓; `256·1024 = 262144` ✓. Header lookup case-insensitivity (eq_ignore_ascii_case) ✓.

## 3. Findings

### F-001 [S3]
Where: apps/web/src/io/fetch_port.rs:21-25
Code:
```rust
//! - **A correlation id.** Requests ride the Worker MessagePort
//!   round trip (the glue serves them asynchronously), so every
//!   [`FetchRequest`] carries the id its [`FetchResponse`] must
//!   echo — the same discipline the command layer's `requestId`
//!   uses.
```
What is wrong: the round trip does not exist. `HostToWorker` (protocol.rs:55-107) and `WorkerToHost` have no fetch variants; nothing surfaces a `FetchRequest` upstream and nothing absorbs a `FetchResponse`; open.rs:184 refuses network URLs ("network streaming is card 7.H4"); worker.js performs exactly one fetch — `lumen_web.wasm` for its own instantiation (worker.js:134). The whole `io` subtree (fetch_port → seekability → staged_source) is compiled, mock-tested, and referenced by no production path.
Why it matters: card 7.H4 reads as "landed" in the module tree while the lane is unreachable — the network playback capability it implies cannot run. The correlation-id discipline the doc describes has no carrier. (The io/mod.rs evidence labels are honest that this is mock-proven only; the transport-split docs are not — they describe the message flow in the present tense.)
What correct looks like: either the `Fetch{request}`/`FetchResponse{…}` protocol variants + glue handling, or the docs state the lane is library-only until 7.H4's wiring card.
Confidence: HIGH — Verification: read + rg enumerations (documented above).

### F-002 [S3]
Where: apps/web/src/io/fetch_port.rs:238-242
Code:
```rust
            FetchError::ResponseTooLarge { .. } => media_types::ErrorKind::ResourceLimit {
                limit: media_types::error::LimitKind::Memory,
                budget: DEFAULT_WINDOW_BYTES as u64,
                attempted: 0,
            },
```
What is wrong: the variant carries `limit` (the budget that was exceeded) and `actual` (the offending size) — both are discarded (`{ .. }`). The engine error always reports budget = `DEFAULT_WINDOW_BYTES` (8 MiB) and attempted = 0. `FetchRequest::new` defaults `response_limit` to `DEFAULT_CHUNK_BYTES` (256 KiB), so the common refusal ("body exceeded the 256 KiB chunk budget") surfaces as "memory budget 8 MiB, attempted 0 bytes" — internally inconsistent and unfalsifiable.
Why it matters: the typed-error discipline this codebase advertises ("every failure names itself") is broken on the facts: a debugger or UI reading the ResourceLimit error gets the wrong budget and a zero attempt. Contrast the Truncated arm directly below it, which does forward `expected`/`got`.
What correct looks like: `budget: *limit, attempted: *actual` (bind the fields instead of `{ .. }`).
Confidence: HIGH — Verification: read (variant fields at l.175-180; default limit at l.112).

### F-003 [S4]
Where: apps/web/src/io/fetch_port.rs:78-84
Code:
```rust
    pub fn to_header_value(self) -> String {
        format!(
            "bytes={}-{}",
            self.start,
            self.end_exclusive.saturating_sub(1)
        )
    }
```
What is wrong: no validation of the range. `RangeRequest { start: 0, end_exclusive: 0 }` (an empty span) serializes as `bytes=0-0` — a request for ONE byte; an inverted range (`start: 10, end_exclusive: 5`) serializes as `bytes=10-4` — nonsense the server answers 416. Only the saturating_sub on the end is defensive.
Why it matters: latent while the lane is dead (F-001); the day it ships, a degenerate window computation becomes a wire-visible protocol error instead of a typed local refusal.
What correct looks like: refuse `end_exclusive <= start` typed (ProtocolViolation) at construction or serialization.
Confidence: HIGH — Verification: read (format semantics; no caller exists to produce the input today).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` | 60-62 | REAL (dead subtree) | — | stage-Open constructor, stable code. |
| `RangeRequest::to_header_value` | 77-84 | REAL (defect) | S4 | F-003 unvalidated degenerate ranges. |
| `FetchRequest::new` | 106-114 | REAL (dead subtree) | — | default budget = chunk size (256 KiB) — deliberate per-request granularity. |
| `FetchRequest::with_range` | 116-121 | REAL (dead subtree) | — | builder. |
| `FetchRequest::range_header` | 123-127 | REAL (dead subtree) | — | delegates. |
| `FetchResponse::header` | 152-159 | REAL (dead subtree) | — | ASCII case-insensitive first match ✓ (test-pinned). |
| `Display for FetchError` | 208-222 | REAL (dead subtree) | — | 5 arms, each carries its facts in prose. |
| `FetchError::as_engine_error` | 230-254 | REAL (defect) | S3 | F-002 facts discarded on ResponseTooLarge; Truncated arm forwards facts correctly; kind mapping (HttpStatus→ConnectionLost) claimed to mirror streaming's ranged.rs — UNVERIFIED against that file (other wave's assignment). |
| `FetchPort::fetch` (trait) | 261-268 | REAL (dead subtree) | — | the seam mock_fetch implements; no production implementor exists. |
| `validate_http_url` | 278-302 | REAL (dead subtree) | — | scheme/control/space checks then defers to `SourceUri::parse`; to_ascii_lowercase allocates a copy per call (cold path — fine); the two refusals share code `web.fetch.url`; validated against test vectors incl. `https://` empty-host and embedded space. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `range_header_is_inclusive_wire_form` | YES | exact `bytes=0-262143` string + None case. |
| `url_validation_is_early_and_typed` | YES | accept http/https + port + query; refuse ftp/empty-host/space; stage assert on every refusal; scheme round-trip. |
| `response_header_lookup_is_case_insensitive` | YES | two casings hit, absent header → None. |
| `fetch_errors_carry_their_facts` | PARTIAL | pins Display of HttpStatus and the stage/code of ResponseTooLarge — but does NOT assert the budget/attempted values, which is exactly how F-002 survived. |
All four → `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lines 3129-3132).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Requests ride the Worker MessagePort round trip (the glue serves them asynchronously)" | module docs l.21-23 | FALSE | no protocol variant, no glue handler (F-001). |
| "the JS glue reads `response.arrayBuffer()` and enforces the limit before staging" | module docs l.19-20 | FALSE today | no such JS exists; worker.js only self-loads the module. |
| "browser's own [redirect] budget applies (20 on Chromium-class engines `[VERIFY per-browser]`)" | module docs l.11-12 | honest gap | labeled VERIFY; no browser in baseline. |
| "8 MiB … web-tuned mirror of streaming's 64 MiB" | l.40-45 | REAL-UNPROVEN | constant exists; the streaming-side 64 MiB mirror claim not re-verified here (crates/streaming is another wave's file). |
| "The mock implements it; the same types ride the port contract — one vocabulary" | l.32-33, 257-260 | HALF-TRUE | the types exist; the port contract (protocol) has no such lane (F-001). |

## 7. Dead / duplicated / unreachable code
- The entire module, transitively with seekability.rs and staged_source.rs (F-001) — production-compiled, zero production callers.
- mock_fetch.rs is `#[cfg(test)]` (honest).

## 8. Dependencies & flags
- media-types (EngineError/ErrorContext/SourceUri/PipelineStage), serde (wire types). No feature gates. `pub(crate) mint` used by staged_source/seekability (in-subtree).

## 9. Open questions for the next agent
- A09/A10 (staged_source parts): does staged_source construct degenerate RangeRequests anywhere (F-003's trigger)? And does it read the ResourceLimit budget/attempted fields (F-002's consumer)?
- A07 (io/mod.rs): the mod docs claim "the pump gates engine polls on the source's readiness" — the pump (host.rs part2) has no StagedSource reference; that claim needs their verdict too.
