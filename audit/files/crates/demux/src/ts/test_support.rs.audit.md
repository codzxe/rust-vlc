# AUDIT: crates/demux/src/ts/test_support.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-21 |
| Lines / functions | 243 / 12 |
| File verdict | REAL (test-support surface, correct for every value the suite uses; builder invariants unasserted) |
| Reachable from shipped entry point? | COMPILED-IN BUT NOT EXECUTED: `ts/mod.rs:75 pub mod test_support;` is un-gated, so it ships in the release binary; every caller is `#[cfg(test)]` or a fuzz harness — `crates/demux/src/ts/tests.rs`, `crates/demux/src/ts/fuzz.rs:38`, `crates/demux/tests/ts_fuzz_survival.rs:11`, `crates/demux/src/es/tests.rs`, `apps/cli/src/containers/ts.rs:685` (inside the CLI's `#[cfg(test)]` module). No runtime call chain exists. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -ln "ts::test_support" crates apps` (6 consumers, all test/fuzz-gated — verified by opening each call site); `rg -n "mod test_support" crates/demux/src/{avi,wav,ps,mp4,ts}/mod.rs` (same un-gated pattern repo-wide); `rg -n "ts_fuzz_survival" audit/runs/baseline/test-workspace.log.txt` (3/3 ok, lines 2245-2252); `sed -n` verification of every quote. |

## 1. One-paragraph verdict
This is honest test infrastructure: byte-exact builders for PSI sections, PAT/PMT/SDT bodies, TS packets (with PCR and damage flags), and a canonical single-program stream, with correct ISO/IEC 13818-1 reserved-bit patterns (0xB0/0xC0/0xE000/0xF000) and correct 33+6+9 PCR packing. The streams it builds underpin the unit suite (28/28 green in baseline) and the mutation fuzz suite (3/3 green, determinism pinned). Three hygiene defects: the long-form section builder omits the 12-bit ceiling the short form enforces (silently corrupting the header for oversized bodies), `ts_packet_tuned` silently truncates over-long payloads and out-of-range PCR bases instead of asserting, and the whole module ships un-gated in release binaries per a documented repo-wide pattern — dead weight of ~250 lines per container family.

## 2. What actually works (verified)
— Grammar correctness, read against ISO/IEC 13818-1: long-form section header `0xB0 | len>>8` (syntax=1, reserved 11 — test_support.rs:76), version byte `0xC0 | (version<<1 & 0x3E) | current` (79), PAT/PMT reserved-bit patterns `0xE000`/`0xF000` (94, 104-110), SDT 5-byte service-loop header per EN 300 468 §5.2.3 (126-135).
— PCR packing (168-173): base bits 32..1 in four bytes, `(base&1)<<7 | (ext>>8)&1 | 0x7E` reserved-six, `ext&0xFF` — round-tripped by `ts::tests::packet_fields_parse` (baseline log:1936 `packet_fields_parse ... ok`... name verified green via per-name rg, see tests.rs audit).
— Consumers ran green in baseline: `ts::tests::*` 28/28 (test-workspace.log.txt:1928-1945) and `ts_fuzz_survival` 3/3 (2245-2252) — the builders produce deterministic streams stable under a mutation sweep (`mutations_never_panic_and_stay_deterministic ... ok`).
— `psi_packets` continuity counters increment across sections and continuation packets (206-207) — matches what the demuxer's CC tracking expects (no spurious jumps in the green tests).

## 3. Findings
### F-001 [S4]
Where: crates/demux/src/ts/test_support.rs:62-76
Code:
```
    if !syntax {
        let sl = body.len().min(MAX_SECTION_BYTES - 3);
        let mut out = Vec::with_capacity(3 + sl);
        out.push(table_id);
        out.push(0x70 | (sl >> 8) as u8);
        out.push((sl & 0xFF) as u8);
        out.extend_from_slice(&body[..sl]);
        return out;
    }
    // Long form: section_length counts everything after its own
    // field — 5 fixed bytes + body + the 4-byte CRC.
    let section_length = 9 + body.len();
```
What is wrong: Asymmetric clamping. The short form clamps to `MAX_SECTION_BYTES - 3` (4095); the long form computes `section_length = 9 + body.len()` with no ceiling. For a body > 4086 bytes the 12-bit `section_length` field overflows: `0xB0 | (section_length >> 8) as u8` (line 76) drops the overflow bits and ORs them over the reserved-bit pattern (e.g. body 5000 → sl 5009 = 0x1391 → `0xB0|0x13` = 0xB3), silently minting a corrupt header the parser will reject as truncation/bad-structure for a reason the test author never intended.
Why it matters: a future test that legitimately wants a max-size section gets a confusing typed failure (or worse, a section of the wrong length that still parses if the truncated length happens consistent) instead of a builder refusal. Test-support bugs waste debugging time and can mask the code under test.
What correct looks like: `assert!(9 + body.len() <= MAX_SECTION_BYTES)` or clamp + debug_assert like the short form.
Confidence: HIGH — Verification: read (MAX_SECTION_BYTES = 3 + 4095, psi.rs:53).

### F-002 [S4]
Where: crates/demux/src/ts/test_support.rs:166-190
Code:
```
        // 33-bit base, 6 reserved bits, 9-bit extension.
        af.push((base >> 25) as u8);
        af.push((base >> 17) as u8);
        af.push((base >> 9) as u8);
        af.push((base >> 1) as u8);
        af.push(((((base & 1) as u16) << 7) | ((ext >> 8) & 1) | 0x7E) as u8);
        af.push((ext & 0xFF) as u8);
```
and
```
    let take = payload.len().min(188 - at);
    out[at..at + take].copy_from_slice(&payload[..take]);
```
What is wrong: Two silent corruptions: (a) a `base` ≥ 2^33 (the parameter is `u64`; the field is 33 bits) has its high bits dropped by the `as u8` casts — the packet carries a different PCR than the caller asked for; (b) a `payload` longer than the packet's remaining capacity is silently truncated (`take = min`), so the caller's bytes beyond 188−at simply vanish. Both violate the builder contract without a peep.
Why it matters: tests built on corrupted packets still pass against a parser that happens to agree — a test author cannot tell their input was mangled. Every current call site stays within bounds (verified by reading the callers in ts/tests.rs and containers/ts.rs tests), so today it is latent.
What correct looks like: `assert!(base < (1u64 << 33) && ext < 512)` and `assert!(payload.len() <= 188 - at)` (or pad-and-warn) — builders should fail loudly on contract violations.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/demux/src/ts/mod.rs:75 (governing this file) + test_support.rs:3-7
Code:
```
//! Public like [`crate::avi::test_support`] on purpose: rustdoc
//! examples and the integration/fuzz suites need deterministic
//! transport-stream shapes (including deliberately malformed ones).
//! Test-support surface, not player surface — nothing in the demuxer
//! uses it.
```
What is wrong: `pub mod test_support` is not `#[cfg(test)]`/`#[cfg(any(test, feature="…"))]`-gated, so these ~240 lines of test-only builders are compiled into every shipped `lumen` binary (pattern repeated in avi, wav, ps, mp4 — rg-verified). The doc comment honestly declares the intent (cross-crate test suites need it) — the cost is production-binary dead weight and a widened public API surface for the demux crate.
Why it matters: shipped binaries carry never-executed parsing code (attack/review surface, binary size); the API can never be tightened without breaking test crates. A `testkit`/dev-feature gate would keep cross-crate tests and drop the payload.
What correct looks like: `#[cfg(feature = "test-support")] pub mod test_support;` with the feature enabled in dev-dependencies of the consuming crates.
Confidence: HIGH — Verification: read + rg (all 6 consumers test/fuzz-gated; no runtime caller).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 20-24 | REAL | — | thin delegate to avi::test_support::mem_stream |
| psi_section | 33-35 | REAL | — | current=true wrapper |
| psi_section_tuned | 40-48 | REAL | — | syntax=true wrapper |
| psi_section_full | 54-86 | REAL | S4 | F-001: short form clamped, long form unclamped; CRC via crc32_mpeg (self-referential for tests — see ts/tests.rs F-001) |
| pat_body | 90-97 | REAL | — | 0xE000 reserved bits correct; NIT-pointer entry (0, pid) expressible |
| pmt_body | 102-114 | REAL | — | PCR/PID/es_info lengths with correct masks; program_info > 4095 silently masked (same class as F-001, lower reach) |
| sdt_body | 122-139 | REAL | — | 5-byte service loop + 0x48 descriptor, lengths consistent |
| ts_packet | 148-150 | REAL | — | tuned(tei=false, disc=false) |
| ts_packet_tuned | 155-192 | REAL | S4 | F-002: silent PCR >33-bit and payload truncation; AF/PCR/TEI/PUSI bit packing verified correct |
| psi_packets | 198-211 | REAL | — | one section per PUSI packet — cannot build multi-section payloads (cross-ref ts/tests.rs F-002, mod.rs:498-500 documented gap) |
| stream_of | 215-221 | REAL | — | concatenation |
| single_program_ts | 226-243 | REAL | — | SDT→PAT→PMT→3 PES packets; CC held; underpins 12 of the 28 unit tests + the fuzz suite (all green in baseline) |

## 5. Test honesty
Not applicable directly (no tests in this file), but its output is only as honest as its invariants: per F-002 a future caller with an over-long payload gets a silently truncated packet, and per ts/tests.rs F-001 the CRC lane built here is self-referential (builder and parser share `crc32_mpeg`). Mutation check: corrupt `ts_packet_tuned`'s PID packing (`((pid >> 8) & 0x1F)`) and `ts::tests::packet_fields_parse` fails (pid 0x1ABC asserted) — the builders do exercise real parser behavior.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Test-support surface, not player surface — nothing in the demuxer uses it" | test_support.rs:6-7 | REAL | rg: all 6 consumers are test/fuzz-gated; zero runtime callers |
| Builders needed by "integration/fuzz suites" | test_support.rs:3-5 | REAL | ts_fuzz_survival.rs:11, containers/ts.rs:685 consume them; both green in baseline (log:2245-2252; CLI suite log rows) |
| "card 5.I1a" unit-support scope | file header :1 | REAL | the 28 ts::tests all consume these builders, all green (log:1928-1945) |

## 7. Dead / duplicated / unreachable code
- The whole module is dead code at runtime (by design, F-003) — compiled in, never called by the player.
- `mem_stream` duplicates the avi::test_support::mem_stream re-export (one-line delegate; acceptable seam).
- No other duplication found.

## 8. Dependencies & flags
- No feature gates (F-003 is precisely about that). No `unsafe`, no external crates. Depends on `plugin_api::SourceStream`, `ts::psi` (crc32_mpeg, MAX_SECTION_BYTES).

## 9. Open questions for the next agent
- ts/mod.rs owner: the SectionAssembler's multi-section-payload gap (mod.rs:498-500) means `psi_packets`' one-section-per-packet shape is the ONLY shape this repo can build or parse — real broadcast streams violate it daily (see ts/tests.rs F-002).
- avi/wav/ps/mp4 test_support modules share the un-gated pattern (F-003) — a single workspace-wide dev-feature would retire the whole class; product-config owners should weigh in.
