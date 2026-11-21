# AUDIT: apps/control-server/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-21 |
| Lines / functions | 52 / 0 functions (1 module doc, 4 module declarations, 10 re-exports) |
| File verdict | REAL (module root whose every behavioral doc claim is executed-verified by the crate's own baseline tests) |
| Reachable from shipped entry point? | NO from `lumen`/`lumen-tauri` (nothing in apps/cli, apps/desktop, or apps/desktop-tauri references control_server — rg-verified: the only `control_server::` consumers are its own main.rs and tests/e2e.rs). Its own binary `lumen-control-server` builds (baseline #1) and its 11 lib tests + 6 e2e tests ran ok (orchrun-test-workspace-nofailfast.log.txt:1454-1530) — but with the SHIPPED product.toml the binary refuses to start at the feature gate (`control-server = false`, product.toml:167), and that refusal is itself an executed test (`config::tests::repo_product_today_refuses_the_feature_gate ... ok`). Runtime-dead-by-gate in the shipped config, exactly as documented. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read lib.rs 1-52 (full); rg `control_server::` workspace-wide minus audit/target (consumers: main.rs, tests/e2e.rs only); Read main.rs 1-270 (which re-exports the binary actually imports); rg `^use crate::` in run.rs/dispatch.rs/config.rs/logging.rs (internal modules use module paths, not the lib root); sed product.toml 165-168 (`control-server = false`); sed baseline log 1454-1530 (11 lib tests + 6 e2e tests, all ok); rg wave-13 assignments (run.rs→A01, logging.rs→A04, config.rs→A05, dispatch.rs→A06, main.rs→A07 — submodule depth is theirs). |

## 1. One-paragraph verdict
A 52-line composition root that does nothing but declare four modules, re-export ten names, and make five factual claims — and every load-bearing claim checks out against executed baseline evidence: the product gate exists and refuses today's product.toml (test ok), v1 embeds no engine and answers everything outside the two catalog-introspection commands with typed `not-ready` (three dispatch tests ok, including a 65-command live-catalog sweep), the session cap works (run test ok), and the full stack serves a real session (e2e ok). For the wave's hostile question this file is a terminus, not a lane: no argv → open → demux path can exist here because the dispatch layer has no engine to drive — that absence is pinned by test, not just prose. The single finding is dead re-export surface: four of the ten re-exports have no consumer via the lib root. The honest-scope section (deferred security review, no rate limits, no Origin allowlist, no signal handling) is candid about the crate's limits and matches the code shape.

## 2. What actually works (verified)
- The crate's tests all ran and passed in baseline: 11/11 lib tests (log:1454-1467) — including `config::tests::repo_product_today_refuses_the_feature_gate` (the feature gate refuses the SHIPPED product.toml — the doc's "config alone enables nothing" rule, executed), `dispatch::tests::everything_else_is_a_typed_not_ready_never_a_fake_success`, `dispatch::tests::the_catalog_lists_all_65_commands_from_the_live_spec` — plus 6/6 e2e tests (log:1518-1530), including `e2e_full_session_through_the_app_stack`, `a_non_loopback_bind_in_the_operator_config_is_refused_at_bind`, `a_typoed_sensitive_grant_is_a_typed_startup_error`.
- The binary's process contract (main.rs:5-9, exit 0/2/3/4) is consumed via six of the ten re-exports (`AppError`, `CatalogDispatch`, `JsonStderr`, `load_config_file`, `resolve_server_config`, `serve_loop` — main.rs:26-31) — the re-export set is not decorative for those six.
- Module structure matches the doc's four-way split; no engine, WS, or framing code lives in this crate (the transport claims belong to control_protocol — main.rs imports `ControlServer`, `TokenSource`, `PROTOCOL_VERSION` from there; nothing transport-shaped is re-implemented here).

## 3. Findings

### F-001 [S4] — Four of the ten re-exports have no consumer via the lib root (dead API surface in an app crate)
Where: apps/control-server/src/lib.rs:48-51
Code:
```
pub use logging::LogRecord;
pub use logging::LogSink;
pub use logging::MemoryLog;
pub use run::RunOutcome;
```
What is wrong: No code imports any of these four through the crate root. main.rs (the only binary) imports six names (main.rs:26-31 — none of these four); tests/e2e.rs imports `control_server::logging::MemoryLog` via the MODULE path (e2e.rs:27), not the root; the crate's own submodules use `crate::logging::…` (run.rs:16-19). No external crate depends on control-server at all (rg over the workspace: only main.rs and e2e.rs reference `control_server::`).
Why it matters: An app crate (not a library) exports four root-level names nobody can reach through; each is a maintained surface that suggests an API consumer that does not exist. `RunOutcome` in particular is the return type of the used `serve_loop`, so the re-export looks load-bearing while being path-dead.
What correct looks like: Delete the four lines (e2e already demonstrates the module path is the working import), or have e2e import `control_server::MemoryLog` and keep exactly the names with consumers.
Confidence: HIGH — Verification: read + rg (all import sites enumerated).

## 4. Per-function table (no functions; every item in the file)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc | 1-36 | REAL | — | Every behavioral claim executed-verified (see §6). Honest-scope gaps (no rate limits, no Origin allowlist, no signal handling, deferred security review) stated plainly. |
| `pub mod config;` | 38 | REAL | — | Submodule audited by W13-A05. |
| `pub mod dispatch;` | 39 | REAL | — | Submodule audited by W13-A06. |
| `pub mod logging;` | 40 | REAL | — | Submodule audited by W13-A04. |
| `pub mod run;` | 41 | REAL | — | Submodule audited by W13-A01. |
| `pub use config::AppError;` | 43 | REAL | — | Consumed: main.rs:26. |
| `pub use config::load_config_file;` | 44 | REAL | — | Consumed: main.rs:29, e2e.rs:26. |
| `pub use config::resolve_server_config;` | 45 | REAL | — | Consumed: main.rs:30, e2e.rs:28. |
| `pub use dispatch::CatalogDispatch;` | 46 | REAL | — | Consumed: main.rs:27, e2e.rs:25. |
| `pub use logging::JsonStderr;` | 47 | REAL | — | Consumed: main.rs:28. |
| `pub use logging::LogRecord;` | 48 | REAL / dead-at-root | S4 (F-001) | No root-path consumer. |
| `pub use logging::LogSink;` | 49 | REAL / dead-at-root | S4 (F-001) | run.rs uses `crate::logging::LogSink`. |
| `pub use logging::MemoryLog;` | 50 | REAL / dead-at-root | S4 (F-001) | e2e uses the module path. |
| `pub use run::RunOutcome;` | 51 | REAL / dead-at-root | S4 (F-001) | Type inferred in main; never named. |
| `pub use run::serve_loop;` | 52 | REAL | — | Consumed: main.rs:31, e2e.rs:29. |

## 5. Test honesty
No tests in this file. The crate's tests (which verify this file's doc claims) all ran in baseline:
| Test (crate) | Would it fail if the code were broken? | Reason |
|---|---|---|
| config::tests::repo_product_today_refuses_the_feature_gate | YES | Runs the REAL shipped product.toml through the gate — a flipped gate or broken check flips the outcome. Executed ok (log:1457). |
| dispatch::tests::everything_else_is_a_typed_not_ready_never_a_fake_success | YES | Enumerates non-served commands asserting typed refusals — a fake-success regression fails. Executed ok. |
| dispatch::tests::the_catalog_lists_all_65_commands_from_the_live_spec | YES | Live-spec count pin (65) — catalog drift fails it. Executed ok. |
| e2e_full_session_through_the_app_stack | YES | Real bind + serve + session through the composed stack. Executed ok (log:1530). |
| a_non_loopback_bind_in_the_operator_config_is_refused_at_bind | YES | The loopback-only posture, executed. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "product.toml's [features] control-server must be true for the server to start at all (doc 07 rule F2 — config alone enables nothing)" | lib.rs:9-12 | REAL | product.toml:167 `control-server = false`; executed test `repo_product_today_refuses_the_feature_gate ... ok` (log:1457) — with the shipped config the binary refuses to start. |
| "v1 embeds no engine: it serves the catalog-introspection subset (system.get-engine-info, system.list-commands) … answers every other command with a typed not-ready" | lib.rs:13-18 | REAL | Three dispatch tests executed ok (log:1459-1461), incl. the 65-command live-catalog sweep and the never-a-fake-success pin. Hostile-question relevance: NO media-open path exists through this crate — proven by test, not prose. |
| "logging … token-free (V8: the token reaches the controlling terminal through its single sanctioned reader, never the log stream)" | lib.rs:19-22 | REAL (spot) | main.rs:229-242: token printed to stderr as the sanctioned reader, structured logs separate; e2e `the_production_sink_renders_one_parseable_json_line_per_record` ok. Deep verification is W13-A04's (logging.rs). |
| "run: the accept/serve event loop with a session cap for scripted use" | lib.rs:23-24 | REAL | `run::tests::the_cap_stops_the_loop_after_serving_that_many_sessions ... ok` (log:1466); e2e full session ok. Deep: W13-A01. |
| "Everything transport-shaped — WS framing, handshake, token compare, deny-by-default policy, request validation — is control_protocol's; this app composes" | lib.rs:26-29 | REAL (structural) | No framing/token code in this crate's four modules (module list + main.rs imports from control_protocol); per-transport audits live with control_protocol (another wave). |
| "Deep security review is deferred to Phase 8 … [PROPOSAL] rate limits and the Origin allowlist … are recorded gaps. No signal handling" | lib.rs:31-36 | REAL (honest gaps) | Stated limits match the code shape (std-only main, no signal hooks — main.rs:264-270). These are accepted, documented risks, not hidden ones. |

