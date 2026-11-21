# AUDIT: crates/control-protocol/src/base64.rs
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2026-11-21 |
| Lines / functions | 220 / 8 production items (encode, decode, group_window, symbol_value, Base64Error, Display, Error impl, ALPHABET) + module doc + 3 tests |
| File verdict | REAL (executed evidence: baseline test run) |
| Reachable from shipped entry point? | NO for `lumen`/`lumen-tauri` (apps/cli, apps/desktop-tauri, apps/desktop do not depend on control-protocol — rg verified, no edge). YES from the workspace binary `apps/control-server`: control-server main → `crates/control-protocol/src/server.rs:522` `http::read_upgrade_request` → `server.rs:538` `http::accept_value` → `http.rs:50/54` `base64::encode`; handshake key validation `http.rs:259` `base64::decode`; token minting `token.rs:48/93` `base64::encode`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg "base64::" crates/control-protocol/src` → 7 sites (5 production: token.rs:48,93, http.rs:50,54,259; 2 in tests: sha1.rs:192, token.rs:140). `rg "control-protocol" apps/{cli,desktop-tauri,desktop}/Cargo.toml` → no hits (unreachable from shipped shells). `rg "rfc4648\|base64" audit/runs/baseline/test-workspace.log.txt` → lines 1414/1415/1424: all three tests PASSED in baseline run #3 (control_protocol lib, 47 tests). `sed -n` quote verification. ADR-011/C5 citation traced (docs/13-agent-wave-program.md:118 C5 = dependency policy). |

## 1. One-paragraph verdict
Correct, strict, panic-free RFC 4648 implementation with honest typed errors and real executed proof: all three in-crate tests (RFC vectors, full 0..255 roundtrip, dirty-input taxonomy) ran and passed in the baseline workspace test run (test-workspace.log.txt:1414-1424). I hand-traced encode/decode bit-by-bit — alphabet mapping, tail canonical-bit checks (`window & 0xff` / `& 0xffff` masks whose extra bits are provably zero by the shift construction), padding position rules, and the unreachable-by-construction `(1,_)` tail arm — and found no input that panics, mis-accepts, or mis-rejects. The only defect is doc-precision: "Two call sites" is five production sites across three files (two named *features*, but a blast-radius reader counts sites). The in-crate-not-a-dependency claim checks out (no base64 dep in Cargo.toml) and the C5/ADR-011 citation is real (C5 = the no-new-deps-without-Lead-approval policy, doc 13:118).

## 2. What actually works (verified)
— Executed: `audit/runs/baseline/test-workspace.log.txt` lines 1414-1424 — `base64::tests::dirty_inputs_are_typed_errors ... ok`, `base64::tests::rfc4648_vectors ... ok`, `base64::tests::roundtrip_all_byte_values ... ok` (control_protocol lib section, line 1411, 47 tests).
— Hand-traced correctness: encode packs 3 bytes into a 24-bit window and emits 4/3+pad/2+2pad symbols per chunk (47-78) — canonical by construction (zero-filled tail bytes). decode (88-137): length %4 gate (90-92), pad ≤ 2 (93-96), '=' confined to the trailing run (99-102), symbol-count/padding congruence `symbols.len() % 4 == 4 − pad` (103-105) — which provably excludes every malformed tail shape before the match; full groups decode 3 bytes each; the (3,1) and (2,2) tails reject non-canonical trailing bits via masks that are exact (the sub-mask bits below the symbol bits are zero from the `group_window` left-shift at 150). `symbol_value` (155-164) matches RFC 4648 §4 exactly. No unwrap/expect/index-out-of-bounds/shift-overflow anywhere in production lines 1-164 (chunk[0] at 48 is safe — `chunks(3)` never yields empty slices).
— Error offsets are absolute byte offsets in the input (base_offset = group start, + j within group) — pinned by the `Zm 9` → offset 2 test.
— Cold-path claim holds: all 5 production sites are per-connection (WS handshake) or per-session (token minting), never per-frame/per-message.

## 3. Findings

### F-001 [S4]
Where: crates/control-protocol/src/base64.rs:3-5
Code:
```
//! Two call sites, both cold-path: the `Sec-WebSocket-Accept` encoding
//! and the session-token rendering. Strict decode: exactly the standard
//! alphabet, mandatory canonical padding, no whitespace — a dirty-world
//! input is a typed [`Base64Error`], never a panic (D-3).
```
What is wrong: "Two call sites" is a feature-level statement sold as a site count. Actual production sites: five, across three files — `token.rs:48` (encode urandom token), `token.rs:93` (encode weak-fallback token), `http.rs:50` and `http.rs:54` (both inside `accept_value`), `http.rs:259` (decode `Sec-WebSocket-Key` in handshake validation) — plus two test sites (sha1.rs:192, token.rs:140). The two named features are right, and all sites are cold-path, but a maintainer reasoning "only two places touch this" underestimates the surface — notably the third *use* (handshake key decode at http.rs:259) which the doc does not name at all.
Why it matters: Doc-precision debt on a security-adjacent module (WS handshake + session tokens): the inventory of who calls it should be countable from the doc, and the handshake-key decode surface should be named because it is the one production DECODE (i.e. the one place dirty-world input enters).
What correct looks like: "Two uses, five production sites in three files (token minting; WS handshake accept computation and key validation), all cold-path; the only production decode is the `Sec-WebSocket-Key` check."
Confidence: HIGH — Verification: read + `rg "base64::"` enumerated every site.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-9 | REAL | S4 | F-001; every other claim verified (strict decode ✓, no-panic ✓, no-dep ✓, RFC vectors ✓ executed) |
| `enum Base64Error` | 12-20 | REAL | — | three-variant typed taxonomy; no raw input echoed beyond lengths/offsets (offset is a BYTE offset — fine for diagnostics, worth knowing on non-ASCII input) |
| `Display for Base64Error` | 22-34 | REAL | — | messages match each variant's actual trigger condition |
| `impl Error` | 36 | REAL | — | |
| `const ALPHABET` | 38 | REAL | — | RFC 4648 §4 standard alphabet, verified against spec |
| `encode` | 41-81 | REAL | — | capacity `div_ceil(3)*4` exact; `& 0x3f` after `>>18` is redundant-but-harmless (window ≤ 0xFFFFFF); zero tail-fill ⇒ canonical output by construction |
| `decode` | 88-137 | REAL | — | strict; every malformed shape routed to a typed error before any allocation-heavy work; canonical-tail masks exact (§2); `(0,_)`/`(3,1)`/`(2,2)`/`_` arm partition is exhaustive and the `_` arm is unreachable-by-construction defense-in-depth (the 103-105 congruence check excludes (1,_) and mismatched pairs) |
| `group_window` | 140-152 | REAL | — | short groups left-aligned to the top of the 24-bit window — the invariant that makes decode's canonical-bit masks exact; shift amount ∈ {0,6,12} always valid |
| `symbol_value` | 155-164 | REAL | — | branch ranges correct: A-Z 0-25, a-z 26-51, 0-9 52-61, + 62, / 63; everything else None |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `rfc4648_vectors` (171-189) | YES | external oracle — reordering ALPHABET, wrong tail padding, or a decode bug breaks at least one of the 7 vectors; EXECUTED in baseline (log:1415) |
| `roundtrip_all_byte_values` (191-201) | YES | all 256 byte values × every prefix length 0..254 — any off-by-one in chunking/tail breaks it; EXECUTED (log:1424) |
| `dirty_inputs_are_typed_errors` (203-219) | YES | removing the canonical-tail checks (121-123/129-131) or the padding-position checks (100-105) flips at least one assertion; EXECUTED (log:1414) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "RFC 4648 §4, standard alphabet" | module doc 1 | REAL | ALPHABET (38) + symbol_value match the RFC; vectors test executed in baseline |
| "Two call sites, both cold-path" | module doc 3-4 | SHALLOW (imprecise) | F-001: 5 production sites / 3 files; cold-path itself holds |
| "Strict decode… never a panic (D-3)" | module doc 4-6 | REAL | hand-traced (§2) + dirty-inputs test executed |
| "Implemented in-crate rather than pulling a `base64` crate (C5 / ADR-011 ethos)" | module doc 8-9 | REAL | no base64 dep in crates/control-protocol/Cargo.toml (comment at 19 confirms deliberate); C5 = dependency policy (docs/13-agent-wave-program.md:118); ADR-011 = the recorded-exception process doc |
| "Pinned by the RFC 4648 test vectors" | module doc 9 | REAL | test exists (171-189) and PASSED in baseline run #3 |

## 7. Dead / duplicated / unreachable code
None. Every production function has live callers (http.rs/token.rs; chain to apps/control-server in the header). `decode`'s only production caller is http.rs:259 — noted in F-001 as the un-named surface. No duplication: this is the single base64 in the workspace tree (control-protocol Cargo.toml deliberately has no base64 dep; lockfile base64 entries belong to unrelated external crates).

## 8. Dependencies & flags
std-only, no deps, no feature flags, no unsafe, no cfg gates. In-tree per C5 dependency policy (verified, §6).

## 9. Open questions for the next agent
- A04/A06 (frame.rs parts) and A07/A09 (server.rs parts): the WS framing path does NOT use base64 (only the handshake does) — confirm no per-frame base64 sneaks in via your files, which would falsify the cold-path claim.
- Whoever audits token.rs: the weak-fallback token path (token.rs:88-93) encodes a splitmix-derived value through this encoder — the entropy question is token.rs's, but the base64 layer treats it identically (verified here).
- apps/control-server's shipped status: baseline run #1 built the whole workspace (incl. control-server) but the audit's "shipped entry points" list only names `lumen`/`lumen-tauri` — if control-server ships, this file's reachability upgrades from UNREACHABLE to REAL-reachable; the call chain is already documented in the header.
