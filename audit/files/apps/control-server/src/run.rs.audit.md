# AUDIT: apps/control-server/src/run.rs
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2025-06-27 |
| Lines / functions | 128 / 1 fn + 1 struct + 1 test |
| File verdict | UNREACHABLE (real, test-executed code — no shipped entry point reaches it: flag off + not in release lane) |
| Reachable from shipped entry point? | NO — release lane builds only lumen-cli + desktop (release.yml:193); no shipped app depends on control-server (rg: zero deps in apps/{desktop,cli,web}/Cargo.toml); product.toml:167 `control-server = false` and config.rs:149-157 `require_feature` refuses startup (baseline test `repo_product_today_refuses_the_feature_gate ... ok`). Dev/test lane only: ci.yml:233-234 workspace build+test. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full Read (128 lines); `rg` for RunOutcome/capped/serve_loop users; `rg` product.toml feature flag; `sed -n` main.rs:246-258; baseline citation (cargo-test-workspace.txt:1649-1697) |

## 1. One-paragraph verdict
The loop itself is real and honest: accept, `serve_one` (upgrade → hello → requests → close), emit a session record, repeat, with a saturating counter and a documented exit contract — and it is exercised end-to-end by a real client in the e2e suite (`e2e_full_session_through_the_app_stack ... ok`, baseline :1686), not just by the cap-0 unit test in this file. But nothing shipped can ever reach it: the control server is a standalone binary that its own product model refuses to start (`product.toml` `[features] control-server = false`, enforced at startup), and the release lane builds only lumen-cli + desktop. The one code-level defect is cosmetic-but-real API debt: `RunOutcome::capped` is constant-`true` on every path that can produce it, its documented `false` state is unreachable, and main.rs carries a dead `else` branch for it.

## 2. What actually works (verified)
- Baseline execution: cargo-test-workspace.txt:1667 `run::tests::the_cap_stops_the_loop_after_serving_that_many_sessions ... ok`; :1686 `e2e_full_session_through_the_app_stack ... ok` — the e2e test (tests/e2e.rs:271-392) runs `serve_loop` "exactly as main runs it, capped at one session" with a real `TestClient` doing hello/requests/close and asserts `outcome.sessions == 1 && outcome.capped`.
- Cap semantics: read of 46-53 — `is_some_and(|cap| served >= cap)` checked BEFORE accept, so cap 0 returns without blocking (unit test proves it: sink empty, sessions 0).
- Overflow safety: `served.saturating_add(1)` (L57) — no panic at u32::MAX.
- Hostile-question answer for this app: the dispatch it drives is honest — baseline :1662-1663 `dispatch::tests::everything_else_is_a_typed_not_ready_never_a_fake_success ... ok` and `the_catalog_lists_all_65_commands_from_the_live_spec ... ok` (lib.rs doc: "v1 embeds no engine: it serves the catalog-introspection subset … and answers every other command with a typed `not-ready`").

## 3. Findings
F-001 [S3]
Where: apps/control-server/src/run.rs:26-29, 48-53; apps/control-server/src/main.rs:253
Code:
```
    /// Whether the session cap stopped the loop (`false` = the loop
    /// was asked for unbounded service and is only returning because
    /// the caller capped it or the caller's test ended the stream).
    pub capped: bool,
```
What is wrong: `capped` can never be `false`. The only `Ok` construction is L49-52 (`capped: true`); with `max_sessions: None` the loop has no exit except `Err` (transport failure at L59-64). The doc comment describes a `false` state ("the caller capped it or the caller's test ended the stream") that no code path produces — incoherent on its face (if the caller capped it, `capped` would be `true`), and evidently a vestige of a removed shutdown/stream-end mechanism. main.rs:253 `if outcome.capped { " (cap reached)" } else { "" }` carries a dead `else` branch. What correct looks like: delete the field (the cap is caller-known) or restore whatever graceful-stop path the comment remembers. Confidence: HIGH — Verification: read + rg (only construction is run.rs:51; only readers are main.rs:253 and tests asserting `true`).

