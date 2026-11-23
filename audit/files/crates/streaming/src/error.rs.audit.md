# AUDIT: crates/streaming/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 486 (337 production + 149 tests) / 8 production fns + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli demux_host.rs:427 http arm → open_http_source → RangedResource/schemes/UreqClient, all of which construct `NetworkError` on every failure path (e.g. ranged.rs:204 `map_fetch_err(err: NetworkError)`, client.rs:143 `map_ureq_error`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed -n (full read), rg (is_retryable callers, baseline), baseline citations |

## 1. One-paragraph verdict
A clean, complete typed error vocabulary: 20 variants, each with a distinct stable dotted code (pinned by a table test over the whole enum), structured secret-free Display (verified by exact-string assertions), `io::Error` preserved in the source chain, `Send + Sync` proven by a thread-crossing test. The retry table (`is_retryable`) is the security- and correctness-relevant core and it is right: TLS verification, redirects, 416, non-retryable 4xx, over-budget, and generation-cancelled fetches are all non-retryable; only deadline/DNS/refused/socket-weather/BodyTruncated and the 408/429/500/502/503/504 status classes retry. Two hygiene notes only: `Io { .. }` is unconditionally retryable (non-weather `io::ErrorKind`s like `Unsupported`/`InvalidData` get retried before failing), and this file's careful `is_client_error`/`is_server_error` classification is thrown away by ranged.rs's `map_fetch_err` ( HttpStatus → `ConnectionLost` ) — the information this vocabulary exists to carry dies at the bridge (cross-referenced as F-001 S2 in the ranged.rs audit).

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` (PACKAGE streaming): all 5 `error::tests::*` pass ("212 passed; 0 failed; 2 ignored" for the package lib).
— verified by read: `codes_are_stable_and_distinct` (L345-393) constructs all 20 variants and asserts code uniqueness + `streaming.net.` prefix; `error_is_send_and_sync` (L476-485) actually moves an error across a thread; the Display test pins exact output strings (L443-455).

## 3. Findings
F-001 [S4]
Where: crates/streaming/src/error.rs:225-236
Code:
```
    pub fn is_retryable(&self) -> bool {
        match self {
            NetworkError::DnsFailure
            | NetworkError::ConnectTimeout
            | NetworkError::ReadTimeout
            | NetworkError::WriteTimeout
            | NetworkError::ConnectRefused
            | NetworkError::BodyTruncated { .. }
            | NetworkError::Io { .. } => true,
```
What is wrong: `Io { .. }` is retryable regardless of the `io::ErrorKind` — a non-weather kind (`Unsupported`, `InvalidData`, `PermissionDenied` from a broken local configuration) is retried 4 times (RetryFetch default) before surfacing. Wasted seconds on deterministic failures; no wrong data. What correct looks like: retry only connection-class kinds (reset/aborted/broken-pipe/unreachable). Confidence: MEDIUM — Verification: read; no test pins Io-retryability against a specific ErrorKind.

F-002 [S4]
Where: crates/streaming/src/error.rs:70-78 (flags) vs crates/streaming/src/ranged.rs:211-217 (consumer)
Code:
```
    /// A non-success HTTP status surfaced as an error (`http-status`).
    HttpStatus {
        /// The status code (e.g. 404).
        code: u16,
        /// `code` is in 400..=499.
        is_client_error: bool,
```
What is wrong: the variant carries exactly the classification a caller needs to distinguish permanent from transient — and the only production bridge (ranged.rs `map_fetch_err`) discards code and flags alike, mapping every status to `NetworkCause::ConnectionLost` (see ranged.rs audit F-001, S2). The vocabulary is fine; its consumer destroys it. Filed here so the next agent fixing the bridge knows the data already exists. Confidence: HIGH — Verification: read both sites.

Checked and clean (zero-finding duty): every variant has a code (table test); Display leaks no URL/header/body text (exact-string test); source chain preserved for Io only (correct — other variants carry no OS error); `http_status` flag math (400..=499 / 500..=599) matches its test matrix incl. 302/499 boundaries; no unwrap/expect/panic; no secrets; AuthSchemeKind is a closed static vocabulary (Basic/Digest), never server-controlled strings.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AuthSchemeKind::as_str | 150-155 | REAL | — | static tokens |
| NetworkError::code | 160-183 | REAL | — | 20 distinct codes, table-tested |
| NetworkError::http_status | 187-193 | REAL | — | flag computation boundary-tested |
| NetworkError::is_timeout | 196-201 | REAL | — | three deadline variants |
| NetworkError::is_retryable | 225-250 | REAL | S4 | F-001 Io unconditional; 416/TLS/redirects/Cancelled correctly false |
| NetworkError::status_code | 255-260 | REAL | — | |
| Display::fmt | 264-326 | REAL | — | structured, code suffix, no caller data |
| Error::source | 330-335 | REAL | — | Io chain preserved |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| codes_are_stable_and_distinct | YES | all 20 variants, dedup + prefix assertions |
| http_status_classifies_client_and_server | YES | 400/404/416/499/503/302 matrix |
| display_is_structured_with_code_suffix | YES | exact full-string equality |
| io_variant_keeps_the_source_chain | YES | asserts source() contains OS text |
| error_is_send_and_sync | YES | real thread::spawn move |
All 5 baseline-passing.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Variants never carry URLs, header values, or body bytes" | error.rs:15-16 | REAL | enum survey L28-135; UnsupportedScheme carries a scheme token only; tests pin Display strings |
| Retry table doc (408/429/500/502/503/504 yes; TLS/redirects/416/other 4xx no) | error.rs:203-224 | REAL | L225-250 matches the doc exactly |
| "never retried on certificate mismatch — that would be a downgrade" | error.rs:39-41 | REAL | TlsFailure → false (L238) |

## 7. Dead / duplicated / unreachable code
None. All variants constructed somewhere (client.rs/auth.rs/mockhttp/ranged.rs); `Cancelled` is produced by the reconnect layer (6.D2).

## 8. Dependencies & flags
std-only (io/fmt/error). No features, no unsafe, no external crates.

## 9. Open questions for the next agent
- Should `is_retryable` consult `io::ErrorKind` (F-001)? Check `crate::reconnect::RetryFetch` tests for an Io-kind matrix before changing.
- The ranged.rs bridge (F-002 / ranged F-001) is where a fix lands: map `HttpStatus{is_client_error: true}` to a non-retryable cause and thread the code into `ErrorContext`.
