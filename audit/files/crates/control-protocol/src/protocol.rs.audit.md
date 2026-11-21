# AUDIT: crates/control-protocol/src/protocol.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-21 |
| Lines / functions | 416 / 16 items (+6 tests) |
| File verdict | REAL (tested, conformance-proven) — but UNREACHABLE from shipped entry points: no workflow ships `lumen-control-server` |
| Reachable from shipped entry point? | NO. `lumen` CLI: rg `control` in apps/cli/src → 0 hits; no Cargo dep. `lumen-tauri`/desktop: no dep. Only consumer is `apps/control-server` (bin `lumen-control-server`), which `.github/workflows/release.yml` never builds (release.yml:193,474,627 build only `-p lumen-cli -p desktop` + desktop-tauri). Chain that DOES exist: `lumen-control-server` → `control_protocol::protocol::{negotiate_version, validate_request_shape, ClientFrame/ServerFrame}` (server.rs:640-676, 700-757). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | rg `capabilities_current\|CAPABILITIES_CURRENT` (worktree) → fn has 1 caller (server.rs:746), const has 0; rg control-protocol in apps/*/Cargo.toml → only apps/control-server; rg `control-server` in .github/workflows → 0; sed verification of quotes; python scan of schemas/commands.schema.json (args objects carry NO `additionalProperties`); baseline log sed 1411-1465 (control_protocol 47 tests ok) + 1464-1496 (conformance 25 ok) |

## 1. One-paragraph verdict
This is a competently built, honestly tested protocol module: version negotiation, a 12-code typed error taxonomy, a flatten-safe manual shape check, and a redaction helper, all round-trip tested (47/47 unit + 25/25 conformance passed in baseline, log lines 1444-1449, 1466-1493). The devil is in the claims around it: doc 05 §5.4 promises runtime validation "against `schemas/commands.schema.json`" and "unknown fields/IDs rejected" for "every frame" — the code validates only request frames, by a hand-rolled key check plus serde, never touching the schema artifact; unknown event IDs in `subscribe` are silently accepted; and the module's own comment overstates `deny_unknown_fields` coverage inside `args`. `capabilities_current` hard-codes `token_auth: true`, contradicting the field's documented `false` case (unauthenticated read-only), and the `CAPABILITIES_CURRENT` const is dead. None of this is shipped: no workflow builds the only binary that links this crate.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/test-workspace.log.txt:1444-1449`: all 6 `protocol::tests` ran **ok** (negotiation, request round-trip, hello ±token, protocol-error shape, shape validation, redaction bounds).
— verified by `audit/runs/baseline/test-workspace.log.txt:1466-1493`: `tests/conformance.rs` — 25 passed, including `handshake_success_full_session_round_trip`, `request_validation_rejects_dirty_payloads_and_survives` (unknown envelope field, unknown cmd, bad arg type, non-object; connection survives), `unauthenticated_read_only_mode_serves_the_safe_subset_only` — real socket flows through the actual server.
— verified by read: `negotiate_version` picks max mutual major (SUPPORTED_VERSIONS=[1]); `redact_name`'s byte-len vs char-take comparison is sound (out is a strict byte-prefix of name, so `out.len() < name.len()` iff truncation occurred); `ProtocolErrorCode` kebab-case serde tags match `as_str` for all 12 codes.
— verified by `crates/control-protocol/Cargo.toml` + baseline build #1: zero external deps beyond `commands`/`serde`/`serde_json` — the in-tree-ethos claim holds for this crate's manifest.

## 3. Findings
### F-001 [S3]
Where: crates/control-protocol/src/protocol.rs:121-128 (against the field doc at 109-113; sole caller server.rs:746-750)
Code:
```rust
pub fn capabilities_current(read_only: bool, max_frame: u64, max_message: u64) -> Capabilities {
    Capabilities {
        token_auth: true,
        read_only,
```
What is wrong: `token_auth` is hard-coded `true`. The field's own doc says it is "Whether the fixed token was checked (**false only in read-only mode's unauthenticated subset**)". The only caller invokes the function with `read_only = policy.read_only() && !self.authenticated` (server.rs:747) — i.e., precisely in that subset — so a tokenless read-only client's `welcome` advertises `"tokenAuth": true` when no token was checked.
Why it matters: The welcome capabilities are the client's only signal of what the server enforced. A client reading `tokenAuth: true` on an unauthenticated session can conclude its session is token-backed when it is not — the exact "negotiated reality, honest numbers" this struct promises (lines 103-105).
What correct looks like: `token_auth: !read_only` (or pass the authenticated flag and derive both fields), plus a conformance assertion on `token_auth` in the read-only test (conformance.rs:848-853 asserts `read_only` but is silent on `token_auth`).
Confidence: HIGH — Verification: read (field doc, fn, sole call site, both conformance tests).

### F-002 [S3]
Where: crates/control-protocol/src/protocol.rs:23-26 and 220-223 (against doc 05 §5.4, `docs/05-command-layer.md:541`)
Code:
```rust
//! Strictness beyond serde (doc 05 §5.4 "unknown fields/IDs rejected"):
//! [`validate_request_shape`] checks the raw JSON object's key set
//! before typed parsing, because `#[serde(flatten)]` envelopes cannot
//! carry `deny_unknown_fields` themselves.
```
What is wrong: Doc 05 §5.4 says "Every frame: max 1 MiB; **schema-validated against `schemas/commands.schema.json`** …; unknown fields/IDs rejected". Three drifts: (a) no JSON-Schema artifact is ever loaded — `crates/control-protocol/Cargo.toml` has no validator dependency; runtime validation is this hand-rolled key check plus serde typed parsing of types that *generated* the schema; (b) unknown fields are rejected ONLY for request frames — server.rs:640-644 applies `validate_request_shape` after `.filter(|kind| *kind == "request")`, so `hello`/`subscribe`/`unsubscribe` frames silently ignore unknown fields (serde default; `deny_unknown_fields` is impossible on the flatten-carrying enum); (c) unknown event IDs in `subscribe` are silently accepted into the subscription set (server.rs:651-659, any string passes).
Why it matters: §5.4's "one schema, all hosts" reuse claim is the doc's central anti-drift argument for the control surface; the artifact is decorative here. A client sending `{"type":"hello","protocol":"lumen-control","versions":[1],"junk":…}` or subscribing to `nope.nope` gets success, not the documented rejection.
What correct looks like: Either a runtime schema validation pass against the committed artifact, or key-set validation for every frame type, or `subscribe` validating IDs against `EventId::ALL_IDS` — and doc 05 §5.4 rewritten to describe what is actually enforced.
Confidence: HIGH — Verification: read (Cargo.toml, server.rs:640-676, conformance.rs:885-950 which tests envelope-field/unknown-cmd/bad-arg-type but never hello/subscribe unknown fields nor unknown event ids).
### F-003 [S3]
Where: crates/control-protocol/src/protocol.rs:222-223
Code:
```rust
/// itself). The catalog's own `deny_unknown_fields` cover the `args`
/// object's interior.
```
What is wrong: Overstated. `deny_unknown_fields` on nested structs inside args (e.g. `OpenOptions`, `SeekTarget` payloads) does hold, but keys sitting BESIDE a command's declared args fields are silently ignored: `Command` is adjacently tagged (`command.rs:68-73`, `tag="cmd", content="args"`), and serde enum-variant content cannot carry `deny_unknown_fields`. The committed schema corroborates: python scan shows args objects carry NO `additionalProperties: false` (unlike the 23 plain structs that do). So `{"cmd":"audio.set-volume","args":{"volume":0.5,"junk":1}}` parses successfully.
Why it matters: This is the documented defense for the args interior; a reader trusts it and stops looking. A client with a typo'd optional arg key gets silent success instead of the typed rejection §5.4 promises.
What correct looks like: State the true boundary (nested struct interiors only), or add an args-level key check to `validate_request_shape` (the variant field sets are statically known per `cmd`), or a conformance case sending junk inside args.
Confidence: HIGH on serde semantics + schema evidence — Verification: read (command.rs:68-73, schema scan); UNVERIFIED by execution (no test exercises junk-inside-args; REQUEST-ORCH-RUN probe listed in log).
### F-004 [S3]
Where: crates/control-protocol/src/protocol.rs:133-138
Code:
```rust
pub const CAPABILITIES_CURRENT: Capabilities = Capabilities {
    token_auth: true,
    read_only: false,
```
What is wrong: `CAPABILITIES_CURRENT` is dead. Worktree-wide rg: the only occurrences are its definition (protocol.rs:133) and the `lib.rs:74` prelude re-export; zero call sites (the server builds its `welcome` via the `capabilities_current` function, server.rs:746).
Why it matters: Dead public API invites drift — it claims to be "what `welcome` carries unless the host config says otherwise" (130-132), but no host reads it, so its values can silently diverge from what `capabilities_current` actually returns (they already differ in intent: the const says `read_only: false` for "this crate's own defaults" while the function takes it as a parameter).
What correct looks like: Delete the const, or make the server use it as the actual default and parameterize the deltas.
Confidence: HIGH — Verification: rg across the worktree.
### F-005 [S4]
Where: crates/control-protocol/src/protocol.rs:8-9 (module doc examples)
Code:
```rust
//! {"type":"request","requestId":1,"cmd":"playback.play","args":{}}
//! {"type":"response","requestId":1,"gen":0,"completed":…}
```
What is wrong: Both examples misstate the real wire form. (1) `playback.play` is a unit command — it serializes with NO `args` key at all (`crates/commands/tests/wire_forms.rs:183-186`: `{"requestId":5,"cmd":"playback.pause"}`). (2) The response example omits the `result` wrapper: the real form is `{"requestId":…,"session":…,"gen":…,"result":{"accepted":…|"completed":…|"error":…}}` (`wire_forms.rs:75`: `{"requestId":42,"session":7,"gen":3,"result":{"accepted":{"session":7,"gen":4}}}`) — `"completed"` is not a top-level key.
Why it matters: These examples are the protocol's front-page documentation; a client implementer copying them produces frames that do not match the conformance-proven wire.
What correct looks like: Fix the two lines to the byte forms the round-trip tests pin.
Confidence: HIGH — Verification: read + wire_forms.rs goldens (both tests ran ok in baseline, log 1414-1416/1466+).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PROTOCOL_NAME` | 37 | REAL | — | `lumen-allow-literal` justified (wire vocabulary). |
| `PROTOCOL_VERSION` | 44 | REAL | — | =1. |
| `SUPPORTED_VERSIONS` | 44 | REAL | — | `[1]`; negotiation input. |
| `ProtocolErrorCode` | 47-75 | REAL | — | 12 codes; serde kebab tags verified equal to `as_str`. |
| `ProtocolErrorCode::as_str` | 79-94 | REAL | — | Total match, no default arm. |
| `Display` | 97-101 | REAL | — | Delegates to as_str. |
| `Capabilities` | 106-118 | REAL | F-001 | camelCase (`tokenAuth`, `readOnly`, `maxFrameBytes`, `maxMessageBytes`). |
| `capabilities_current` | 121-128 | REAL | F-001 | token_auth hard-coded true. |
| `CAPABILITIES_CURRENT` | 133-138 | DEAD | F-004 | Zero users. |
| `Welcome` | 141-151 | REAL | — | `#[serde(rename="v")]` matches §5.4's `{v, engineInfo, capabilities}`; carries the commands-catalog `EngineInfo` (ARCH-5 reuse, no re-implementation). |
| `ClientFrame` | 154-182 | REAL | F-002/F-003 context | `Hello` (token optional, skipped when None — tested), `Request` (double-flatten sandwich: type-tag → WireRequest → Command cmd/args; round-trip proven), `Subscribe`/`Unsubscribe` (plain). |
| `ServerFrame` | 185-209 | REAL | — | `Welcome`/`Response`(flatten)/`Event`(flatten)/`ProtocolError`; flatten keys cannot collide ("type" vs payload keys). |
| `negotiate_version` | 212-218 | REAL | — | max mutual major; 4-case table test. |
| `REQUEST_KEYS` | 224 | REAL | F-002 | 4 keys; matches the round-tripped request form. |
| `validate_request_shape` | 233-276 | REAL | F-002 | Correct for its scope: non-object, unknown key, non-u64 requestId, empty/non-string cmd all typed-rejected; does not check `type=="request"` value or args shape (typed parse covers). |
| `redact_name` | 280-286 | REAL | — | Char-safe 64+ellipsis; byte-vs-char comparison sound because out is a byte-prefix of name; test asserts ≤65 chars. |
| tests ×6 | 294-415 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `negotiation_picks_the_highest_mutual_major` | YES | 4 cases incl. both None paths; exact equality. |
| `client_frame_request_roundtrips` | YES | Round-trip through the full double-flatten; substring pins `type`/`requestId`/`cmd`. Would not catch a spurious extra serialized key (substring, not exact golden). |
| `hello_roundtrips_with_and_without_token` | YES | Asserts `token` ABSENT from JSON when None — catches a dropped `skip_serializing_if`. |
| `server_frame_protocol_error_shape` | PARTLY | Substring only, no parse-back — the weakest test here; a malformed code string would still pass if it contained the substring. |
| `request_shape_validation_rejects_unknown_fields_and_bad_ids` | YES | 4 negative scenarios + 1 positive; missing cases: empty `cmd`, missing `requestId` (behavior exists at 255-264 but untested). |
| `redact_name_bounds_client_controlled_text` | YES | 200-char input → ≤65 chars ending in `…`. |
| (conformance.rs, cross-file) | YES | Real sockets: dirty payloads survive-and-reject, read-only subset denies `playback.play` with `Denied`, non-monotonic id closes — ran ok in baseline (log 1466-1493). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "schema-validated against `schemas/commands.schema.json` (same artifact as §5.1 — one schema, all hosts)" | doc 05 §5.4 (docs/05-command-layer.md:541) | **FALSE** (artifact never loaded; serde-typed parse from the same source types only) | control-protocol Cargo.toml (no validator dep); server.rs:640-648 = key check + `from_value::<ClientFrame>`. See F-002(a). |
| "unknown fields/IDs rejected" (every frame) | doc 05 §5.4 | **SHALLOW** (request frames only; hello/subscribe/unsubscribe ignore unknown fields; unknown event IDs accepted) | server.rs:640-644 (request filter), 651-659 (subscribe accepts any string). See F-002(b)(c). |
| "closes with a protocol error code" on version mismatch; close 4402 | doc 05 §5.4 | REAL | `error::tests::version_mismatch_maps_to_close_4402` ok (baseline log 1423); e2e `a_typoed_sensitive_grant…` etc. ok. |
| token handshake, constant-time, 4401, lockout | doc 05 §5.4 | REAL (crate-level) | server.rs:714-741; `token::tests::equal_and_unequal_comparisons`, `failure_book_engages_the_documented_lockout` ok (baseline 1460-1462); e2e wrong-token test ok (log 1490). |
| `welcome { v, engineInfo, capabilities }` | doc 05 §5.4 | REAL | `Welcome` serde shape (145-150); `handshake_success_full_session_round_trip` asserts all three (conformance.rs:240-247). |
| "max 1 MiB" per frame | doc 05 §5.4 | REAL | `server::tests::documented_limits_are_the_doc_05_numbers` ok (baseline 1450); conformance asserts 1024*1024. |
| `EngineInfo` re-used from the ONE command layer, "never re-implemented" | module doc 15-17 | REAL | `use commands::catalog::types::EngineInfo` (line 31); no local duplicate. |

## 7. Dead / duplicated / unreachable code
- `CAPABILITIES_CURRENT`: dead (F-004).
- The entire module is UNREACHABLE from shipped binaries: `lumen` (no dep), desktop (no dep); only `apps/control-server` links it and no workflow builds/ships that binary (release.yml:193/474/627). CI tests it (`cargo test --workspace` ran control_protocol + control_server, baseline log 1411-1530) but no artifact ships it.

## 8. Dependencies & flags
- Deps: `commands` (path), `serde` 1.0.229 derive, `serde_json` 1.0.154 — matches the commands crate's pins (one serde path). No feature flags, no unsafe, no panics on realistic input (no unwrap/index in non-test code).
- Boundaries: `commands → control-protocol` edge exists in both Cargo.toml and is the documented same-layer pair (doc 01 §1.4) — no layering violation.

## 9. Open questions for the next agent
- server.rs maps BOTH unknown-cmd and bad-arg-type parse failures to `UnknownType` (conformance.rs:904-925 asserts this) — semantically wrong code for a bad arg type (`BadFrame`/`InvalidRequest` fit); server.rs auditor to weigh.
- Is `apps/control-server` intended to ship (doc 05 §5.4 "optional local control surface")? If yes, release.yml's `-p lumen-cli -p desktop` build list is an S2 packaging gap; if no, this crate is dev-only surface.
- REQUEST-ORCH-RUN (probe for F-003): `cargo test -p control-protocol --test conformance` with an added case `{"type":"request","requestId":N,"cmd":"audio.set-volume","args":{"volume":0.5,"junk":1}}` — expected per my analysis: accepted (proving the args-interior gap), contradicting the module comment.