F-002 [S4]
Where: apps/control-server/src/run.rs:47-66 (doc at 6-10)
Code:
```
    loop {
        if max_sessions.is_some_and(|cap| served >= cap) {
```
What is wrong: With `max_sessions: None` the loop is an unbounded blocking `serve_one` chain with no shutdown path in this app layer — documented as deliberate ("std-only, no signal handling … acceptable for a local control surface", L7-10), but it also means a hung client pins the single-connection server indefinitely; whether `serve_one` applies read/write timeouts is the protocol crate's business and is UNVERIFIED here (outside my files). Confidence: HIGH on the loop facts (read), UNVERIFIED on serve_one timeouts.

F-003 [S4]
Where: apps/control-server/src/run.rs:109-127
Code:
```
    #[test]
    fn the_cap_stops_the_loop_after_serving_that_many_sessions() {
```
What is wrong: The unit test only exercises cap 0. If the cap check were (re)moved or reordered after `serve_one`, this test would not fail red — it would HANG forever on accept (no client, no timeout). Failure-by-hang is a weak CI signal (the e2e test does cover a real session, which mitigates). Also four `unreachable!` on setup errors mean an environment without loopback bind panics instead of skipping. Confidence: HIGH — Verification: read + reasoning; not mutated (no-edit rule).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RunOutcome (struct) | 22-30 | REAL | S3 | `capped` constant-true; doc describes unreachable state (F-001) |
| serve_loop | 40-67 | REAL | S4 | executed via baseline unit + e2e; unbounded mode by design (F-002) |
| tests::engine_info (helper) | 78-90 | REAL | — | uses real `catalog_checksum()` for catalog_version |
| tests::UnitDispatch | 93-106 | REAL | — | honest test double: typed Completed(Unit), doesn't fake engine behavior |
| tests::the_cap_stops… | 108-127 | REAL | S4 | cap-0 only; hang-not-red failure mode (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_cap_stops_the_loop_after_serving_that_many_sessions (109) | WEAKLY | cap-0 path asserts sessions/capped/empty-sink; a reordered cap check would hang, not fail |
| e2e_full_session_through_the_app_stack (tests/e2e.rs, not this file) | YES | real client, full session, asserts sessions=1 + capped + session record content |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "accept one connection, run it to completion … log the SessionSummary, repeat" | run.rs:3-5 | REAL | serve_loop 47-66 + e2e baseline :1686 |
| "`max_sessions` caps the loop … (`None` = serve until the process is terminated)" | run.rs:6-7 | REAL | read; no other exit (F-002 notes the cost) |
| "capped … `false` = the loop was asked for unbounded service …" | run.rs:26-28 | FALSE | unreachable state (F-001) |
| main.rs exit contract "exit 0 — … the session cap (`--sessions`) was served" | main.rs:4-5 | REAL-UNPROVEN | read of main.rs:248-256; binary execution not performed (feature gate refuses startup with shipped product.toml — running it would exit 2, proving the gate; UNVERIFIED by execution to keep the tree clean) |

## 7. Dead / duplicated / unreachable code
- `RunOutcome::capped == false`: unreachable state; main.rs:253 `else { "" }` dead branch (F-001).
- The whole app: unreachable from shipped product while `product.toml:167 control-server = false` (enforced by config.rs:149-157; baseline `repo_product_today_refuses_the_feature_gate ... ok`).

## 8. Dependencies & flags
Crate `control-server` (bin `lumen-control-server`). Depends on `control-protocol` (transport/policy), `commands` (catalog), `product-config` (feature gate). No cargo feature flags; the gating is the product-model flag in product.toml. std-only loop; no unsafe; no `#[allow]`.

## 9. Open questions for the next agent
- Same cross-host `catalog_version` split noted in my spec.rs audit: this app reports `catalog_checksum().to_string()` (run.rs:81, config.rs:169-172) while apps/cli/src/host.rs:1634 and apps/web/src/host.rs:715 hardcode `"1"`. Doc 05 §2.12 does not say which is the contract.
- Does `control_protocol::ControlServer::serve_one` enforce read/write timeouts for a stuck client? (F-002; protocol crate not in my assignment.)
- Who flips `product.toml [features] control-server` to true (Phase 5+ per the comment) — until then every audit of this app is academically interesting but user-invisible.
