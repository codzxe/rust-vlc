# AUDIT: crates/control-protocol/src/sha1.rs

| Field | Value |
|---|---|
| Auditor | W02-A10 |
| Date | 2026-11-21 |
| Lines / functions | 210 / 9 units (Sha1::{new, update, finalize, compress}, Default impl, `sha1`, `hex`, 3 tests) |
| File verdict | UNREACHABLE from shipped entry points (real, correct, externally-pinned code — but only apps/control-server links this crate, and RELEASE.md ships `lumen` + `lumen-tauri` only) |
| Reachable from shipped entry point? | NO. Chain that exists: `lumen-control-server` (apps/control-server/src/main.rs:21 `use control_protocol::ControlServer` → server → http.rs:51 `Sha1::new()` in `accept_value` → this file). But `apps/cli/Cargo.toml` has no control-protocol dependency (verified — dep list read), `apps/desktop-tauri` neither (rg over all Cargo.toml: only apps/control-server + the crate itself), and docs/RELEASE.md:23-27: "From v0.1.3 the stub leaves every user payload (wave 30): `lumen` + `lumen-tauri` ship". Executed in baseline as TESTS, not as a shipped lane: run #3 log:1454-1456 `sha1::tests::{external_vectors, rfc6455_handshake_vector, streaming_matches_oneshot_at_boundaries}` ok, :1434 `http::tests::accept_value_matches_rfc6455_example` ok |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full `Read` of sha1.rs; `rg "sha1\|Sha1" -t rust` whole repo (single production caller: http.rs:20/51; `hex`/`sha1` one-shot used only by this file's tests); `sed -n '1,70p' crates/control-protocol/src/http.rs` (accept_value: key-text+GUID hashed, base64-wrapped — RFC 6455 §4.2.2 shape confirmed); `rg "control-protocol" **/Cargo.toml` (dependents: apps/control-server only); `sed -n '40,70p' apps/cli/Cargo.toml` (no control-protocol dep); `rg "control-server" scripts docs/RELEASE.md` (not in packaging; RELEASE.md:23-27 payload list); `rg "sha1::tests\|http::tests" audit/runs/baseline/test-workspace.log.txt` (execution evidence :1434-1456); `sed -n '252,258p' docs/MASTER-PROMPT.md` (MP §12 control-server requirement) |

## 1. One-paragraph verdict
The cryptographic content is the strongest kind of in-tree crypto: a textbook RFC 3174 SHA-1 (correct IVs, message schedule, round constants f/K, wrapping adds, big-endian digest — every line hand-traced against the RFC), whose only production caller is the RFC 6455 WebSocket `Sec-WebSocket-Accept` computation (http.rs:51), exactly as the module doc restricts, and whose tests are external oracles (RFC 3174 examples, the NIST million-`a` vector, the RFC 6455 §1.3 worked handshake) that all executed and passed in baseline run #3 — no self-generated goldens anywhere. The verdict problem is not quality but shipping: nothing that ships reaches it. The `lumen` CLI does not depend on control-protocol; `lumen-tauri` does not; the only consumer is the `lumen-control-server` binary, which RELEASE.md's "What ships" table excludes from the v0.1.3 payload — while MP §12 (docs/MASTER-PROMPT.md:256-257) still mandates "a local control server binds to loopback, is versioned and validates every request" as a product security requirement. Real code, real tests, dead-in-release. Two S4 hygiene notes (dead `hex` helper, per-block copy in `update`).

