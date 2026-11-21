# AUDIT: crates/control-protocol/src/token.rs
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 6 fns + 2 types + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO from the packaging payload (BIN_CLI + BIN_TAURI only, scripts/packaging/stage-linux-payload.sh) — the crate's sole consumer is `lumen-control-server`. That binary IS built by the workspace and its token path executed in the baseline e2e (`e2e_full_session_through_the_app_stack ... ok`, log:1526, fixed-token hello handshake + `token rejected` refusal) and unit tests (log:1457-1459). Verdict per rule 10: UNREACHABLE-from-shipped-payload, REAL-and-executed in the control-server lane. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | `sed -n` (all quotes verified); `rg -n "generate_token|constant_time_eq|EntropySource" crates/ apps/` (callers: server.rs:64-67,387-388,720; main.rs:24,237-240); `rg -n "control-protocol" apps/*/Cargo.toml` (only apps/control-server depends on it); `rg -n "derive" server.rs` (Debug leak chain); baseline `rg -n "token::tests" test-workspace.log.txt:1457-1459` — 3/3 ok |

## 1. One-paragraph verdict
The token core is sound: a textbook OR-accumulate constant-time comparison
(17-26, data-independent loop, length treated as public and documented as
such), 32 bytes from `/dev/urandom` base64'd to 44 chars, a typed
`EntropySource` so callers can warn on the weak fallback — and the one real
caller (control-server main.rs:236-240) does implement that warning. All three
tests executed and passed in the baseline. Two real defects: (1) `Token`
derives `Debug`, and both `ControlServer` and `ServerConfig` derive `Debug`
while holding the raw token — the type design arms a one-statement leak of the
auth secret into any future `{:?}` log, directly contradicting the module's own
"Token material NEVER enters error messages or logs (V8)" invariant; (2) the
timing property that is the function's entire reason to exist has zero test
coverage — every test checks values only, so a regression to `a == b`
(early-exit) passes the suite. Cross-file: server.rs:387 labels an
operator-pinned fixed token as `EntropySource::OsRandom`, breaking this file's
type contract (F-003, filed for the server.rs auditor).

## 2. What actually works (verified)
— Verified by baseline `test-workspace.log.txt:1457-1459`: `equal_and_unequal_comparisons`,
`generated_tokens_are_256_bit_and_distinct`, `weak_fallback_is_well_formed` —
all ok (the OsRandom generation path executed: `/dev/urandom` exists in the
sandbox; the test asserts the 44-char padded shape and decode-to-32-bytes).
— Verified by baseline `test-workspace.log.txt:1526` + e2e.rs:105-106,126,146:
the constant-time compare runs in a real server handshake (fixed-token hello
accepted, tokenless hello refused `Unauthenticated "token rejected"`).
— Verified by reading server.rs:720: the compare is used on the auth path
(`constant_time_eq(presented.as_bytes(), self.token.0.as_bytes())`).
— Verified by reading main.rs:236-240: the `WeakFallback` warning the module
doc demands ("callers MUST log a warning") is implemented by the sole caller.

## 3. Findings

