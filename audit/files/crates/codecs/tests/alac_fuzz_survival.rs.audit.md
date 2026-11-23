# AUDIT: crates/codecs/tests/alac_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 204 / 6 helpers + 3 tests |
| File verdict | REAL — strong mutation-survival suite with determinism and bite asserts; ran live |
| Reachable from shipped entry point? | N/A (integration test); exercises SHIPPED `codecs::audio::alac` (CLI "alac" row, pipeline_host.rs:293) via the free `decode_frame` surface, on packets extracted through the real Mp4Demuxer + FileAccess |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/alac_fuzz_survival-7fdac73a463c231d --nocapture` → 3 passed; 0 failed, exit 0, 0.82s; ls crates/codecs/tests/fixtures/alac/ (5 committed .m4a present) |

## 1. One-paragraph verdict
A well-built fuzz-lite suite: 512 LCG mutation rounds (bit-flips biased
into element headers, truncations, wild-byte bursts, cross-packet splices)
over REAL packets re-extracted at run time from committed .m4a fixtures
through the actual MP4 demuxer — nothing synthetic. Beyond
absence-of-panic it asserts the error taxonomy (only
Corrupt/Unsupported/Decoder kinds), per-input DETERMINISM (same bytes →
same verdict, :150), and that the mutations actually bite (both Ok and
typed outcomes must occur, :155-158) — the two asserts that most
fuzz-survival suites in this repo lack. Cookie mutations and edge shapes
are covered separately. All 3 tests ran live. One nit: the cookie-mutation
test drops the error-kind check the packet test enforces.

## 2. What actually works (verified)
— verified by `./target/debug/deps/alac_fuzz_survival-7fdac73a463c231d
--nocapture` → `test result: ok. 3 passed; 0 failed` (exit 0, 0.82s),
with fixtures confirmed present (5 committed .m4a, loud `unreachable!`
on any extraction failure — no skip path).

## 3. Findings

F-001 [S4]
Where: crates/codecs/tests/alac_fuzz_survival.rs:182-185
Code:
```rust
        match codecs::audio::alac::decode_frame(packet, &bytes) {
            Ok(_) => {}
            Err(_) => rejected += 1,
        }
```
What is wrong: the cookie-mutation loop counts rejections but discards
the error entirely — no kind check. The packet-mutation test enforces the
taxonomy (`try_decode` asserts Corrupt/Unsupported/Decoder at :84-93); an
unmapped error kind (or a panic-adjacent misuse surfaced as some other
kind) escaping through the COOKIE path would pass this test unnoticed.
What correct looks like: route through the same `try_decode` (or repeat
its kind assert).
Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path / open_via_input | 19-30 | REAL | - | real FileAccess open |
| corpus_bytes | 33-58 | REAL | - | real Mp4Demuxer walk; loud fail |
| Lcg | 62-75 | REAL | - | Knuth constants, deterministic |
| try_decode | 80-97 | REAL | - | taxonomy-enforcing wrapper |
| mutated_packets_never_panic_or_hang | 99-159 | REAL | - | 512 rounds; determinism + bite asserts — the model other suites should copy |
| mutated_cookies_are_typed_not_panics | 161-188 | REAL | S4 | F-001: kind check dropped |
| empty_and_edge_packets_are_typed | 190-204 | REAL | - | empty/0x00/0xFF/0x80/0x7F shapes; clean decode at end |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_packets_never_panic_or_hang | YES | panic/hang fails the process; unmapped kind fails :84-93; nondeterminism fails :150; vacuous run fails :155-158 |
| mutated_cookies_are_typed_not_panics | PARTLY | panic/hang and never-rejecting fail; an unmapped error KIND passes (F-001) |
| empty_and_edge_packets_are_typed | YES | exact is_err/is_ok assertions |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seeded-LCG mutations of REAL decoded-corpus bytes ... nothing synthetic" | :1-8 | REAL | corpus_bytes uses the real demuxer on committed fixtures; ran live |
| "no panic, no hang, decode is Ok-or-typed" | :4-5 | REAL | 3/3 ran live, 0.82s |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
demux + input + plugin-api + media-types + testkit; committed fixtures
under crates/codecs/tests/fixtures/alac/. No feature gates.
## 9. Open questions for the next agent
- alac decoder itself (not assigned): does `decode_frame`'s error surface
  really emit only the three asserted kinds (the test would catch drift on
  the packet path — F-001 leaves the cookie path unguarded)?
