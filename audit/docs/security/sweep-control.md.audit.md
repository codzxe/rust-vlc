# CLAIM AUDIT: docs/security/sweep-control.md
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Claims extracted | 14 |
| Verdict counts | TRUE: 12 UNVERIFIED: 2 |
| Internal contradictions found | 1 (minor, test-count wording) |

## 1. Blunt verdict
The best-corroborated security doc I have audited in this wave: every
load-bearing mechanism claim maps to real code at real lines, and the claimed
test results match the orchestrator baseline EXACTLY (control-protocol 47 lib
+ 25 conformance passed; control-server 11 + 6 passed — baseline
cargo-test-workspace.txt lines 1608/1639/1670/1690). The C1 DNS-rebinding fix
is really in the tree (`is_loopback_host` accepting exactly the 4 loopback
forms with optional port; `host-not-loopback` stable refusal; both named
regression tests present). The verified-clean section's mechanisms
(SUPPORTED_VERSIONS=[1] single rung, 4402 typed close, constant_time_eq on the
token path, 1 MiB frame+message caps, 1009 TooBig, 8 KiB head cap (431), 60s
read/10s write deadlines (1001), NonLoopbackBind before socket, read-only
4-command safe subset, redact_name 64-char cap, strictly-increasing requestId,
256-bit /dev/urandom token with EntropySource-logged weak fallback) are ALL
present in the code at the cited shapes. Two claims unverifiable from the
sandbox: the per-crate clippy/fmt run results, and the 6.H1 streaming-scheme
downgrade reference. This doc is what a sweep report should look like.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`is_loopback_host` accepts only `127.0.0.1`, `localhost`, `[::1]`, `::1`, each optionally `:port`" | :30-31 | TRUE | crates/control-protocol/src/http.rs:169-181 — exact 4 forms + digit-only port strip; bare `::1` handled separately |
| 2 | "refused at the handshake with the stable token `host-not-loopback` (HTTP 400, never a 101)" | :32-33 | TRUE | http.rs:244-245 `return Err("host-not-loopback")` in validate_head |
| 3 | Regression tests named `non_loopback_hosts_are_refused_and_loopback_forms_accepted` / `dns_rebound_host_names_never_upgrade` | :34-38 | TRUE | http.rs:427 and tests/conformance.rs:303 — both present by exact name |
| 4 | "`SUPPORTED_VERSIONS = [1]` — no lower rung; typed 4402 close" | :46-47 | TRUE | protocol.rs:44 `pub const SUPPORTED_VERSIONS: &[u32] = &[PROTOCOL_VERSION];` + conformance.rs:440 test asserting close 4402 |
| 5 | "Comparison is `constant_time_eq` ... used on the hello path" | :55-56 | TRUE | server.rs:66 import + :720 `constant_time_eq(presented.as_bytes(), self.token.0.as_bytes())` |
| 6 | "Frame length claims ... 1 MiB cap BEFORE any allocation ... `MessageTooLarge` closes 1009 — conformance proves a 4 GiB claim never allocates" | :71-74 | TRUE | server.rs:83-85 caps (1024*1024); frame.rs:657 test `oversized_length_claims_reject_before_allocating` ("64-bit length claiming 4 GiB with a tiny actual body"); error.rs:48 TooBig->1009 |
| 7 | "HTTP request head is capped at 8 KiB (431 beyond)" | :76-77 | TRUE | http.rs:30 `pub const MAX_REQUEST_HEAD: usize = 8 * 1024;` + server.rs:529 431 |
| 8 | "60 s read / 10 s write deadlines (typed 1001)" | :78-79 | TRUE | server.rs:147-148 `read: Some(Duration::from_secs(60)), write: Some(Duration::from_secs(10))` |
| 9 | "Bind is loopback-only at construction (`NonLoopbackBind` before any OS socket" | :82-84 | TRUE | server.rs:311-382 three NonLoopbackBind returns pre-bind; e2e test `a_non_loopback_bind_in_the_operator_config_is_refused_at_bind` in baseline |
| 10 | "read-only mode admits only the fixed 4-command safe subset" | :86-87 | TRUE | policy.rs:63-64 lists exactly media.get-info / system.get-engine-info / tracks.list / state snapshot |
| 11 | "client-controlled strings entering error messages pass `redact_name` (64-char cap)" | :64-65 | TRUE | protocol.rs:280-286 `take(64)` + `…` marker; used at server.rs:626/672/831 |
| 12 | "Token generation: 256 bits from `/dev/urandom`, documented weak fallback `[VERIFY]`, logged with its EntropySource" | :66-69 | TRUE | token.rs:35-60 generate_token()/EntropySource::{OsRandom, WeakFallback} + weak_fallback_token |
| 13 | "`cargo test -p control-protocol` — 47 lib + 25 conformance green" | :97-98 | TRUE | baseline cargo-test-workspace.txt:1608 `47 passed` + :1639 `25 passed` |
| 14 | "`cargo test -p control-server` — 11 + 6 green" | :98-99 | TRUE | baseline :1670 `11 passed` + :1690 `6 passed` (e2e incl. non-loopback-bind + typoed-grant tests) |
| — | "`cargo clippy ... -D warnings` clean; `cargo fmt --check` clean" | :100-101 | UNVERIFIED | per-crate runs need cargo; workspace clippy CLEAN in baseline implies the clippy claim (crates are workspace members); fmt is RED tree-wide (baseline) but on OTHER files — per-crate fmt UNVERIFIED |
| — | "6.H1 open Medium ... docs/investigations/network-security-review.md" | :118-119 | UNVERIFIED | cross-file reference not checked this audit |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "all 72 pre-existing conformance/e2e tests pass unchanged" (:40-41) | baseline: control-protocol totals exactly 47+25=72 tests INCLUDING the 2 new named regressions | Trivial wording drift: "72 pre-existing" cannot include the 2 new tests the same paragraph says were added; either pre-existing was 70, or 72 counts post-addition. No behavioral impact — LOW severity, flagged for completeness. |

## 4. Fabrication / inflation indicators
NONE. Every checkable claim resolves to code or to a matching test run. The
doc's own "what remains for 8.A2" list (rate caps, Origin allowlist, serial
serve_one, weak-fallback entropy, SHA-1/base64 formal review) is honest
residual disclosure, not hidden debt.

## 5. Matrix rows
N/A — security sweep record.
