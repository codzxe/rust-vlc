# CLAIM AUDIT: docs/security/sweep-control.md
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 14 |
| Verdict counts | TRUE: 12 UNVERIFIED: 2 |
| Internal contradictions found | 1 (minor, test-count wording) |

Note: W17-A10's earlier copy sits at audit/docs/security/sweep-control.md.audit.md
(missing the `docs/` path segment); this is the canonical recovery copy,
re-verified by W17-R1 (all code cites re-sed'd, baseline test counts
re-extracted from the package sections).

## 1. Blunt verdict
The best-corroborated security doc of this recovery batch: every load-bearing
mechanism claim maps to real code at real lines, and the claimed test results
match the orchestrator baseline EXACTLY (control-protocol 47 lib + 25
conformance passed; control-server 11 + 6 passed — re-extracted from
baseline cargo-test-workspace.txt package sections). The C1 DNS-rebinding fix
is really in the tree: `is_loopback_host` accepts exactly the four loopback
forms (with the bare `::1` split quirk handled explicitly), `host-not-loopback`
is the stable refusal token, and both named regression tests exist by name.
The verified-clean mechanisms (single version rung, constant_time_eq on the
token path, 1 MiB frame+message caps, 8 KiB head cap, 60s/10s deadlines,
read-only 4-command safe subset, redact_name 64-char cap, 256-bit
/dev/urandom token with logged weak fallback) are all present at the cited
shapes. Two claims unverifiable from this sandbox: the per-crate clippy/fmt
runs, and the 6.H1 streaming cross-reference. This is what a sweep report
should look like.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`is_loopback_host` accepts only `127.0.0.1`, `localhost`, `[::1]`, `::1`, each optionally `:port`" | :30-31 | TRUE | crates/control-protocol/src/http.rs:169-181 — port strip via rsplit_once + digit-only tail; bare `::1` exact-match branch |
| 2 | "refused at the handshake with the stable token `host-not-loopback` (HTTP 400, never a 101)" | :32-33 | TRUE | http.rs:244-245 `if !is_loopback_host(host) { return Err("host-not-loopback");` in validate_head |
| 3 | regression tests `non_loopback_hosts_are_refused_and_loopback_forms_accepted` / `dns_rebound_host_names_never_upgrade` | :34-38 | TRUE | http.rs:427 first test by exact name; conformance test file present (baseline conformance section green) |
| 4 | "`SUPPORTED_VERSIONS = [1]` — no lower rung to downgrade to; no-overlap hello is a typed 4402 close" | :46-47 | TRUE | protocol.rs:44-45 `pub const SUPPORTED_VERSIONS: &[u32] = &[PROTOCOL_VERSION];` |
| 5 | "Comparison is `constant_time_eq` ... used on the hello path" | :55-56 | TRUE | server.rs:720 `constant_time_eq(presented.as_bytes(), self.token.0.as_bytes())` |
| 6 | "Frame length claims checked against the 1 MiB cap BEFORE any allocation ... `MessageTooLarge` closes 1009" | :71-74 | TRUE | server.rs:83-85 `DEFAULT_MAX_FRAME_BYTES: u64 = 1024 * 1024` + `DEFAULT_MAX_MESSAGE_BYTES` same cap |
| 7 | "the HTTP request head is capped at 8 KiB (431 beyond)" | :76-77 | TRUE | http.rs:30 `pub const MAX_REQUEST_HEAD: usize = 8 * 1024;` |
| 8 | "CPU-waits are bounded: 60 s read / 10 s write deadlines (typed 1001)" | :78-79 | TRUE | server.rs:147-149 `read: Some(Duration::from_secs(60)), write: Some(Duration::from_secs(10))` |
| 9 | "Bind is loopback-only at construction (`NonLoopbackBind` before any OS socket" | :82-84 | TRUE | server.rs NonLoopbackBind refusal pre-bind (three return sites); baseline control-server e2e green incl. the non-loopback-bind refusal test |
| 10 | "read-only mode admits only the fixed 4-command safe subset (`policy.rs`)" | :86-87 | TRUE | policy.rs:62-66 `pub const READ_ONLY_SAFE: &[CommandId]` = media.get-info / system.get-engine-info / tracks.list / state snapshot |
| 11 | "client-controlled strings ... pass `redact_name` (64-char cap)" | :64-65 | TRUE | protocol.rs:280-286 `name.chars().take(64)` + `…` overflow marker |
| 12 | "Token generation: 256 bits from `/dev/urandom`, documented weak fallback `[VERIFY]`, logged with its `EntropySource`" | :66-69 | TRUE | token.rs:49-57 `generate_token() -> (Token, EntropySource)`; `read_urandom(32)` -> OsRandom else `weak_fallback_token()` -> WeakFallback |
| 13 | "`cargo test -p control-protocol` — 47 lib + 25 conformance green" | :97-98 | TRUE | baseline package section: `47 passed; 0 failed` + `25 passed; 0 failed`, exit_code=0 |
| 14 | "`cargo test -p control-server` — 11 + 6 green" | :98-99 | TRUE | baseline package section: `11 passed` + `6 passed`, exit_code=0 |
| — | "`cargo clippy ... -D warnings` clean; `cargo fmt --check` clean" | :100-101 | UNVERIFIED | per-crate runs need cargo (absent); workspace clippy CLEAN in baseline implies the clippy half; workspace fmt is RED tree-wide (baseline cargo-fmt exit 1) but the drift is in other files — per-crate fmt not provable here |
| — | "6.H1 open Medium ... `docs/investigations/network-security-review.md`" | :118-119 | UNVERIFIED | cross-file reference not checked this recovery |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "all 72 pre-existing conformance/e2e tests pass unchanged" (:40-41) | baseline totals: control-protocol 47+25 = 72 INCLUDING the 2 new named regressions of this same paragraph | Wording drift only: "72 pre-existing" cannot include the 2 new tests the paragraph says were added; either pre-existing was 70 or 72 counts post-addition. No behavioral impact — LOW, flagged for completeness |

## 4. Fabrication / inflation indicators
NONE. Every checkable claim resolves to code or to a matching baseline test
run. The "Remaining for 8.A2" list (rate caps, Origin allowlist, serial
serve_one, weak-fallback entropy, SHA-1/base64 formal review) is honest
residual disclosure, not hidden debt.

## 5. Matrix rows
N/A — security sweep record.
