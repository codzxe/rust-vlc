# AUDIT: crates/control-protocol/src/sha1.rs
| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-23 |
| Lines / functions | 210 / 6 (Sha1::new, update, finalize, compress, sha1, hex) + Default impl + 3 tests |
| File verdict | REAL — correct, streaming, externally-anchored SHA-1; single production consumer is the WS handshake |
| Reachable from shipped entry point? | NO for the release lane (same chain as http.rs: `lumen-control-server` → server.rs:538 `http::accept_value` → http.rs:51 `Sha1::new`). No other caller in the repo (rg). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p control-protocol --lib sha1` → 3 passed/0 failed; python3 hashlib independent re-verification of all 6 vectors; rg Sha1/sha1(/5A827999 (duplication check) |

## 1. One-paragraph verdict
Why SHA-1 exists here: RFC 6455 §4.2.2 mandates `Sec-WebSocket-Accept = base64(SHA-1(key + GUID))` for the WebSocket upgrade — http.rs:51 is the one and only production consumer, exactly as the module doc claims. The hand-rolled implementation is CORRECT: I re-derived the buffering, padding (including the buffered∈[56..63] extra-block case), big-endian encoding, message schedule, round constants and functions line-by-line, and independently re-computed all six pinned vectors (five RFC/NIST/Wikipedia digests + the RFC 6455 accept value) with Python's hashlib — all match. The tests are honest external oracles, not self-generated output, plus a boundary-chunk streaming test. The single finding is that the one-shot `sha1()` and `hex()` helpers are `pub` but test-only — dead public API that invites the "used anywhere else" the module doc forbids. Not reachable from the shipped release lane (control-protocol is only in the unshipped, feature-gated-off control-server app).

## 2. What actually works (verified)
— verified by `cargo test -p control-protocol --lib sha1` → `3 passed` (`external_vectors`, `rfc6455_handshake_vector`, `streaming_matches_oneshot_at_boundaries`).
— verified by `python3 -c "import hashlib, base64; …"` → all six constants reproduced exactly: `a9993e36…d89d`, `84983e44…70f1`, `da39a3ee…0709`, `2fd4e1c6…eb12`, million-`a` `34aa973c…16f`, and `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=` — so the test constants are genuine external oracles (the D-5 claim holds).
— hand-traced: `update`'s 64-byte buffering (no path reaches the tail-copy with `buffered > 0`; `buffered+take ≤ 64` always), `finalize`'s one-byte-at-a-time padding terminating at 56 from any residue (63 wraps through compress to 0), 8-byte big-endian length, digest words big-endian — no off-by-one, no panic path (all slice bounds derived from `min`/`split_at`).

## 3. Findings
F-001 [S4]
Where: crates/control-protocol/src/sha1.rs:140-154
Code:
```
/// One-shot SHA-1.
pub fn sha1(data: &[u8]) -> Digest {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize()
}

/// Lowercase hex rendering (test/log convenience).
pub fn hex(digest: &Digest) -> String {
```
What is wrong: `sha1()` and `hex()` have zero production callers — rg over the repo shows their only uses are sha1.rs's own tests (L165-207); no log code calls `hex` despite the "log convenience" doc. They are `pub` test-only API. Why it matters: the module doc itself says SHA-1 "MUST NOT be used anywhere else in Lumen" (L6-7) — exporting a convenient one-shot + hex renderer is the exact surface that invites new uses; it is also dead public surface for a crate that otherwise gates its API tightly. (`hex` additionally builds a temporary String per byte via `format!` — irrelevant while test-only.) What correct looks like: `#[cfg(test)]` (or crate-private) on both. Confidence: HIGH — Verification: read + rg.
No correctness findings: implementation matches RFC 3174 on every point checked (see §2); the only sibling hash code in-tree is MD5 in test commons (rg `0x67452301` → `[u32; 4]` states, i.e. MD5, not a second SHA-1).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Sha1::new (+Default) | 31-52 | REAL | — | RFC 3174 §6.1.1 IVs verbatim |
| Sha1::update | 54-81 | REAL | — | Bounded buffering verified; length in bits via wrapping mul/add |
| Sha1::finalize | 83-101 | REAL | — | Padding incl. extra-block case; big-endian length + digest; saves true length before 0x80 bump |
| Sha1::compress | 103-137 | REAL | — | Schedule, K constants, f functions, rotations, wrapping adds — all RFC-correct; no unsafe |
| sha1 (one-shot) | 140-145 | REAL | S4 | Test-only pub API (F-001) |
| hex | 147-154 | REAL | S4 | Test-only pub API; per-byte format! (F-001) |
| tests ×3 | 156-209 | REAL | — | Executed; externally anchored |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| external_vectors | YES | Five independent external digests (RFC 3174 ×2, empty-string, fox, NIST million-`a`) — re-verified against Python hashlib; any schedule/constant/endian error breaks them |
| rfc6455_handshake_vector | YES | The RFC 6455 §1.3 accept value — breaks on wrong GUID concat, wrong padding, or wrong base64 |
| streaming_matches_oneshot_at_boundaries | YES* | Self-consistency (one-shot vs streamed) at chunk sizes 1/3/7/63/64/65/128/999 — catches buffering bugs, but both sides share compress(); anchored externally by the other two tests |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exists for exactly one purpose: the RFC 6455 §4.2.2 Sec-WebSocket-Accept computation" | sha1.rs:3-4 | REAL | rg: sole production caller is http.rs:51 (`accept_value`) |
| "MUST NOT be used anywhere else in Lumen" | sha1.rs:6-7 | REAL | rg: no other import of `Sha1`/`sha1()` in any crate |
| "Pinned by external vectors … never only by self-generated output (D-5)" | sha1.rs:11-14 | REAL | Vectors re-computed with python3 hashlib — all match; tests executed |
| "Implemented in-crate … rather than pulling a sha1 crate — no new dependencies (C5)" | sha1.rs:7-9 | REAL | control-protocol has no external hash dep (Cargo.toml audit, file 5) |
| "ADR-011 in-tree ethos" | sha1.rs:1 | REAL (loose) | ADR-011 (docs/adr/ADR-011-tls-https.md) is about TLS dependency admission but does articulate the in-tree-vs-dependency posture the citation leans on |

## 7. Dead / duplicated / unreachable code
`sha1()` + `hex()` are test-only pub API (F-001). No duplicate SHA-1 elsewhere in-tree (rg constant sweep: only MD5 states in test commons/tools). Whole module unreachable from the release lane (see header chain).

## 8. Dependencies & flags
Std-only, zero external crates, no unsafe, no feature gates. Workspace lints (deny unwrap/expect/panic) hold.

## 9. Open questions for the next agent
- The frame codec's self-roundtrip tests lack an external byte anchor (flagged by the conformance.rs sibling audit); sha1/http are externally anchored but frames are not — worth one RFC 6455 §5.7 pinned vector.
- If control-server ever ships, F-001's cfg(test) fix should land first so no new caller grows on the one-shot API.
