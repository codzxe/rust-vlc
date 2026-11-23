# AUDIT: apps/control-server/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-23 |
| Lines / functions | 52 / 1 facade (4 pub mods + 12 re-exports; no logic) |
| File verdict | REAL (facade over verified-real modules) |
| Reachable from shipped entry point? | NO — the binary `lumen-control-server` is not in the release lane (`-p lumen-cli -p desktop`, baseline/cargo-build-release-lumen-cli.txt:1), and product.toml:167 `control-server = false` makes `require_feature` refuse startup under the repo product model (proven by the executed test `repo_product_today_refuses_the_feature_gate`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/control_server-58398a5795b79e6e` → 11 passed, EXIT=0; `./target/debug/deps/e2e-2485aaf028bcffb2` → 6 passed, EXIT=0; sed/rg module verification (below) |

## 1. One-paragraph verdict
Every concrete claim in this facade's doc comment checks out against the modules it maps: `config` really enforces the product-model gate (config.rs:149-157 `require_feature` reads `features.control-server`; the executed test proves today's product.toml refuses startup — the app is honestly unshippable, not silently dead); `dispatch` really is the honest gap — `CatalogDispatch` serves exactly `system.get-engine-info` + `system.list-commands` from the live catalog and answers the other 63 commands with a typed `NotReady` (dispatch.rs:54-75, pinned by the executed test `everything_else_is_a_typed_not_ready_never_a_fake_success`); `run` really is a serial accept/serve loop with a session cap (run.rs:40-56, calls the same `serve_one` the conformance suite drives); `logging` really emits JSON records. One test-honesty defect at the module boundary this facade advertises: the e2e test that "proves" the V8 token-free log stream checks only that no log KEY is named "token" — a VALUE containing the token would pass. The app composes rather than re-implements (ARCH-5) as claimed: all transport lives in control-protocol.

## 2. What actually works (verified)
— verified by `./target/debug/deps/control_server-58398a5795b79e6e` → `test result: ok. 11 passed` (incl. `repo_product_today_refuses_the_feature_gate`, `the_catalog_lists_all_65_commands_from_the_live_spec`, `everything_else_is_a_typed_not_ready_never_a_fake_success`).
— verified by `./target/debug/deps/e2e-2485aaf028bcffb2` → `test result: ok. 6 passed` (incl. `e2e_full_session_through_the_app_stack` — a real loopback session through config→bind→serve→dispatch→log).
— module-claim verification by read: config.rs:144-157 (F2 gate), dispatch.rs:50-95 (2 served + typed NotReady + `catalog_listings()` over `CommandId::ALL`), run.rs:40-56 (`serve_loop` → `server.serve_one(dispatch)`), logging.rs:1-10 (JSON sink trait), main.rs:229-241 (single sanctioned token print to the terminal).

## 3. Findings
F-001 [S3]
Where: apps/control-server/tests/e2e.rs:393-410 (the test backing lib.rs:19-22 and logging.rs:3-5)
Code:
```
    // The structured log carries the session record (no token, V8).
```
and the actual assertion:
```
    for (key, _) in &fields {
        assert_ne!(*key, "token", "the log stream must never carry the token");
    }
```
What is wrong: the test claims to prove the log carries "no token, V8" but only checks KEY NAMES. If any field VALUE contained the token string (e.g. a future `"message"` or `"remote"` field echoing hello input), this test passes. The V8 property lib.rs advertises ("token-free … the token never appears in this crate's own errors or traces") is unproven at the value level. Why it matters: this is the security invariant of the whole control surface; its only e2e guard is vacuous against the realistic leak (values, not keys). What correct looks like: `assert!(!format!("{fields:?}").contains("e2e-fixed-token"))` — the fixed token is already in the test's config (e2e.rs:126). Confidence: HIGH — Verification: read + executed (the e2e binary passes with the key-only check).

## 4. Per-function table (facade items)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| doc comment (module map + honest scope) | 1-36 | REAL | every claim verified per §2; deferred gaps (rate limits, Origin allowlist, signal handling) match code reality (corroborates W02-A05 server.rs F-003) |
| `pub mod config/dispatch/logging/run` | 38-41 | REAL | modules real, tested (11 lib tests executed) |
| 12 re-exports (AppError…serve_loop) | 43-52 | REAL | used by main.rs/e2e.rs; `LogSink`/`RunOutcome` re-exports unused by name outside the crate — facade API surface, not dead code |

## 5. Test honesty (module tests behind this facade)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| repo_product_today_refuses_the_feature_gate | YES | gate removed → resolve succeeds → test fails (the honest-unshippable proof) |
| everything_else_is_a_typed_not_ready_never_a_fake_success | YES | any fake success (Ok) for a 3rd command fails it |
| the_catalog_lists_all_65_commands_from_the_live_spec | YES | count/content from live spec (corroborates my ids.rs count of 65) |
| e2e_full_session_through_the_app_stack | PARTIAL | real socket session, welcome/response/denial/log asserted; the token-free-log sub-check is key-names only (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 embeds no engine … typed `not-ready` — an honest gap, never a silent no-op" | lib.rs:15-18 | REAL | dispatch.rs:54-75 + executed test (11 passed) |
| "`product.toml`'s `[features] control-server` must be `true` for the server to start" | lib.rs:9-12 | REAL | config.rs:149-157 + executed refusal test; product.toml:167 = false today |
| "machine-parseable, token-free (V8 …)" logging | lib.rs:19-22 | REAL at key level, UNPROVEN at value level | e2e.rs:407-410 (F-001) |
| "Everything transport-shaped … is control_protocol's; this app composes" | lib.rs:26-29 | REAL | run.rs/dispatch.rs contain no transport code (read) |
| "rate limits and the Origin allowlist … recorded gaps" | lib.rs:33-35 | REAL (honest) | no rate caps exist (W02-A05 server.rs F-003) |

## 7. Dead / duplicated / unreachable code
None in this file. Crate unreachable from shipped lane (see Field) — the app is a built-and-tested workspace member (`cargo test --workspace` builds it, baseline :1649-1700) whose product gate is intentionally OFF.

## 8. Dependencies & flags
apps/control-server depends on control-protocol + commands + product-config (+ serde_json for e2e); no feature flags; no unsafe. Bin name `lumen-control-server` (Cargo.toml:37).

## 9. Open questions for the next agent
- F-001's fix is one line in e2e.rs; worth bundling with any Phase-8 (8.A2) security-review card.
- When the feature gate flips true (Phase 5+), re-audit reachability: this facade becomes the shipped composition root.
