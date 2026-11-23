# AUDIT: crates/control-protocol/src/token.rs
| Field | Value |
|---|---|
| Auditor | W02-A04 |
| Date | 2026-11-23 |
| Lines / functions | 153 / 8 (5 fns/types + 1 impl + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (crate-level: only apps/control-server depends on control-protocol; release lane = lumen-cli + desktop; product.toml:167 `control-server = false`) — but `constant_time_eq`/`Token` execute on every control-server run and in 3 unit tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/control_protocol-30329beb10b168e9 token::` → 3 passed, EXIT=0; `sed -n` quote verification; Grep for Debug-formatting call sites (none format Token/ControlServer today) |

## 1. One-paragraph verdict
Real, std-only token plumbing: a textbook constant-time compare (OR-accumulated XOR, length treated as public and documented as such), 32 bytes from `/dev/urandom`, base64'd in-tree, with a splitmix64-based weak fallback whose constants I verified against the reference algorithm (0x9E3779B97F4A7C15 / 0xBF58476D1CE4E5B9 / 0x94D049BB133111EB, shifts 30/27/31 — all correct). Two genuine defects. First, a test gap on the security-critical path: `generate_token()`'s `EntropySource` is discarded by the only test that could check it (`let _ = source;`), so if `read_urandom` broke — the whole CSPRNG dependency — every test still passes and every server start silently degrades to the time-derived weak token; the sole detector would be the app's stderr warning. Second, `#[derive(Debug)]` on `Token` (and on `ControlServer`, which contains it, server.rs:295-297) structurally contradicts this module's own V8 claim that token material NEVER enters logs — the discipline is convention, not type-system; one `{:?}` on the server leaks the live token. No current call site formats it (verified by search), so the footgun is loaded but not yet stepped on.

## 2. What actually works (verified)
— verified by `./target/debug/deps/control_protocol-30329beb10b168e9 token::` → `test result: ok. 3 passed; 0 failed` (`equal_and_unequal_comparisons` incl. one-bit differences at every position, `generated_tokens_are_256_bit_and_distinct` (44-char padded base64 → decodes to 32 bytes, two draws differ), `weak_fallback_is_well_formed`).
— splitmix64 constants checked against the reference algorithm by reading lines 86-90 — all three multipliers and the 30/27/31 shift sequence are the canonical values.

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/src/token.rs:132-145 (test), 45-53 (code under test)
Code:
```
    fn generated_tokens_are_256_bit_and_distinct() {
        let (token, source) = generate_token();
        ...
        let _ = source;
    }
```
and:
```
    match read_urandom(32) {
        Ok(bytes) => (
            Token(crate::base64::encode(&bytes)),
            EntropySource::OsRandom,
        ),
        Err(_) => (Token(weak_fallback_token()), EntropySource::WeakFallback),
    }
```
What is wrong: the test discards `source`, and no test in the crate asserts `EntropySource::OsRandom`. The 44-char/distinct/decodes-to-32 assertions pass identically for BOTH paths. If `read_urandom` regressed (wrong path, permission failure, read error), zero tests fail, and every server start uses the weak time-derived token — the module doc's "256-bit token per start" silently becomes false. Why it matters: the entropy source IS the security property; the type exists precisely to distinguish the paths and nothing consumes the distinction in tests. What correct looks like: on Unix CI, `assert_eq!(source, EntropySource::OsRandom)`; plus a unit test forcing the fallback via a `read_urandom` seam. Confidence: HIGH — Verification: read + executed (3 tests pass; none check the source).
F-002 [S3]
Where: crates/control-protocol/src/token.rs:29-30 (+ server.rs:295-297, 203-204)
Code:
```
/// A generated token (base64 of 32 random bytes — 256 bits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(pub String);
```
and (server.rs:294-297):
```
/// The loopback control server (card 7.J1).
#[derive(Debug)]
pub struct ControlServer {
```
and the module's own claim (token.rs:4-5):
```
//! it in `hello`; comparison is constant-time. Token material NEVER
//! enters error messages or logs (V8 redaction —
```
What is wrong: the derived `Debug` prints the token verbatim, and `ControlServer` (which holds `token: Token`) derives `Debug` too, as does `ServerConfig` (holding `TokenSource::Fixed(String)`). The V8 rule is enforced by nothing but review discipline; the very next debugging `eprintln!("{server:?}")` or a `#[derive(Debug)]` added to any future struct containing a Token leaks the live secret to the log stream the crate's JSON logging explicitly promises to keep token-free. Why it matters: this crate's security posture (docs/security/sweep-control.md, V8) rests on the redaction rule; a derived Debug on a secret type is the standard way that rule dies. No current formatter exists (Grep over apps/control-server + crate: no `{:?}` of server/config/token), so no active leak — a loaded footgun. What correct looks like: manual `impl Debug for Token { write!(f, "Token(<redacted>)") }`. Confidence: HIGH — Verification: read + rg (no formatting call sites).
F-003 [S4]
Where: crates/control-protocol/src/token.rs:35-41, 64-71
Code:
```
/// 1. `/dev/urandom` (the OS CSPRNG) — the primary, Unix-only path.
```
What is wrong: on Windows `/dev/urandom` does not exist, so `generate_token` ALWAYS returns the weak fallback there. Honestly documented ("The Windows entropy path joins with the platform adapter crates … recorded gap") and the app warns on `WeakFallback` (main.rs:236-241, eprintln at :239), and control-server ships nowhere today — hence S4, not higher. But the parity program targets Windows, and doc 05 §5.4's "per-start token" would there be time+pid-derived. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/control-protocol/src/token.rs:11-16 + docs/security/sweep-control.md:55-57
Code:
```
/// Compare two byte strings in constant time w.r.t. their CONTENT.
```
What is wrong: nothing in the implementation — but note the evidence wording: the sweep doc says "tests pin the one-bit-at-every-position behavior", which is a CORRECTNESS pin at every bit position; the constant-TIME property itself is neither tested nor testable functionally (and the code's own doc is careful to scope the claim to content). Recorded so no one cites the test as a timing guarantee. Confidence: HIGH — Verification: read + executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| constant_time_eq | 17-26 | REAL | — | OR-accumulated XOR; length public (documented); used at server.rs:720; correctness pinned at every bit position |
| Token (newtype) | 29-30 | REAL | S3 | derived Debug leaks the secret (F-002) |
| generate_token | 45-53 | REAL | S3 | OsRandom path unpinned by tests (F-001); Windows always fallback (F-003) |
| EntropySource (enum) | 56-62 | REAL | — | consumed by main.rs warning + summaries; NOT by tests (F-001) |
| read_urandom | 65-71 | REAL | — | open + read_exact 32; correct |
| weak_fallback_token | 75-94 | REAL | — | splitmix64 constants verified canonical; `state \| 1` avoids zero seed; wrapping ops, no panic |
| RandomSeed::next | 100-107 | REAL | — | RandomState-derived, std-only, no unsafe |
| tests (3) | 113-152 | REAL | S3/S4 | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| equal_and_unequal_comparisons | YES | equal/unequal, length mismatch both ways, 1-bit diffs at all 8 positions |
| generated_tokens_are_256_bit_and_distinct | PARTIAL | catches wrong LENGTH/shape/distinctness; does NOT catch CSPRNG-path breakage (source discarded, F-001) |
| weak_fallback_is_well_formed | YES | shape + distinctness of fallback |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Token material NEVER enters error messages or logs (V8)" | token.rs:4-5 | UNENFORCED-BY-TYPE | derived Debug on Token/ControlServer/ServerConfig (F-002); no current leak (rg) |
| "The server generates a 256-bit token per start" | token.rs:3 | REAL on Unix, WEAK on Windows | F-003; structure pinned by test (44 chars → 32 bytes) |
| "tests pin the one-bit-at-every-position behavior" | docs/security/sweep-control.md:55-57 | REAL but is a correctness pin, not timing | F-004; test executed by me |
| "callers MUST log a warning when this path is used" | token.rs:38-39 | REAL | apps/control-server/src/main.rs:236-241 warns on WeakFallback (read; eprintln at :239) |

## 7. Dead / duplicated / unreachable code
None in this file. `EntropySource` is consumed by the app (warning) and `ControlServer::entropy_source` — but by no test (F-001). Crate-level unreachability as in §Field (unshipped stack).

## 8. Dependencies & flags
std-only (no crates); no unsafe; no features; base64 is the in-tree `crate::base64` (has RFC 4648 vector tests, baseline :1558-1559).

## 9. Open questions for the next agent
- If control-server ever ships, F-002 (redacting Debug) should be a pre-req — worth a phase-gate line in docs/security/sweep-control.md?
- Is a `read_urandom` seam (injectable failure) planned so the fallback path can be driven in tests on any OS?
