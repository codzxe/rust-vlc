# AUDIT: crates/streaming/src/mockhttp.rs#part3 (lines 1039-1559)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part3 = lines 1039-1559, per wave-00 assignments.tsv LOC split (part1=519 → A05: 1-519, server config/auth/logged-request plumbing; part2=519 → A08: 520-1038, server loop + MockClient + redirect/classify; part3=521 → this audit: response-head parsing + fuzz entry + body readers + the entire `#[cfg(test)] mod tests`, 1188-1559). |
| Lines / functions | 521 / 28 (1 parser fn + 1 type alias + 1 pub fuzz fn + 3 body/error fns + 3 test helpers + 18 test fns + FakeFs-free) |
| File verdict | UNREACHABLE (test-only testkit by explicit design — but compiled into the shipped crate un-gated, see F-001) |
| Reachable from shipped entry point? | NO production call path (module doc lines 1-2: "test-only; never used by production code paths"; rg confirms only test callers: `apps/cli/tests/unsupported_path.rs:233`, `apps/cli/src/demux_host.rs:881` (cfg(test)), in-crate tests). BUT the module IS compiled into the release library: `crates/streaming/src/lib.rs:36` `pub mod mockhttp;` with no `cfg` gate, and `lumen-cli` depends on `streaming` as a regular dependency (baseline gate-check_architecture: that edge is an UNDECLARED violation). Whether the linker strips the dead testkit from the final `lumen` executable is UNVERIFIED (baseline binary not present in this sandbox). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "mockhttp" apps/ crates/ fuzz/` (all callers test-only) · `sed -n '28,41p' crates/streaming/src/lib.rs` (`pub mod mockhttp;` unconditional) · `ls fuzz/fuzz_targets/` (fuzz_http_headers.rs exists) · baseline `orchrun-test-workspace-nofailfast.log.txt:5068-5086` (all 18 mockhttp::tests ok) · baseline `gate-check_architecture.log.txt` (lumen-cli→streaming edge FAIL) · `sed -n` quote verification |

## 1. One-paragraph verdict
Solid, discriminating test infrastructure: every one of the 18 tests in this chunk pins exact typed outcomes (error variants with payload fields, hop-path sequences, header strings, budget numbers) and all ran green in the baseline no-fail-fast run; the response-head parser, bounded/EOF body readers, and error mapper are correct on every path I traced (over-budget refusal happens before allocation in both readers, truncation is typed, never silent). The defects are structural: a TCP-server-spawning testkit is compiled into the production crate behind an unconditional `pub mod` — forced there because the CLI's integration tests import it — while its own doc claims test-only status; the head grammar silently swallows colonless header lines, which undercuts the fuzz entry's "hostile bytes → typed verdict" story; and a doc cites a fuzz-target filename that does not exist.

## 2. What actually works (verified)
— All 18 `mockhttp::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:5068-5086` (release profile).
— Hand-traced `read_bounded_body` (1112-1143): budget refusal at 1118-1123 precedes the `vec![0u8; expected]` allocation; `Ok(0)` mid-body → `BodyTruncated` with exact expected/received; errors routed through `map_body_read_error`. `read_eof_body` (1147-1174): the `body.len() + n > limit` check at 1160-1165 fires BEFORE `extend_from_slice` — no over-allocation window.
— `fuzz_response_head` (1088-1099) is genuinely wired to the crate-external harnesses: `crates/streaming/tests/netfuzz/fuzz.rs` (doc lines 12-15 call it) and `fuzz/fuzz_targets/fuzz_http_headers.rs` exists on disk (ls).
— `parse_response_head_text` is panic-free on arbitrary lossy-decoded text (empty head → "" version → typed refusal; non-numeric code → typed refusal; u16 overflow code → typed refusal via parse).

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/lib.rs:36 with crates/streaming/src/mockhttp.rs:1-2
Code:
```
pub mod mockhttp;
```
```
//! Mock HTTP/1.1 testkit — **test-only; never used by production code
//! paths** (card 6.A1a).
```
What is wrong: the module is declared with no `cfg(test)`/feature gate, so the whole testkit — a TCP listener spawning threads, an HTTP/1.1 server, auth-challenge machinery — is type-checked and compiled into the release `streaming` rlib that the shipped `lumen` CLI links as a regular dependency (baseline `gate-check_architecture`: "`lumen-cli` depends on `streaming`, which is not in [crates.lumen-cli].internal_deps" — an edge violation). The doc's "never used by production code paths" is true of call paths and false of compilation. The forcing function is real: `apps/cli/tests/unsupported_path.rs:233` imports `streaming::mockhttp::MockServer` from an integration test, which cannot see `#[cfg(test)]` items — so the testkit stays pub and ships.
Why it matters: test-only code in the shipped artifact is dead weight and a standing misuse surface (any future caller can spawn a mock server inside the product binary); the architecture gate already flags the dependency edge that drags it in. The repo's own `testkit-direction` check passes only because `testkit` (the crate) is dev-only — this module escapes that net.
What correct looks like: gate the module behind an off-by-default feature (`#[cfg(feature = "testutil")] pub mod mockhttp;`) with the feature enabled in `[dev-dependencies]`/dev builds, or move the testkit to a dedicated dev-only crate as was done for `testkit`.
Confidence: HIGH — Verification: read (lib.rs:36 unconditional; CLI regular dep per baseline arch-gate log). Linker stripping of the final executable UNVERIFIED — binary not present in this sandbox (`ls target/release/lumen` empty here; baseline ran in the isolated clone).

