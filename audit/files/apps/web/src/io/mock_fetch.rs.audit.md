# AUDIT: apps/web/src/io/mock_fetch.rs
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Lines / functions | 776 / 19 (18 prod items + 10 tests + 1 test helper) |
| File verdict | UNREACHABLE (real, executed test double; compiled out of every shipped artifact by the `#[cfg(test)]` gate) |
| Reachable from shipped entry point? | NO. `apps/web/src/io/mod.rs:66-67` gates the module: `#[cfg(test)] pub mod mock_fetch;` — it is compiled only under `cargo test`. Every consumer is a test: own `mod tests` (:418-776) and `staged_source.rs:825` inside `#[cfg(test)] mod tests` (:821). No production caller exists (consumer census: `Grep MockFetch` → mock_fetch.rs + staged_source.rs tests only). Deeper still, nothing ships `lumen-web` at all: no dist/, no index.html, no packaging or CI reference to apps/web (Grep over .github/, scripts/ → zero hits); the wasm module is only built by the `web-budget` xtask (baseline gate-web_budget___check.log.txt, floor row rebuilt 1,301,846 B raw) and staged by the ui/e2e/web matrix harness, which never ran in baseline. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `Read` full file; `Grep MockFetch|mock_fetch` (consumer census + baseline test hits); `Grep worker.js|audio-worklet` + `Grep apps/web|wasm32` over .github/, scripts/ (ship-path census); `sed -n` on 26-33, 78-79, 256-262, 355-372 (verbatim quote verification); read of io/mod.rs:65-70 (cfg gate), fetch_port.rs (port contract), staged_source.rs:150-229/820-1009 (how the mock is driven); baseline `orchrun-test-workspace-nofailfast.log.txt:3133-3142` (10/10 tests ok) and `docs/gates/logs/phase7-exit-test.log:3021-3082` (repo's own gate, same). |

## 1. One-paragraph verdict
A competent, genuinely exercised test double: scripted routes covering the whole seekability matrix (206/200/truncating/redirect/fixed-status/network-fail), a request log with real assertion power, Chrome-accurate 20-hop redirect budget, clamping and 416 semantics that match RFC 9110, and 10 tests that ran green in the baseline workspace run. It is honestly test-only — properly `#[cfg(test)]`-gated (better than streaming's un-gated `mockhttp`, W10-A05 F-001) — so it costs the shipped artifacts nothing. The defects are in its fidelity to the contract it claims to mirror: the 206 budget check computes an absolute end offset instead of the advertised span (false refusals for any window fetched past the budget offset), and an inverted `RangeRequest` panics a `fetch()` whose trait contract promises "typed, never a panic".

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3133-3142` → 10/10 `io::mock_fetch::tests` pass (matrix rows 1-5, clamping, 416, budget, port contract).
— verified by read: redirect discipline is loop-safe (`serve` bounds the chain at `0..=MAX_REDIRECTS` = 21 iterations = original + 20 hops, the real Chromium budget; the `/a↔/b` self-loop test terminates in the typed `Network` refusal, test :582-591) and 3xx answers are consumed internally, never surfaced.
— verified by read: clamping arithmetic is correct for well-formed ranges — `end = end_exclusive.min(total) - 1`, `Content-Range bytes 4-7/8` on an over-long request, `416` with `bytes */total` on start-past-end, and `416 → Seekability::Empty` on the empty resource (test :654-704 asserts all three).
— verified by read: the truncating route delivers half the promised span with `Content-Length` agreeing with the delivered body (not the promised span) — a faithful mid-stream-cut model, and `detect_seekability` refuses it typed (`Truncated { expected: 4, got: 2 }`, test :504-530).

## 3. Findings

### F-001 [S3]
Where: apps/web/src/io/mock_fetch.rs:359-367 (doc claim at 355-358)
Code:
```
/// delivered size must fit the request's `response_limit` (a `206`'s
/// Content-Range span is the advertisement; the body is the delivery;
/// a `200`'s Content-Length is the advertisement).
fn enforce_budget(request: &FetchRequest, response: &FetchResponse) -> Result<(), FetchError> {
    let advertised: Option<u64> = match response.status {
        206 => response.header("Content-Range").and_then(|value| {
            value
                .split_once('/')
                .and_then(|(span, _)| span.split_once('-'))
                .and_then(|(_, end)| end.trim().parse::<u64>().ok())
                .map(|end| end.saturating_add(1))
        }),
```
What is wrong: for a `206` the code parses the INCLUSIVE END `b` out of `bytes a-b/total` and uses `b + 1` as the "advertised" size. `b + 1` is the absolute end offset (`start + span`), not the span (`b - a + 1`) the doc names as the advertisement. The `start` half of the header is parsed-and-discarded (bound to `_`).
Why it matters: the check is offset-relative. A `StagedSource` window fetch `[frontier, frontier+chunk)` carries `response_limit = window_bytes` (staged_source.rs:174-190); the mock refuses it typed `ResponseTooLarge` whenever `frontier + chunk > window_bytes` even though the delivered span (chunk ≤ window) fits the budget. With the defaults (8 MiB window, 256 KiB chunk) every read deeper than ~7.75 MiB into a resource would falsely refuse. No current test trips it — every budget-path test fetches at `start = 0` (`the_budget_is_enforced_on_every_answer_shape`, :711-750, probe 0..2048 vs limit 1024: `end+1 = 2048 = span`, the two arithmetics coincide) and `tiny_ranged`'s 12-byte body never leaves the 16-byte window. The suite cannot distinguish the wrong arithmetic from the right one — the defect is invisible to every test that exists.
What correct looks like: parse both halves and compute `b - a + 1` (the span), or simply check the delivered body alone — which is all the real JS glue does ("reads `response.arrayBuffer()` and enforces the limit before staging", fetch_port.rs:19-20).
Confidence: HIGH — Verification: read (arithmetic traced end-to-end); no run (test double; would need a new test at nonzero offset to demonstrate — REQUEST-ORCH-RUN not needed, mutation reasoning recorded).

### F-002 [S3]
Where: apps/web/src/io/mock_fetch.rs:260-262
Code:
```
                let end = range.end_exclusive.min(total).saturating_sub(1);
                let span = end.saturating_sub(range.start).saturating_add(1);
                let slice = &body[range.start as usize..=(end as usize)];
```
What is wrong: an inverted or empty `RangeRequest` (`end_exclusive <= start`, with `0 < start < total`) panics: `end` becomes `< start`, `span` saturates to 1, and `&body[start..=end]` with `end < start` is a slice-index panic. Concretely `RangeRequest { start: 1, end_exclusive: 0 }` against a 12-byte `HonorsRange` route → `&body[1..=0]` → panic. `RangeRequest` has no constructor invariant (fetch_port.rs:68-73 — two public `u64` fields), and `FetchPort::fetch`'s contract is "typed, never a panic" (fetch_port.rs:265-267).
Why it matters: the mock is the reference implementation of the port contract; it violates the contract's only blanket promise on degenerate input. Blast radius is confined to tests (module is `#[cfg(test)]`), so no S1 — but a future test scripting a bad range gets a panic backtrace instead of a typed error, and the `TruncatesRange` arm (:329) does its own slicing without the panic (it clamps `delivered`), so the two arms disagree on robustness.
What correct looks like: guard `end_exclusive <= start` (or `start >= end_exclusive.min(total)`) into the 416/typed-refusal arm before slicing, mirroring the truncating arm's clamp discipline.
Confidence: HIGH — Verification: read (index arithmetic traced; no run performed — mutation-style reasoning, per §5).

### F-003 [S4]
Where: apps/web/src/io/mock_fetch.rs:262 (and :329)
Code:
```
                let slice = &body[range.start as usize..=(end as usize)];
```
What is wrong: `u64 as usize` truncates on 32-bit targets — and this crate's shipped target IS `wasm32-unknown-unknown` (apps/web/Cargo.toml:47-49 cdylib). A scripted offset ≥ 2^32 would silently wrap to a wrong slice instead of refusing.
Why it matters: wrong-by-silence on the one target the crate exists for; unreachable today only because no test scripts 4 GiB offsets.
What correct looks like: `usize::try_from(range.start).map_err(...)` into the typed refusal vocabulary.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/web/src/io/mock_fetch.rs:150-154
Code:
```
    fn route(&self, path: &str) -> Option<&(String, RouteBehavior, Vec<u8>)> {
        self.routes
            .iter()
            .find(|(route_path, _, _)| route_path == path)
    }
```
What is wrong: first-match wins; scripting the same path twice silently shadows the second route. No duplicate detection, no test asserting the shadowing behavior.
Why it matters: a test author re-scripting a path mid-test gets the stale route with no signal — the classic silent-mock failure that costs hours.
What correct looks like: `with_routes` panicking (test code may panic) or asserting on duplicate paths.
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: apps/web/src/io/mock_fetch.rs:30-32 vs :78-79
Code:
```
//! the Chromium-class budget `[VERIFY
//! per-browser]`), a surfaced 3xx is impossible from THIS transport,
```
(+)
```
    /// A fixed status with an empty body (404, 500, 503, …).
    Status(u16),
```
What is wrong: `RouteBehavior::Status(u16)` accepts 302/304/307 — scripting `Status(302)` DOES surface a 3xx from this transport, contradicting the module-doc claim. The seekability layer then classifies it as a protocol violation rather than never seeing it.
Why it matters: the doc's invariant is enforceable in code (reject 300..=399 at route-construction time) but is only prose; a matrix test can be built on a shape the real transport can never produce.
What correct looks like: `Status(u16)` validated (or debug-asserted) to non-3xx in `with_routes`.
Confidence: HIGH — Verification: read of both sites.

### F-006 [S4]
Where: apps/web/src/io/mock_fetch.rs:361-370
Code:
```
                .and_then(|(_, end)| end.trim().parse::<u64>().ok())
```
(200 arm:)
```
            .and_then(|value| value.trim().parse::<u64>().ok()),
```
What is wrong: malformed `Content-Range`/`Content-Length` parse failures are swallowed (`.ok()` → `None` → no advertised check, only the delivered-body check). The module's whole stated discipline is "never silent".
Why it matters: a mock that silently skips its advertised-budget check on exactly the responses whose headers lie is a mock that cannot be used to write the lying-header test. (Today the mock itself always writes well-formed headers, so the path is dead — which is itself the point: the branch exists only to be silently skipped.)
What correct looks like: propagate the parse failure as `FetchError::ProtocolViolation` — the vocabulary entry that exists for exactly this (fetch_port.rs:181-187).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_REDIRECTS` (const) | 56-59 | REAL | — | 20 = Chromium net::kMaxRedirects; honest `[VERIFY per-browser]` label. |
| `RouteBehavior` (enum, 6 variants) | 61-83 | REAL | — | Covers the seekability matrix rows; `Status` accepts 3xx (F-005). |
| `LoggedFetch` (struct) | 85-94 | REAL | — | path + Range header + budget — the asserted evidence. |
| `MockFetch` (struct) | 96-103 | REAL | — | `Mutex<Vec<..>>` log matches the `&self` port signature; poison-recovery on every lock. |
| `MockFetch::new` | 106-110 | REAL | — | `Default` delegate; used by the 404 test. |
| `MockFetch::with_routes` | 112-122 | REAL | S4 | No duplicate-path detection (F-004). |
| `MockFetch::log` | 124-131 | REAL | — | Poison-recovering clone; arrival order. |
| `MockFetch::fetches` | 133-140 | REAL | — | Unused by tests in this file (used via `log()`); trivially correct. |
| `MockFetch::range_headers` | 142-147 | REAL | — | Asserted by matrix test 1 (:466-469). |
| `MockFetch::route` | 149-154 | REAL | S4 | First-match shadowing (F-004). |
| `MockFetch::serve` | 156-215 | REAL | — | Bounded redirect walk (21 iters = 1+20 hops), typed exhaustion error, 404-on-unmatched; logs every hop including the original request. |
| `MockFetch::record_redirect_hop` | 217-228 | REAL | — | Logs per hop — the "both hops in order" assertion holds. |
| `MockFetch::answer_honoring` | 230-294 | REAL | S3/S4 | Correct clamp/416 for well-formed ranges; panics on inverted range (F-002); `as usize` truncation (F-003). |
| `MockFetch::answer_truncating` | 296-344 | REAL | — | Half-span delivery, honest `Content-Length`, `bytes a-b/*`; no panic path (clamped `delivered`). |
| `impl FetchPort::fetch` | 347-353 | REAL | — | serve → enforce_budget composition; dyn-dispatch proven by :754-775. |
| `enforce_budget` | 355-389 | REAL | S3/S4 | 206 advertisement = `end+1` not span (F-001); malformed-header swallow (F-006). |
| `origin_of` | 391-401 | REAL | — | `scheme://host[:port]` slice math traced on "https://media.example/a.bin" → correct; no `://` → `""`. |
| `path_of` | 403-416 | REAL | — | First `/` after authority; defaults `/`; traced correct. |
| `mod tests` (10 tests + `request` helper) | 418-776 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `accept_ranges_route_answers_206_and_detects_ranged` (:439) | YES | Exact status/header/body + `Seekability::Ranged{total:10}` + logged `bytes=0-3`; any drift in clamping or header format fails it. |
| `no_range_route_answers_200_and_detects_unseekable` (:474) | YES | 200 + whole body + `Unseekable{length:6}` + logged path. |
| `truncating_route_refuses_the_typed_truncation` (:503) | YES | Asserts `bytes 0-3/*`, body len 2 (half), and `Truncated{expected:4, got:2}` — pins the truncation model exactly. |
| `redirect_chain_is_followed_with_observables` (:536) | YES | Terminal 206 body `fina`, `redirected`, `final_url`, both hops logged in order, and the `/a↔/b` loop → typed `Network` (would hang/fail on an unbounded loop under the test timeout). |
| `fixed_status_route_answers_the_typed_http_refusal` (:596) | YES | 503 → `HttpStatus{503}` via detect_seekability. |
| `failing_route_answers_the_typed_network_refusal` (:617) | YES | Exact `Network{detail:"dns: NXDOMAIN"}`. |
| `unmatched_path_answers_404` (:639) | YES | Weak but honest (status only). |
| `ranges_clamp_to_the_end_and_refuse_past_it` (:653) | YES | `bytes 4-7/8` + `4567`; start-past-end 416; empty-resource 416→`Empty`. |
| `the_budget_is_enforced_on_every_answer_shape` (:710) | PARTIALLY | Catches a missing 200 check and missing 206 check entirely (asserts exact `ResponseTooLarge{64,1024}` and a `matches!` for 206) — but CANNOT catch F-001's wrong 206 arithmetic: both `end+1` and span equal 2048 at `start=0`. The one behavior the test names ("a 206's Content-Range span is the advertisement") is the one it cannot discriminate. |
| `the_mock_speaks_the_port_contract` (:754) | YES | dyn `&dyn FetchPort` call — pins the trait implementation. |
All 10 ran and passed in baseline (orchrun-test-workspace-nofailfast.log.txt:3133-3142).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "test-only, `#[cfg(test)]` by the parent module" | mock_fetch.rs:1-2 | REAL | io/mod.rs:66-67 `#[cfg(test)] pub mod mock_fetch;` — verified. |
| "a surfaced 3xx is impossible from THIS transport" | mock_fetch.rs:30-32 | FALSE (narrowly) | `Status(u16)` (:78-79) scripts any 3xx; no guard. F-005. |
| "a `206`'s Content-Range span is the advertisement" | mock_fetch.rs:356-357 | FALSE (arithmetic) | Code computes `end+1` (absolute), not the span. F-001. |
| "the mock enforces the same bound on every answer shape" | mock_fetch.rs:38-39 | SHALLOW | Enforced, but with the wrong quantity for nonzero-offset 206s (F-001) and silently skipped on malformed headers (F-006). |
| "Mirrors mockhttp's `LoggedRequest` discipline" | mock_fetch.rs:49-50 | REAL | Log rows carry path+Range+budget; asserted :466-469, :569-573. |
| "Everything here is proven headless-native against the mock double … Real-`fetch()` behavior … is `[VERIFY]`" | io/mod.rs:88-93 | REAL | Honest labeling; the browser lane indeed never ran in baseline (no browser in sandbox; INDEX.md has no web-matrix run). |

## 7. Dead / duplicated / unreachable code
- The whole module is test-only by design and gate — unreachable from shipped entry points, compiled out of release artifacts (the CORRECT posture; contrast streaming/mockhttp which ships, W10-A05).
- `MockFetch::fetches` (:133-140): zero callers in this file's tests; `staged_source` tests use `log()`/`stats()` — near-dead convenience accessor (kept honest: it is `pub` test API).
- No duplication with streaming's mockhttp beyond the shared *pattern* (different vocabulary: typed FetchError vs NetworkError) — acceptable, deliberate mirror.

## 8. Dependencies & flags
- Depends only on `crate::io::fetch_port` types + `std::sync::Mutex` — no external deps, consistent with the crate's zero-new-externals stance (apps/web/Cargo.toml header).
- `testkit::{Must, assert_err}` in tests — dev-dependency, declared (apps/web/Cargo.toml [dev-dependencies]).
- No feature flags touch this file. `unsafe_code = deny` workspace lint respected (no unsafe).

## 9. Open questions for the next agent
- seekability.rs auditor: `detect_seekability` treats a 3xx how? (mock_fetch F-005 makes this reachable in tests; fetch_port.rs:181-187 says "a surfaced 3xx" is a ProtocolViolation — confirm the code path exists.)
- staged_source.rs auditors: do any of your tests fetch a window at `frontier` where `frontier + chunk > window_bytes`? If yes, F-001's false-refusal should have fired — if your tests pass, the configs chosen never cross it (tiny_ranged: 12-byte body in 16-byte window confirms).
- Whoever audits `worker.js`: the REAL glue's budget enforcement (arrayBuffer byteLength vs response_limit) is the semantic the mock diverges from (F-001) — pin the JS side's actual check.
