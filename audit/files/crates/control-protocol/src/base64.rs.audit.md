# AUDIT: crates/control-protocol/src/base64.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-23 |
| Lines / functions | 220 / 9 (2 pub fns, 2 private fns, 2 impls, 3 tests) |
| File verdict | REAL (correct RFC 4648; hostile focus cleared) |
| Reachable from shipped entry point? | NO (same crate as server.rs: only apps/control-server consumes control-protocol, gated off; in-crate callers: token.rs:48/93/140, http.rs:50/54/259/276) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/control_protocol-30329beb10b168e9 base64` -> 3 passed, exit 0; sed traces of every arithmetic/slicing path |

## 1. One-paragraph verdict
Hand-rolled base64 done right. Encode packs 3-byte chunks into a 24-bit window with correct quartet extraction and canonical `=` padding; decode is STRICT RFC 4648 §4 — standard alphabet only (URL-safe `-`/`_`, whitespace, and misplaced `=` are typed errors), mandatory canonical padding, and — the part most hand-rolled implementations get wrong — canonical trailing-bit enforcement (`window & 0xff` / `window & 0xffff` checks, L121/L129), which I re-derived from the bit layout and confirmed correct: for a 3-symbol tail the last symbol's low 2 bits must be zero, for a 2-symbol tail its low 4 bits. The external oracle is present and executed: all 7 RFC 4648 §10 vectors pass, an exhaustive round-trip over every byte value at every length 0..=255 passes, and the dirty-input suite pins exact typed variants (offsets included). No panic path exists: every slice index is either preceded by a length multiple-of-4 check, a `pad > 2` rejection, or uses `chunks`/`get`. Zero findings.

## 2. What actually works (verified)
— `./target/debug/deps/control_protocol-30329beb10b168e9 base64` -> "test result: ok. 3 passed" exit 0 (rfc4648_vectors, roundtrip_all_byte_values, dirty_inputs_are_typed_errors).
— Manual bit-level re-derivation of both canonical-tail checks (L120-133) against RFC 4648 padding semantics — see §3 note.

## 3. Findings
None. What was checked, and how (per spec §12 zero-findings rule):
- **Alphabet** (L38, L155-163): byte-range match implements exactly `A-Z a-z 0-9 + /`; no URL-safe variants, no whitespace tolerance — matches the strict-decode contract in the module doc (L4-6).
- **Padding** (L93-105): trailing `=` count capped at 2 (`"===="` → BadPadding); `=` inside the symbol run (`"=g=="`) → BadPadding; symbol-count/pad congruence (`symbols.len() % 4 == 4 - pad`) enforced; length not multiple of 4 → BadLength. All pinned by executed test lines 205-218.
- **Canonical trailing bits** (L120-133): re-derived — 3-symbol tail = 18 bits of which 16 are data ⇒ last symbol low 2 bits must be 0, and `window & 0xff` checks exactly that (bits 5..0 are structurally zero from the `<<= 6 * (4 - len)` shift at L150); 2-symbol tail = 12 bits of which 8 are data ⇒ last symbol low 4 bits must be 0, and `window & 0xffff` checks exactly that. Both correct; `"Zh=="` rejection pinned by executed test.
- **Panic surface**: `bytes[..bytes.len() - pad]` (L99) cannot underflow (len ≥ 4 multiple of 4; pad ≤ 2 rejected earlier); `symbols[..full_groups * 4]` (L108) is a floor slice; `chunk[0]` (L48) is safe because `chunks(3)` yields non-empty chunks; `chunk.get(1)/(2)` use `unwrap_or(&0)`. Multibyte UTF-8 input is byte-checked per `as_bytes()` and rejected as InvalidSymbol — no char-boundary slicing anywhere.
- **Unreachable-but-required arms**: `(2,0)/(3,0)/(1,_)` tails (L134) cannot occur given the multiple-of-4 + congruence invariants; the `_` arm is required for match totality and is defensive, not removable dead code.
- **Capacity** (L42, L106): `div_ceil(3) * 4` exact for encode; decode over-reserves ≤2 bytes on 2-symbol tails — immaterial.
- **Mutation reasoning**: break the alphabet → rfc4648_vectors fails; break padding rules or canonicality → dirty_inputs_are_typed_errors fails; break any length path → roundtrip_all_byte_values (256 lengths) fails. The three tests are not tautologies and would fail on each of the classic base64 bugs.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Base64Error (enum) + Display | 12-34 | REAL | — | typed, no raw input echoed (offsets/lengths only) |
| ALPHABET (const) | 38 | REAL | — | standard RFC 4648 §4 |
| encode | 41-81 | REAL | — | correct quartets; `XX==`/`XXX=` tails; no panics |
| decode | 88-137 | REAL | — | strict; canonical padding + trailing bits; no panics |
| group_window | 140-152 | REAL | — | top-aligned short groups (the subtle part — correct) |
| symbol_value | 155-164 | REAL | — | exact alphabet match |
| test rfc4648_vectors | 171-189 | HONEST | — | external oracle (D-5), encode + decode both asserted |
| test roundtrip_all_byte_values | 191-201 | HONEST | — | 256 prefixes, both directions |
| test dirty_inputs_are_typed_errors | 203-219 | HONEST | — | exact variant + offset assertions |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rfc4648_vectors | YES | independent external oracle |
| roundtrip_all_byte_values | YES | exhaustive length sweep catches tail bugs |
| dirty_inputs_are_typed_errors | YES | pins exact error variants incl. offsets, whitespace, non-canonical bits |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "In-tree base64 (RFC 4648 §4, standard alphabet) … Pinned by the RFC 4648 test vectors" | base64.rs:1-9 | REAL | vectors test executed, exit 0 |
| "a dirty-world input is a typed Base64Error, never a panic (D-3)" | base64.rs:5-6 | REAL | §3 panic-surface trace: no reachable panic path |
| "Strict decode: … mandatory canonical padding, no whitespace" | base64.rs:4-6 | REAL | dirty-input test executed; unpadded input ("Zg") → BadLength |
| "~100 lines each, specified by RFC 3174 / RFC 4648" | lib.rs (in-tree note) | REAL (base64 half) | 220 lines incl. 55 lines of tests/docs |

## 7. Dead / duplicated / unreachable code
None. The `(2,0)/(3,0)/(1,_)` match arms (L134) are unreachable under the length invariants but required for match totality — defensive, correct as written.

## 8. Dependencies & flags
std-only; no external crates (the point of the in-tree implementation, C5/ADR-011). No feature flags, no unsafe, no unwrap/expect outside tests. In-crate consumers: token.rs (256-bit token render/parse), http.rs (Sec-WebSocket-Key strict decode with 16-byte length enforcement at http.rs:259-266, and Accept encode) — wire-facing strictness is real.

## 9. Open questions for the next agent
- sha1.rs (same crate, not assigned to me) shares the "in-tree, pinned by external vectors" claim — its tests passed in my run; a dedicated audit should re-derive the RFC 3174 padding logic the way this file's tails were re-derived.
- If the control server ever fronts tokens to non-test clients, note decode() rejects unpadded base64url tokens by design — any future UI client must use standard padded base64.
