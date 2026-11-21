# AUDIT: crates/control-protocol/src/error.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-21 |
| Lines / functions | 327 / 16 (2 enums, 8 methods/impls, 6 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it — same proof as frame.rs#part1: only consumer apps/control-server is product-gated off, product.toml:167, and absent from the deb/rpm payload) |
| Reachable from shipped entry point? | NO (no shipped app depends on control-protocol; `rg -n "to_command_error\|is_recoverable" crates apps` → zero production callers repo-wide, only error.rs's own tests) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -n "to_command_error" . --glob '!audit/**'` (definition + tests only — zero host callers); `rg -n "is_recoverable" crates apps` (same: zero callers); `rg -n "BadEnvelope" crates/control-protocol/src` (constructed only by client.rs, the in-crate conformance client); `sed -n '596,650p' + '846,880p' crates/control-protocol/src/server.rs` (the server's ACTUAL recoverable policy: `fn recoverable` at 813 with BadJson/InvalidRequest/UnknownType/BadFrame; fail_session → close_with(err.close_code()) at 866); `rg -n "fn redact_name" -A 6 crates/control-protocol/src/protocol.rs` (64-char cap + ellipsis — the V8 redaction claim's mechanism); `rg -n "#\[test\]" crates/control-protocol/src/error.rs` (6 tests, inside the baseline-executed 47/47). |

## 1. One-paragraph verdict
A clean, well-documented 14-variant error taxonomy with correct RFC 6455 §7.4.1 close-code mappings where they matter (1002/1007/1009, the 4400+ app range per doc 05 §5.4), a real redaction discipline (Unauthorized carries no token material — verified: the variant is a bare `bool`; UnknownProtocol routes through `protocol::redact_name`'s 64-char cap), and six mapping tests that ran in the baseline (part of the 47/47 control_protocol unittest pass). The rot is in the wiring, not the math: the module's headline feature — `to_command_error`, advertised in the header as the bridge that makes "hosts answer with the same vocabulary every other host uses" — has zero production callers in the entire repo; `is_recoverable` is equally dead AND wrong-by-comparison (it claims only BadEnvelope survives, while the live server recovers BadJson/InvalidRequest/UnknownType/BadFrame through a parallel mechanism that never consults it). Mapping coverage in tests is partial (3 of 14 close_code arms asserted), and an OS I/O failure closes the WebSocket with 1000 "normal" instead of an honest 1011-class code.

## 2. What actually works (verified)
— All 6 tests executed in baseline: `audit/runs/baseline/test-workspace.log.txt:1411-1412` "Running unittests src/lib.rs (control_protocol…)" → "test result: ok. 47 passed; 0 failed" (error.rs contributes 6 of the 47; census 3+4+6+5+3+6+3+6+11 = 47 matches the log exactly). — Close-code values pinned by test `close_codes_are_the_documented_values` (error.rs:263-273): 1000/1001/1002/1007/1009/4400/4401/4402 — all correct against RFC 6455 §7.4.1 and doc 05 §5.4. — V8 redaction claim verified by reading: no variant carries token material; Unauthorized Display is the literal "token rejected" (error.rs:233-238).

## 3. Findings

### F-001 [S3]
Where: crates/control-protocol/src/error.rs:3-7 (claim) and 173-176 (the dead method)
Code:
```rust
//! Every failure mode of the control plane is a variant here — never a
//! stringly error, never a panic. [`ProtocolError::to_command_error`]
//! projects the taxonomy onto the ONE command layer's `CommandError`
//! (stage [`Stage::Control`], doc 05 §2.13) so hosts answer with the
//! same vocabulary every other host uses.
```
What is wrong: `to_command_error` has ZERO production callers in the repository. `rg -n "to_command_error" . --glob '!audit/**'` finds only the definition (173), the module-doc mention (4), and three test calls (278/290/311). The one host that exists (apps/control-server) never calls it — its error path is `fail_session` → app-level JSON error frames + close, built from a parallel `ProtocolErrorCode` enum in protocol.rs.
Why it matters: The module header sells this method as the doc 05 §2.13 bridge that unifies host vocabularies. It is aspirational fiction today: the bridge exists, is tested, and is wired to nothing. When the control-server eventually ships (Phase 5+), whoever believes the header will assume vocabulary unification that no code performs.
What correct looks like: Either call it (e.g., in the control-server's error-frame construction, replacing the ad-hoc code mapping at server.rs:855-863) or rewrite the header to "[planned] once hosts consume it".
Confidence: HIGH — Verification: repo-wide rg census + read of server.rs:846-867.

### F-002 [S3]
Where: crates/control-protocol/src/error.rs:166-168
Code:
```rust
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ProtocolError::BadEnvelope { .. })
    }
```
What is wrong: Zero production callers (repo-wide rg). Worse, it is not the policy the server implements: server.rs's live recoverable path (`fn recoverable` at 813, invoked at 621/646/674 with `ProtocolErrorCode::BadJson`/`InvalidRequest`/`UnknownType`, plus the BadFrame arm seen at server.rs:583-590) continues the session after several error kinds, while this method answers "only BadEnvelope" — and `BadEnvelope` itself is constructed exclusively by client.rs (the conformance test client), never by the server.
Why it matters: Two divergent definitions of "recoverable" in one crate. A future maintainer calling `is_recoverable` to decide session fate gets a different answer than the shipped server loop; the method is a trap, not a helper.
What correct looks like: Delete it, or make it the single source of truth the server's `recoverable()` consults (and extend it to the app-level error kinds the server actually recovers from).
Confidence: HIGH — Verification: rg census + read of server.rs:613-650, 813+, 846-867.

### F-003 [S4]
Where: crates/control-protocol/src/error.rs:148-151
Code:
```rust
        match self {
            ProtocolError::NonLoopbackBind { .. }
            | ProtocolError::Io { .. }
            | ProtocolError::Closed => CloseCode::Normal,
            ProtocolError::TimedOut { .. } => CloseCode::GoingAway,
```
What is wrong: An OS-level I/O failure (socket broke mid-write, accept error) maps to CloseCode::Normal (1000 — "normal, deliberate close"). server.rs:866 `close_with(stream, err.close_code())` really does emit that close frame. RFC 6455 §7.4.1 reserves 1011 (internal error) for exactly this "server encountered an unexpected condition" case; the enum cannot even represent 1011.
Why it matters: The peer's telemetry will record transport failures as clean, deliberate disconnects — the 1000 code lies about what happened. (NonLoopbackBind never reaches a socket, and Closed mirrors the peer's own close — those two are defensible; Io is not.)
What correct looks like: `ProtocolError::Io { .. } => CloseCode::InternalError` (add a 1011 variant; keep Closed → Normal).
Confidence: HIGH — Verification: read + server.rs:866-875 close_with body.

### F-004 [S4]
Where: crates/control-protocol/src/error.rs:71-73 (variant doc) vs 202 (reality)
Code:
```rust
    /// OS-level I/O failure (bind/listen/accept/read/write). The inner
    /// std error is preserved for the log sink; `Display` stays
    /// redaction-safe (no paths beyond the operation name).
```
(error.rs:202: `ProtocolError::Io { what, source } => write!(f, "{what}: {source}")`)
What is wrong: The doc says Display is redaction-safe with "no paths beyond the operation name", but Display prints the full `{source}` — the entire OS error string, which is whatever the platform error carried, not just the operation name.
Why it matters: The claim is enforced by nothing; a future io::Error that does carry a path (e.g., a TLS or filesystem-backed source added to this server) would leak it into every log line while the comment vouches that it cannot. In practice current socket errors are pathless, so this is a doc lie, not a live leak.
What correct looks like: Reword to "Display includes the OS error string; socket io::Error Display carries no paths — if a path-bearing source is ever added, redact here", or actually scrub.
Confidence: HIGH — Verification: read of doc + Display arm + From impl.

### F-005 [S4]
Where: crates/control-protocol/src/error.rs:262-326 (the test module)
Code:
```rust
    #[test]
    fn only_bad_envelope_is_recoverable() {
        assert!(ProtocolError::BadEnvelope { detail: "x".into() }.is_recoverable());
        assert!(!ProtocolError::Unauthorized { locked_out: false }.is_recoverable());
```
What is wrong: Coverage is partial in both mapping tables. `close_code()` is asserted for only 3 of 14 variants (MessageTooLarge→1009, VersionMismatch→4402, TimedOut→1001); the Unauthorized→4401, Frame-passthrough, InvalidUtf8→1007, the three Normal mappings, and the entire 4400 group (NotHello/HelloTwice/UnknownProtocol/BadEnvelope) are never asserted. `is_recoverable` is asserted for 3 of 14 variants. A mutation swapping `Unauthorized`'s close code to `AppProtocolError`, or `InvalidUtf8`'s to 1002, passes the whole suite.
Why it matters: These are pure data tables whose entire failure mode is silent drift; the tests exist to pin them and pin only a fifth of them.
What correct looks like: Table-driven tests iterating all variants against expected (code, close, recoverable, ErrorCode) triples.
Confidence: HIGH — Verification: read of all 6 tests + mutation reasoning.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CloseCode (enum) | 19-38 | REAL | — | 8 variants; RFC 6455 §7.4.1 values + doc 05 §5.4 app range 4400+. Values pinned by test. |
| CloseCode::as_u16 | 42-53 | REAL | — | Correct constants (1000/1001/1002/1007/1009/4400/4401/4402). |
| Display for CloseCode | 56-60 | REAL | — | Prints the numeric value. |
| ProtocolError (enum) | 64-142 | REAL | — | 14 variants; V8 redaction honored (Unauthorized is a bare bool). |
| ProtocolError::close_code | 146-162 | REAL | S4 | Mapping correct for Frame/MessageTooLarge/InvalidUtf8/app-range; Io→Normal dishonest (F-003). |
| ProtocolError::is_recoverable | 166-168 | REAL (dead) | S3 | Zero callers; diverges from the server's actual policy (F-002). |
| ProtocolError::to_command_error | 173-192 | REAL (dead) | S3 | Zero production callers; header sells it as the host bridge (F-001). Mapping itself is sane (Io→Io, TimedOut→Network, Unauthorized→PermissionDenied, Closed→Canceled). |
| Display for ProtocolError | 195-245 | REAL | S4 | All 14 arms covered, no panic; UnknownProtocol redacted via redact_name (64-char cap); F-004 doc-vs-body nit on Io. |
| impl Error for ProtocolError | 247 | REAL | — | No source() override — Io's inner cause not exposed via the Error chain (Display includes it, so practical impact nil). |
| From<std::io::Error> | 249-256 | REAL | — | Generic `what` label; used where `?` converts io errors. |
| tests: close_codes_are_the_documented_values | 262-273 | REAL | — | Pins all 8 wire values. Executed in baseline. |
| tests: auth_errors_map_to_permission_denied | 275-282 | REAL | — | Asserts code+stage+summary string. Executed. |
| tests: oversized_maps_to_resource_limit | 284-292 | REAL | — | Executed. |
| tests: version_mismatch_maps_to_close_4402 | 294-302 | REAL | — | Executed. |
| tests: timeouts_map_to_going_away_and_network | 304-313 | REAL | — | Executed. |
| tests: only_bad_envelope_is_recoverable | 315-326 | REAL | — | Partial coverage (F-005). Executed. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| close_codes_are_the_documented_values | YES | Pins all 8 constants — any value change fails. |
| auth_errors_map_to_permission_denied | YES | Asserts ErrorCode, Stage, and the summary string "[control.permission-denied]". |
| oversized_maps_to_resource_limit | YES | Asserts both the ErrorCode and close code. |
| version_mismatch_maps_to_close_4402 | YES | Asserts close code + Display substring. |
| timeouts_map_to_going_away_and_network | YES | Asserts close code (×2 forms) + ErrorCode + Display substring. |
| only_bad_envelope_is_recoverable | PARTIAL | Covers 3 of 14 variants; is_recoverable regressions on the other 11 ship silently (F-005). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every failure mode of the control plane is a variant here — never a stringly error, never a panic" | error.rs:3-4 | REAL | 14 variants; no panic path in file. |
| "to_command_error projects the taxonomy … so hosts answer with the same vocabulary every other host uses" | error.rs:4-7 | FALSE (unwired) | Zero production callers (F-001); the host (control-server) uses a parallel ProtocolErrorCode mapping instead. |
| "token material NEVER enters an error message — the auth variants carry no payload beyond the fact of failure" (V8) | error.rs:9-10 | REAL | Unauthorized { locked_out: bool } (120-124); Display "token rejected" (233-238). |
| "1009 — frame/message exceeded the configured cap" / app range 4400+ | error.rs:29-37 + docs/05-command-layer.md §5.4 | REAL | as_u16 values + server.rs wiring. |
| Io "Display stays redaction-safe (no paths beyond the operation name)" | error.rs:71-73 | FALSE (as worded) | Display arm 202 prints the full OS error string (F-004). |

## 7. Dead / duplicated / unreachable code
Dead: `to_command_error` (zero callers, F-001), `is_recoverable` (zero callers, F-002). Duplicated: recoverability policy exists twice — this file's `is_recoverable` vs server.rs's `fn recoverable` + `ProtocolErrorCode` (divergent answers). The entire file is unreachable from shipped entry points (control-protocol's only consumer is the product-gated-off, not-in-payload control-server).

## 8. Dependencies & flags
Depends on the `commands` crate (CommandError/ErrorCode/Stage) — the crate's only external workspace dep beyond std; consistent with boundaries (control-protocol → commands is the documented direction, doc 01 §2.8). No feature gates. Tests import only super::*.

## 9. Open questions for the next agent
1. server.rs auditor: `fail_session` maps TimedOut → ProtocolErrorCode::Internal (server.rs:858-859) — a peer idle-timeout reported to the client as an INTERNAL error; check whether doc 05 §5.4's error-frame table sanctions that.
2. If Phase 5 turns the control-server on: F-001/F-002 become shipping defects — the error-frame vocabulary and CommandError projection must be unified before the product gate flips.
3. protocol.rs `ProtocolErrorCode` vs this file's `CloseCode` — two error vocabularies in one crate with a hand-maintained bridge table (server.rs:855-863); a single-source-of-truth refactor question.
