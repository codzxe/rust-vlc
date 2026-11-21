# AUDIT: crates/streaming/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 487 / 12 items in table |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `NetworkError` is constructed on the shipped HTTP lane: `UreqClient` maps ureq errors to it (client.rs:165-166 `ureq::Error::Rustls(_) \| ureq::Error::Tls(_) \| ureq::Error::Pem(_) => NetworkError::TlsFailure`, read), and the lane is `lumen open <http-url>` → demux_host.rs:491 `SchemeRegistry::new().open_http` → `RangedResource` → `UreqClient`. `is_retryable` is consumed by the reconnect layer (reconnect.rs:736, read). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "is_retryable" crates/streaming/src"` (reconnect.rs:736 consumer); `rg -n "pub fn get|pub fn post|HttpMethod" crates/streaming/src/fetch.rs` (GET-only port verified — fetch.rs:33-43 "Only GET exists for v0"); `sed -n '165,166p' crates/streaming/src/client.rs` (TLS→variant mapping); baseline: orchrun-test-workspace-nofailfast.log.txt:4941-4944 (streaming 214 unit tests) + 5/5 `error::tests` ok (lines within that run, verified by awk over 4941-5120). |

## 1. One-paragraph verdict
A disciplined, thiserror-free transport error vocabulary: 20 variants, each with a stable `streaming.net.*` code, structured `Display`, OS errors preserved in the `source` chain, no URLs/header values/body bytes carried (the no-secrets claim holds — every payload field is a count, a status number, a static `&'static str`, or a lowercased scheme name). The retryable-weather table is the one soft spot: `Io { .. } => true` retries every OS error including deterministic local failures the doc's own justification ("resets, aborts") does not cover. All five in-file tests are real and pass in baseline; the vocabulary is live code on the shipped HTTP lane.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` (streaming lib run at 4941: "running 214 tests", 0 failures) → all 5 `error::tests` pass: `codes_are_stable_and_distinct`, `http_status_classifies_client_and_server`, `display_is_structured_with_code_suffix`, `io_variant_keeps_the_source_chain`, `error_is_send_and_sync`.
— verified by read: `is_retryable`'s GET-only justification is true — `fetch.rs:33-43` declares `HttpMethod` with `Get` as "the only method the streaming layer issues for v0", so re-issue-after-failure is idempotent as claimed.
— verified by read: the TLS-failure mapping exists and is reachable code (`client.rs:165-166`).

## 3. Findings

