# AUDIT: crates/subtitles/src/dvb/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-21 |
| Lines / functions | 75 / 1 (`fuzz_sub_dvb`) + module doc |
| File verdict | REAL-UNPROVEN (driven green by in-crate tests; no libFuzzer target exists — documented honestly in-file; crate UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO (crate-level, see W09-A10 wave context). Sole in-crate caller: tests/dvb_vobsub_fuzz_survival.rs:33,214 (`fuzz_sub_dvb(payload)` inside `dvb_invariants`). Verified NO cargo-fuzz target: `ls fuzz/fuzz_targets/` → 35 files, only fuzz_sub_{srt,ass,vtt} among subtitle targets — no fuzz_sub_dvb (also none for pgs/cc/vobsub); fuzz/Cargo.toml [[bin]] list matches (no dvb row) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `ls fuzz/fuzz_targets/` + `rg -n "fuzz_sub" fuzz/Cargo.toml` (target absence); `rg -n "fuzz_sub_dvb" crates/subtitles/tests/` (sole caller); `sed -n 190,220p tests/dvb_vobsub_fuzz_survival.rs` (invariants that drive it); `rg -n "MAX_DAMAGE_EVENTS\|MAX_REGION_PIXELS\|DvbCueBuilder" src/dvb/mod.rs` (:77=64, :82=4 MiB, :859-876 builder API — doc bracket-refs all real); `sed -n 20,75p scripts/fuzz_registry_check.sh` (gate compares Cargo.toml↔files↔README, NOT doc 04 §8.5 — the doc-04 registry rows for missing formats are unenforced); baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269 (267 unit green), dvb_vobsub_fuzz_survival 2/0 (:5552ff area); baseline INDEX #18 (fuzz-registry PASS, 35 targets) |

## 1. One-paragraph verdict
A 34-line harness that does exactly what its unusually honest module doc says:
feed mutated/truncated pseudo-PES payloads through every `DvbDecoder` surface
(chunked on [0x0F,0x10] pairs, plus a truncation sweep, a reset + re-feed, and
a final empty-PES flush with a render pass), swallow all damage (the contract
is absence-of-panic/hang/unbounded-allocation, "nothing else"), and rely on
the AC suites for outcome pinning. The bounds claims check out: chunk Vec is
O(input), the decoder caps it references (`MAX_DAMAGE_EVENTS`=64,
`MAX_REGION_PIXELS`=4 MiB) exist, and the indexing (`data[start..i]` with the
`i > start` guard, `data[..cut]` with cut ≤ len) is panic-free by
construction. The wiring section is truthful: no cargo-fuzz target exists for
DVB and the doc says so. Two nits: the chunk-split comment says "every 0x0F
sync byte" while the code splits on the PAIR [0x0F, 0x10]; and the "One call =
one full … pass through every decode surface" line overstates a single call —
random bytes hit the multi-display-set path with probability ~2⁻¹⁶ per
position, so the "every surface" property lives in the committed test corpus
(`dvb_seed_stream` builds a grammatical multi-set stream), not in any one
call.

## 2. What actually works (verified)
- Baseline executed the driving tests green:
  audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt —
  tests/dvb_vobsub_fuzz_survival.rs "2 passed; 0 failed" (within the suite
  block after :5269's 267-unit lib run).
- The harness's chunking is panic-free by hand-trace: `windows(2)` bounds
  `i ≤ len-2`, so `start ≤ len-2` and the tail `data[start..]` is always ≥ 2
  bytes; the `i > start` guard prevents empty chunks; `cut` values
  (`len/3`, `len/2`, `len.saturating_sub(1)`) are all ≤ len.
- The invariants asserted around each call are real
  (tests/dvb_vobsub_fuzz_survival.rs:192-214: damage-ledger cap, no
  DamageOverflow-inside-cap, region RGBA stride = w·h·4) — they pin the
  DECODER, the harness adds the panic-freedom dimension.

## 3. Findings
### F-001 [S4]
Where: crates/subtitles/src/dvb/fuzz.rs:43-49
Code:
```
    // Split the payload into pseudo-PES chunks at every 0x0F sync
    // byte the mutator left us, so the page state machine is
    // exercised across display sets, not just one.
    let mut chunks: Vec<&[u8]> = Vec::new();
    let mut start = 0usize;
    for (i, w) in data.windows(2).enumerate() {
        if w == [0x0F, 0x10] && i > start {
```
What is wrong: The comment says the split fires "at every 0x0F sync byte";
the code requires the two-byte pattern `[0x0F, 0x10]` (sync byte followed by
page-id-high 0x10). An input whose segments use a different page-id high byte
is never chunked, and a reader tuning the heuristic will look for the wrong
thing. What correct looks like: "at every 0x0F-sync + 0x10 page-id-high pair"
or make the predicate actually the sync byte alone. Confidence: HIGH — read.

### F-002 [S4]
Where: crates/subtitles/src/dvb/fuzz.rs:4-13
Code:
```
//! One call = one full untrusted-PES pass through **every** decode
//! surface: the segment walk (sync/length/type), page composition
```
What is wrong: A single call exercises only what its bytes contain. Under a
libFuzzer-style mutation loop, the multi-display-set path (the whole point of
the chunking) is reached with probability ≈ 2⁻¹⁶ per byte position — random
corpora essentially never cross display sets; the committed test corpus does
(`dvb_seed_stream`, tests/dvb_vobsub_fuzz_survival.rs:184). The claim is true
of the corpus+harness pair, overstated for one call. Why it matters: someone
relying on the harness alone (the not-yet-existing libFuzzer target) would
believe state-machine cross-set behavior is covered by random exploration.
What correct looks like: "one call = one bounded pass over whatever surfaces
these bytes reach; the seed corpus carries the cross-set coverage".
Confidence: HIGH — probability reasoning is arithmetic, not evidence.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_sub_dvb` | 41-75 | REAL | S4 (F-001/F-002) | fresh decoder per pass; chunk split (pair-predicate); truncation sweep (3 cuts); reset + re-feed; empty-PES flush + `DvbCueBuilder::cues` render pass; all results `let _ =` per documented contract; allocations O(input) + decoder caps |
| module doc | 1-34 | REAL (honest) | — | wiring gap (no libFuzzer target) stated in-file; bracket-refs verified (`MAX_DAMAGE_EVENTS` dvb/mod.rs:77, `MAX_REGION_PIXELS` :82) |

## 5. Test honesty (tests live in tests/dvb_vobsub_fuzz_survival.rs — the only caller)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| dvb_fuzz_survival_mutations_and_truncations | PARTIAL | the invariants (cap/stride) would fail on decoder regressions; a no-op harness body would NOT fail anything — the harness is pinned only by not panicking, which is its documented contract |
| vobsub_fuzz_survival_mutations_and_truncations | (not this file's harness) | drives vobsub/fuzz.rs, another agent's file |
Both green in baseline (2 passed / 0 failed).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the harness logic is proven on stable by the deterministic survival runs in crates/subtitles/tests/" | dvb/fuzz.rs:25-28 | REAL | tests/dvb_vobsub_fuzz_survival.rs exists, 2/0 green in baseline |
| "The cargo-fuzz target file belongs at fuzz/fuzz_targets/fuzz_sub_dvb.rs exactly as doc 04 §8.5 registers it … Until the Lead wires it" | dvb/fuzz.rs:20-25 | REAL (honest gap) | target absent (ls); doc 04 §8.5 row :355 registers `fuzz_sub_dvb` with status "missing" — consistent |
| registry gate covers doc 04 §8.5 | implied by fuzz.rs:21 + baseline #18 | MISLEADING-infrastructure | scripts/fuzz_registry_check.sh:20-33 diffs Cargo.toml↔files↔README only; doc-04 rows for dvb/pgs/cc/vobsub/subviewer/… targets are NOT gate-enforced (S4 observation for the doc-04 owner, not this file) |
| "One call = one full … pass through every decode surface" | dvb/fuzz.rs:4-5 | OVERSTATED (F-002) | corpus-level property, not call-level |

## 7. Dead / duplicated / unreachable code
No dead code in-file (the single fn is called by the survival test).
Unreachable from shipped builds (crate-level). The missing libFuzzer shim is
documented, not dead code.

## 8. Dependencies & flags
Uses `crate::dvb::{DvbCueBuilder, DvbDecoder}` only. No external deps, no
unsafe, no feature gates, no cfg. The `let _ =` pattern is deliberate and
documented (not a swallowed-error defect: outcomes are pinned by the AC
suites, which is exactly what the module doc promises).

## 9. Open questions for the next agent
- dvb/mod.rs owner (part1/part2): does `decode_pes(&[], None)` (fuzz.rs:72,
  the flush idiom) clear/keep page state, and does it append damage on every
  empty call (which would burn the 64-event ledger before the render pass)?
- Whoever audits fuzz/README.md or doc 04 §8.5: the registry table lists
  subtitle targets (dvb/pgs/cc/vobsub/subviewer/microdvd/mpl2/sami/ttml) that
  the fuzz/ tree does not implement — the gate cannot see this by design;
  worth a doc-level finding.