### F-002 [S4]
Where: crates/streaming/src/mockhttp.rs:1076-1082 (doc comment)
Code:
```
/// the mock's response-head grammar (status line + `Name: value`
/// lines). `pub` for the crate-external fuzz harness
/// (`crates/streaming/tests/netfuzz/fuzz.rs`, `#[path]`-included by
/// `fuzz/fuzz_targets/fuzz_http_header.rs`); test-support only — no
```
What is wrong: the cited libFuzzer shim is `fuzz/fuzz_targets/fuzz_http_headers.rs` (plural) — `fuzz_http_header.rs` does not exist (verified by `ls fuzz/fuzz_targets/`).
Why it matters: a wrong path in the doc that explains the fuzz wiring sends the next reader hunting a nonexistent file.
What correct looks like: fix the filename in the doc.
Confidence: HIGH — Verification: ran `ls fuzz/fuzz_targets/` (fuzz_http_headers.rs present, fuzz_http_header.rs absent).

### F-003 [S4]
Where: crates/streaming/src/mockhttp.rs:1059-1062
Code:
```
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect();
```
What is wrong: header lines without a colon are silently dropped — a malformed header line produces neither an error nor a record. The `read_response_head` contract (part2, lines 1014-1016: "Every malformation is a typed `NetworkError::ProtocolViolation`") is contradicted for this shape, and the fuzz entry (`fuzz_response_head`, whose doc at 1076-1078 promises "one **bounded, typed** parse … hostile head bytes") will return `Ok` with silently-fewer headers for garbage header lines instead of a typed refusal.
Why it matters: the fuzz harness proves no-panic (its real job), but the "typed verdicts" story oversells: hostile garbage in the header block is accepted-with-loss, not refused. The mock's grammar is also the dialect every mockhttp-based test runs over, so leniency here hides server shapes a stricter real client might refuse.
What correct looks like: `filter_map` → `map` returning `Err(ProtocolViolation { detail: "malformed header line" })` for colonless non-empty lines (blank-line terminators are already excluded by the head split).
Confidence: HIGH — Verification: read (both the parse path and the fuzz entry traced).

### F-004 [S4]
Where: crates/streaming/src/mockhttp.rs:1101-1106
Code:
```
fn content_length(headers: &[(String, String)]) -> Option<u64> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<u64>().ok())
}
```
What is wrong: a non-numeric `Content-Length` folds to `None` via `.ok()` — silently switching the read to EOF-body mode instead of refusing; duplicate `Content-Length` headers are first-match-wins with no cross-check. Both are shapes a hostile server can send and the real transport (client.rs) may treat differently — a fidelity gap between the mock dialect and the production dialect that the e2e tests then cannot catch.
Why it matters: tests over MockClient prove the mock's contract; where the mock is more forgiving than ureq, green tests overstate production readiness.
What correct looks like: `.ok()` → typed `ProtocolViolation` on parse failure; reject duplicate conflicting `Content-Length`.
Confidence: MEDIUM — Verification: read (mock path verified; the ureq client's exact strictness is A01/A06 territory — client.rs part1/part2 — flagged for them in §9).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_response_head_text` | 1041-1069 | REAL | S4 (F-003) | status-line grammar strict (version prefix, u16 code); header grammar lossy. |
| `FuzzResponseHead` (type alias) | 1074 | REAL | — | keeps crate-private struct out of the pub signature. |
| `fuzz_response_head` | 1088-1099 | REAL | — | HEAD_LIMIT bound before parse; lossy UTF-8; wired to netfuzz + libFuzzer shim. |
| `content_length` | 1101-1106 | REAL | S4 (F-004) | first-match + `.ok()` leniency. |
| `read_bounded_body` | 1112-1143 | REAL | — | budget-before-alloc; typed truncation; `expected as usize` safe (expected ≤ limit, u64 ≤ practical budgets). |
| `read_eof_body` | 1147-1174 | REAL | — | pre-extend budget check; ReadTimeout on stall; Ok on clean EOF. |
| `map_body_read_error` | 1178-1186 | REAL | — | correct kind→variant mapping. |
| `tests::body_1000` / `spawn` / `fetcher` | 1195-1207 | REAL | — | port exercised as `Arc<dyn HttpFetch>` trait object. |
| `roundtrip_200_body_equality` | 1210-1226 | REAL | — | body + reason + logged method/path. |
| `partial_content_206_with_content_range_echo` | 1229-1241 | REAL | — | exact `bytes 100-199/1000` echo + slice body. |
| `open_ended_range_reads_to_end` | 1244-1252 | REAL | — | `From { start }` shape. |
| `range_ignored_config_serves_200_full_body` | 1255-1269 | REAL | — | the no-range server shape ranged.rs's scan mode consumes. |
| `status_416_is_range_not_satisfiable` | 1272-1284 | REAL | — | variant + stable code string. |
| `redirect_chain_of_three_lands_on_final_body` | 1287-1303 | REAL | — | hop paths pinned in order. |
| `too_many_redirects_is_bounded_by_policy` | 1306-1329 | REAL | — | `limit: 2, followed: 2` exact. |
| `self_loop_is_detected_as_redirect_loop` | 1332-1345 | REAL | — | `RedirectLoop { hops: 1 }` + one wire request. |
| `read_timeout_fires_on_delayed_response` | 1348-1372 | REAL | — | 120 ms read deadline vs 400 ms delay; elapsed bound 3 s. |
| `hard_reset_mid_body_is_body_truncated` | 1375-1393 | REAL | — | expected=1000/received=400 exact. |
| `garbage_first_line_is_a_framing_error` | 1396-1413 | REAL | — | detail string + code pinned. |
| `request_log_captures_the_range_header` | 1416-1427 | REAL | — | `bytes=250-749` exact + Host present. |
| `refused_port_is_connect_refused` | 1430-1441 | REAL | — | bind/drop/refuse dance. |
| `response_over_budget_is_response_too_large` | 1444-1458 | REAL | — | `limit: 64, actual: 1000` exact. |
| `header_injection_is_rejected_typed` | 1461-1478 | REAL | — | dirty value smuggled via direct field push — pins transport-level defense. |
| `range_spec_parser_accepts_and_rejects` | 1481-1487 | REAL | — | accepts `0-499`/`100-`, rejects inverted/wrong-unit/non-numeric. |
| `scripted_route_advances_and_repeats_last` | 1493-1515 | REAL | — | advance → repeat-last → handle count → static coexist → 404 typed. |
| `scripted_status_answers_then_recovers` | 1522-1558 | REAL | — | 503→500→recovery + count + other-path untouched. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| roundtrip_200_body_equality | YES | full-body equality + reason + request log. |
| partial_content_206 / open_ended_range | YES | exact Content-Range echo strings + exact body slices. |
| range_ignored_config_serves_200_full_body | YES | status + full body vs sliced. |
| status_416 / too_many_redirects / self_loop / response_over_budget / hard_reset / garbage_first_line | YES | error variant WITH payload fields pinned (limit/followed, hops, expected/received, detail string, limit/actual). |
| redirect_chain_of_three | YES | exact 4-path hop sequence. |
| read_timeout_fires_on_delayed_response | YES | ReadTimeout variant + `is_timeout()` + wall-clock bound. |
| request_log_captures_the_range_header | YES | exact range string from the server-side log. |
| refused_port_is_connect_refused | YES | variant match on a provably-dead port. |
| header_injection_is_rejected_typed | YES | dirty header reaches the transport only via direct field assembly — the refusal must come from the client, not the builder. |
| range_spec_parser_accepts_and_rejects | YES | pure-function accept/reject table. |
| scripted_route / scripted_status | YES | sequencing + shared-handle counters + recovery semantics. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "test-only; never used by production code paths" | mockhttp.rs:1-2 | REAL (call paths) / FALSE (compilation) | rg: zero production callers; but `pub mod mockhttp;` un-gated (lib.rs:36) compiles it into the shipped crate — F-001. |
| "Every malformation is a typed `NetworkError::ProtocolViolation`" (head parsing) | mockhttp.rs:1014-1016 (part2 doc, grammar implemented in my chunk) | SHALLOW | colonless header lines silently dropped (F-003). |
| "one hostile response-head byte slice → one bounded, typed parse" | mockhttp.rs:1076-1078 | SHALLOW | bounded+no-panic: REAL; typed for header-block garbage: not (F-003). |
| "`#[path]`-included by `fuzz/fuzz_targets/fuzz_http_header.rs`" | mockhttp.rs:1080-1081 | FALSE (filename) | file is `fuzz_http_headers.rs` (ls) — F-002. |

## 7. Dead / duplicated / unreachable code
No dead code inside the chunk. `parse_range_spec` (part2, 780) is directly unit-tested here (1481-1487). The whole module is production-dead by design (see F-001 for the shipped-binary compilation caveat). `LoggedRequest::header` (part1) exercised by request_log test.

## 8. Dependencies & flags
Testkit uses `crate::auth` helpers (base64_decode, digest_response, parse_auth_params — part1's MockAuth machinery), `crate::fetch` port types, `std::net`/threads. No feature gates; no unsafe. All 18 tests green in baseline (`:5068-5086`).

## 9. Open questions for the next agent
- A01/A06 (client.rs parts): does the ureq adapter refuse non-numeric/duplicate `Content-Length` where the mock folds to EOF-mode (F-004)? If yes, mock/production dialects have drifted apart and mockhttp-based e2e tests overstate coverage.
- A05/A08 (mockhttp parts 1-2): same F-001 gating question applies to the server half (MockServer::spawn threads) — the finding is module-wide, cited from lib.rs.
- Orchestrator: a `strings`/symbol check of the release `lumen` binary for mockhttp symbols would settle whether the linker strips the testkit (I could not — the baseline binary lives in the wave-0 sandbox clone).
