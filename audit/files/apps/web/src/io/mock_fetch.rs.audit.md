# AUDIT: apps/web/src/io/mock_fetch.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 776 / 14 non-test + 10 tests |
| File verdict | REAL (as a test double; test-only, cfg(test)) |
| Reachable from shipped entry point? | NO — `#[cfg(test)]` module (apps/web/src/io/mod.rs:66-67); never compiled into the wasm/shipped binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n '64,69p' apps/web/src/io/mod.rs` (gate verified); `rg -n "deliver" apps/web/src` -> exit 1 (no deliver harness); `rg -n "MockFetch" apps/web/src` (users: staged_source tests only); baseline citation audit/runs/baseline/cargo-test-workspace.txt:3374-3383 (10/10 tests ok) |

## 1. One-paragraph verdict
This is a genuine, functioning in-process HTTP test double: scripted routes (HonorsRange/IgnoresRange/TruncatesRange/RedirectsTo/Status/Fails), a Mutex-guarded request log, internal redirect-chain following with a 21-iteration budget (`0..=MAX_REDIRECTS`), and typed FetchError answers. It is test-only — the parent module gates it `#[cfg(test)]` — so no shipped path uses it (the hostile-focus answer). Its two real defects: the 206 budget check miscomputes the advertised span as `end+1` instead of `end-start+1` (false refusals for windows at nonzero offsets), and a malformed `RangeRequest` (`end_exclusive <= start < total`) panics via an inverted slice range instead of a typed refusal. The parent mod.rs doc claims a `deliver` harness in this module that does not exist anywhere in apps/web/src.

## 2. What actually works (verified)
— All 10 unit tests pass: audit/runs/baseline/cargo-test-workspace.txt:3374-3383 (`test io::mock_fetch::tests::accept_ranges_route_answers_206_and_detects_ranged ... ok` through `unmatched_path_answers_404 ... ok`). Tests exercise real `detect_seekability` (crate::io::seekability) against each scripted shape, so they are not pure tautologies over the mock.
— `#[cfg(test)]` gate verified by `sed -n '64,69p' apps/web/src/io/mod.rs`.

## 3. Findings
F-001 [S3]
Where: apps/web/src/io/mock_fetch.rs:359-367
Code:
```
206 => response.header("Content-Range").and_then(|value| {
    value
        .split_once('/')
        .and_then(|(span, _)| span.split_once('-'))
        .and_then(|(_, end)| end.trim().parse::<u64>().ok())
        .map(|end| end.saturating_add(1))
}),
```
What is wrong: The "advertised" size of a 206 is computed as `end + 1`, ignoring the range start. The advertised span is `end - start + 1`. For any ranged window at a nonzero offset (e.g. Content-Range `bytes 10485760-10551295/…`), the computed value is the absolute end, which exceeds any realistic `response_limit` (the window budget, 8 MiB default per fetch_port.rs:46). The mock then answers `FetchError::ResponseTooLarge` for a response whose delivered span fits the budget. Why it matters: the module doc (L37-43) claims the mock mirrors the JS glue's budget enforcement ("The JS glue reads `response.arrayBuffer()` and enforces the request's `response_limit`" — delivered bytes only); the mock is stricter AND wrong, so staged_source tests validated against the mock do not validate the real transport's budget semantics, and any future test with a small limit at a nonzero offset will false-fail. What correct looks like: parse both `a` and `b` from `bytes a-b/total` and compare `b - a + 1` (and mirror only the check the real glue actually performs). Confidence: HIGH — Verification: read (panic-free reasoning over parse chain; no execution needed).

