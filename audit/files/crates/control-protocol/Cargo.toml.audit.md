# AUDIT: crates/control-protocol/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-21 |
| Lines / functions | 34 / 4 config blocks ([package], [lints], [dependencies]) |
| File verdict | REAL (crate); UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO. Sole dependent is `apps/control-server/Cargo.toml` (rg over all Cargo.tomls) — a workspace binary that is NOT `lumen`, NOT `lumen-tauri`, and appears in NO packaging payload (rg "control-server\|control_server" scripts/packaging/* → no matches). The CLI itself reports the protocol as unwired: `apps/cli/src/host.rs:1635` `protocol_version: "unwired (control-protocol is a later card)".to_string(),`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `ls crates/control-protocol/src/` (module list vs header claims); `wc -l` (4,522 LOC); `rg -l control-protocol --glob Cargo.toml` (dependents); python3 Cargo.lock parse (block consistency); `rg -c "#\[test\]"` (47 tests); `rg control scripts/packaging/` (payload absence); sed -n quotes. |

## 1. One-paragraph verdict
The manifest's every checkable claim is true: the header's component list (in-tree
SHA-1, base64, WebSocket frame codec, HTTP upgrade, handshake, token auth, policy)
maps 1:1 onto real source files (sha1.rs, base64.rs, frame.rs, http.rs, protocol.rs,
token.rs, policy.rs, server.rs, client.rs — 4,522 LOC, 47 in-crate tests that ran in
baseline #3's 1681-pass workspace run); the pinned serde/serde_json versions are
byte-identical to the `commands` crate's (1.0.229 / 1.0.154), as the "same versions
the commands crate pins" comment requires; Cargo.lock lists exactly commands+serde+
serde_json; no external transport crates (tungstenite/sha1/base64) — the C5/ADR-011
clean-room claim holds. The one hostile fact: none of this is reachable from a shipped
`lumen` or `lumen-tauri` binary — the only consumer is the unpackaged
`apps/control-server` binary, and product.toml itself exposes `control-server = false`
(Phase 5+ [PROPOSAL]).

## 2. What actually works (verified)
- Version-family claim: control-protocol/Cargo.toml:33-34 pins `serde = { version =
  "1.0.229", features = ["derive"] }`, `serde_json = "1.0.154"` — identical to
  crates/commands/Cargo.toml:39-40 (both files read). Single lock versions (python3
  Cargo.lock parse, prior file audit).
- Cargo.lock block for `control-protocol 0.1.0` lists exactly `commands`, `serde`,
  `serde_json` — manifest/lock consistent.
- Header claims vs sources: `ls crates/control-protocol/src/` → base64.rs client.rs
  error.rs frame.rs http.rs lib.rs policy.rs protocol.rs server.rs sha1.rs token.rs —
  every named component exists; `rg "tungstenite|sha1 =|base64 =" Cargo.toml`-style
  check: no such deps in the manifest.
- sha1.rs is a real RFC 3174 streaming implementation with external test vectors
  (doc comment: "Pinned by external vectors (RFC 3174 examples, the NIST million-`a`
  vector, and the RFC 6455 §1.3 end-to-end handshake example) ... (D-5)").
- 47 `#[test]`s across the crate ran in baseline #3 (`cargo test --workspace`:
  1681 passed / 1 failed — failure elsewhere).

## 3. Findings

### F-001 [S3] 4,522-LOC protocol plane unreachable from any shipped binary
Where: crates/control-protocol/Cargo.toml:1-11 (cross-evidence: rg dependents;
scripts/packaging greps; apps/cli/src/host.rs:1635; product.toml:167)
Code:
```
# The remote-control protocol crate: an in-tree minimal RFC 6455
# WebSocket server (HTTP upgrade with an in-tree SHA-1 + base64 for
# Sec-WebSocket-Accept, full frame codec incl. fragmentation and control
# frames), the `lumen-control/1` versioned application handshake, the
# per-start token authentication, the deny-by-default sensitive-command
# allowlist policy, and request validation against the ONE command layer
```
What is wrong: not the manifest itself — the reachability ledger. The crate's only
dependent is `apps/control-server` (rg over every Cargo.toml). That binary is not
`lumen`, not `lumen-tauri`, and is referenced by NO packaging script
(`rg "control-server|control_server" scripts/packaging/*` → empty), so no shipped
artifact contains this code. The shipped CLI hard-codes
`protocol_version: "unwired (control-protocol is a later card)"` (host.rs:1635), and
product.toml:167 exposes `control-server = false # Phase 5+ [PROPOSAL]`.
Why it matters: users cannot reach 4.5k LOC of tested protocol code; reviewers must
know this entire plane (plus apps/control-server) is currently dead weight in every
shipped build — cost without user-visible benefit, and a large surface that only CI
exercises.
What correct looks like: fine to keep as a Phase 5+ runway, but the workspace should
say so in one place (a "not yet shipped" note in doc 01's registry row would do), or
gate the crate behind a workspace feature until the phase lands.
Confidence: HIGH — Verification: read + rg (dependents, packaging) + baseline #3.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments 1-19 | 1-19 | REAL | — | every component claim maps to a real file (verified by ls); version-family claim verified; "no tungstenite/sha1/base64 crates" verified |
| [package] name/version/edition/publish | 21-25 | REAL | — | "control-protocol" 0.1.0, edition 2024, publish=false; lock block consistent |
| [package] description | 26 | REAL | — | matches actual contents (server, handshake, token auth, policy all exist) |
| [lints] workspace = true | 28-29 | REAL | — | inherits; crate's own doc vows "no unsafe/unwrap/expect/panic" — spot-check consistent with baseline clippy -D warnings clean (#5) |
| [dependencies] commands (path) | 32 | REAL | — | same-layer whitelisted pair; used (lib.rs validates request payloads against the catalog) |
| [dependencies] serde / serde_json | 33-34 | REAL | — | identical pins to commands crate; single lock versions |

## 5. Test honesty
47 in-crate tests (rg -c "#\[test\]": token 3, server 4, policy 6, http 5, error 6,
base64 3, frame 11, protocol 6, sha1 3) executed in baseline #3's workspace run
(1681 passed / 1 failed / 24 ignored — the failure and this crate are unrelated, per
INDEX.md: apps/desktop zero-alloc). sha1.rs pins EXTERNAL vectors (RFC 3174/NIST/RFC
6455 examples), not self-generated goldens — the honest pattern.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the same versions the `commands` crate pins" | Cargo.toml:13-15 | REAL | both manifests read: 1.0.229 / 1.0.154 identical |
| "Everything transport-level ... implemented in-tree, std-only ... no tungstenite/sha1/base64 crates" | Cargo.toml:17-19 | REAL | src/ has sha1.rs/base64.rs/frame.rs/http.rs; manifest has no such deps |
| "request validation against the ONE command layer" | Cargo.toml:10-11 | REAL | path dep on commands; lock block |
| (implied) the protocol is shipped | — | FALSE-by-omission → recorded as F-001 | CLI: "unwired"; no packaging refs; product.toml control-server=false |

## 7. Dead / duplicated / unreachable code
The whole crate + apps/control-server are unreachable from shipped entry points
(F-001) — but internally alive (tests, e2e in apps/control-server/tests/e2e.rs).

## 8. Dependencies & flags
commands (path), serde 1.0.229+derive, serde_json 1.0.154. No feature flags, no
dev-dependencies. No new dependency classes — C5 claim holds.

## 9. Open questions for the next agent
- apps/control-server/src/config.rs reads product.toml (earlier rg) — worth auditing
  whether it duplicates the mirror-constant pattern seen in apps/cli (F-004 of
  product.toml audit).
- lib.rs defers rate limits and Origin allowlist to Phase 8 (card 8.A2) — a security
  reviewer should confirm `server::Limits` actually enforces the serial-connection
  bound claimed ("serves connections serially").
