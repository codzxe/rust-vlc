# AUDIT: crates/plugin-host-wasm/src/http.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 270 / 7 items + 4 tests |
| File verdict | REAL (policy chain executed by tests) — transport is deliberately NoTransport; crate UNREACHABLE from shipped builds |
| Reachable from shipped entry point? | NO — reached only via imports.rs http_get and the crate's own tests; no app embeds plugin-host-wasm |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p plugin-host-wasm --lib http` → 5 passed, exit 0 |

## 1. One-paragraph verdict
A tight, hostile-minded URL policy gate: https-only (never downgraded), userinfo refused, non-443 ports refused, dotted-quad and bracketed-IPv6 literals refused, DNS-charset hostnames only, exact-host allowlist with adversarial tests (subdomain/suffix/scheme smuggling all fail-closed). The transport seam is honest: v1 ships `NoTransport`, so every `http_get` returns typed `E_TRANSPORT` — plugins can never actually fetch, by design and by admission. The one structural hole: the "allowlist holds for every possible transport" claim is unenforceable against a redirect-following transport, and two IP-encoding edge forms slip the literal check. None of it is reachable in any shipped build.

## 2. What actually works (verified)
— verified by `cargo test -p plugin-host-wasm --lib http` → 5 passed, exit 0 (https_only incl. case normalization; userinfo/IP/port refusals incl. the :443 exception; allowlist exact-match with 3 adversarial near-misses; NoTransport typed refusal).
— verified by earlier run `--test plugin_sandbox_security` → 9 passed (the http_get policy chain end-to-end through a real wasm guest).
— Baseline: cargo-test-workspace.txt:4884-5022 plugin-host-wasm exit_code=0.

## 3. Findings
F-001 [S3]
Where: crates/plugin-host-wasm/src/http.rs:14-16 (claim) + 61-71 (trait)
Code:
```
//! The allowlist is enforced *here*, in front of the seam, so it
//! holds for every possible transport implementation.
```
What is wrong: the claim is false for redirect-following transports. `fetch` takes one already-allowlisted URL and returns a body; if an embedder's transport follows a 30x to `evil.test`, the redirect target is never re-checked — no hook, callback, or redirect-denial contract exists in the trait. The "seam does not follow redirects" note (line 29-32) is a comment on `HttpResponse::status`, not an enforcement. Why it matters: the module's headline security invariant is only as strong as an unwritten transport discipline; the deny-by-default posture this crate otherwise enforces mechanically is here left to convention. What correct looks like: the trait documents-and-enforces `max redirects = 0` semantics or re-validates each hop via a callback. Today: unreachable (NoTransport only). Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/plugin-host-wasm/src/http.rs:151-156
Code:
```
let is_ipv4 = !host.is_empty()
    && host
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    && host.matches('.').count() == 3;
```
What is wrong: only the canonical dotted-quad is refused. `https://2130706433/` (decimal 127.0.0.1), `https://0x7f.1/` (hex form), and short forms like `1.2.3` parse as "hostnames" — inet_aton-style resolvers treat them as IPs. Mitigations: no transport exists; TLS cert binding makes literal-IP certs near-impossible for an attacker to own. Confidence: MEDIUM — Verification: read; mutation reasoning (not run against a resolver).

F-003 [S4]
Where: crates/plugin-host-wasm/src/http.rs:90, 124-132
Code:
```
/// Path+query (starts with `/`; `""` becomes `"/"`).
```
What is wrong: for `https://host?q` the authority ends at `?`, tail is `?q` — the stored path starts with `?`, not `/` (the doc is wrong for the query-only form); `url()` then reconstructs `https://host?q`. Harmless to any real HTTP client, but the documented invariant does not hold. Confidence: HIGH — Verification: read (traced both branches).

F-004 [S4]
Where: crates/plugin-host-wasm/src/http.rs:175
Code:
```
port: port.map(|_| 443),
```
What is wrong: an explicit `:443` and an absent port become indistinguishable (`Some(443)` vs `None`→`Some`), i.e. the field lies about "explicit port, if any". Cosmetic — the value is never read elsewhere (rg: `parsed.port` unused outside tests/serde-free struct). Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (>100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TransportError Display/Error | 47-59 | REAL | — | honest messages |
| HttpTransport trait | 61-71 | REAL (contract) | S3 | no redirect re-check hook (F-001) |
| NoTransport::fetch | 79-83 | REAL | — | typed Unavailable, test-pinned |
| ParsedUrl::parse | 107-177 | REAL | S4 | strong policy gate; IP edge forms (F-002), path doc (F-003), port lossy (F-004) |
| ParsedUrl::allowed_by | 181-185 | REAL | — | exact + case-insensitive; trailing-dot fails closed (traced) |
| ParsedUrl::url | 188-190 | REAL | — | canonical reconstruction, test-pinned |
| HttpResponse | 27-36 | REAL | — | status+body; body cap enforced upstream (imports.rs) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| https_only | YES | 3 refusals + normalization + reconstruction asserted (ran: ok) |
| userinfo_ip_and_ports_refused | YES | 5 refusals + :443 exception (ran: ok) — no decimal-IP case (F-002 uncovered) |
| allowlist_is_exact_match | YES | subdomain/suffix/scheme-smuggling adversarial cases (ran: ok) |
| no_transport_is_honestly_unavailable | YES | Unavailable + message asserted (ran: ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 defines the seam and enforces everything policy-side before any transport runs: HTTPS-only, exact-host allowlist, per-minute rate, transfer caps" | http.rs:4-6 | REAL | parse+allowed_by+imports.rs chain; sandbox-security tests execute it |
| "the seam stays unplugged until the Lead adds the edge" (no client wired) | http.rs:9-13 | REAL | only NoTransport impl in tree (rg); http_get returns E_TRANSPORT |
| "allowlist … holds for every possible transport implementation" | http.rs:15-16 | FALSE for redirect-followers | F-001: no re-check hook in the trait |
| "does not follow redirects — policy decision" | http.rs:29-32 | UNVERIFIABLE at the seam | convention only, no enforcement |

## 7. Dead / duplicated / unreachable code
`ParsedUrl::port` is write-only (F-004). Whole crate unreachable from shipped builds (imports.rs F-001).

## 8. Dependencies & flags
std-only. No features. No unsafe. The `lumen-allow-literal` markers are dense and honest (test URLs are RFC-2606-style doubles).

## 9. Open questions for the next agent
- ADR-011 ("in-tree https client precedent") — if the Lead ever wires `streaming`'s ureq client here, F-001's redirect hole becomes live; flag at that review.
- Who validates the manifest's `network_host_allowlist` entries at install time (are IP literals/subdomains pre-refused there)? install.rs, not my file.
