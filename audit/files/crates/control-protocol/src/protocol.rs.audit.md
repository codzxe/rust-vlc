# AUDIT: crates/control-protocol/src/protocol.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-23 |
| Lines / functions | 416 / 21 items (12 fns/consts/types + 2 enums w/ methods + 6 unit tests) |
| File verdict | REAL (implemented + executed) — but see reachability |
| Reachable from shipped entry point? | NO — only apps/control-server depends on control-protocol; release lane builds `-p lumen-cli -p desktop`; product.toml:167 `control-server = false` (same chain as W02-A05's server.rs audit) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `./target/debug/deps/control_protocol-30329beb10b168e9 protocol::` → 6 passed, EXIT=0; per-module runs `server::`(4) `http::`(5) `frame::`(11) `token::`(3) `sha1::`(3) `policy::`(6) `error::`(6) `base64::`(3) all passed EXIT=0 (=47 total, matches baseline :1638); `--list` → 47 tests. UNVERIFIED: full UNFILTERED parallel run hung >60 s twice on this box (see §9) |

## 1. One-paragraph verdict
The `lumen-control/1` envelope is real, serde-wired, and exercised: `negotiate_version`, `validate_request_shape`, `redact_name`, and the frame enums all run in the 47-test unit battery and the 25-test conformance suite (both executed by me). The correctness core holds up: negotiation picks the highest mutual major; the raw-value shape check exists because `#[serde(flatten)]` silently drops unknown fields (server.rs:634-639 documents that the conformance suite found this — a real fix with a real regression test); redaction is char-correct at the 64-char boundary. Three defects: (1) `capabilities_current` hard-codes `token_auth: true`, contradicting the field's own doc ("false only in read-only mode's unauthenticated subset") — the welcome frame misreports authentication posture in exactly the case the doc names, and both test assertions of `token_auth` are vacuous because the server can never send `false`; (2) `ServerFrame::Event` is never constructed anywhere in the repo — the event-push half of the protocol is envelope-only; (3) `CAPABILITIES_CURRENT` is dead public API. The whole stack ships nowhere today (product gate off, not in the release lane).

## 2. What actually works (verified)
— verified by `./target/debug/deps/control_protocol-30329beb10b168e9 protocol::` → `test result: ok. 6 passed; 0 failed` (negotiation, request roundtrip incl. wire strings `"type":"request"`/`"requestId":7`/`"cmd":"playback.play"`, hello±token, protocol-error shape, shape validation, redact bounds).
— verified by the conformance run in my conformance.rs audit: `validate_request_shape`'s "extra" rejection and `negotiate_version`'s 4402 path are both exercised over a real socket (`request_validation_rejects_dirty_payloads_and_survives`, `version_mismatch_is_a_typed_error_and_close_4402` — 25 passed).

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/src/protocol.rs:109-128 (+ server.rs:746-750; docs/security/sweep-control.md:52)
Code:
```
    /// Whether the fixed token was checked (false only in read-only
    /// mode's unauthenticated subset).
    pub token_auth: bool,
```
and
```
pub fn capabilities_current(read_only: bool, max_frame: u64, max_message: u64) -> Capabilities {
    Capabilities {
        token_auth: true,
```
and the only caller (server.rs:746-750):
```
            capabilities: crate::protocol::capabilities_current(
                self.policy.read_only() && !self.authenticated,
```
What is wrong: `token_auth` is hard-coded `true` for every welcome. In the read-only unauthenticated subset — the exact case the field's doc says must be `false` — an unauthenticated session is told `token_auth: true`. docs/security/sweep-control.md:52 claims "carries the server's honest capabilities (`token_auth`, caps)". The doc is false; the wire value misreports auth posture. Consequence for evidence quality: `assert!(welcome.capabilities.token_auth)` at conformance.rs:244 and e2e.rs:305 can NEVER fail (the server cannot emit `false`) — vacuous assertions posing as auth checks. Why it matters: a remote-control client reading capabilities to decide whether to offer/prompt token auth is lied to; and the "honest capabilities" security claim is untested in its one falsifiable case. What correct looks like: `token_auth: !read_only_subset` (pass the authenticated flag) or fix the doc to match a server-static meaning and delete the false parenthetical. Confidence: HIGH — Verification: read + rg (all `token_auth` writers are the two hard-coded `true`s).
F-002 [S3]
Where: crates/control-protocol/src/protocol.rs:195-199 (+ 10-12 doc header)
Code:
```
    /// An engine event push.
    Event {
        #[serde(flatten)]
        event: WireEvent,
    },
```
and the header advertising it:
```
//! {"type":"event","seq":1,"kind":"…"}
```
What is wrong: `ServerFrame::Event` has zero constructors and zero matchers in the entire repo (`rg "ServerFrame::Event"` → no matches; the enum's own definition is the bare variant `Event {`). The event-push half of the protocol is envelope-only paper: subscribe/unsubscribe are accepted and stored (server.rs:651-667) but no event can ever be pushed. Why it matters: the module doc's wire catalog lists a frame type that never travels; a client implementer reading protocol.rs builds an event handler for a producer that does not exist. (Corroborates W02-A05 server.rs F-001/F-002: subscriptions unbounded and inert.) What correct looks like: mark the event path `[PROPOSAL]`/not-yet-implemented in the header until a producer exists. Confidence: HIGH — Verification: read + rg (no matches repo-wide).
F-003 [S3]
Where: crates/control-protocol/src/protocol.rs:130-138 (+ lib.rs:74)
Code:
```
pub const CAPABILITIES_CURRENT: Capabilities = Capabilities {
    token_auth: true,
    read_only: false,
```
What is wrong: `CAPABILITIES_CURRENT` is defined, doc-commented ("what `welcome` carries unless the host config says otherwise" — it is NOT what welcome carries; server.rs:746 calls `capabilities_current(...)), and re-exported at lib.rs:74, but has zero users in the repo. Dead public API whose doc comment misdescribes the real welcome path (the real path calls the fn, not the const — and passes dynamic limits). What correct looks like: delete it, or make it the single source the server uses. Confidence: HIGH — Verification: read + rg (only definition + re-export).
F-004 [S4]
Where: crates/control-protocol/src/protocol.rs:77-101
Code:
```
    pub fn as_str(self) -> &'static str {
        match self {
            ProtocolErrorCode::NotHello => "not-hello",
```
What is wrong: `as_str` hand-duplicates the `#[serde(rename_all = "kebab-case")]` wire mapping (lines 47-48) — two sources of truth for the same 12 strings. `as_str` is consumed only by `Display` (line 99), and no in-tree code Displays a `ProtocolErrorCode` (rg: the wire path serializes via serde; error.rs Displays the distinct `ProtocolError` type). Only `VersionMismatch`'s string is pinned by a test (line 372). A drift between the two mappings changes nothing today (as_str is dead) but is a trap for the first future consumer. What correct looks like: one mapping (serde or as_str), or a test pinning all 12 strings against serde output. Confidence: HIGH — Verification: read + rg.
F-005 [S4]
Where: crates/control-protocol/src/protocol.rs:220-224
Code:
```
/// fields … rejected" — the flatten envelope cannot enforce this
/// itself). The catalog's own `deny_unknown_fields` cover the `args`
/// object's interior.
pub const REQUEST_KEYS: [&str; 4] = ["type", "requestId", "cmd", "args"];
```
What is wrong: `validate_request_shape` checks the key SET and the types of `requestId`/`cmd`, but not that `args` (when present) is an OBJECT; a scalar `args` passes shape validation and relies on the downstream typed parse (`CommandRequest<Command>`'s adjacent tag) to reject it. Behavior is safe (typed parse rejects), but the comment's division of labor ("catalog covers the args object's interior") presumes an object. No test sends a scalar `args`. What correct looks like: one `as_object` check on `args` in the shape validator, plus a conformance case. Confidence: MEDIUM — Verification: read; scalar-args rejection reasoned from serde adjacent-tag semantics, not executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PROTOCOL_NAME / PROTOCOL_VERSION / SUPPORTED_VERSIONS | 37-44 | REAL | — | wire consts; name pinned by conformance `unknown_protocol_name_is_rejected` |
| ProtocolErrorCode (enum) | 49-75 | REAL | — | 12 kebab-case codes; exercised end-to-end by conformance |
| ProtocolErrorCode::as_str | 79-94 | REAL (dead-in-tree) | S4 | duplicates serde mapping; only Display consumes it (F-004) |
| Display for ProtocolErrorCode | 97-101 | REAL (dead-in-tree) | S4 | no in-tree formatter of this type |
| Capabilities (struct) | 108-118 | REAL | S3 | `token_auth` doc vs value contradiction (F-001) |
| capabilities_current | 121-128 | REAL | S3 | hard-codes token_auth:true (F-001); sole caller server.rs:746 |
| CAPABILITIES_CURRENT | 133-138 | STUB/dead | S3 | zero users; doc misdescribes welcome path (F-003) |
| Welcome (struct) | 143-151 | REAL | — | `v` rename + camelCase pinned by conformance assertions |
| ClientFrame::Hello | 158-166 | REAL | — | token optional via default+skip_serializing_if; tested ±token |
| ClientFrame::Request | 168-171 | REAL | — | flatten of `CommandRequest<Command>`; wire shape pinned by unit test |
| ClientFrame::Subscribe / Unsubscribe | 173-181 | REAL (wired but inert) | — | handled at server.rs:651-667; no observable effect, no ack (cross-ref F-002 + W02-A05) |
| ServerFrame::Welcome/Response/ProtocolError | 188-194, 200-209 | REAL | — | Response/ProtocolError exercised by conformance |
| ServerFrame::Event | 195-199 | PAPER-ONLY | S3 | never constructed repo-wide (F-002) |
| negotiate_version | 212-218 | REAL | — | max mutual major; [] and [2,3] → None (tested) |
| REQUEST_KEYS / validate_request_shape | 224, 233-276 | REAL | S4 | keys ⊆ set, requestId u64, cmd non-empty; args-type gap (F-005); "extra" rejection has a real conformance case |
| redact_name | 280-286 | REAL | — | 64 chars + ellipsis; byte/char edge cases correct (tested); used 4× (server.rs:626,672,831; error.rs:226) |
| tests (6) | 294-415 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| negotiation_picks_the_highest_mutual_major | YES | 4 cases incl. `[2,1]→Some(1)` (highest, not first) and `[]→None` |
| client_frame_request_roundtrips | YES for wire shape | asserts exact substrings `"type":"request"`, `"requestId":7`, `"cmd":"playback.play"` — a rename breaks it; roundtrip half is self-consistency |
| hello_roundtrips_with_and_without_token | YES | asserts token absent from JSON when None (read-only shape) |
| server_frame_protocol_error_shape | YES (1 of 12 codes) | pins `version-mismatch` wire string only — 11 codes unpinned (F-004) |
| request_shape_validation_rejects_unknown_fields_and_bad_ids | YES | unknown field, string id, non-object all asserted Err |
| redact_name_bounds_client_controlled_text | YES | 200→≤65 chars + ellipsis |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| token_auth "false only in read-only mode's unauthenticated subset" | protocol.rs:109-111 | FALSE | code sends `true` unconditionally (F-001) |
| "welcome carries the server's honest capabilities (token_auth, caps)" | docs/security/sweep-control.md:52 | FALSE for token_auth | F-001 |
| wire catalog incl. `{"type":"event",...}` | protocol.rs:10-12 | PAPER-ONLY | zero constructors of ServerFrame::Event (F-002) |
| "The catalog's own deny_unknown_fields cover the args object's interior" | protocol.rs:222-223 | PARTLY | true for object args; scalar args unchecked by shape (F-005) |
| "unknown fields/IDs rejected" beyond serde | protocol.rs:23-26 | REAL | validate_request_shape + conformance case `request_validation…` (25 passed, my run) |

## 7. Dead / duplicated / unreachable code
- `CAPABILITIES_CURRENT` — dead (F-003).
- `ProtocolErrorCode::as_str`/`Display` — dead in-tree (F-004).
- `ServerFrame::Event` — dead wire vocabulary (F-002).
- Whole crate unreachable from shipped entry points: control-protocol ← apps/control-server only; release lane = lumen-cli + desktop (baseline/cargo-build-release-lumen-cli.txt:1); product.toml:167 `control-server = false`.

## 8. Dependencies & flags
Imports only `commands` wire types + serde/serde_json (protocol.rs:28-33) — no feature gates, no unsafe, no ignored tests. Same-layer pair per ARCH-5 (Cargo.toml comment).

## 9. Open questions for the next agent
- The UNFILTERED lib-test binary hung twice >60 s on this 2-core box while every per-module filtered run passes in <0.1 s; baseline shows 47/47 in 0.06 s. UNVERIFIED whether that is load or a parallel test interaction (e.g., two tests binding loopback sockets concurrently) — worth a `--test-threads=1` full run.
- Does any planned consumer read `capabilities.token_auth`? If yes, F-001 becomes S2.
- e2e.rs:305 asserts token_auth in an authenticated session — same vacuity as conformance.rs:244 (F-001).
