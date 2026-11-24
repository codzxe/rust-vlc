# AUDIT: apps/web/src/io/fetch_port.rs
| Field | Value |
|---|---|
| Auditor | W12-A10 |
| Date | 2026-11-24 |
| Lines / functions | 381 / 13 (9 prod items, 4 tests) |
| File verdict | UNREACHABLE (real, mock-tested seam — compiled into the shipped wasm module but no shipped path constructs a request; the JS glue it documents does not exist) |
| Reachable from shipped entry point? | NO — the only network-source entry (open.rs `resolve_source`) refuses `http`/`https` before anything in `io::` runs; zero references to `FetchPort`/`FetchRequest`/`detect_seekability`/`StagedSource` outside `apps/web/src/io/`; no JS glue in `ui/` or `apps/web` (rg for `arrayBuffer`/`fetch(` in non-Rust sources: zero) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `sed -n`/`od -c` quote verification (note: Bash display layer eats literal `[m` sequences — quotes cross-checked via od/md5); `rg -n 'Fetch' protocol.rs abi.rs host.rs open.rs source.rs` (no hits); `rg -rln 'arrayBuffer|fetch\(' ui/ apps/web` (no hits); baseline cite cargo-test-workspace.txt:3370-3373 (4/4 tests ok) |

## 1. One-paragraph verdict
The types are clean, the validation is early and typed, and 4/4 unit tests pass — but this is a port into nowhere. The module documents a production topology ("the JS glue reads `response.arrayBuffer()`", "production rides the port-contract messages") in which a Worker-side glue serves fetches; that glue does not exist anywhere in the tree, the worker protocol (protocol.rs) carries no fetch message, and the only entry that could need a network source (open.rs `resolve_source`) refuses `http`/`https` with a typed error before `io::` is ever touched. Everything here is exercised exclusively by the in-crate mock (mock_fetch.rs) and the seekability/staged-source tests. Two concrete API defects: `RangeRequest::to_header_value` emits malformed RFC 9110 headers for empty/inverted ranges, and `FetchError::ResponseTooLarge::as_engine_error` discards its own facts and reports a wrong budget with `attempted: 0`.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/cargo-test-workspace.txt:3370-3373` -> 4/4 `io::fetch_port::tests` ok (wire form of the Range header, early typed URL validation incl. scheme/stage asserts, case-insensitive header lookup, typed error facts). `validate_http_url` verified by read: scheme gate, control-byte/space gate, empty-rest gate all precede `SourceUri::parse`.

## 3. Findings
F-001 [S2]
Where: apps/web/src/io/fetch_port.rs:16-33 (module doc) vs the tree
Code:
```
//! - **Whole bodies, bounded.** Like the 6.A1a port this is a
//!   whole-body contract under [`FetchRequest::response_limit`] (the
//!   ReadableStream incremental lane is a later card's follow-up —
//!   see doc 09 §6.1). The JS glue reads `response.arrayBuffer()`
//!   and enforces the limit before staging.
```
What is wrong: The documented production topology does not exist. (a) No JS glue: `rg` for `arrayBuffer`/`fetch(` across `ui/` and `apps/web` non-Rust sources returns zero hits — nobody serves these requests. (b) The worker protocol carries no fetch message: `rg -n 'Fetch' apps/web/src/protocol.rs apps/web/src/abi.rs` — zero hits. (c) The only network-source entry refuses before this module runs:
```
                "http" | "https" => Err(unsupported_for_host(
                    scheme,
                    "network streaming is card 7.H4 (fetch + Range seekability detection)",
                )
                .with_detail("capability", "http-streaming")),
```
(open.rs:182-186). So "production rides the port-contract messages" (line 32-33) describes a future, not the build. The seam is real and mock-exercised, but it is dead weight in the shipped wasm artifact. Why it matters: capability row "http-streaming: NotStarted" (capabilities.rs:272-278) plus this dead lane means the repo carries a complete-looking fetch implementation that cannot be reached by any user action — a maintenance trap and size cost. What correct looks like: either wire `StagedSource` into `resolve_source`'s http arm behind the typed capability row, or gate the io tree behind a not-yet-wired feature so it stops compiling into the shipped module. Confidence: HIGH — Verification: read + rg.
F-002 [S3]
Where: apps/web/src/io/fetch_port.rs:75-85
Code:
```
impl RangeRequest {
    /// The `Range:` header value (inclusive wire form).
    #[must_use]
    pub fn to_header_value(self) -> String {
        format!(
            "bytes={}-{}",
            self.start,
            self.end_exclusive.saturating_sub(1)
        )
    }
```
What is wrong: No validation of the range's geometry. An empty range (`start: 0, end_exclusive: 0`) serializes to `bytes=0-0` — a request for byte 0 of a zero-length span; an inverted range (`start: 5, end_exclusive: 2`) serializes to `bytes=5-1` — a malformed RFC 9110 header (last-byte-pos < first-byte-pos) that a compliant server must reject with 200 (full body) — silently defeating the whole-body budget. The struct is public with no constructor discipline. Why it matters: when this lane is eventually wired, a geometry bug upstream silently converts every ranged read into a full-file download (the exact OOM the `response_limit` exists to prevent). What correct looks like: refuse (typed) or clamp on `end_exclusive <= start`. Confidence: HIGH — Verification: read.
F-003 [S3]
Where: apps/web/src/io/fetch_port.rs:238-242
Code:
```
            FetchError::ResponseTooLarge { .. } => media_types::ErrorKind::ResourceLimit {
                limit: media_types::error::LimitKind::Memory,
                budget: DEFAULT_WINDOW_BYTES as u64,
                attempted: 0,
            },
```
What is wrong: The variant carries `limit` and `actual` (lines 175-180) and the engine-error mapping throws both away: `budget` is hard-coded to the 8 MiB window default even when the request's `response_limit` was the 256 KiB chunk default (`FetchRequest::new`, line 112), and `attempted: 0` asserts zero bytes were attempted — false by construction (the error exists because more than `limit` arrived). The surfaced error therefore reports the wrong budget and a provably wrong attempted size. Why it matters: `ResourceLimit` errors are the MP §11 budget surface — a UI showing "8 MiB budget, attempted 0 B" for a 256 KiB-chunk violation is garbage diagnostics. What correct looks like: `budget: *limit, attempted: *actual`. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/web/src/io/fetch_port.rs:278-302
Code:
```
pub fn validate_http_url(url: &str) -> Result<SourceUri, EngineError> {
    let lower = url.to_ascii_lowercase();
```
What is wrong: The scheme check runs on a lowercased copy but `SourceUri::parse(url)` receives the original — a mixed-case URL like `HTTP://host/x` passes the gate and then lives or dies by `SourceUri`'s own case handling (UNVERIFIED — media-types grammar not read). Also the scheme-name in the error message is taken from `url.split(':').next()`, which for a colon-less input echoes the entire string as the "scheme". Cosmetic, bounded blast radius. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_WINDOW_BYTES / DEFAULT_CHUNK_BYTES | 40-51 | REAL | — | 8 MiB / 256 KiB, documented divergence from streaming's 64 MiB |
| mint | 60-62 | REAL | — | stage-Open error constructor |
| RangeRequest::to_header_value | 75-85 | REAL | S3 | no geometry validation (F-002) |
| FetchRequest::new/with_range/range_header | 104-128 | REAL | — | default limit = chunk budget |
| FetchResponse::header | 151-160 | REAL | — | ASCII case-insensitive, first-match |
| FetchError (5 variants) + Display | 162-222 | REAL | — | facts preserved in the variant |
| FetchError::as_engine_error | 224-255 | REAL | S3 | ResponseTooLarge facts discarded (F-003) |
| FetchPort (trait) | 257-268 | REAL | S2 | no production implementor; mock only (F-001) |
| validate_http_url | 270-302 | REAL | S4 | early+typed; case quirk (F-004) |
| 4 tests | 304-381 | REAL | — | honest for formatting/validation; no transport test (by design: [VERIFY] browser) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| range_header_is_inclusive_wire_form | YES | exact header string asserted (but only a well-formed range — F-002 invisible) |
| url_validation_is_early_and_typed | YES | accept/reject table + stage asserted |
| response_header_lookup_is_case_insensitive | YES | hit both cases + miss |
| fetch_errors_carry_their_facts | PARTIAL | asserts Display + code/stage, never the ResourceLimit numbers (F-003 invisible) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the JS glue reads `response.arrayBuffer()` and enforces the limit before staging" | module doc :18-20 | FALSE (no such glue exists) | rg zero hits in ui/ + apps/web non-Rust |
| "production rides the port-contract messages" | module doc :32-33 | FALSE (today) | protocol.rs/abi.rs carry no fetch message |
| "browser's own redirect budget (20 on Chromium-class) [VERIFY per-browser]" | module doc :11-12 | honest [VERIFY] label | no test claims it |
| io/mod.rs:76-79 "proven headless-native against the mock … Real-`fetch()` … is `[VERIFY]`" | io/mod.rs | REAL-UNPROVEN | mock tests pass; browser rows honestly deferred |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped entry points (F-001): compiled into the wasm artifact (lib.rs `pub mod io`), zero non-io callers. The `FetchPort` trait's only implementor is the `#[cfg(test)]` mock (mock_fetch.rs).

## 8. Dependencies & flags
media-types (EngineError/SourceUri), serde. No feature gates on this file. No unsafe. `pub(crate) mint` used by seekability/staged_source.

## 9. Open questions for the next agent
- io/seekability.rs and io/staged_source.rs (other agents' files): confirm they are equally caller-free — if so, the whole 7.H4 lane is present-but-unwired, and the capabilities row + open.rs refusal are the only honest signs.
- Environment note for all agents: the Bash output display eats literal `[m` byte pairs (e.g. `#[must_use]` renders as `#ust_use]`); od -c / md5 / the Read tool show true bytes. Not a repo defect.