### F-001 [S4]
Where: crates/streaming/src/error.rs:227-233
Code:
```
            NetworkError::DnsFailure
            | NetworkError::ConnectTimeout
            | NetworkError::ReadTimeout
            | NetworkError::WriteTimeout
            | NetworkError::ConnectRefused
            | NetworkError::BodyTruncated { .. }
            | NetworkError::Io { .. } => true,
```
What is wrong: every `io::Error` is classified retryable, but the doc comment justifies the arm as "raw socket I/O (resets, aborts)". Deterministic local failures — `ErrorKind::InvalidInput`, `Unsupported`, a broken pipe caused by this side — are not weather; retrying them burns `max_retries + 1` attempts plus backoff sleeps for a failure whose next outcome is identical.
Why it matters: bounded (RetryPolicy caps attempts and the clock is injectable), so no hang — but each pointless retry costs a full reconnect + deadline wait on the shipped open lane, and the doc/code mismatch is exactly the kind of drift this audit exists to catch.
What correct looks like: match on `source.kind()`: retry `ConnectionReset | ConnectionAborted | TimedOut | WouldBlock | UnexpectedEof`; refuse `InvalidInput | InvalidData | Unsupported`.
Confidence: HIGH — Verification: read; the module's own test constructs `Io { source: io::Error::new(io::ErrorKind::Unsupported, …) }` (382) — an error the table happily retries.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL | — | No-secrets claim verified against every variant payload (read). |
| `NetworkError` (enum, 20 variants) | 27-135 | REAL | — | Payloads: counts, u16 status, static detail, lowercased scheme, io::Error — no caller data. |
| `AuthSchemeKind` + `as_str` | 140-156 | REAL | — | Static vocabulary, matches auth.rs's Basic/Digest scope. |
| `NetworkError::code` | 158-183 | REAL | — | 20 distinct codes, prefix-checked by test. |
| `NetworkError::http_status` | 185-193 | REAL | — | Class flags computed, pinned by test. |
| `NetworkError::is_timeout` | 195-201 | REAL | — | Three deadline variants only; correct. |
| `NetworkError::is_retryable` | 203-250 | REAL | S4 (F-001) | Weather table otherwise sound: TLS/redirects/416/framing/4xx/cancelled refused; 408/429/500/502/503/504 retried; GET-only port verified. |
| `NetworkError::status_code` | 252-260 | REAL | — | 416→None documented quirk (folds into RangeNotSatisfiable). |
| `Display for NetworkError` | 263-327 | REAL | — | Structured, code suffix; "unexpected status" arm is defensive (client maps only non-success). |
| `Error for NetworkError` | 329-336 | REAL | — | Io source preserved, pinned by test. |
| `tests::codes_are_stable_and_distinct` | 344-393 | REAL | — | All 20 variants, dedup + prefix asserts. Would fail on a code collision. |
| `tests::http_status_classifies_client_and_server` | 396-433 | REAL | — | 4xx/5xx/3xx boundaries + is_timeout matrix. |
| `tests::display_is_structured_with_code_suffix` | 437-456 | REAL | — | Exact Display strings. |
| `tests::io_variant_keeps_the_source_chain` | 459-471 | REAL | — | Real chain walk. |
| `tests::error_is_send_and_sync` | 475-485 | REAL | — | Bounds + thread crossing. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| codes_are_stable_and_distinct | YES | Dedup over all 20 codes; a duplicated or renamed code fails. |
| http_status_classifies_client_and_server | YES | Exact flag matrix incl. 499/503 boundaries. |
| display_is_structured_with_code_suffix | YES | Byte-exact Display match. |
| io_variant_keeps_the_source_chain | YES | Asserts the OS text survives `source()`. |
| error_is_send_and_sync | YES for bounds | Compile-time + runtime thread crossing. |
| (gap) is_retryable has NO in-file test | — | The weather table (20 arms) is untested here; reconnect.rs's tests cover it indirectly (retry weather e2e in schemes.rs/reconnect tests). An in-file table test (like codes) would pin the classification directly. Noted, not counted as a finding. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Variants never carry URLs, header values, or body bytes" | error.rs:15-17 | REAL | Every payload field inspected (read): counts/status/static str/lowercased scheme/io::Error. |
| "GET is idempotent, which is what makes re-issue correct" | error.rs:212-213 | REAL | fetch.rs:33-43 — HttpMethod::{Get} only (read). |
| "never retried on certificate mismatch — that would be a downgrade" | error.rs:39-41 | REAL | is_retryable:238 — TlsFailure in the false arm (read). |
| "the bridge into the engine taxonomy … is card 6.A1c's job" | error.rs:8-11 | REAL | client.rs mints EngineError from these variants (read of mapping sites); CLI wire_error surfaces them. |

## 7. Dead / duplicated / unreachable code
Nothing dead. `code()` strings are duplicated inside `Display` (house style, deliberate). The `HttpStatus` Display arm's "unexpected status" branch is unreachable through the current client (only non-2xx/3xx map to the variant) — defensive, fine.

## 8. Dependencies & flags
std only (`std::error`, `fmt`, `io`). No thiserror (deliberate house style, doc at 3-5). No feature gates. No unsafe. 39/39 lint inheritance per baseline hygiene.

## 9. Open questions for the next agent
- A09/A10 (reconnect.rs parts): does RetryFetch re-issue on `Io(Unsupported)` (F-001) with full backoff? If the policy's sleeps are wall-clock in production (SystemClock), a deterministic local failure adds latency to error surfacing on the open lane.
- A01/A10 (client.rs parts): confirm `UnsupportedScheme.scheme` is lowercased at every construction site (the field doc promises it; schemes.rs does, client-side sites not yet audited by me).
