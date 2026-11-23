# AUDIT: apps/control-server/src/dispatch.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-23 |
| Lines / functions | 196 / 5 (1 struct, 1 ctor, 1 trait impl, 1 free fn, 3 tests) |
| File verdict | REAL (does exactly what it says: 2 of 65 commands served, honest typed refusal for 63) — but the "control server" cannot control anything |
| Reachable from shipped entry point? | YES from `lumen-control-server` — apps/control-server/src/main.rs:21-31 → `serve_loop` (run.rs:45) → `ControlServer::serve_one` → `handle_request` → `dispatch.dispatch(request)` (crates/control-protocol/src/server.rs:803). Binary is a CI workspace member (ci.yml:233) but NOT in the release lane (release.yml:193 builds `-p lumen-cli -p desktop` only). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/control_server-58398a5795b79e6e dispatch::` → 3 passed (binary mtime 1795457230 > source 1795455358); rg cross-checks of policy/dispatch/conformance/e2e/matrix/KNOWN-LIMITS |

## 1. One-paragraph verdict
Dispatch does dispatch — into a two-command introspection surface. `CatalogDispatch::dispatch` serves `system.get-engine-info` (a startup-built block clone) and `system.list-commands` (65 rows built live from the spec table); the other 63 catalog commands, including `session.open` and every playback verb, answer the typed `not-ready` "this control-server build embeds no engine" (L34-36) — an honest refusal, no fake success, exactly as the module doc discloses, and the three tests (all run, pass) pin that honesty. The hostile answer: nothing here forwards to nothing; it forwards to a deliberately tiny surface and says so. The real defects are around it: (1) the policy layer's `READ_ONLY_SAFE` set (doc 05 §5.4's promised unauthenticated subset) contains `media.get-info`, `tracks.list`, `system.get-state-snapshot` — commands this dispatch structurally cannot serve, so a read-only client passes policy and then hits `not-ready` for 3 of the 4 blessed commands (the conformance suite hides this behind an `EchoDispatch` that fakes `Unit` success for everything); (2) the product's status-truth documents carry no row or limit entry for the control server at all — 252 feature-matrix rows, zero mentioning it; KNOWN-LIMITS.md never mentions it — so "remote-control-style UIs" (doc 05 §5.4) remains a spec sentence with an opt-in, unshipped, introspection-only implementation; (3) the `subscribe` frame is a silent no-op — the subscription set is write-only and `ServerFrame::Event` is never constructed anywhere in the tree.

## 2. What actually works (verified)
— `./target/debug/deps/control_server-58398a5795b79e6e dispatch::` → `test result: ok. 3 passed` (65-listing count/shape, both served commands' results, typed not-ready for play/open).
— Call chain verified by read: server.rs:803 `let response = dispatch.dispatch(request);` runs AFTER envelope validation, anti-replay and policy check (server.rs:752-802) — the dispatch is genuinely wired into the WS event loop, not dead code.
— `catalog_listings()` (L84-101) builds rows from `CommandId::ALL` + `command_spec` — "never a copy" claim true; the count test pins 65 = ALL.len().

## 3. Findings
F-001 [S2]
Where: apps/control-server/src/dispatch.rs:66-70 vs crates/control-protocol/src/policy.rs:65-70
Code:
```
            _ => Err(CommandError::new(
                Stage::Control,
                ErrorCode::NotReady,
                NOT_READY,
            )),
