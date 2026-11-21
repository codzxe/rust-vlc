# AUDIT: crates/subtitles/src/cc/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-21 |
| Lines / functions | 99 / 2 (harness `fuzz_sub_cc` + `feed`; no in-file tests — the survival suite lives in tests/cc_acceptance.rs) |
| File verdict | REAL-UNPROVEN (harness logic real and deterministically executed; NEVER run under an actual fuzzer — the `fuzz_sub_cc` cargo-fuzz target does not exist; the "fuzz surface" is PAPER-ONLY) |
| Reachable from shipped entry point? | NO — crate unshipped; in-repo callers of `fuzz_sub_cc`: tests/cc_acceptance.rs:452,487,489 only (rg). fuzz/fuzz_targets/ contains 35 targets: `fuzz_sub_{srt,ass,vtt}` only — no CC/PGS/DVB/VobSub target. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `ls fuzz/fuzz_targets/` (35 files; no fuzz_sub_cc) · `rg -n "fuzz_sub_cc" --glob '!crates/subtitles/**'` (docs only: doc 04 §8.5 roadmap, ADR-012, sub-cc.md admission) · read tests/cc_acceptance.rs:440-495 (survival suite + cap asserts) · baseline: `orchrun-test-workspace-nofailfast.log.txt:5552-5568` (cc_acceptance 11/11 ok, incl. both survival tests) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
The harness body is competent: marker-based chunking with no panic paths (traced), a truncation sweep, every decode surface fed (GA94 SEI, MPEG-2 GA94, SCTE-20, raw synthesized triplets, reset-relock), and the honesty section admits the cargo-fuzz wiring is missing. But two claims do not survive contact with the code: (1) "the stateful cross-payload paths are covered by the multi-chunk splits" is false — `feed` constructs a fresh `CcDecoder` per chunk, so multi-chunk splits exercise zero cross-payload state, and the structurally-forced real-world case (a DTVCC packet spanning consecutive cc_data payloads — forced because cc_count ≤ 31 triplets but a max packet needs 64 continuations) has no deterministic coverage anywhere in the crate; (2) "the damage ledger is capped at MAX_DAMAGE_EVENTS" repeats the cap claim that the 708 path bypasses. And the file's nominal purpose — being the `fuzz_sub_cc` libFuzzer target registered by doc 04 §8.5 and ADR-012 — is unfulfilled: no target file exists, so no mutation-driven fuzzing of the CC stack has ever run; only a seeded LCG and two-seed truncation sweep inside a unit test.

## 2. What actually works (verified)
— `tests/cc_acceptance.rs` 11/11 ok in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5552-5568`, including `fuzz_lite_survival_seeded_mutations` (>100 KiB mutated budget, asserts `total > 100*1024`) and `systematic_truncation_sweep_survives` (every cut of 2 seeds) — both drive `fuzz_sub_cc`; absence-of-panic contract executed.
— Chunking panic-traced: `windows(3)` indices are in-bounds slice points; the `i > start` guard prevents zero-length splits at position 0; `data.len().saturating_sub(1)` handles the 0/1-byte inputs; empty payload → `feed(&[])` → all four parses fail/empty, `as_chunks::<3>()` yields nothing — no panic path exists in the file.
— Within-feed cross-call state exists by construction: one `CcDecoder` persists across the four grammar passes (lines 71-98), so an unterminated 708 packet from the GA94 pass can receive continuations from the raw-triplet pass.

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/cc/fuzz.rs:66-70 (vs 59-63)
Code:
```rust
/// One payload through every surface (fresh decoder state each
/// time — the stateful cross-payload paths are covered by the
/// multi-chunk splits and the survival suite).
fn feed(payload: &[u8]) {
    let mut decoder = CcDecoder::new();
```
```rust
    } else {
        for chunk in &chunks {
            feed(chunk);
        }
    }