### F-001 [S2] `Token` derives `Debug` and is embedded in two Debug-deriving server structs — the V8 "never in logs" invariant is one careless `{:?}` away from violation
Where: crates/control-protocol/src/token.rs:29-30 (+ server.rs:296-300, 203-213)
Code:
```
/// A generated token (base64 of 32 random bytes — 256 bits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(pub String);
```
and (server.rs:296-300):
```
#[derive(Debug)]
pub struct ControlServer {
    listener: TcpListener,
    config: ServerConfig,
    token: Token,
```
What is wrong: the module's first paragraph asserts "Token material NEVER
enters error messages or logs (V8 redaction …)" (token.rs:5-7), but the
secret type prints itself verbatim under `{:?}`. `ControlServer` (server.rs:296)
derives `Debug` holding `token: Token`, and `ServerConfig` (server.rs:203)
derives `Debug` holding `TokenSource::Fixed(String)` — the raw operator token.
Today no production call site Debug-prints them (verified: the only `{:?}` in
server.rs is line 912, a bind address), so there is no ACTIVE leak — but the
derive chain means any future `tracing`/`eprintln!("{server:?}")`/panic
backtrace-with-locals or error-context wrapper leaks the loopback auth secret
into stderr/logs, silently. There is also no zeroization of the material.
Why it matters: the token is the sole authenticator for the deny-by-default
control server (sensitive-command allowlist rides on it); the security
invariant is stated, and the type system actively undermines it — unlintable,
undetectable at review time.
What correct looks like: manual `impl Debug for Token { write!(f, "Token(<redacted>)") }`
(same for `TokenSource::Fixed`), or a `redact` feature; keep `PartialEq` (fine
— value equality does not leak through logs).
Confidence: HIGH — Verification: read (derives enumerated; grep for `{:?}`/`{self:?}`/`{server:?}` over control-protocol + control-server → no active print site).

### F-002 [S3] The constant-time property has zero test coverage — an early-exit regression passes the whole suite
Where: crates/control-protocol/src/token.rs:113-129
Code:
```
    #[test]
    fn equal_and_unequal_comparisons() {
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(b"token", b"token"));
        assert!(!constant_time_eq(b"token", b"tokeN"));
        assert!(!constant_time_eq(b"token", b"toke"));
```
What is wrong: all three tests assert VALUE equality/inequality only. The
function's entire reason to exist — no timing side channel — is not exercised
by anything. Mutation analysis: replace the body with `a == b`; every
committed test still passes (baseline included). The one-bit-difference loop
(121-128) checks correctness at each position, not timing.
Why it matters: timing resistance is the security claim; it is currently
enforced by code review alone. (Timing tests are genuinely hard/portable-hostile
— that is an explanation, not an excuse: the gap should at least be documented
at the test, and the OR-accumulate idiom pinned by a comment/test on the
accumulation structure.)
What correct looks like: an explicit note + ideally a statistical timing
harness behind `#[ignore]` (manual), or at minimum a test that the function
compares ALL bytes (e.g. assert it never short-circuits via a counting wrapper
— not possible with `&[u8]`, which is exactly why the note is the honest fix).
Confidence: HIGH — Verification: read (all test bodies checked for any timing
assertion — none).

