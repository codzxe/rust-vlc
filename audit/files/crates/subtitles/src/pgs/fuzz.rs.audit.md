# AUDIT: crates/subtitles/src/pgs/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-21 |
| Lines / functions | 77 / 1 (`fuzz_sub_pgs`; no in-file test module) |
| File verdict | REAL (in-crate survival harness, baseline-executed green) / UNREACHABLE from shipped build; registered cargo-fuzz target name is a registry ghost |
| Reachable from shipped entry point? | NO (crate census, cross-ref layout.rs F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `ls fuzz/fuzz_targets/` (35 targets; no fuzz_sub_pgs); `rg "fuzz_sub_pgs"` repo-wide; `rg "MAX_DAMAGE_EVENTS|MAX_OBJECT_PIXELS" pgs/{mod,rle}.rs`; read of tests/pgs_fuzz_survival.rs (full); baseline: orchrun-test-workspace-nofailfast.log.txt:5608-5614 (pgs_fuzz_survival 2/0 ok) + :5269-5540 (lib 267/0) |

## 1. One-paragraph verdict
The harness is real, bounds-safe by construction (verified line-by-line: the END-marker scan cannot over-index, slices are always in-range, the decoder's 4 MiB canvas cap and 64-event damage cap exist and are asserted by its integration suite), and it is the rare file in this wave with actual baseline execution proof: `tests/pgs_fuzz_survival.rs` drives it through 400 seeded-mutation rounds and full truncation sweeps in both framings, green in `orchrun-test-workspace-nofailfast.log.txt:5608-5614`. But its registered name promises more than the tree delivers: `fuzz/fuzz_targets/` holds 35 targets with no `fuzz_sub_pgs.rs` — the "survival harness" the matrix evidence cites is the in-crate integration suite wearing the cargo-fuzz target's name, and the whole crate still ships nowhere.

## 2. What actually works (verified)
— Bounds-safety of the chunker by read: `i + 3 <= data.len()` (:47) guards `data[i..i+2]`; `i > start` (:48) makes `data[start..i+3]` non-degenerate and always valid; the tail push `data[start.min(data.len())..]` (:55) cannot panic (start ≤ len proven by the loop invariant). Empty input → `[b""]` → single-chunk path → `decode_display_set(&[], Some(0))` + zero-length truncation sweep — total.
— The three-phase exercise (whole → truncation sweep :60-63; multi-chunk with increasing PTS :65-67; reset + re-feed + final flush + render :70-76) genuinely crosses display sets and the epoch state machine, matching the module doc's claimed coverage (:4-10).
— Execution: baseline `orchrun-test-workspace-nofailfast.log.txt` — subtitles lib **267 passed; 0 failed** (:5269-5540) and `tests/pgs_fuzz_survival.rs` **2 passed; 0 failed** (:5608-5614: `truncation_sweep_never_panics_and_stays_typed`, `seeded_mutation_survival_stays_typed_and_bounded` — 400 rounds, <200 KB budget, damage-ledger cap and rgba-stride asserted per pass, tests/pgs_fuzz_survival.rs:172-185, :224-233).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/pgs/fuzz.rs:39-77
Code:
```
pub fn fuzz_sub_pgs(data: &[u8]) {
    let mut decoder = PgsDecoder::new();
    // Split the payload into pseudo display sets at every END
    // segment the mutator left us, so the epoch state machine is
```
What is wrong: Crate-level unreachability (cross-ref layout.rs F-001: only reverse-dependency of `subtitles` is fuzz/Cargo.toml:50; no shipped binary links the crate). The harness, its decoder, and the survival evidence all describe a component no user can execute.
Why it matters: Matrix row subtitle-pgs claims "implemented ×3 desktop platforms" on the strength of this test surface; nothing shipped reaches it.
What correct looks like: TS/MP4 PGS-track wiring in the demux/pipeline path, or UNREACHABLE status.
Confidence: HIGH — Verification: rg census + baseline engine-info features.

### F-002 [S3]
Where: crates/subtitles/src/pgs/fuzz.rs:18-23
Code:
```
//! The cargo-fuzz target file belongs at
//! `fuzz/fuzz_targets/fuzz_sub_pgs.rs` exactly as doc 04 §8.5
//! registers it, delegating here. Building it needs the
//! `libfuzzer-sys` nightly scaffolding — a Lead-owned dependency
```
What is wrong: The registry slot is empty: `fuzz/fuzz_targets/` holds 35 targets — `fuzz_sub_{srt,ass,vtt}` among them — but no `fuzz_sub_pgs.rs` (nor the `fuzz_sub_dvb`/`fuzz_sub_vobsub`/`fuzz_sub_cc` ADR-012:176 promises; W09-A05 found the same for CC). The doc is honest that wiring is pending, but the names in the evidence chain overstate: docs/feature-matrix.toml:992 cites "fuzz_sub_pgs survival harness, all green" — the green thing is the in-crate integration suite (`tests/pgs_fuzz_survival.rs`), not the named target; no coverage-guided engine has ever run this code.
Why it matters: Evidence-string drift: a reader assumes a registered fuzz target exists and runs; in reality the "target" is a function the integration tests call. Same class as the fuzz-registry violations the baseline's own gate (#18) would catch if the registry listed it.
What correct looks like: Either the shim file (20 lines, srt/ass/vtt precedent) or evidence strings that say "in-crate survival suite".
Confidence: HIGH — Verification: ls + rg census; A05's cc/fuzz.rs audit corroborates the pattern.

### F-003 [S4]
Where: crates/subtitles/src/pgs/fuzz.rs:36-39 (whole file has no test module)
Code:
```
/// Run one fuzz pass over `data` (see the module docs). Never
/// panics, never hangs, never allocates without a ceiling — by
/// construction of the decoder's bounds-checked paths.
pub fn fuzz_sub_pgs(data: &[u8]) {
```
What is wrong: `ass::fuzz` and `vtt::fuzz` each carry in-file survival tests (`empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive` — baseline :5272, :5516-5517); this file has none — its first execution is the integration suite. The chunk-splitting logic specifically (the `0x80 00 00` scan) has no direct unit pinning; it is exercised only via mutated seeds that rarely hit its edge cases deterministically.
Why it matters: Inconsistent test discipline across the crate's four fuzz harnesses; the chunker's degenerate branches (END at offset 0, END at the very tail, empty input) are untested by name.
What correct looks like: A 4-case in-file test mirroring ass/fuzz.rs's, feeding `[]`, `[0x80,0,0]`, END-at-0, END-at-tail.
Confidence: HIGH — Verification: read + census of in-file test modules.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_pgs` | 39-77 | REAL | S2/S4 | bounds-safe (§2); baseline-executed green via tests/pgs_fuzz_survival.rs:228,250 (orchrun log :5608-5614); unreachable from shipped builds (F-001); no in-file tests (F-003); registry name ghost (F-002) |

## 5. Test honesty
No in-file tests. `tests/pgs_fuzz_survival.rs` (both tests) would fail on any decoder panic, ledger overflow past `MAX_DAMAGE_EVENTS + 1`, or rgba-stride violation — genuine assertions, not tautologies; executed green in baseline (:5608-5614). The harness's OWN chunker has no named tests (F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "32 unit tests + fuzz_sub_pgs survival harness, all green in this tree 2026-10-26" | docs/feature-matrix.toml:992 | REAL (tests) / SHALLOW (name) | 32 pgs unit tests green in orchrun log :5436-5467 (4 palette + 8 rle + 9 segment + 11 mod, per W09-A05 count); survival suite green :5608-5614; but "fuzz_sub_pgs" the cargo-fuzz target does not exist (F-002) |
| "fuzz_target = `fuzz_sub_pgs` (in-crate survival harness…)" | docs/formats/sub-pgs.md:16 | REAL (as phrased) | the "in-crate" qualifier is accurate — the harness function exists and runs |
| "One pass, output ≤ O(input) + the object canvas cap… damage ledger capped" | module doc :28-32 | REAL | rle.rs:32 `MAX_OBJECT_PIXELS = 4 MiB`, mod.rs:77 `MAX_DAMAGE_EVENTS = 64`; caps asserted per pass by the integration suite |
| "One call = one full untrusted-payload pass through every decode surface" | module doc :4-10 | REAL | trace: decode_display_set covers framing/segments/PCS/WDS/PDS/ODS/epoch; builder.cues covers render (:74-76) |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped builds (F-001). No internal dead code (single function, called by the integration suite).

## 8. Dependencies & flags
- `crate::pgs` only. No features, no unsafe, no panics (§2 bounds analysis), allocations bounded by chunk count + decoder caps.

## 9. Open questions for the next agent
- A05/A06 (pgs/mod.rs parts, dvb): the `decode_display_set(chunk, Some(90_000 * (n + 1)))` PTS ladder assumes 1 s per display set — if real PGS PTS spacing matters to epoch flush semantics, confirm the harness's PTS choice exercises the flush paths (my read: epoch state depends on PCS state byte, not PTS, so it does).