## 2. What actually works (verified)
— Algorithm correctness — verified by line-by-line trace against RFC 3174 §6.1: IVs (67452301/EFCDAB89/98BADCFE/10325476/C3D2E1F0, lines 41-47); schedule `w[i] = rol(1, w[i-3]^w[i-8]^w[i-14]^w[i-16])` (110); f/K per round band — Ch=(b&c)|(~b&d)/0x5A827999, parity=b^c^d/0x6ED9EBA1, Maj=(b&c)|(b&d)|(c&d)/0x8F1BBCDC, parity/0xCA62C1D6 (114-118); temp = rol5(a)+f+e+k+w wrapping (120-125); rol30 for c (128); state adds wrapping (132-136); big-endian word output (96-99).
— Padding/finalize — verified by trace of all boundary cases: buffered==56 after 0x80 (loop skipped, 8 length bytes fill to 64 → compress); buffered in 57..=63 (zeros advance to 64 → compress → buffered resets to 0 → zeros continue to 56); buffered==0 after 0x80 (64-byte-exact messages); the saved pre-pad `length_bits` (85) correctly excludes the pad bytes from the length field; the byte-at-a-time `while self.buffered != 56` loop (90-92) is correct though slow (≤63 single-byte updates, once per hash).
— Streaming buffer discipline — `update`'s fill-compress-drain-tail order traced for all interleavings; the implicit invariant (tail branch at 77-80 only runs with `buffered==0`) holds for every path. Pinned behaviorally by `streaming_matches_oneshot_at_boundaries` (chunk sizes 1,3,7,63,64,65,128,999) — EXECUTED ok (log:1456).
— External vectors — 5 known-answer SHA-1 vectors + the RFC 6455 handshake example (`s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`) — EXECUTED ok (log:1454-1455); the handshake vector additionally exercises the real consumer path shape (two `update` calls + base64).
— Single-purpose claim — `rg "Sha1|sha1"` whole repo: one production caller (http.rs:51 `accept_value`); the "MUST NOT be used anywhere else" doc rule holds today.

## 3. Findings

### F-001 [S4]
Where: crates/control-protocol/src/sha1.rs:147-154
Code:
```rust
/// Lowercase hex rendering (test/log convenience).
pub fn hex(digest: &Digest) -> String {
    let mut out = String::with_capacity(40);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
```
What is wrong: `hex` is a public API whose only callers are this file's own tests (whole-repo rg: no other use). The doc says "test/log convenience" but no log path renders digests — there is no production log consumer. It also formats one `String` per byte (the same per-element `format!` pattern as ids.rs's encoder).
Why it matters: Dead public surface on a crypto module is worse than dead surface elsewhere: every future auditor must re-verify that a `pub fn` touching `Digest` isn't a hex-encoding side channel or an accidental exposure point. Trivial cost to keep, non-zero cost to review.
What correct looks like: Move `hex` into the `#[cfg(test)] mod tests` (it is used only there), or delete it and inline the two-line formatting in tests.
Confidence: HIGH — Verification: read (whole-repo caller census; no non-test caller exists).