```
versus policy.rs:
```
pub const READ_ONLY_SAFE: &[CommandId] = &[
    CommandId::MediaGetInfo,
    CommandId::SystemGetEngineInfo,
    CommandId::TracksList,
    CommandId::SystemGetStateSnapshot,
];
```
What is wrong: The two halves of the same app contradict each other. Policy admits unauthenticated read-only clients to 4 commands; dispatch serves exactly 2 commands, overlapping the safe set in ONE (`system.get-engine-info`). A read-only client sending `media.get-info`, `tracks.list`, or `system.get-state-snapshot` passes the policy gate (policy.rs:215-219) and then receives `not-ready` from this catch-all — the doc-05 §5.4 read-only contract is unfulfillable end-to-end in the shipped app. Why it matters: read-only mode exists precisely so unauthenticated local automation can query state; 3 of its 4 promised verbs always fail, and `system.list-commands` (the one other working command) is NOT in the safe set, so it demands a token. Nothing tests the real pairing: the read-only conformance test (conformance.rs:832-877) uses `EchoDispatch`, which answers `CommandResult::Unit` for every command (conformance.rs:141) — a test double that fakes success for exactly the commands the real dispatch refuses; the app's e2e asserts read_only is OFF (e2e.rs:175,306) and never exercises the mode with the real dispatch. What correct looks like: read-only safe set = served set (drop the 3 unservicable ids or implement them), plus one e2e test of read-only mode through `CatalogDispatch`. Confidence: HIGH — Verification: read (policy check path server.rs:785-802 + dispatch arm) + rg (no read-only e2e with real dispatch).
F-002 [S3]
Where: apps/control-server/src/dispatch.rs:1-15 (module doc) vs docs/feature-matrix.toml (252 rows) and docs/KNOWN-LIMITS.md
Code:
```
//! Everything else answers with the command layer's typed `not-ready`
//! (stage `control`) — an honest gap, never a silent no-op and never a
//! fake success. The wire vocabulary is 100% the ONE command layer's
```
What is wrong: The honesty lives ONLY in this module doc and the refusal message. The status-truth documents are silent: `rg control docs/feature-matrix.toml` → zero matches in 252 rows; KNOWN-LIMITS.md (120 lines, refreshed 2026-11-15) never mentions the control server. Doc 05 §5.4 sells "An optional local control surface for automation and remote-control-style UIs" with [PROPOSAL] markers but no "v1 serves introspection only" caveat. Why it matters: a reader of the docs (or the welcome frame's `engineInfo`) has no way to learn the control surface cannot control playback; the gap between spec prose and 2/65 reality is undocumented everywhere except the source. What correct looks like: a KNOWN-LIMITS row ("control server: introspection-only, no embedded engine") or a feature-matrix row with status `partial` + evidence. Confidence: HIGH — Verification: rg over both docs.
F-003 [S4]
Where: apps/control-server/src/dispatch.rs:72-77
Code:
```
        WireResponse {
            request_id: request.request_id,
            session: None,
            generation: Generation(0),
            result,
        }
