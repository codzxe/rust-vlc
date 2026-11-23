# AUDIT: crates/streaming/tests/auth_scheme_gate.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 117 / 2 (2 tests) |
| File verdict | REAL (honest security regression over real sockets; the lane it guards is itself unwired) |
| Reachable from shipped entry point? | N/A (test binary). The code under test (`AuthFetch`) has NO production constructor: rg `AuthFetch` outside streaming/src/auth.rs and tests → doc-comment references only (remote_playlist.rs:37,104 suggests composing it; nothing does). No app passes `Credentials` anywhere (rg apps/ → 0). The gate is real; the street it guards has no traffic in the shipped binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
Two exact, well-reasoned security regressions over REAL loopback sockets: the AUD-07 https→http downgrade inside the auth wrapper's manual follow loop is refused with the transport's own `CrossSchemeRedirect { hops: 1 }` (asserted non-retryable), the https origin is hit exactly once WITHOUT credentials (challenge-first pinned), and the plaintext target is asserted NEVER contacted — the credential-leak shape is pinned dead from three angles. The negative space is tested too (https→https cross-port and http→https upgrade both follow). Both pass (I ran them). The catch is reachability: no shipped code constructs `AuthFetch`, so this guards an unwired lane — the test proves the vault door locks, but no shipped product leads anyone to the vault.

## 2. What actually works (verified)
— `cargo test -p streaming --test auth_scheme_gate` → `2 passed; 0 failed; 0 ignored; finished in 0.01s`. Assertions are exact (error variant + hop count + dotted code + retryability + per-server request logs + header absence). Deleting the gate in auth.rs fails test 1 on the `matches!` or the `target.requests().is_empty()` assertion; loosening the follow policy fails test 2's request counts.

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/auth_scheme_gate.rs:26 (and the lane it guards)
Code:
```
use streaming::auth::{AuthFetch, Credentials};
```
What is wrong: the tested lane is unreachable from any shipped entry point — `AuthFetch` is never constructed outside auth.rs's own tests and these integration tests; no app or CLI path supplies `Credentials` (the CLI's `open_http_source` builds `SchemeRegistry::new()` with no auth wrapper; `remote_playlist.rs` — the documented composition site — is itself unwired, zero consumers outside streaming/). So a URL like `http://user:pass@host/x.flac` on the shipped CLI is refused at URL parse (fetch.rs userinfo rejection) rather than authenticated: the entire auth card (challenge parsing, Basic/Digest, the scheme gate these tests protect) is dead code in the release lane. What correct looks like: wire `AuthFetch` into the CLI http lane (its docs already specify the composition order: RetryFetch around AuthFetch).
Confidence: HIGH — Verification: rg construction sites + apps scan (0 hits).
F-002 [S4]
Where: crates/streaming/tests/auth_scheme_gate.rs:19-24
Code:
```
//! The same-origin/cross-host trust boundary (credentials re-attach
//! only on allowed-host hops) is pinned in `src/auth.rs`'s in-file
//! tests (`downgrade_hop_never_receives_credentials`,
//! `cross_host_hop_never_receives_credentials`) — the one-shot
```
What is wrong: the cross-host credential boundary — the OTHER half of the AUD-07 residual — is pinned only in in-file unit tests with scripted (non-socket) doubles, not in this real-socket suite; the stated reason (one-shot MockConfig cannot express challenge-then-redirect) is plausible but leaves the strongest trust-boundary claim on weaker evidence than the downgrade claim next to it. What correct looks like: a scripted multi-route mock (the LiveRouter/ScriptedRoute pattern already exists in this crate's tests) covering challenge→cross-host-redirect→assert-no-credentials over a socket.
Confidence: HIGH — Verification: read + rg the named in-file tests (present in src/auth.rs per the doc; their content is that file's audit lane).

## 4. Per-function table (test file)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| scheme_downgrade_redirect_is_refused_in_auth_follow | 38-72 | REAL | — | 3-angle leak pin: typed error, credential-less origin hit, target never contacted |
| same_scheme_and_upgrade_redirects_still_follow_in_auth | 78-117 | REAL | — | negative space: cross-port https hop + upgrade follow, request counts exact |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| scheme_downgrade_redirect_is_refused_in_auth_follow | YES | removing the gate → the fetch succeeds → `err().must()` panics; partial gates fail the `matches!` or the target-contacted assert |
| same_scheme_and_upgrade_redirects_still_follow_in_auth | YES | over-refusal (e.g. blanket same-scheme policy) fails the follow + request-count asserts |
No tautologies. The https labeling is a string scheme-swap over the plaintext testkit — the documented pinning strategy, honest about what is and is not real (only the scheme DECISION is real; no TLS handshake occurs).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the wrapper refuses the downgrade itself, with the transport's own typed error" | auth_scheme_gate.rs:8-11 | REAL | executed: CrossSchemeRedirect{hops:1}, code `streaming.net.cross-scheme-redirect` |
| "credentials cannot ride a plaintext hop" | auth_scheme_gate.rs:11 | REAL (in the tested lane) | target.requests() empty + no Authorization on the origin probe |
| "challenge-shaped fixtures live there [src/auth.rs]" | auth_scheme_gate.rs:22-24 | REAL (cross-ref) | named tests exist in src/auth.rs (that file's lane) |

## 7. Dead / duplicated / unreachable code
Nothing dead in this file. The guarded production lane (`AuthFetch`) is unreachable from shipped entries (F-001) — a crate-level wiring gap, not a defect of this suite.

## 8. Dependencies & flags
`streaming` (auth, error, fetch, mockhttp), `testkit`. Integration test target; no features. mockhttp = real TCP sockets on loopback.

## 9. Open questions for the next agent
- src/auth.rs audit: does `follow` also enforce a redirect BUDGET (max hops) and loop detection in its manual loop, or only the scheme gate? (client.rs's follower has both; the wrapper's manual loop must not be unbounded.)
- If the CLI ever gains credential support (e.g. `--http-auth user:pass`), F-001 flips to S1-relevant wiring work.
