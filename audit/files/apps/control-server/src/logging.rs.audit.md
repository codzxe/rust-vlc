# AUDIT: apps/control-server/src/logging.rs
| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-24 |
| Lines / functions | 271 / 14 prod items + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | From its own bin YES: `lumen-control-server` main.rs:247 `JsonStderr::new()` -> `serve_loop` (run.rs:56 `sink.emit(&session_record(&summary))`, run.rs:65 serve-failed record). From the release lane (lumen-cli + desktop) NO: no app depends on control-server (rg over apps/*/Cargo.toml), release.yml has no control-server reference; its tests run only in CI's `cargo test --workspace --all-features` lane (ci.yml:234; baseline exit 0: lib 11 passed, e2e 6 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `cargo test -p control-server --lib logging::` -> 3 passed, exit 0; scratch program under audit/runs/W02-A10.scratch/jsonmap with the LOCKED serde_json 1.0.154 -> `{"addr":"127.0.0.1:0","event":"startup","level":"info","ts":1234.5}` (exit 0); rg wiring checks (main.rs/run.rs/e2e.rs, release.yml, apps manifests); Read-tool quote verification. |

## 1. One-paragraph verdict
Real, small, and correctly wired: a `LogSink` trait with a production stderr JSON-lines sink and an in-memory test sink, a `session_record` mapping used per-connection by the serve loop, and honest tests including an e2e that parses the rendered line and asserts the token never appears. The JSON-escape discipline is tested against hostile input. All defects found are documentation-level falsehoods and one API sharp edge: the doc claims "ts first" key ordering that serde_json's BTreeMap-backed `Map` cannot produce (I proved the actual alphabetical order with the exact locked crate version), the `session_record` doc mis-describes its own level rule, swallowed write/flush errors are a documented-acceptable risk, and `with("event"/"level"/"ts", ...)` can silently clobber envelope keys in the rendered JSON. No S0-S3 findings.

## 2. What actually works (verified)
- 3/3 logging unit tests pass — `cargo test -p control-server --lib logging::` -> `test result: ok. 3 passed; 0 failed` (stable JSON keys, newline discipline, hostile-value escaping, level mapping, close kinds, memory-sink ordering).
- Production wiring is live: main.rs:247 -> run.rs serve_loop -> `session_record` per session and a `Level::Error` "serve-failed" record on loop failure (run.rs:56-66).
- V8 (no token in the log stream) is enforced by test: e2e.rs:408-410 asserts no field key equals "token"; `session_record`'s field list (logging.rs:177-188) carries no token.
- The rendered line is machine-parseable: e2e.rs:424 `line.trim().parse::<serde_json::Value>()` succeeds and ts/level/event/addr values are asserted.
- Escaping: the in-file test feeds `"x\"\ny"` and asserts zero raw newlines in the JSON — log-injection discipline is tested, not assumed.

## 3. Findings
F-001 [S4]
Where: apps/control-server/src/logging.rs:65
Code:
```
    /// The record as a JSON object (ts first, then level/event/fields).
```
What is wrong: the claimed key order is impossible. `serde_json::Map` without the `preserve_order` feature is a `BTreeMap`, so `to_string()` emits keys alphabetically. Proof with the exact locked version (Cargo.lock: serde_json 1.0.154, deps itoa/memchr/serde/serde_core — no indexmap), scratch run output: `{"addr":"127.0.0.1:0","event":"startup","level":"info","ts":1234.5}` — `ts` is LAST, not first. Why it matters: it is a false statement about the production wire format in the one place a consumer would look; harmless in practice (JSON object order is not semantic and both tests use `contains`/index access). What correct looks like: delete the ordering claim or enable `preserve_order` if order is ever wanted. Confidence: HIGH — Verification: ran (scratch program, output pasted above).
F-002 [S4]
Where: apps/control-server/src/logging.rs:162-174
Code:
```
/// The structured record for one finished session (the event loop's
/// per-connection heartbeat). Level: `warn` when the session ended in
/// a protocol close or auth failure, `info` otherwise.
```
What is wrong: the doc's level rule does not match the code below it. The code escalates to `Warn` only for `ServerClosed` sessions with `auth_failures > 0 && !authenticated`; a server-side protocol-error close (the natural reading of "protocol close") and any session with `protocol_errors > 0` stays `Info`. `PeerClosed` is always `Info` regardless of why the peer went away. Why it matters: an operator tuning alerts off the doc (rather than the code) will expect warns that never come. What correct looks like: fix the doc to state the actual rule, or make `protocol_errors > 0` escalate if that was intended. Confidence: HIGH — Verification: read (rule quoted verbatim); the tests pin the CODE's rule, not the doc's.
F-003 [S4]
Where: apps/control-server/src/logging.rs:134-139
Code:
```
impl LogSink for JsonStderr {
    fn emit(&mut self, record: &LogRecord) {
        let _ = write!(self.inner, "{}", render_line(record, Self::now()));
        let _ = self.inner.flush();
    }
}
```
What is wrong: write and flush errors are silently dropped — with stderr a closed pipe, every record vanishes with no signal anywhere. Why it matters: the checklist flags swallowed errors; here it is the deliberate lesser evil (a log sink cannot report its own failure without recursing, and panicking in `emit` is banned by the documented contract at line 95). Recorded as accepted risk, not as a demand for change. What correct looks like: at most a one-shot eprintln fallback or a process-global "logging broken" flag for the serve loop to see. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/control-server/src/logging.rs:79-81
Code:
```
        for (key, value) in &self.fields {
            object.insert((*key).to_owned(), serde_json::Value::String(value.clone()));
        }
```
What is wrong: field keys are unvalidated, and `Map::insert` replaces — `.with("event", ...)` / `.with("level", ...)` / `.with("ts", ...)` silently overwrites the envelope's own keys in the rendered JSON while `record.event`/`record.level` (what MemoryLog consumers assert) keep the true values: the rendered line and the record disagree. Duplicate keys are likewise allowed (last wins in JSON, both retained in `fields`). No current caller passes those keys (session_record and the tests use distinct names). What correct looks like: reject or prefix reserved keys in `with`. Confidence: HIGH — Verification: read (insert-replace semantics; no current trigger).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Level / Level::as_str | 16-35 | REAL | — | stable lowercase strings, tested. |
| LogRecord (struct/new/with) | 39-63 | REAL | S4 | builder; F-004 reserved-key hazard. |
| LogRecord::to_json | 66-83 | REAL | S4 | alphabetical key order (F-001); `if let Ok` on to_value(f64) is dead defensiveness (cannot fail). |
| render_line | 89-93 | REAL | — | JSON + '\n'; public for tests; e2e parses it. |
| LogSink (trait) | 96-99 | REAL | — | &mut self, no panics contract. |
| JsonStderr / Default / new | 104-121 | REAL | — | thin stderr holder. |
| JsonStderr::now | 126-131 | REAL | — | f64 secs; pre-epoch -> 0.0, documented; "millisecond precision" understates as_secs_f64. |
| LogSink for JsonStderr | 134-139 | REAL | S4 | F-003 swallowed errors (accepted risk). |
| MemoryLog / new | 143-160 | REAL | — | clone-capture; used by run.rs tests + e2e. |
| session_record | 165-189 | REAL | S4 | full summary field set; level rule contradicts its doc (F-002); no token (V8, e2e-enforced). |
| close_kind | 193-199 | REAL | — | "peer-closed:{}"/"server-closed:{}"/"eof"; pinned by tests. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| records_serialize_to_stable_json_lines | YES | key renames/drops, missing trailing '\n', newline injection all fail it; would NOT fail on key reordering (contains-only) — which is why F-001 survived. |
| session_summaries_map_to_records_with_close_kinds | YES | pins level for auth-failed (Warn) and clean (Info) summaries plus close-kind formatting. |
| memory_sink_captures_in_order | YES (weakly) | catches dropped/reordered emits. |
| (cross-file) e2e.rs the_production_sink_renders_one_parseable_json_line_per_record + session assertions | YES | parses the real rendered line, asserts values, asserts requests_served/ops_denied/authenticated, asserts no "token" key. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One JSON object per record on the sink — machine-parseable, stable keys" | lines 2-3 | REAL | e2e.rs:424 parses; key set pinned by tests. |
| "no token material (V8: the token's single sanctioned reader is the controlling-terminal print in main, never the log stream)" | lines 3-5 | REAL | field list has no token; e2e.rs:408-410 asserts it. |
| "writes one line per record with a trailing newline flush" | lines 7-8 | REAL | emit(): write + flush; '\n' asserted. |
| "(ts first, then level/event/fields)" | line 65 | FALSE | F-001: locked serde_json emits alphabetical; scratch run output above. |
| "Level: warn when the session ended in a protocol close or auth failure, info otherwise" | lines 163-164 | FALSE | F-002: code warns only on ServerClosed + auth_failures>0 + !authenticated. |

## 7. Dead / duplicated / unreachable code
None dead. `render_line` is pub-for-tests but also the production emit path (JsonStderr::emit calls it) — not dead. MemoryLog is test-only but lives in the lib (line 141) — acceptable seam, used by run.rs inline tests and e2e.rs.

## 8. Dependencies & flags
control-protocol (SessionSummary/SessionEnd/CloseCode), serde_json 1.0 (no features — the root of F-001), std only otherwise. No unsafe, no feature gates. Crate lints = workspace.

## 9. Open questions for the next agent
- Is `lumen-control-server` ever shipped in a release artifact? release.yml never builds it; if it is product-surface, its absence from the release lane is a packaging gap owned elsewhere.
- The catalog ledger's SystemLog row says "no logging bridge" (engine side) — this file is the control-server's own logging, a different stream; whoever audits doc 10 §5 should confirm no one assumes this file feeds `system.log` events.
