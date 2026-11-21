# AUDIT: crates/control-protocol/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-21 |
| Lines / functions | 81 / 0 functions (10 module decls, 4 re-export groups, 1 module doc block) |
| File verdict | REAL (crate root of a real, tested, executed crate) — one cited spec leg (wire fuzzing) SHALLOW |
| Reachable from shipped entry point? | NO from `lumen` CLI / `lumen-tauri` (neither depends on control-protocol — only apps/control-server links it, Cargo.toml grep); YES from `lumen-control-server` (repo app; built clean in baseline #1, its 11 lib + 6 e2e tests passed) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | `rg "unsafe|unwrap|expect|panic|todo|unimplemented"` over src/*.rs → 0 matches (hygiene claim); `rg "control-protocol"` over apps/*/Cargo.toml → apps/control-server only; sed -n of server.rs:300-330 (bind gate), :429-441 (serve_one), :102-124 (AuthLimits); rg over fuzz/README.md table → 35 targets, none control-protocol; doc 01 §2.3 (e) row read at 420-432; sha1.rs:160-166 + base64.rs:170-172 vectors read; token.rs read 28-137; wc -l all 9 modules; baseline: control_protocol 47 passed (1411-1412), conformance 25 passed (1413-1417), control_server 11 passed (1519-1521), e2e 6 passed (1525-1529), clippy clean (#5), boundaries gate (#11) — control-protocol→commands edge declared |

## 1. One-paragraph verdict
The crate root is a rare specimen: a module doc whose every checkable claim survives hostile verification. Loopback-only bind is code, not prose (server.rs:316-323, typed `NonLoopbackBind`, re-verified after resolve; executed conformance test `non_loopback_bind_is_refused_at_construction_conformance`); the 5-failure/60-s lockout is `AuthLimits::documented()` and executed; close codes 4401/4402 are implemented and tested; SHA-1/base64 are in-tree and pinned by genuinely external vectors (RFC 3174 §7.3 + NIST million-`a`, RFC 4648 §10); the "never does" hygiene list (`no unsafe/unwrap/expect/panic`) is literally true — zero matches across 4,130 lines of src; and the honest-scope section records real gaps (rate limits, Origin allowlist, Phase-8 security deferral, serial serving). The one spec leg that is NOT met: doc 01 §2.3 (e) requires **wire fuzzing** for this crate, the repo's fuzz registry carries 35 targets — none for control-protocol's network-facing frame/http parsers — and the honest-scope list does not record that miss. Two nits: the "~100 lines each" size claim (actual: 210 and 220), and `pub mod sha1`/`base64` exporting crypto primitives the same paragraph forbids general use of.

## 2. What actually works (verified)
— Crate tests EXECUTED: baseline test-workspace.log.txt:1411-1412 `control_protocol … 47 passed`; conformance 25 passed (:1413-1417); app e2e 6 passed (:1525-1529).
— Hygiene claim EXECUTED+GREPPED: zero `unsafe`/`unwrap`/`expect`/`panic`/`todo!`/`unimplemented!` in src (rg); baseline #5 clippy `-D warnings` CLEAN.
— Loopback gate EXECUTED: conformance.rs:727 `non_loopback_bind_is_refused_at_construction_conformance` … ok (baseline).
— Token auth EXECUTED: `five_failures_lock_out_even_the_right_token` (conformance.rs:393), `frames_before_authentication_are_refused` (:373) — ok.
— Version negotiation EXECUTED: `version_mismatch_is_a_typed_error_and_close_4402` (conformance.rs:440), `unknown_protocol_name_is_rejected` (:460) — ok.
— External vectors EXECUTED: sha1.rs:163 `external_vectors` (incl. SHA-1("abc") = `a9993e364706816aba3e25717850c26c9cd0d89d`) and base64.rs:172 `rfc4648_vectors` run in the 47-passed lib suite.
— Boundaries: the same-layer pair `control-protocol → commands` declared in Cargo.toml:32 and clean in gate #11 (the only architecture violation is lumen-cli→streaming, unrelated).

## 3. Findings

### F-001 [S3] — Doc 01 §2.3 (e) "Wire fuzzing" leg is not implemented and not recorded as a gap
Where: crates/control-protocol/src/lib.rs:32-34 (citation), docs/01-architecture.md:428-429 (requirement), fuzz/README.md:56-90 (registry — absent)
Code:
```rust
//! * [`client`] — the in-tree WS conformance client used by this crate's
//!   and the app's tests (doc 01 §2.3 (e): "wire fuzzing, version
//!   negotiation, round-trip properties").
```
docs/01-architecture.md:428-429:
```markdown
**(e) Tests.** Wire fuzzing (untrusted input — MP §12), version
negotiation, round-trip properties.
```
What is wrong: the citation is accurate (the doc row does demand wire fuzzing), but the repo's fuzz registry — 35 targets in fuzz/README.md, every one a demux/codec/subtitle/playlist/streaming parser — contains no control-protocol target. The crate's two network-facing parsers (frame.rs, 909 lines: length forms, masks, fragmentation, control frames; http.rs, 493 lines: upgrade head parsing) have zero coverage-guided fuzzing. What exists is directed dirty-world conformance (truncated header, unmasked frame, 4-GiB claim, bad mask, invalid UTF-8, dirty JSON payloads — good, but hand-written, not fuzzing). The lib.rs "Honest scope" section records the rate-limit/Origin/Phase-8 gaps but omits this one; the fuzz-registry gate (scripts/fuzz_registry_check.sh, baseline #18 PASS) only checks registry-vs-implementation consistency, so a parser that never registered cannot fail it.
Why it matters: doc 01 §2.3 (e) is a per-crate test requirement at the same level as "version negotiation" and "round-trip properties" (both implemented and executed); the wire-fuzzing leg is silently unmet for the one crate whose parsers face untrusted network bytes. The crate's own honesty pattern makes the omission stand out.
What correct looks like: a `fuzz_control_frame` (+ optionally `fuzz_control_http_head`) target with seed corpus in fuzz/, registered in fuzz/README.md — or a line in lib.rs's honest-scope list recording the deferral.
Confidence: HIGH — Verification: read registry table + doc row + lib.rs; rg "fuzz" over crates/control-protocol → only lib.rs:33 (comment).

### F-002 [S4] — "~100 lines each" undersells: sha1.rs=210, base64.rs=220
Where: crates/control-protocol/src/lib.rs:38-39
Code:
```rust
//! SHA-1 ([`sha1`]) and base64 ([`base64`]) are implemented in-crate
//! (~100 lines each, specified by RFC 3174 / RFC 4648 and pinned by
```
What is wrong: `wc -l` — sha1.rs 210 lines, base64.rs 220 lines (each ~50 of test). Even implementation-only, both exceed 100 by half again.
Why it matters: a size claim in the security-review narrative; the real number is 2× the claim. Trivial, but this audit's job is to make every number checkable.
What correct looks like: "~200 lines each" or drop the count.
Confidence: HIGH — Verification: wc -l.

### F-003 [S4] — `pub mod sha1` / `pub mod base64` export crypto primitives the same paragraph forbids using
Where: crates/control-protocol/src/lib.rs:40-42 + 60, 68
Code:
```rust
//! `Sec-WebSocket-Accept` computation — it is not a general security
//! primitive and must not be used as one.
```
lib.rs:60, 68:
```rust
pub mod base64;
```
```rust
pub mod sha1;
```
What is wrong: the doc says SHA-1 "must not be used as [a general security primitive]", yet both modules are `pub` — full public API of the crate — with zero consumers outside the crate (rg: only internal `crate::base64`/`crate::sha1` uses in token.rs/sha1.rs/http.rs). The doc prohibition is advisory where the visibility could enforce it.
Why it matters: any sibling crate can `use control_protocol::sha1::sha1` for something security-relevant, and nothing but a doc comment stops it.
What correct looks like: `mod sha1; mod base64;` (crate-private) with the tiny surface http.rs/token.rs need re-exported, or `pub(crate)`.
Confidence: HIGH — Verification: read + rg over whole tree for external users (none).

## 4. Per-function table (modules + re-exports — lib.rs has no functions)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc block | 1-58 | REAL | S3 (F-001), S4 (F-002) | 12 concrete claims checked — 11 hold (see §6); wire-fuzzing leg unmet. |
| `pub mod base64` | 60 | REAL | S4 (F-003) | 220 lines; RFC 4648 §10 external vectors; internal-only consumers. |
| `pub mod client` | 61 | REAL (test infra) | — | Audited separately (client.rs audit this agent); unconditional compile noted there. |
| `pub mod error` | 62 | REAL | — | ProtocolError/CloseCode; TimedOut mapping verified in frame.rs/http.rs. |
| `pub mod frame` | 63 | REAL | S3 context (F-001) | 909-line wire parser, no fuzz target. |
| `pub mod http` | 64 | REAL | S3 context (F-001) | 493-line upgrade parser, no fuzz target. |
| `pub mod policy` | 65 | REAL | — | Sensitive-op deny-by-default; executed tests (conformance 743/783). |
| `pub mod protocol` | 66 | REAL | — | lumen-control/1 handshake; 4402 executed. |
| `pub mod server` | 67 | REAL | — | 987 lines; bind gate + serve_one serial API + AuthLimits verified. |
| `pub mod sha1` | 68 | REAL | S4 (F-002/F-003) | 210 lines; RFC 3174 + NIST vectors executed. |
| `pub mod token` | 69 | REAL | — | 256-bit urandom + documented weak fallback + constant_time_eq with per-position tests. |
| `pub use error::{CloseCode, ProtocolError}` | 71 | REAL | — | Consumed by conformance/e2e/app. |
| `pub use policy::{…}` | 72 | REAL | — | PolicyFile/ServerPolicy consumed by app config + tests. |
| `pub use protocol::{…}` | 73-76 | REAL | — | PROTOCOL_NAME/VERSION used by apps/control-server config.rs/main.rs. |
| `pub use server::{…}` / `pub use token::{…}` | 77-81 | REAL | — | ControlServer/TokenSource/constant_time_eq/generate_token — app + tests consume; API surface of a lib crate, fine. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (No tests in lib.rs; the crate's 47 lib tests + 25 conformance + 6 e2e all executed PASS in baseline — see §2) | Yes, collectively | The doc claims this file makes are each pinned by at least one executed test (loopback, lockout, 4401/4402, vectors, constant-time). The one unpinned claim (wire fuzzing) is exactly the one that is unmet (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bind refuses any non-127.0.0.1/::1 address with a typed error" | lib.rs:11-13 | REAL | server.rs:316-323 + executed conformance test :727. |
| "first frame must be hello … mismatch is a typed error + close" | lib.rs:14-17 | REAL | Executed tests :440 (4402), :460. |
| "per-start 256-bit token … constant-time comparison … 5 failures ⇒ 60 s lockout" | lib.rs:20-23 | REAL | token.rs:45-53 (urandom 32B), AuthLimits::documented() server.rs:115-119; executed :393, :373. |
| "deny-by-default sensitive ops" | lib.rs:24-27 | REAL | Executed :743, :783. |
| "malformed input is a typed error … and the connection survives" | lib.rs:28-31 | REAL | Executed :562 (garbage masked payload), :886 (dirty payloads), :978 (binary frames). |
| "SHA-1 and base64 are implemented in-crate … pinned by external test vectors" | lib.rs:38-40 | REAL | sha1.rs:163 (RFC 3174 §7.3 + NIST million-a), base64.rs:172 (RFC 4648 §10); executed in 47-passed suite. |
| "~100 lines each" | lib.rs:39 | FALSE (inaccurate) | 210 / 220 lines (F-002). |
| "Deep security review deferred to Phase 8 … rate limits and Origin allowlist are recorded gaps … serves serially" | lib.rs:44-51 | REAL | server.rs Limits docs; `serve_one` API (server.rs:429-441); no per-connection spawn. |
| "No … `unsafe`/`unwrap`/`expect`/`panic`" | lib.rs:57-58 | REAL | rg: zero matches in src; clippy -D warnings clean (baseline #5). |
| doc 01 §2.3 (e) "wire fuzzing … version negotiation … round-trip properties" (as crate charter) | lib.rs:32-34 citing doc 01:428-429 | SHALLOW (fuzzing leg) / REAL (other two legs) | F-001; version+roundtrip executed. |
| "requests are `commands::WireRequest` … re-used, never re-implemented (ARCH-5)" | lib.rs:17-19 | REAL | Cargo.toml dep; e2e round-trips the command layer (6 passed). |

## 7. Dead / duplicated / unreachable code
- `pub mod sha1`/`base64`: externally unused pub surface (F-003).
- The crate as a whole: unreachable from `lumen` CLI and `lumen-tauri` (only apps/control-server links it) — by architecture (doc 01 §2.3 registry: control-protocol serves control-server clients).

## 8. Dependencies & flags
Deps: `commands` (path), `serde` (derive), `serde_json` — exactly the declared same-layer whitelist pair + the Lead-admitted wire family (Cargo.toml comment verified). `[lints] workspace = true` (the 39/40 crates pattern, baseline INDEX). No feature gates, no unsafe. Edition 2024.

## 9. Open questions for the next agent
- F-001 follow-up: whoever owns fuzz/ — was control-protocol ever in a fuzz plan (doc 04 §8.5 registry card 5.Q1)? If the registry gate's charter is "every parser", the gate itself has a blind spot for crates added after the registry.
- server.rs/frame.rs/http.rs auditors: the lib.rs claims verified here from outside should be re-verified inside those files (esp. the re-verify-after-bind loopback check at server.rs:314+ and the 1 MiB/4 GiB limit boundaries in frame.rs).