```
What is wrong: Every response hardcodes `session: None, generation: 0`. Correct for an engine-less introspection host, but the doc-05 §1.6 rule is that these fields "carry the context at answering time"; when the engine-embedding composition lands (the module doc's own promise), this literal is the kind of thing that survives the join and lies. Confidence: MEDIUM — Verification: read.
F-004 [S4]
Where: apps/control-server/src/dispatch.rs:84-101
Code:
```
    CommandId::ALL
        .iter()
        .filter_map(|id| {
            command_spec(*id).map(|spec| CommandListing {
```
What is wrong: `filter_map` silently drops any id lacking a spec row — an under-report, not an error. Today impossible (catalog_shape's cross-set test pins ids ↔ spec rows; the count test pins 65), so the pin absorbs the risk; the pattern would still prefer an explicit unreachable/expect for an invariant break. Confidence: HIGH — Verification: read + the pinning tests (ran).

F-005 [S3]
Where: crates/control-protocol/src/server.rs:651-666 (subscribe path) and protocol.rs:195-199 (`ServerFrame::Event`), cross-file consequence of this dispatch's no-engine design
Code:
```
            Ok(ClientFrame::Subscribe { events }) => {
                for kind in events {
                    if !self.subscriptions.contains(&kind) {
                        // Bounded: the message cap bounds the union of
                        // kinds (each kind costs ≥ its JSON bytes).
                        self.subscriptions.push(kind);
                    }
                }
                Flow::Continue
            }
```
What is wrong: The subscription bookkeeping is write-only. rg for `self.subscriptions` shows only push/clear/retain (server.rs:653-665) — nothing ever reads the set to forward events; `ServerFrame::Event` (protocol.rs:196-199) has ZERO construction sites in the entire tree (rg `ServerFrame::Event` → no matches outside the enum definition). A client sending `subscribe` gets `Flow::Continue` — accepted silently — and then no events, ever. Why it matters: this is exactly the "silent no-op" dispatch.rs:12-13 declares the design never does; the subscribe frame is a promise the server cannot keep and does not refuse. (Consistent with no-engine, but undisclosed: no error, no capability flag saying events are unavailable.) What correct looks like: either refuse subscribe with a typed `denied`/`not-ready` while no engine is embedded, or wire the event relay. Confidence: HIGH — Verification: rg (all `subscriptions` uses + zero `ServerFrame::Event` constructors).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct CatalogDispatch` | 39-44 | REAL | — | One field (EngineInfo), built at startup. |
| `CatalogDispatch::new` | 46-51 | REAL | — | Trivial ctor. |
| `impl Dispatch::dispatch` | 53-78 | REAL (2/65 served) | S2 | The hostile answer: real dispatch, tiny surface, honest refusals; policy/dispatch set incoherence (F-001). |
| `catalog_listings` | 84-101 | REAL | S4 | Live build from spec; silent filter_map drop pattern (F-004). |
| `tests::the_catalog_lists_all_65…` | 122-141 | REAL | — | Ran: pass. Pins count, first id, row fields. |
| `tests::engine_info_and_list_commands…` | 143-167 | REAL | — | Ran: pass. Pins both served results. |
| `tests::everything_else_is_a_typed_not_ready…` | 169-195 | REAL | — | Ran: pass. 2 sample commands of 63; refusal is structural (catch-all), so adequate. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_catalog_lists_all_65_commands_from_the_live_spec | YES | Exact count 65, first id "session.open", every row's since/deprecated/requires pinned. |
| engine_info_and_list_commands_serve_the_catalog_subset | YES | Destructures both Ok results; unreachable! on wrong shape. |
| everything_else_is_a_typed_not_ready_never_a_fake_success | YES | Pins NotReady + Control stage + "no engine" message for play and open. |
| (conformance, cross-crate) unauthenticated_read_only_mode_serves_the_safe_subset_only | NO — masks F-001 | Uses `EchoDispatch` answering `Unit` for every command (conformance.rs:129-142); proves the policy gate, says nothing about the real dispatch's answers. |
| (e2e, this app) the full session test | PARTIAL | Exercises list-commands, denied set-setting, not-ready session.open, engine-info — but never read-only mode with the real dispatch (asserts read_only == false at L175/306). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 embeds no engine … the served subset is catalog introspection" | dispatch.rs:5-6 | REAL | The match serves exactly the 2 named commands (L55-65); tests pin it (ran). |
| "an honest gap, never a silent no-op and never a fake success" | dispatch.rs:12-13 | REAL | All 63 others → typed NotReady; test `everything_else…` (ran). |
| "system.list-commands — … built from `commands::catalog::spec`, never a copy" | dispatch.rs:9-10 | REAL | `catalog_listings()` iterates `CommandId::ALL` + `command_spec` (L84-101). |
| "An optional local control surface for automation and remote-control-style UIs" | doc 05 §5.4 | SHALLOW | Surface exists (WS+auth+policy real, conformance-tested) but cannot control playback (2/65); no status row anywhere discloses it (F-002). |
| read-only safe subset = media.get-info, engine-info, tracks.list, snapshot | doc 05 §5.4 + policy.rs:65-70 | FALSE (end-to-end) | 3 of 4 answer not-ready through this dispatch (F-001). |

## 7. Dead / duplicated / unreachable code
- None in this file. (The served/not-served asymmetries live in policy.rs and the catalog; see F-001/F-002 and the command.rs audit.)
- Duplication note: `catalog_listings()` here and `command_listings()` in apps/cli/src/host.rs:1665-1676 build the same table two ways (spec-lookup vs `catalog_data()` map) — same output, pinned by both crates' tests; a shared constructor in the `commands` crate would remove the twin.

## 8. Dependencies & flags
- Deps: `commands` (catalog types/envelope/error), `control-protocol` (Dispatch trait). No engine dependency — the entire reason for the 2-command surface.
- Feature-gated at the PRODUCT level: doc 05 §5.4 "`control-server.enabled` defaults to false"; the app is also feature-gated by product.toml per e2e.rs (config + product + wire + dispatch under a feature gate). Not in the release lane (release.yml:193).

## 9. Open questions for the next agent
- Does any wave/card own the "engine-embedding composition" the NOT_READY message defers to? If the roadmap card was dropped, the control server's terminal state is introspection-only forever and doc 05 §5.4's safe subset should shrink to what exists.
- The `EngineInfo` served here is "product-identity-derived" — check apps/control-server/src/lib.rs/main.rs construction for whether `decoders`/`features` are populated or empty (the run.rs test builds them empty; if the real path also serves empty decoder lists, remote clients see an engine that decodes nothing — verify against product.toml features).
- RESOLVED during this audit (promoted to F-005): subscribe/unsubscribe are silent no-ops — `self.subscriptions` is write-only (server.rs:653-665) and `ServerFrame::Event` has zero constructors in the tree. No event forwarding exists at all.