## 7. Dead / duplicated / unreachable code
- F-001's four dead re-exports (the only dead surface in the file).
- The crate itself is runtime-unreachable in the shipped product (feature gate false) — by documented design, not defect; the gate's refusal is tested against the real product.toml.

## 8. Dependencies & flags
- Declared modules consume: `control_protocol` (transport/token/policy — the ARCH-5 reuse), `commands` (catalog), `product_config` (schema pipeline, via config/main). No feature gates in this crate; the RUNTIME gate is product.toml's `[features] control-server` (false today). No unsafe.

## 9. Open questions for the next agent
- W13-A05 (config.rs): the doc's "fixed-token" option — verify a fixed token never reaches the log stream either (the V8 claim's other half).
- W13-A06 (dispatch.rs): `engine_info` — is it the SAME hard-coded-literal disease as `lumen engine-info` (baseline standing defect: `apps/cli/src/host.rs` engine_info under-reports decoders)? `config::tests::engine_info_carries_product_identity_and_catalog_checksum` suggests product identity + checksum, but whether decoder claims are live-registry-derived or literals is A06's to answer.
- Feature-matrix auditors: "control-server" rows should be verdict=UNREACHABLE-in-shipped-config (gate false, tested refusal) — no baseline run demonstrates a serving control-server against the shipped product.toml (the e2e runs use test fixtures).
