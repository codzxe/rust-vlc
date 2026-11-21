# AUDIT: crates/control-protocol/src/server.rs (part1, lines 1–493 of 988)
> Chunk scope: part1 = lines 1–493 (module doc, `Limits`/`AuthLimits`/
> `Timeouts`/`FailureBook`, `TokenSource`, `ServerConfig`, `Dispatch`,
> `SessionSummary`/`SessionEnd`, `ControlServer` incl. `bind`/
> `serve_one`, and the `Session` struct). Part2 (494–988: `impl Session`
> — the session loop, hello/auth, request handling — and `mod tests`)
> is W02-A09's assignment — not audited here. File name carries
> `.part1` to avoid colliding with A09's part2 audit.

| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-21 |
| Lines / functions | 493 (of 988) / 19 units |
| File verdict | REAL |
| Reachable from shipped entry point? | YES via the dedicated `lumen-control-server` binary (apps/control-server; built in baseline run #1; full stack executed in baseline run #3: `e2e_full_session_through_the_app_stack ... ok`, test-workspace.log.txt:1518-1528). NOT reachable from the `lumen` CLI binary itself (rg: no ControlServer use in apps/cli, apps/desktop, apps/desktop-tauri — the desktop "engine-embedding join" is a future card per apps/control-server/Cargo.toml header). Opt-in surface: doc 05 §5.4 "`control-server.enabled` defaults to **false**". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n` verbatim-quote verification (30-33, 142-151, 386-389, 412-422, 610-729, 904-1000); `rg` consumers of `ControlServer`/`serve_one`/`TokenSource`/`entropy_source` (apps/control-server only); read `crates/control-protocol/src/token.rs` (EntropySource enum, generate_token); read doc 05 §5.4 (bind/auth/validation/Origin claims); baseline log greps: `server::tests` 4/4 ok + `tests/conformance.rs` 25/25 ok (test-workspace.log.txt:1451-1462, 1464-1490) + control-server e2e 6/6 ok (:1518-1528); `grep` packaging scripts for control-server (no hits — not in packaging payload). |

## 1. One-paragraph verdict
This is the honest half of a genuinely implemented control server: the loopback-only bind is enforced before any OS call and re-verified after resolution (both unit- and conformance-tested, including DNS-rebind hostile cases), the 1 MiB/5-failure/60 s numbers match doc 05 §5.4 exactly and are pinned by tests, and the whole construct executed in the baseline runs (server unit tests, 25 conformance tests, and the app-stack e2e all passed). The defects: (1) an operator-pinned token (`TokenSource::Fixed`) is reported as `EntropySource::OsRandom` — the security-observability API whose stated purpose is to warn about weak tokens never warns about the weakest possible ones; (2) doc 05 §5.4's per-connection concurrency cap, rate limit, and Origin allowlist are unimplemented — the module header honestly records this as a gap "for 8.A2", but the consequence is that any local process or webpage can connect and burn the failure book into a persistent lockout of the real user (one bad token per 60 s keeps it locked forever); (3) two doc-comment defects (a dangling "see [`Limits`]'s honest-scope notes" reference and "documented defaults" for timeouts that no document documents).

## 2. What actually works (verified)
— Loopback gate: `bind()` checks the host string before resolution (server.rs:318-325), refuses every resolved non-loopback address (344-347), and re-verifies the bound address (380-385). Verified by code read + baseline `server::tests::non_loopback_binds_are_refused_at_construction` and `loopback_binds_succeed_on_both_families` (test-workspace.log.txt:1452-1453, `ok`) and conformance `non_loopback_bind_is_refused_at_construction_conformance` + `dns_rebound_host_names_never_upgrade` (:1472, 1467, `ok`). The `"::1"` spelling works despite producing the unbracketed `"::1:port"` string — std's host:port fallback parses it; proven by the passing `loopback_binds_succeed_on_both_families` (covers `"::1"`).
— Numbers match doc 05 §5.4: 1 MiB frame/message caps, 5 failures / 60 s lockout — `documented_limits_are_the_doc_05_numbers` ok (test-workspace.log.txt:1451), `failure_book_engages_the_documented_lockout` ok (:1455), conformance `five_failures_lock_out_even_the_right_token` + `wrong_token_closes_4401_and_counts` ok (:1469-1470).
— Deadline boundedness: `serve_one` sets read/write socket deadlines before any session work (448-459); conformance `idle_session_is_closed_by_the_read_deadline_1001` ok (test-workspace.log.txt:1489).
— Full-stack execution: `e2e_full_session_through_the_app_stack ... ok` (test-workspace.log.txt:1528) — the `lumen-control-server` binary + `ControlServer::bind` + `serve_one` + session loop ran end-to-end in baseline run #3.
— Module-header claims about part2 behavior verified against baseline conformance: `frames_before_authentication_are_refused` ok (:1468), `ping_is_answered_by_a_matching_pong` ok (:1483).

## 3. Findings

### F-001 [S3]
Where: crates/control-protocol/src/server.rs:386-389 (contract at :418-422)
Code:
```
        let (token, entropy) = match &config.token {
            TokenSource::Fixed(token) => (Token(token.clone()), EntropySource::OsRandom),
            TokenSource::Generated => generate_token(),
        };
```
What is wrong: For an operator-pinned token, the stored `EntropySource` is `OsRandom` — factually false. The enum (token.rs:57-61) has only `OsRandom` and `WeakFallback`; there is no variant for "operator config". The `entropy_source()` doc (:418-419) says it answers "Which entropy source produced the token (log WITH the token, never instead of the warning the weak path deserves)" — for `Fixed("hunter2")` the answer is a lie, and the weak-token warning this API exists to deliver never fires for the weakest possible token class.
Why it matters: `TokenSource::Fixed` is a public, product-config-reachable path (apps/control-server/src/config.rs:203 maps an operator `control-server.toml` token to it). A host that logs `entropy_source()` unconditionally (the API invites exactly that) tells the operator their hand-typed token has OS CSPRNG entropy. Mitigating: the one shipped caller (apps/control-server/src/main.rs:226-243) only reads `entropy_source()` when the token was `Generated`, so the lie is currently unobservable in production — this is why S3, not S2.
What correct looks like: `EntropySource::OperatorPinned` variant returned for `Fixed`, and the shipped host warned on it; or `entropy_source()` documented as meaningful only for generated tokens.
Confidence: HIGH — Verification: read of :386-389, :412-422, token.rs:43-61, config.rs:203, main.rs:226-243.

### F-002 [S3]
Where: crates/control-protocol/src/server.rs:30-33 (gap acknowledged; enforcement would live in serve_one/Session, part2, and http.rs)
Code:
```
//! The server is deliberately single-connection, synchronous: a
//! control surface for a local single-user product (bounded by design,
//! D-1). Concurrency/rate caps and the Origin allowlist are recorded
//! gaps for 8.A2 — see [`Limits`]'s honest-scope notes.
```
What is wrong: Doc 05 §5.4 specifies "per-connection concurrency cap (8 in-flight requests) and rate limit (100 req/s burst 200)" and an "Origin check … rejects any origin not on the configured allowlist" — none of this exists in the shipped server (rg: no origin parsing in this crate's session path; no rate state anywhere). The header admits it, but the availability consequence is unstated: because the failure book is server-global and never resets (`note_failure`, :179-187 — re-locks on a single failure after expiry, by design per the unit test at :936-960), any local process — or any webpage the user visits, since browsers will happily open `ws://127.0.0.1:<port>` and the token handshake is the only gate — can send 5 bad tokens, lock the surface for 60 s, and repeat forever. The legitimate user is then locked out of their own control surface.
Why it matters: This is an acknowledged gap, not hidden fraud — but "recorded for 8.A2" is a worklog note, not a shipped protection, and the doc's §5.4 validation paragraph reads as implemented unless you notice the `[PROPOSAL]` markers. Blast radius is capped by: opt-in surface (disabled by default per §5.4), loopback-only, single connection, 60 s read deadline (a rogue idler cannot wedge the accept loop: conformance `idle_session_is_closed_by_the_read_deadline_1001` proves it). Hence S3, not S2.
What correct looks like: The 8.A2 card lands: Origin allowlist at upgrade time + a connection-rate/lockout budget that separates "unknown origin hammered the book" from "the real user mistyped the token once", or the doc's §5.4 numbers are explicitly marked unimplemented-in-v0.1.3 rather than [PROPOSAL].
Confidence: HIGH for the absence (rg over crate + http.rs surface) — the webpage-to-localhost-WS vector is standard browser behavior, not run in this sandbox (UNVERIFIED by execution).
Confidence: HIGH — Verification: read + rg; baseline cites above prove what IS enforced.

### F-003 [S4]
Where: crates/control-protocol/src/server.rs:32-33 vs :69-79
Code:
```
//! gaps for 8.A2 — see [`Limits`]'s honest-scope notes.
```
and Limits' actual doc (:69-72):
```
/// Byte caps (doc 05 §5.4: "Every frame: max 1 MiB"). A frame or
/// reassembled message above its cap is a typed
/// [`ProtocolError::MessageTooLarge`] close — the claim is rejected
```
What is wrong: The module header directs the reader to "`Limits`'s honest-scope notes" for the concurrency/Origin gap — `Limits`' doc comment contains only byte-cap prose; no scope notes about concurrency, rate caps, or Origin exist there. The gap statement lives solely in the module header itself.
Why it matters: Dangling cross-reference; a maintainer chasing the pointer finds nothing and may conclude the notes were removed (or the gap closed).
What correct looks like: Header references itself or a tracking worklog entry; or move the honest-scope note into `Limits`/`ServerConfig` docs where it claims to be.
Confidence: HIGH — Verification: read of both sites.

### F-004 [S4]
Where: crates/control-protocol/src/server.rs:143-150
Code:
```
    /// The documented defaults: 60 s read (a control client may idle
    /// between requests, but not forever), 10 s write.
    pub const fn documented() -> Timeouts {
        Timeouts {
            read: Some(Duration::from_secs(60)),
            write: Some(Duration::from_secs(10)),
        }
    }
```
What is wrong: "The documented defaults" — no document specifies 60 s read / 10 s write. `grep` over docs/ and guides/ finds no socket-deadline numbers for the control server; doc 05 §5.4 says nothing about read/write deadlines at all (its only deadline mention is the 2 s decoder-join in §2.3). The sibling types (`Limits`, `AuthLimits`) genuinely are doc-05-pinned; `Timeouts` borrows the same "documented" phrasing without a doc behind it.
Why it matters: The word "documented" implies a spec anchor that does not exist; drift between this comment and any future doc numbers would be invisible. (The behavior itself is fine and tested: conformance `idle_session_is_closed_by_the_read_deadline_1001` ok.)
What correct looks like: "Defaults (not doc-pinned): 60 s read, 10 s write" or an actual doc 05 §5.4 sentence to cite.
Confidence: HIGH — Verification: read + `grep -rn "deadline|60 s read" docs/ guides/` (no control-server socket-deadline doc found).

## 4. Per-function table
| Function / unit | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| Module doc | 1-33 | REAL | S4 | Claims verified: pre-OS loopback refusal ✓, post-bind re-verify ✓, lockout numbers ✓, pre-auth frame refusal ✓ (conformance). F-002 (honest gap), F-003 (dangling ref). |
| `Limits` struct + consts + `documented()` | 74-94 | REAL | — | 1 MiB/1 MiB = doc 05 §5.4 exactly; pinned by `documented_limits_are_the_doc_05_numbers` (baseline ok). |
| `Default for Limits` | 96-100 | REAL | — | Delegates to `documented()`. |
| `AuthLimits` + `documented()` + `Default` | 107-128 | REAL | — | 5 / 60 s = §5.4; pinned by tests. |
| `Timeouts` + `documented()` + `Default` | 135-157 | REAL | S4 | F-004: "documented" with no doc source; values sane, deadline path conformance-tested. |
| `FailureBook` struct | 162-166 | REAL | — | Server-global (correct: lockout must survive reconnects). |
| `FailureBook::is_locked` | 170-175 | REAL | — | `Instant::now() < deadline`; no panics. |
| `FailureBook::note_failure` | 179-187 | REAL | — | saturating_add (no overflow); re-lock-after-expiry on a single failure is INTENDED and unit-tested (:952-960); count never resets on success (design; amplifies F-002's DoS). |
| `TokenSource` enum | 192-200 | REAL | — | Fixed/Generated; Fixed reachable from operator config (config.rs:203). |
| `ServerConfig` + `strict()` | 204-240 | REAL | — | Strict default: loopback, ephemeral port, deny-all-sensitive policy, generated token. All fields consumed. |
| `Dispatch` trait + blanket closure impl | 245-259 | REAL | — | Host seam; implemented by apps/control-server's `CatalogDispatch` (e2e-proven). |
| `SessionSummary` | 263-282 | REAL | — | All fields populated by `serve_one`; consumed by app logging (logging::tests in baseline). |
| `SessionEnd` | 286-293 | REAL | — | PeerClosed/ServerClosed/Eof; used in part2. |
| `ControlServer` struct | 297-303 | REAL | — | listener + config + token + entropy + failures. |
| `ControlServer::bind` | 316-397 | REAL | — | No panics; typed errors throughout; hostile inputs covered by unit test (0.0.0.0/192.168.1.5/example.com/::/[::]/empty); `"::1"` unbracketed spelling works via std fallback (execution-proven). Resolved-address loop returns first bind success; non-loopback resolution aborts immediately (deny-by-default ✓). |
| `ControlServer::local_addr` | 403-410 | REAL | — | Thin typed wrapper. |
| `ControlServer::token` | 414-416 | REAL | — | Sanctioned reader; shipped app prints to terminal only when generated. |
| `ControlServer::entropy_source` | 420-422 | REAL | S3 | F-001: lies for `Fixed` tokens (returns OsRandom). |
| `ControlServer::is_locked_out` | 425-427 | REAL | — | Delegates to FailureBook. |
| `ControlServer::serve_one` | 437-473 | REAL | — | Accept → set deadlines → Session::new → run → summary. No lockout pre-check before accept (lockout enforced at hello, part2 :716-719 — wasted accept+upgrade for locked-out peers, bounded by deadlines); no panics; summary correctly reports cumulative `auth_failures` from the server-global book. |
| `Session` struct | 477-492 | REAL | — | State fields only; `subscriptions` bounded by message cap (part2 :654-655 comment); all fields consumed in part2. |

## 5. Test honesty
In-file `mod tests` (server.rs:~890-1000, part2's line range) + `tests/conformance.rs`, evaluated for part1 constructs (all executed in baseline run #3):
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `non_loopback_binds_are_refused_at_construction` (server.rs:904-916) | YES | Six hostile hosts, asserts the typed error AND the echoed address. Mutation: accept `0.0.0.0` → unreachable! panics the test. |
| `loopback_binds_succeed_on_both_families` (server.rs:918-934) | YES | Binds all three spellings incl. `"::1"`; asserts bound addr is loopback. Mutation: break the IPv6 arm → test fails on bind error. |
| `failure_book_engages_the_documented_lockout` (server.rs:936-960) | YES | Counting, engagement flag, expiry, and the intended re-lock-on-one-failure are each asserted; a `sleep`-based expiry check (60 ms) — mildly timing-sensitive but tolerances are 10x. |
| `documented_limits_are_the_doc_05_numbers` (server.rs:962-966) | YES | Pins 1 MiB/1 MiB/5/60 s — catches silent cap changes. |
| conformance `dns_rebound_host_names_never_upgrade`, `non_loopback_bind_..._conformance`, `five_failures_lock_out_even_the_right_token`, `wrong_token_closes_4401_and_counts`, `idle_session_is_closed_by_the_read_deadline_1001`, `frames_before_authentication_are_refused` | YES | Real sockets, real token attempts, real clock (deadline test) — these exercise bind+serve_one+FailureBook+Timeouts end-to-end. All `ok` in baseline (test-workspace.log.txt:1464-1490). |
No tautologies, no mock-of-subject (conformance drives a real TCP stream); names match bodies.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Any configuration that requests a non-loopback address is a startup error, not a warning" | doc 05 §5.4 | REAL | bind():318-325, 344-347, 380-385 + unit/conformance tests (baseline ok). |
| "The server generates a 256-bit random token per start" | doc 05 §5.4 | REAL | `TokenSource::Generated` → `generate_token()` (token.rs:45; `generated_tokens_are_256_bit_and_distinct` ok, baseline). |
| "Comparison is constant-time" | doc 05 §5.4 | REAL | `constant_time_eq` called at :720 (part2); token.rs tests ok. |
| "5 failures impose a 60 s lockout and a log entry" | doc 05 §5.4 | REAL | AuthLimits::documented + failure-book tests + conformance `five_failures_lock_out_even_the_right_token` ok. Caveat: lockout is permanent-regime (one failure re-locks after expiry; never resets on success) — stricter than the doc sentence, not weaker. |
| "Every frame: max 1 MiB" | doc 05 §5.4 | REAL | Limits::documented (1 MiB/1 MiB) + `documented_limits_are_the_doc_05_numbers` ok; enforcement in frame.rs (A04/A06 scope). |
| "per-connection concurrency cap (8 in-flight) and rate limit (100 req/s burst 200)" | doc 05 §5.4 [PROPOSAL] | PAPER-ONLY | Not implemented; module header :32 records the gap "for 8.A2" (F-002). |
| "Origin check … rejects any origin not on the configured allowlist" | doc 05 §5.4 [PROPOSAL] | PAPER-ONLY | Not implemented (F-002). |
| "welcome { v, engineInfo, capabilities }" / version negotiation | doc 05 §5.4 | REAL-UNPROVEN in part1 | Wire shape built in part2 (A09); conformance `version_mismatch_is_a_typed_error_and_close_4402`, `handshake_success_full_session_round_trip` ok in baseline. |
| "request_id monotonicity enforced per connection (anti-replay)" | doc 05 §5.4 | REAL | `Session.last_request_id` (:487) + conformance `non_monotonic_request_id_closes_the_connection` ok. |
| Token distribution (file 0600 / fd / --print-token) | doc 05 §5.4 [PROPOSAL] | SHALLOW | Shipped app prints to stderr unconditionally when generated (main.rs:230-234); no file/fd paths — app-level drift, apps/control-server's auditor's scope, noted for the record. |

## 7. Dead / duplicated / unreachable code
- No dead code found in part1: every type/method has a consumer (app, part2, or tests).
- `is_locked_out()` (:425) — public accessor; used by tests and available to hosts; the shipped app does not call it (minor: no production reader; keep as observability).
- Unreachable from the `lumen` CLI / desktop apps (see header) — reachable only via `lumen-control-server`, which is itself not in the packaging payload (`grep` scripts/packaging: no hits) — the "shipped" status of the binary itself is a packaging-wave question.

## 8. Dependencies & flags
- std-only networking (TcpListener/TcpStream/ToSocketAddrs) — no tokio/async: synchronous by design, documented (D-1 bounded).
- Uses commands crate wire types (WireRequest/WireResponse, EngineInfo) — ARCH-5 vocabulary reuse ✓.
- No `unsafe`, no feature gates in this chunk; no panics outside test `unreachable!`s.
- `Instant`/`Duration` arithmetic: `Instant::now() + limits.lockout` — addition of Duration to Instant; a 60 s lockout cannot panic (Instant + Duration is saturating in Rust ≥1.60? — actually `Instant::add` panics on overflow only for astronomically large durations; 60 s is safe. UNVERIFIED edge is irrelevant here).

## 9. Open questions for the next agent
- A09 (part2): confirm the hello path refuses a locked-out peer BEFORE the token compare (read of :714-729 says yes — cite precisely), and check `Session::run`'s handling of `read` timeout hitting mid-frame (partial frame + deadline = typed close 1001, not a panic).
- A09: `handle_text`'s Subscribe arm pushes into `self.subscriptions` with a dedup `contains` check — O(n²) for n kinds, but n is bounded by the 1 MiB message cap; confirm the bound reasoning holds for repeated Subscribe frames (unbounded total session lifetime growth? each new kind must appear in some frame, so total kinds ≤ distinct strings ever sent — memory bounded only by peer patience across the whole session; likely worth an S4 there).
- 8.A2 owner: F-002's Origin/concurrency/rate gap — decide or descope in doc 05.
- App auditor (apps/control-server): token distribution drift (§6 table row) and the `--print-token`/file/fd modes doc 05 promises.