F-002 [S3]
Where: apps/web/src/io/mock_fetch.rs:260-262
Code:
```
let end = range.end_exclusive.min(total).saturating_sub(1);
let span = end.saturating_sub(range.start).saturating_add(1);
let slice = &body[range.start as usize..=(end as usize)];
```
What is wrong: No validation that `range.end_exclusive > range.start`. For `RangeRequest { start: 5, end_exclusive: 5 }` on a 10-byte body: `end = 4`, `span = 1` (saturating math hides the inversion), slice = `body[5..=4]` — a Rust slice-index panic ("slice index starts at 5 but ends at 4"), not a typed FetchError. Why it matters: a test double whose failure mode is a panic rather than a typed error converts any malformed-request bug in staged_source into an opaque panic instead of a diagnostic. What correct looks like: guard `range.end_exclusive <= range.start` with a typed refusal (or debug_assert). Confidence: HIGH — Verification: read (mutation reasoning; not executed — cargo-build-budget; no existing test constructs an inverted range).

F-003 [S4]
Where: apps/web/src/io/mod.rs:31-33 (doc claim touching this file)
Code:
```
//!   redirecting, fixed-status), a request log tests assert on, and
//!   the `deliver` harness that answers the host's outstanding
//!   `FetchRequest`s through the REAL message path.
```
What is wrong: No `deliver` harness exists. `rg -n "deliver" apps/web/src` -> exit 1 (zero matches). The closest thing is `fn serve`/`fn pipeline`, private test helpers inside staged_source.rs's own `#[cfg(test)]` module (staged_source.rs:831-847), which call `transport.fetch(...)` directly — not "through the REAL message path". The doc names a component that was never written. Why it matters: doc asserts an integration surface (mock → real message lane) that no test exercises; the mock never rides the real message path anywhere. What correct looks like: either write the deliver harness or delete the claim. Confidence: HIGH — Verification: ran (`rg` exit 1).

F-004 [S4]
Where: apps/web/src/io/mock_fetch.rs:133-140
Code:
```
/// How many fetches were issued (the log's length).
#[must_use]
pub fn fetches(&self) -> usize {
```
What is wrong: The doc lies about what is counted for redirected chains: `record_redirect_hop` (L218-228) logs one row per HOP, so a single user-visible `fetch()` that follows 3 hops reports `fetches() == 4`. The name promises call count; the log records HTTP requests. What correct looks like: rename doc to "recorded requests (one row per hop, redirects included)". Confidence: HIGH — Verification: read (redirect test at L569-573 asserts two log rows for one fetch call).

## 4. Per-function table (mandatory, file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_REDIRECTS` const | 59 | REAL | — | 20; loop `0..=20` = 21 lookups, matches "up to 20 hops" doc |
| `RouteBehavior` enum | 63-83 | REAL | — | 6 variants, all handled in `serve` |
| `LoggedFetch` | 87-94 | REAL | — | plain data row |
| `MockFetch::new` | 108-110 | REAL | — | 404-only transport |
| `MockFetch::with_routes` | 114-122 | REAL | — | copies routes; fine |
| `MockFetch::log` | 126-131 | REAL | — | poison-tolerant clone |
| `MockFetch::fetches` | 135-140 | REAL | S4 | F-004: counts hops, not fetches |
| `MockFetch::range_headers` | 145-147 | REAL | — | projection over log |
| `MockFetch::route` | 150-154 | REAL | — | linear find; OK for tests |
| `MockFetch::serve` | 158-215 | REAL | — | redirect walk + 404 default + terminal dispatch; budget loop correct |
| `MockFetch::record_redirect_hop` | 218-228 | REAL | — | one row per hop incl. terminal |
| `MockFetch::answer_honoring` | 233-294 | REAL | S3 | F-002 inverted-range panic; 416/206/200 shapes correct for valid ranges |
| `MockFetch::answer_truncating` | 299-344 | REAL | — | half-span cut, `bytes a-b/*`, clamp(1,span) redundant; same inverted-range hazard as F-002 |
| `FetchPort for MockFetch` | 347-353 | REAL | — | serve + enforce_budget |
| `enforce_budget` | 359-389 | REAL | S3 | F-001 wrong advertised math |
| `origin_of` | 393-401 | REAL | — | index arithmetic correct (checked by hand) |
| `path_of` | 405-416 | REAL | — | "/" default; no-scheme fallback returns whole string (tests never hit) |
| 10 tests | 439-775 | REAL | — | see §5 |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accept_ranges_route_answers_206_and_detects_ranged | YES | asserts exact 206 + `bytes 0-3/10` + body `0123` + Seekability::Ranged{10} + logged Range header |
| no_range_route_answers_200_and_detects_unseekable | YES | exact 200 + body + Unseekable{6} |
| truncating_route_refuses_the_typed_truncation | YES | asserts half-span body and exact FetchError::Truncated{4,2} from real detect_seekability |
| redirect_chain_is_followed_with_observables | YES | 206 at terminal, redirected flag, final_url, ordered log, loop chain -> Network |
| fixed_status_route_answers_the_typed_http_refusal | YES | 503 -> FetchError::HttpStatus{503} |
| failing_route_answers_the_typed_network_refusal | YES | exact Network detail |
| unmatched_path_answers_404 | YES (weakly) | only status asserted |
| ranges_clamp_to_the_end_and_refuse_past_it | YES | 4-7/8 clamp + 416 past-end + Empty on empty body |
| the_budget_is_enforced_on_every_answer_shape | PARTIAL | covers delivered>limit (200) and start-0 advertised span (206); does NOT cover nonzero-offset 206 — exactly the case F-001 breaks |
| the_mock_speaks_the_port_contract | YES | dyn FetchPort dispatch, exact body/header |

