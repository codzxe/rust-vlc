# AUDIT: apps/control-server/src/logging.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-21 |
| Lines / functions | 271 / 15 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES, via the `lumen-control-server` binary (NOT the `lumen` CLI): main.rs:247 `JsonStderr::new()` → `serve_loop` (main.rs:248) → run.rs:56 `sink.emit(&session_record(&summary))` per finished session + run.rs:61 `serve-failed` error record. Binary built and its unittests ran in baseline (build #1 clean; orchrun-test-workspace-nofailfast.log.txt:1471 `Running unittests src/main.rs … lumen_control_server`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Grep JsonStderr/session_record/LogSink/LogRecord/MemoryLog across apps/control-server/src (callers: run.rs:16-19/:56/:61/:118; lib.rs:47-50 re-exports); Read main.rs (JsonStderr::new at :247, serve_loop at :248); rg control-server in baseline nofailfast log (unit tests :1463-1465 green; main unittests :1471); Grep docs for session-end/JSON-lines claims → docs/security/sweep-control.md:55-69 (V8 token claim) read and cross-checked against session_record's field set; Read logging.rs in full; quotes verified via Read |

## 1. One-paragraph verdict
Small, disciplined, and honest: a 3-level severity, a builder record, serde_json-escaped one-line JSON on stderr, a memory sink for tests, and the session-summary mapping — all real, all tested, all three tests green in the baseline (orchrun log:1463-1465). The security-critical claim (token material never enters the log stream — V8) holds: `session_record` emits exactly remote/authenticated/version/counts/close, matching docs/security/sweep-control.md:61-62's claim verbatim. Only hygiene-level defects: the `session_record` doc comment misdescribes the level mapping it sits on, the record builder permits silently clobbering the reserved `ts`/`level`/`event` keys, and the stderr sink swallows write failures without documenting the best-effort contract.

## 2. What actually works (verified)
- All 3 tests green in baseline: `test logging::tests::{memory_sink_captures_in_order, records_serialize_to_stable_json_lines, session_summaries_map_to_records_with_close_kinds} ... ok` (orchrun-test-workspace-nofailfast.log.txt:1463-1465).
- JSON-line discipline pinned: trailing `\n`, `"ts":1234.5`, `"level":"warn"`, `"event":"session-end"`, field order; newline-injection hostile value produces zero raw `\n` (test :219-221) — serde_json escaping is the guard, and it is tested.
- V8 (no token in logs): `session_record` (logging.rs:176-188) emits exactly remote, authenticated, version, requests_served, ops_denied, protocol_errors, auth_failures, locked_out, close — no token path exists in this file; consistent with docs/security/sweep-control.md:60-62.
- Pre-epoch clock: `now()` → `unwrap_or_default()` → `0.0` logged as-is, never a panic (documented :123-125).
- `close_kind` stable strings pinned by test (`server-closed:1001`, :245).

## 3. Findings
### F-001 [S4]
Where: apps/control-server/src/logging.rs:162-175
Code:
```rust
/// The structured record for one finished session (the event loop's
/// per-connection heartbeat). Level: `warn` when the session ended in
/// a protocol close or auth failure, `info` otherwise.
pub fn session_record(summary: &SessionSummary) -> LogRecord {
    let level = match summary.close {
        SessionEnd::PeerClosed(_) | SessionEnd::Eof => Level::Info,
```
What is wrong: The doc comment does not describe the code. Per the comment, a "protocol close" (PeerClosed IS a protocol close — a close frame) or an "auth failure" should map to `warn`; the code maps `PeerClosed(_)` and `Eof` to `Info` unconditionally (auth failures on a peer-closed session stay `info`), and `ServerClosed` to `Warn` only when `auth_failures > 0 && !authenticated`. No path examines the close CODE, so a server-side protocol-error close with zero auth failures logs at `info`; `protocol_errors` is counted in the fields but never influences the level.
Why it matters: An operator writing level-based alerts from the comment's contract (e.g. `level=warn` for protocol closes) misses token-hammering sessions that end in peer-close and protocol-error closes; the drift is exactly where observability consumers look. (Mitigation: the raw counts are in the fields, so field-based alerting still works.)
What correct looks like: Fix the comment to the code's actual policy ("warn when the server closed an unauthenticated session with auth failures"), or fix the code to the comment (peer-closed/eof sessions with `auth_failures > 0` or `protocol_errors > 0` → warn).
Confidence: HIGH — Verification: read; both tests pin the CODE's mapping (:239 Info for clean ServerClosed, :259 Warn for auth-failed), never the comment's.

### F-002 [S4]
Where: apps/control-server/src/logging.rs:60-63, 79-81
Code:
```rust
    pub fn with(mut self, key: &'static str, value: impl Into<String>) -> LogRecord {
        self.fields.push((key, value.into()));
        self
    }
```
and in `to_json`:
```rust
        for (key, value) in &self.fields {
            object.insert((*key).to_owned(), serde_json::Value::String(value.clone()));
        }
```
What is wrong: No reserved-key guard. A call site writing `.with("level", …)`, `.with("event", …)` or `.with("ts", …)` silently OVERWRITES the built-in (`Map::insert` replaces), and two `.with("remote", …)` calls silently last-wins — the module's "stable keys" contract (:3-4) is unenforced. Today's call sites (run.rs:61, session_record) are all hard-coded and collision-free, so there is no live defect — the guard exists for the next contributor.
Why it matters: Structured-log consumers key on exactly these names; a future field named `level` would corrupt the severity column of every record with no error anywhere.
What correct looks like: `debug_assert!(!matches!(key, "ts" | "level" | "event"))` in `with`, or prefix/reject reserved keys, or insert fields BEFORE the built-ins so built-ins always win.
Confidence: HIGH — Verification: read (serde_json Map::insert replace semantics are documented).

### F-003 [S4]
Where: apps/control-server/src/logging.rs:134-139
Code:
```rust
impl LogSink for JsonStderr {
    fn emit(&mut self, record: &LogRecord) {
        let _ = write!(self.inner, "{}", render_line(record, Self::now()));
        let _ = self.inner.flush();
    }
}
```
What is wrong: Both the write and the flush discard their results — a closed stderr, ENOSPC, or a broken pipe silently loses the record with no counter, no fallback, no note. For a session-summary sink this is conventional best-effort logging and the trait doc does say implementations must not panic (:95), but the best-effort contract (records may vanish) is stated nowhere, and unlike the rest of this codebase's counting discipline (cf. mailbox stats, bench concealment counts), drops are unobservable.
Why it matters: The one component whose entire job is observability degrades invisibly; an operator debugging "missing session records" has no signal pointing at the sink.
What correct looks like: A `failed_writes: u64` counter on `JsonStderr` surfaced somewhere (even a single stderr retry), or one doc line: "emit is best-effort; write failures are dropped silently by design".
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Level` + `as_str` | 16-35 | REAL | — | Three stable strings; exhaustive match. |
| `LogRecord::new` | 49-57 | REAL | — | Empty-fields builder root. |
| `LogRecord::with` | 59-63 | REAL | S4 (F-002) | No reserved-key guard. |
| `LogRecord::to_json` | 65-83 | REAL | S4 (F-002) | ts/level/event first, fields replace on collision; `to_value(ts)` Ok for every finite f64 (NaN arm silently omits ts — defensive, unreachable via `now()`). |
| `render_line` | 86-93 | REAL | — | Public for the app's line-discipline tests — used by tests here and pinnable elsewhere. |
| `LogSink` trait | 95-99 | REAL | — | Emit-only, no panic contract documented beyond the doc line. |
| `JsonStderr` + `Default`/`new` | 104-132 | REAL | — | Wraps `io::Stderr` (line-buffered by the flush). |
| `JsonStderr::now` | 123-131 | REAL | — | Pre-epoch → 0.0 documented; f64 seconds carry far more than the "millisecond precision" the comment promises (understatement, harmless). |
| `JsonStderr::emit` | 134-139 | REAL | S4 (F-003) | Silent best-effort; one line + flush per record. |
| `MemoryLog` + `new` + `emit` | 143-160 | REAL | — | Clone-per-record capture; test-only, fine. |
| `session_record` | 162-189 | REAL | S4 (F-001) | Field set matches the security doc's V8 claim exactly; level mapping contradicts its own doc comment. |
| `close_kind` | 191-199 | REAL | — | `unwrap_or_default` on the peer-close code (default = 1000-shaped); "no dynamic payload" holds — codes are static formats. |
| `tests::records_serialize_to_stable_json_lines` | 206-222 | REAL | — | Shape + escaping pinned. |
| `tests::session_summaries_map_to_records_with_close_kinds` | 224-260 | REAL | — | Both level arms + close kinds + fields pinned. |
| `tests::memory_sink_captures_in_order` | 262-270 | REAL | — | Order pinned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| records_serialize_to_stable_json_lines | YES | Exact key substrings + trailing newline + zero-raw-newline injection check. |
| session_summaries_map_to_records_with_close_kinds | YES | Both level outcomes, `server-closed:1001`, `requests_served` value asserted. |
| memory_sink_captures_in_order | YES | Length + order + level access. |
| Mutation NOT caught | NO | A doc-matching change (peer-closed-with-auth-failures → Warn) would FAIL the current tests — the tests pin the code, the comment is the odd one out (F-001); no test can catch a comment. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Token material never enters … the structured log stream (`logging.rs` fields: remote/authenticated/counts/close only — V8)" | docs/security/sweep-control.md:60-62 | REAL | session_record :176-188 emits exactly that field set; no token path in the file. |
| "One JSON object per record on the sink — machine-parseable, stable keys" | module docs :3-4 | REAL (unenforced) | render_line + test; "stable keys" has no guard (F-002). |
| "the production sink writes one line per record with a trailing newline flush" | module docs :7-8 | REAL | emit :134-139. |
| "Level: `warn` when the session ended in a protocol close or auth failure, `info` otherwise" | doc comment :163-164 | FALSE | F-001: code maps PeerClosed/Eof → Info unconditionally; ServerClosed → Warn only for auth-failed unauthenticated sessions; close codes and protocol_errors never consulted. |
| "exit 4 — the serve loop failed" + serve-failed error record | main.rs:9 + run.rs:61 | REAL (run.rs owner's lane) | LogRecord::new(Level::Error, "serve-failed") wired in run.rs:61. |

## 7. Dead / duplicated / unreachable code
- Nothing dead. `MemoryLog` and `render_line`'s pub-ness are test-driven by design (documented :86-88). The NaN arm of `to_value(ts)` is unreachable via the only ts source (`now()` always finite) — defensive, harmless.

## 8. Dependencies & flags
- `serde_json` (the command layer's own wire-format family per Cargo.toml), `control_protocol` (SessionSummary/SessionEnd). No feature gates; ships in the `lumen-control-server` binary (baseline #1 built clean).

## 9. Open questions for the next agent
- run.rs owner (W13 sibling): does any serve-loop error string (run.rs:61 `source.to_string()`) ever embed client-controlled data? The security doc's redaction claims cover the hello path; the serve-failed record relies on the error source being listener IO only.
- e2e.rs owner: is the stderr line discipline (render_line format) asserted end-to-end through the REAL binary, or only via lib-level tests? Baseline ran the e2e suite (control_server target :1454) — confirm it captures the sink, not real stderr.
- If alerting rules are ever written against `level`, F-001's policy question (peer-closed auth-failures at info) should be settled before the first consumer depends on it.