### F-002 [S4]
Where: crates/control-protocol/src/sha1.rs:70-76
Code:
```rust
        while rest.len() >= 64 {
            let (block, tail) = rest.split_at(64);
            let mut owned = [0u8; 64];
            owned.copy_from_slice(block);
            self.compress(&owned);
            rest = tail;
        }
```
What is wrong: Every 64-byte block is memcpy'd into a stack array before compression, while `compress` itself already demonstrates the allocation-free idiom (`block.as_chunks::<4>()`, line 106). The same `as_chunks::<64>()` iteration over `rest` would hash in place.
Why it matters: Zero user-visible cost — the hasher runs once per WebSocket handshake over ~60 bytes (24-char key text + 36-char GUID), so at most one block ever passes through here in production. Pure consistency/hygiene: the file carries two different block-handling idioms and the slower one is the outer loop.
What correct looks like: `for block in rest.as_chunks::<64>().0 { self.compress(block); }` then buffer the remainder.
Confidence: HIGH — Verification: read (both idioms in the same file compared; workload size from http.rs:48-57 traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Digest` type alias | 16-17 | REAL | — | [u8; 20] |
| `Sha1` struct | 20-29 | REAL | — | state/buffer/buffered/length_bits; 2^64-bit cap via u64 — wrap at 2^61 bytes documented as unreachable (frames capped at 1 MiB) and physically unconstructible (allocation limit); `wrapping_add` at 56 silently wraps in the impossible case — documented, accepted |
| `impl Default` | 31-35 | REAL | — | delegates to new() |
| `Sha1::new` | 39-52 | REAL | — | RFC IVs exact |
| `Sha1::update` | 55-81 | REAL | S4 (F-002) | fill→compress→drain→tail traced correct for all interleavings; implicit `buffered==0` precondition of the tail branch holds (not stated in code — comment-worthy); per-block copy (F-002) |
| `Sha1::finalize` | 84-101 | REAL | — | saves pre-pad length; 0x80 + zero pad to 56 + 8-byte BE length; boundary cases traced; byte-at-a-time pad loop slow-but-correct; consumes self (no reuse hazard) |
| `Sha1::compress` | 104-137 | REAL | — | schedule + 80 rounds + constants exact per RFC 3174; wrapping adds; `as_chunks::<4>` allocation-free |
| `sha1` (one-shot) | 141-145 | REAL | — | test-only caller today (production goes through streaming `Sha1` in http.rs) — keep: it is the tests' oracle entry |
| `hex` | 148-154 | REAL | S4 (F-001) | production-dead, tests-only |
| tests: `external_vectors` / `rfc6455_handshake_vector` / `streaming_matches_oneshot_at_boundaries` | 162-209 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| external_vectors (163-184) | YES | 5 independent known-answer vectors (RFC 3174 "abc"/448-bit examples, empty string, fox, million-`a`) — any wrong constant, rotation, or pad logic fails at least one; EXECUTED ok (baseline log:1454) |
| rfc6455_handshake_vector (188-194) | YES | RFC 6455 §1.3's own worked example through the exact production call shape (two updates + base64) — catches endianness and integration; EXECUTED ok (log:1455) |
| streaming_matches_oneshot_at_boundaries (199-209) | YES | chunk sizes 63/64/65/… stress the buffer boundaries against the one-shot; sound because the one-shot is itself externally pinned by external_vectors (not goldens generated by the code under test); EXECUTED ok (log:1456) |
Consumer integration: `http::tests::accept_value_matches_rfc6455_example` (log:1434) — the only production caller's output pinned to the RFC example. No tautologies; no self-generated goldens (doc claim "never only by self-generated output (D-5)" is TRUE).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Exists for exactly one purpose: the RFC 6455 §4.2.2 Sec-WebSocket-Accept computation" | sha1.rs:3-4 | REAL | sole production caller http.rs:51 (accept_value); whole-repo rg |
| "SHA-1 is not a security primitive here … MUST NOT be used anywhere else in Lumen" | sha1.rs:5-7 | REAL (today) | no other use; unenforced by tooling — a future import would not trip any gate (open question §9) |
| "Pinned by external vectors … never only by self-generated output (D-5)" | sha1.rs:11-14 | REAL | 5 external vectors + RFC 6455 example, all executed in baseline run #3 |
| "~100 lines" implementation | sha1.rs:7-8 | REAL | hash machinery spans ~105 lines (37-145) — honest to the tilde |
| MP §12: "A local control server binds to loopback, is versioned and validates every request" | MASTER-PROMPT.md:256-257 | UNREACHABLE-in-release (doc vs payload tension) | the control server exists (apps/control-server, loopback + version + validation implemented per its own audits) but RELEASE.md:23-27 ships only `lumen` + `lumen-tauri`; the MP-required surface is not in the shipped payload |

## 7. Dead / duplicated / unreachable code
The file's production code is entirely live WITHIN control-protocol's only consumer (apps/control-server), but that binary is outside the shipped payload (see verdict). In-tree dead surface: `hex` (F-001). No duplication: this is the repo's only SHA-1 (rg; the streaming crate's auth and checksums do not roll their own crypto — not re-verified per-file here, out of scope).

## 8. Dependencies & flags
None: std-only module inside control-protocol (crate deps per W01 manifest audit; sha1.rs itself imports nothing). No unsafe, no cfg beyond tests. The in-tree-vs-sha1-crate decision (ADR-011, "no new dependencies (C5)") is honored: no `sha1`/`sha2` crate appears in the workspace for this purpose.

## 9. Open questions for the next agent
1. Ownership question for the control-server app auditor + orchestrator: MP §12 mandates a loopback control server in the product; RELEASE.md v0.1.3 excludes it from the payload. Either the release plan is mid-transition (control server ships in a later card) or a mandated security surface is dead weight — the whole control-protocol crate (frame/server/http/sha1, waves 2's A04/A06/A09 chunks) shares this UNREACHABLE-from-shipped verdict until that's resolved.
2. "MUST NOT be used anywhere else" (sha1.rs:6-7) is enforced by nothing. A one-line architecture/lint rule (no `use crate::sha1` outside http.rs) would make the doc claim mechanical; today it is review discipline only.
3. If the control server ever ships, fuzz coverage claim "Fuzz every parser" (MP §12) should be checked for the handshake parser (http.rs) — sha1 itself needs no fuzzer, but its input path is network-facing.