```
What is wrong: The doc claims multi-chunk splits cover stateful cross-payload paths; the code gives every chunk a FRESH decoder (`CcDecoder::new()` inside `feed`), so multi-chunk inputs exercise zero cross-payload state — the opposite of the claim. The cross-call state that IS exercised (same decoder across the four grammar passes within one `feed`) is an unstated accident, pinned by no assertion.
Why it matters: The primary real-world 708 scenario is a DTVCC packet spanning consecutive cc_data payloads — structurally forced: `cc_count` is 5 bits (≤ 31 triplets per GA94 payload) while a maximum-size packet needs 1 start + 64 continuations. No test in the crate (cea708.rs unit tests use one reassembler directly; cc_acceptance decodes single payloads; this harness resets per chunk) feeds continuation triplets to a persistent `CcDecoder` across `decode()` calls. `CcDecoder.dtvcc` DOES persist across calls (mod.rs:214, `decode` never clears it) — that persistence is completely untested.
What correct looks like: `fuzz_sub_cc` keeps ONE decoder across chunk feeds (the chunks are consecutive payloads by construction), or a dedicated deterministic test: start triplet in payload A, continuations in payload B, assert the packet completes.
Confidence: HIGH — Verification: read of fuzz.rs + cc/mod.rs + cea708.rs tests + cc_acceptance.rs.

### F-002 [S3]
Where: crates/subtitles/src/cc/fuzz.rs:19-25 (vs fuzz/fuzz_targets/ listing)
Code:
```rust
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_cc.rs` exactly as doc 04 §8.5 would
//! register it, delegating here. The `fuzz/` tree is Lead-owned
//! (AGENTS.md §3.2, C5) and outside this card's file scope; until
//! the Lead wires it, the harness logic is proven on stable by the
//! deterministic survival runs in `crates/subtitles/tests/`
```
What is wrong: The registered fuzz surface does not exist: `fuzz/fuzz_targets/` holds 35 targets, `fuzz_sub_{srt,ass,vtt}` among them, but no `fuzz_sub_cc` (doc 04 §8.5 line 357 promises it; ADR-012:176 promises `fuzz_sub_pgs`/`fuzz_sub_dvb`/`fuzz_sub_vobsub`/`fuzz_sub_cc` — none of the four exist). The CC stack has therefore never been fuzzed by an actual mutation engine — coverage-guided or otherwise; the only executions are the seeded-LCG and 2-seed truncation sweep inside `cc_acceptance` unit tests.
Why it matters: The three unwired siblings (PGS/DVB/VobSub) show a systematic pattern: the bitmap-subtitle fuzz story is aspirational docs + deterministic unit tests. For CC specifically, the class of bugs a real fuzzer finds (state-machine desync across payloads, see F-001) is exactly the class the deterministic suite structurally cannot generate. The wiring-gap admission is honest, but honesty does not execute code.
What correct looks like: A 4-line shim at fuzz/fuzz_targets/fuzz_sub_cc.rs (the fuzz_sub_srt.rs pattern: `fn fuzz_target(data) { subtitles::cc::fuzz::fuzz_sub_cc(data) }`) + registry entry; until then the matrix/records should say "deterministic survival suite only", which sub-cc.md:14-17 does.
Confidence: HIGH — Verification: ls of fuzz/fuzz_targets + rg of docs; baseline #18 (fuzz_registry_check PASS, 35 targets) confirms the registry matches the 35 existing files — i.e., CC is absent by design, not drift.
### F-003 [S4]
Where: crates/subtitles/src/cc/fuzz.rs:29-31
Code:
```rust
//! One pass, output ≤ O(input) + the fixed 15×32 grids; the damage
//! ledger is capped at [`crate::cc::MAX_DAMAGE_EVENTS`] and the
//! DTVCC reassembly buffer at [`crate::cc::cea708::MAX_DTVCC_PACKET`].
```
What is wrong: Repeats the damage-cap claim that the 708 layer bypasses (cea708.rs audit F-003: `DtvccReassembler::push`/`parse_dtvcc_packet` push directly into the raw vec, not through `CcDecoder::note()`'s `MAX_DAMAGE_EVENTS` gate). The harness's own raw-triplet pass (line 94: `decoder.decode(&triplets, Some(2))` with `payload.len()/3` triplets) can generate an input-proportional damage vec that exceeds 256 for inputs > ~768 bytes — the ceiling protecting it is O(input), not the claimed 256.
Why it matters: The bounds section is the harness's safety argument; one of its two named caps is not enforced on the path the harness itself drives hardest. `tests/cc_acceptance.rs:456-464` asserts `u.damage.len() <= MAX_DAMAGE_EVENTS` — but only after GA94/SCTE-20 parses (≤ 31 triplets, structurally under the cap): the assertion is vacuous for the bypass path.
What correct looks like: Either enforce the cap on all paths (cea708 F-003 fix) or scope the doc: "≤ O(input) on the raw-triplet path".
Confidence: HIGH — Verification: read of both files; vacuity of the acceptance assert reasoned from cc_count's 5-bit domain.
### F-004 [S4]
Where: crates/subtitles/src/cc/fuzz.rs:52-58
Code:
```rust
    if chunks.len() == 1 {
        // Single chunk: feed whole plus a truncation sweep so the
        // typed truncation paths run.
        feed(chunks[0]);
        for cut in [data.len() / 3, data.len() / 2, data.len().saturating_sub(1)] {
            feed(&data[..cut]);
        }
```
What is wrong: The truncation sweep runs only for single-chunk inputs; multi-chunk inputs (any input containing a second `B5 00 31` marker) get exactly one feed per chunk and zero truncated feeds — the carriage grammars' typed-truncation paths are not exercised for marker-bearing inputs.
Why it matters: Real SEI blobs ARE marker-bearing (every GA94 payload starts with `B5 00 31`), so the class of inputs most likely to reach deep decode state gets the least truncation coverage in-harness. Mitigated by cc_acceptance's 2-seed systematic sweep.
What correct looks like: Also sweep `chunks.last()` (or one chunk) at 2-3 cut points in the multi-chunk branch.
Confidence: HIGH — Verification: read; branch condition traced.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_cc` | 39-64 | REAL-UNPROVEN | S3/S4 | Chunking panic-free (§2); fresh-decoder-per-chunk contradicts doc (F-001); truncation sweep single-chunk-only (F-004) |
| `feed` | 69-99 | REAL | S3 | Four grammar passes + raw triplets + reset-relock on ONE decoder (accidental cross-call state, unasserted); `cc_valid: t[0] & 0x04`, `cc_type: t[0] & 0x03` synthesis reaches decoder surfaces the grammars can't produce |
| module docs | 1-31 | — | S3/S4 | Honest wiring gap; two false bounds/coverage claims (F-001, F-003) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| cc_acceptance `fuzz_lite_survival_seeded_mutations` | YES for panics/hangs (its contract), NO for anything semantic | Mutates a growing buffer, calls fuzz_sub_cc, asserts budget spent + damage cap (vacuously for the bypass path, F-003) |
| cc_acceptance `systematic_truncation_sweep_survives` | YES for panics | Every cut of 2 seeds × all surfaces |
| both | NO for cross-payload reassembly bugs | F-001: the stateful path is never reached by design |
| cea708 unit tests (reassembly) | cover single-reassembler multi-triplet state only | `r.push(2..); r.push(3..); r.push(3..)` — one reassembler, one logical payload |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "registry name `fuzz_sub_cc`" (doc 04 §5) / ADR-012 lists it | fuzz.rs:1-2, doc 04 §8.5 (line 357), ADR-012:176 | PAPER-ONLY | No such target in fuzz/fuzz_targets (ls, 35 targets) |
| "the stateful cross-payload paths are covered by the multi-chunk splits" | fuzz.rs:67-68 | FALSE | F-001: fresh decoder per chunk |
| "the damage ledger is capped at MAX_DAMAGE_EVENTS" | fuzz.rs:30 | FALSE (708 path) | cea708.rs audit F-003 bypass |
| "output ≤ O(input) + the fixed 15×32 grids" | fuzz.rs:29 | REAL | Grids fixed-size; per-call allocations (chunks vec, triplets vec) are O(input) — holds |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:12-15 | REAL | Honesty up front; the suite asserts exactly that |
| sub-cc.md "no fuzz_sub_cc target exists in fuzz/ yet" | sub-cc.md:14-17 | REAL | Verified by ls |

## 7. Dead / duplicated / unreachable code
- The module's entire value proposition (libFuzzer harness) is dormant: executed only from two unit tests.
- No duplication; the marker constant `0xB5 0x00 0x31` mirrors cc_data.rs's grammar constants (acceptable coupling, same crate).

## 8. Dependencies & flags
- In-crate only (`crate::cc::*`); no features, no unsafe, no I/O; std-only. The `#[allow]`-free file is clippy-clean per baseline #5.

## 9. Open questions for the next agent
- Who owns the Lead-side wiring backlog: four promised fuzz targets (cc/pgs/dvb/vobsub per ADR-012:176) are missing — should the wave report carry this as one systemic item rather than four per-file notes?
- cc/mod.rs owner: should `CcDecoder::decode` reset `dtvcc` on stream discontinuity (pts jumps)? If not, F-001's coverage gap hides a potential stale-reassembly bug class across seeks.
