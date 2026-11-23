# AUDIT: crates/control-protocol/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-23 |
| Lines / functions | 34 / manifest: 1 package, 3 dependencies, no features, 1 bin-less lib |
| File verdict | REAL — every header claim verified against code, lockfile, boundaries.toml, and executed tests |
| Reachable from shipped entry point? | NO (crate not in lumen-cli/desktop graphs — verified manifests; live only via apps/control-server, itself feature-gated off: product.toml:167) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg over Cargo.lock (`control-protocol`, `tungstenite|sha1|base64`), boundaries.toml, src (serde/serde_json usage, PROTOCOL consts); cross-read crates/commands/Cargo.toml; prior executed test runs (3+5+25 lib/conformance) as evidence |

## 1. One-paragraph verdict
This manifest's 19-line header makes six checkable claims and all six hold. The dependency list (commands, serde 1.0.229+derive, serde_json 1.0.154) is exactly what the lockfile shows for the package (Cargo.lock:148-154), every dependency is actually used in src (rg), the version pins are byte-identical to the `commands` crate's pins (crates/commands/Cargo.toml:39-40), the same-layer `control-protocol->commands` pair is recorded in boundaries.toml:45, and the "no tungstenite/sha1/base64 crates" claim is true for this crate's closure — the `base64 0.23.1` entry in the workspace lockfile belongs to ureq/streaming, not to this package. Zero findings; the checks are itemized below per the zero-findings rule.

## 2. What actually works (verified)
— Header claims vs reality, each verified:
| Claim (Cargo.toml line) | Check | Result |
|---|---|---|
| "in-tree minimal RFC 6455 WebSocket server … HTTP upgrade with an in-tree SHA-1 + base64" (L5-7) | http.rs/sha1.rs/base64.rs audits + executed tests (5 http unit, 3 sha1 unit incl. RFC-vector, 25 conformance over real TCP) | HOLDS |
| "full frame codec incl. fragmentation and control frames" (L7-8) | conformance run included `fragmented_hello_round_trips_with_interleaved_ping`, `ping_is_answered_by_a_matching_pong`, close-echo tests — all passed (executed) | HOLDS |
| "`lumen-control/1` versioned application handshake" (L8) | protocol.rs:37-40 `PROTOCOL_NAME: &str = "lumen-control"`, `PROTOCOL_VERSION: u32 = 1`; conformance `unknown_protocol_name_is_rejected`, `version_mismatch_is_a_typed_error_and_close_4402` passed | HOLDS |
| "per-start token authentication" (L9) | conformance `wrong_token_closes_4401_and_counts`, `five_failures_lock_out_even_the_right_token` passed | HOLDS |
| "deny-by-default sensitive-command allowlist policy" (L9-10) | conformance `sensitive_command_denied_by_default_and_connection_survives` passed; policy.rs `deny_unknown_fields` + `ServerPolicy` verified in config.rs audit | HOLDS |
| "request validation against the ONE command layer (`commands` — same-layer whitelisted pair)" (L10-11) | protocol.rs:233 `validate_request_shape(...) -> Result<(), commands::CommandError>`; boundaries.toml:45 `same_layer_pairs = [… "control-protocol->commands"]` | HOLDS |
| "serde/serde_json … the same versions the `commands` crate pins" (L13-15) | crates/commands/Cargo.toml:39-40 = `serde = { version = "1.0.229", features = ["derive"] }`, `serde_json = "1.0.154"` — byte-identical | HOLDS |
| "Everything transport-level … implemented in-tree, std-only … no tungstenite/sha1/base64 crates" (L17-19) | Cargo.lock:150-154 package deps = commands/serde/serde_json only; no tungstenite anywhere in the lock; the lock's `base64 0.23.1` belongs to ureq (streaming), not this closure | HOLDS (crate-scoped) |

## 3. Findings
None. Zero-findings checklist (what was checked and how):
- Unused dependencies: none — `commands` (server.rs/dispatch/protocol.rs types), `serde` (protocol.rs:32-33, policy.rs derives), `serde_json` (server.rs:618-686, client.rs:281) all referenced (rg).
- Version-skew between paired crates: none (pins byte-identical to commands).
- Undeclared usage: none — transport is std-only (`std::io`, `std::net`); no extern crate in src outside the three deps.
- Feature flags: none declared, none needed — nothing gated off that the shipped build would enable.
- `[lints] workspace = true` inheritance: honored — no unwrap/expect/panic in this crate's production code (workspace denies them; builds clean per baseline and my test compiles).
- publish=false, edition 2024: consistent with every sibling crate.
- Layering: L1→L1 pair whitelisted (boundaries.toml:41-45); no upward dependency.

## 4. Per-function table
Not applicable (manifest, no functions). Items: [package] (name/version/edition/publish/description) ✓ consistent with lib.rs doc header; [lints] ✓; [dependencies] ×3 ✓ all used.

## 5. Test honesty
Not applicable (no tests in this file). Evidence for claims drawn from executed suites: `cargo test -p control-protocol --lib` (47 tests total per sibling protocol.rs audit; my filtered runs: sha1 3, http 5) and `--test conformance` (25 passed).

## 6. Doc claims touching this file
See §2 table — all eight header claims verified REAL.

## 7. Dead / duplicated / unreachable code
The crate itself is unreachable from the release lane (lumen-cli/desktop manifests have no edge on it); reachable only via apps/control-server (feature-gated off in the shipped product model, product.toml:167). No dead manifest entries.

## 8. Dependencies & flags
commands (path), serde 1.0.229+derive, serde_json 1.0.154. No [features], no dev-dependencies (conformance tests are std-only + the three deps). No [bin] — library crate; the app owns the binary.

## 9. Open questions for the next agent
- None for this file. Adjacent: base64.rs audit (sibling) — the "in-tree base64" leg of the L5-7 claim rests on it; my http/sha1 test runs transitively exercised it (RFC 6455 accept vector round-trips through base64::encode/decode).