### F-003 [S3] (cross-file, contract defined here) `TokenSource::Fixed` is reported as `EntropySource::OsRandom`
Where: crates/control-protocol/src/server.rs:387 (contract type: token.rs:55-62)
Code:
```
        let (token, entropy) = match &config.token {
            TokenSource::Fixed(token) => (Token(token.clone()), EntropySource::OsRandom),
            TokenSource::Generated => generate_token(),
```
What is wrong: an operator-pinned fixed token (config `[auth] token = …`,
e2e uses exactly this shape) is labeled `OsRandom`. `EntropySource`'s doc
("Where a token's entropy came from") is violated; `main.rs:237-240`'s warning
logic keys off it, and any audit/log consumer is told the token has OS entropy
when it has operator-chosen entropy (potentially a reused/weak string).
Why it matters: the entropy reporting exists precisely to surface weak tokens;
the Fixed path lies by construction. Blast radius is small (the operator chose
the token and main.rs only prints it to the terminal they own), hence S3 not S2.
What correct looks like: `EntropySource::Fixed` variant (or report WeakFallback
semantics) so the warning path can fire for pinned tokens.
Confidence: HIGH — Verification: read (server.rs:386-389 + main.rs:236-240
cross-checked; e2e.rs:126 shows the Fixed config shape in real use).
Filed here because the type contract lives in this file; the server.rs auditor
should carry it.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `constant_time_eq` | 17-26 | REAL | — | Standard OR-accumulate idiom, data-independent loop; length early-return documented as public info. Correct by construction; timing untested (F-002). Executed in baseline e2e handshake. |
| `Token` (newtype) | 29-30 | REAL | S2 | `Debug` derive contradicts V8 invariant (F-001). |
| `generate_token` | 45-53 | REAL | — | OsRandom primary; weak fallback typed and surfaced. Executed (baseline unit test 1458). |
| `EntropySource` | 56-62 | REAL | S3 | Contract sound; violated by server.rs:387 for Fixed tokens (F-003). |
| `read_urandom` | 65-71 | REAL | — | Fresh open + `read_exact`; correct error propagation to the fallback. |
| `weak_fallback_token` | 75-94 | REAL | — | splitmix64 finalizer, std-only; entropy = nanos ^ pid<<32 ^ RandomState-base — honestly labeled weak `[VERIFY]` (37). 44-char shape pinned by test. |
| `RandomSeed` + `next` | 98-107 | REAL | — | Comment "RandomState::new() is itself seeded randomly by std" is accurate for the per-process base (OS-seeded); successive calls within one process differ only by std's internal counter — adequate for the fallback's stated threat model (local trivial guessing), note for the record. |
| `tests::equal_and_unequal_comparisons` | 113-129 | REAL | — | Executed+passed (1457). Value-correctness only (F-002). |
| `tests::generated_tokens_are_256_bit_and_distinct` | 131-145 | REAL | — | Executed+passed (1458). Pins 44-char padded shape, decode-to-32-bytes, draw-distinctness. Cannot catch weak entropy (inherent). |
| `tests::weak_fallback_is_well_formed` | 147-152 | REAL | — | Executed+passed (1459). Shape + distinctness of the fallback. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| equal_and_unequal_comparisons | YES for values / NO for timing | Value mutations caught at every bit position; timing regression (early exit) NOT caught (F-002). |
| generated_tokens_are_256_bit_and_distinct | YES for shape | Length/padding/decode/distinctness pinned; entropy quality untestable here. |
| weak_fallback_is_well_formed | YES for shape | Same scope. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "comparison is constant-time" | token.rs:4 | REAL-UNPROVEN | Idiom is correct by inspection (17-26); no test or run measures timing (F-002). |
| "Token material NEVER enters error messages or logs (V8 redaction)" | token.rs:5-7 | SHALLOW | No active print site (grep-verified), but `Debug` derives on Token/ControlServer/ServerConfig arm accidental leakage (F-001); sanctioned terminal print exists (main.rs:232-235). |
| "callers MUST log a warning when this path [WeakFallback] is used" | token.rs:38-39 | REAL | Sole caller implements it (main.rs:236-240). |
| "The returned `EntropySource` says which path won" | token.rs:43-44 | FALSE for Fixed tokens | server.rs:387 hardcodes OsRandom for operator-pinned tokens (F-003). |
| "A per-process `RandomState`-derived seed (address randomness without unsafe code)" | token.rs:96-97 | REAL | std RandomState base is OS-seeded per process; no unsafe — accurate. |

## 7. Dead / duplicated / unreachable code
- Nothing dead in-file. Whole module is unreachable from the two packaging-
  payload bins (only `lumen-control-server` consumes control-protocol) — see
  reachability field; the lane itself is built and baseline-executed.

## 8. Dependencies & flags
- std-only by design (comment cites the std-only constraint; the hand-rolled
  base64 lives at crate::base64 — no external base64/subtle/getrandom deps).
  Consequence: no `subtle` crate for constant-time ops and no `getrandom` —
  the reasons F-001/F-002 exist. No feature gates.

## 9. Open questions for the next agent
- server.rs auditor: carry F-003 (Fixed→OsRandom mislabel); also check the
  auth-failure/lockout paths for token material in error payloads (V8).
- Whoever owns packaging: is `lumen-control-server` meant to ship? If yes the
  payload omits it (same note as my ids.rs audit); if no, this whole crate is
  dev-lane only and F-001's severity arguably drops one notch.