Caveat: 9 of 10 tests partially test the mock against itself (scripted -> asserted). Their real value is as fixtures for `detect_seekability` (real code), which they do exercise.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "test-only, `#[cfg(test)]` by the parent module" | mock_fetch.rs:1-2 | REAL | mod.rs:66-67 `#[cfg(test)]` / `pub mod mock_fetch;` (sed verified) |
| "the `deliver` harness that answers the host's outstanding FetchRequests through the REAL message path" | mod.rs:31-33 | FALSE | `rg deliver apps/web/src` exit 1 (F-003) |
| "The mock enforces the same bound on every answer shape … the advertised span of a `206`" | mock_fetch.rs:38-40 | SHALLOW | it enforces A bound, but with wrong math (F-001); the real glue enforces delivered bytes only (per same doc) |
| "a surfaced 3xx is impossible from THIS transport" | mock_fetch.rs:32-33 | REAL | serve() only returns 206/200/416/<Status>/404; RedirectsTo never surfaces |
| "[VERIFY per-browser]" redirect budget 20 | mock_fetch.rs:56-59 | REAL-UNPROVEN | constant is honest about being unverified against real engines |
| "The REAL transport on this lane is the Worker's JS glue calling `fetch()` (doc 09 §6.1)" | mock_fetch.rs:9-10 | FALSE (today) | Addendum (W12-A08, post-worker.js audit): the shipped web glue contains NO media-fetch code — `rg "fetch" apps/web/worker.js` hits only the module fetch (L134); apps/web/src/protocol.rs defines no FetchRequest/FetchResponse message; the committed browser matrix refuses http opens (`open-http-pending-lane ... unsupported-for-host`, docs/web/browser-matrix-20261110.md:72). The "real transport" this mock mirrors does not exist in any shipped lane yet; the io fetch lane is native-test-only |

## 7. Dead / duplicated / unreachable code
None dead. Whole module is unreachable from shipped binaries BY DESIGN (test double) — not a defect. Duplication note: `answer_honoring` and `answer_truncating` duplicate the 416 shape and clamp math (L244-255 vs L314-323) — S4-level copy-paste.

## 8. Dependencies & flags
Depends only on `crate::io::fetch_port` types and std Mutex; tests additionally on `crate::io::seekability` and testkit (`Must`/`assert_err`). No feature gates. No unsafe.

## 9. Open questions for the next agent
- Does any future web glue enforce `response_limit` against the delivered arrayBuffer only, or also against an advertised span? Determines whether F-001 makes the mock stricter than reality or merely wrong. CORRECTION (W12-A08, after the worker.js audit): worker.js contains NO media-fetch glue at all (only the module fetch, L134) — the transport this mock mirrors does not exist yet (see the addendum in §6), so there is no real-glue budget behavior to mirror; F-001's advertised-span check is extra checking no real transport performs, against math that is wrong either way.
