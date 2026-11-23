# AUDIT: crates/subtitles/tests/pgs_fuzz_survival.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 269 / 12 (10 helpers, 2 tests) |
| File verdict | REAL (executed: prebuilt test binary run by me, 2/2 pass) — with one dimensional defect in its cue-timing calls |
| Reachable from shipped entry point? | NO — test file; the crate itself is unlinked from all shipped apps (see parser.rs F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | target/debug/deps/pgs_fuzz_survival-b5033fc8d6f54700 (exit 0: `test result: ok. 2 passed`); rg PgsCueBuilder/cues signatures (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
A genuinely honest fuzz-lite survival suite: it declares in its header that there is NO
reference oracle (hand-crafted seeds against ffmpeg pgssubdec semantics), pins real bounded
invariants (damage cap with the +1 overflow marker — the design CC's ledger lacks), runs a
400-round seeded mutation loop with four mutation kinds plus an every-prefix truncation sweep
in both framings, and asserts the intact seed decodes clean and composes. I executed the
prebuilt binary: 2/2 pass. Two real gaps: the mutation loop's `cues` calls pass 90 kHz-tick
magic numbers into a nanosecond parameter, so the "next pts" it thinks it is passing (2 s) is
actually 180 µs — smaller than the set's own start, silently flipping the builder onto its
`end = None` branch and leaving the timed-end path untested; and no seed contains a
multi-fragment ODS (continuation) sequence, so the fragmented-object path — the way real PGS
streams ship large bitmaps — is never exercised by this corpus.

## 2. What actually works (verified)
— Ran the prebuilt test binary directly: `pgs_fuzz_survival-b5033fc8d6f54700` →
  `test seeded_mutation_survival_stays_typed_and_bounded ... ok`,
  `test truncation_sweep_never_panics_and_stays_typed ... ok`, `test result: ok. 2 passed`
  (exit 0, 0.02s). Baseline concurs (cargo-test-workspace.txt:6078-6084).
— Invariant math checked by hand: 8×4 object → rgba.len() == 8*4*4 == 128 ✓ (line 264);
  LCG = Knuth MMIX constants, deterministic.

## 3. Findings
F-001 [S3]
Where: crates/subtitles/tests/pgs_fuzz_survival.rs:233 and 265 vs crates/subtitles/src/pgs/mod.rs:302,601-611
Code: (verbatim, test)
```
        let set = dec.decode_display_set(&buf, Some(90_000));
        assert_invariants(&set);
        let builder = PgsCueBuilder::new();
        let _ = builder.cues(&set, Some(90_000 * 2), 0);
```
vs (verbatim, pgs/mod.rs)
```
    pub fn decode_display_set(&mut self, payload: &[u8], pts_90khz: Option<i64>) -> PgsDisplaySet {
```
and
```
    pub fn cues(
        &self,
        set: &PgsDisplaySet,
        next_pts_ns: Option<i64>,
        z_base: u16,
    ) -> Result<Vec<CompositedCue>, CueError> {
```
What is wrong: `decode_display_set` takes 90 kHz TICKS (`Some(90_000)` = 1 s → set.pts_ns =
1_000_000_000 ns, conversion at mod.rs:327-331); `cues` takes NANOSECONDS. The test passes
`Some(90_000 * 2)` = 180_000 ns as the "next" pts — 180 µs, i.e. BEFORE the set's own start,
so `next > start` (mod.rs:610) is false and the builder silently takes the `end = None`
branch. Line 265 repeats the confusion against a start of 0 (end = 89_999 ns = 90 µs, a span
the author plausibly meant to be 1 s). Net effect: the mutation loop never tests the timed-end
path of the cue builder, and no assertion exists to catch it (results discarded / only is_ok
checked). Spec 6.2 lists exactly this defect class: wrong units (ticks vs ns).
Confidence: HIGH — Verification: read + executed binary (timing branch not asserted anywhere).
F-002 [S4]
Where: crates/subtitles/tests/pgs_fuzz_survival.rs:82-92, 117-156
Code: (verbatim)
```
fn ods_first(id: u16, width: u16, height: u16, rle: &[u8]) -> Vec<u8> {
```
What is wrong: the only ODS builder in the corpus is `ods_first`; no seed contains a
continuation fragment (sequence flag 0x40), so multi-fragment objects — how every real PGS
stream ships bitmaps larger than one segment — are unreachable by both the mutation loop and
the truncation sweep (truncating a first-fragment only produces typed truncation, never the
reassemble-across-fragments state machine). The pgs fuzz harness doc explicitly lists
"ODS (first/continuation fragments, length accounting)" as a covered surface
(pgs/fuzz.rs:7-8) — this corpus never drives it. Confidence: HIGH — Verification: read (rg
'ods_first' is the sole ODS constructor in the file).
F-003 [S4]
Where: crates/subtitles/tests/pgs_fuzz_survival.rs:233
Code: (verbatim)
```
        let _ = builder.cues(&set, Some(90_000 * 2), 0);
```
What is wrong: the cue-render result of every mutated round is discarded without even counting
Err outcomes — the header invariant "the cue render never errors on a well-formed display set"
is only pinned for the intact seed (line 266 `assert!(cues.is_ok())`). For mutated sets that is
fine per the survival contract, but the discarded Result means a regression making `cues`
return Err on EVERY input (well-formed included) would still pass test 1. Confidence: MEDIUM —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg::next_u32 / below | 26-40 | REAL | — | Knuth MMIX; n==0 guarded |
| seg | 43-48 | REAL | — | kind+len(BE)+data |
| pcs | 50-72 | REAL | — | PCS grammar incl. object list |
| pds | 74-80 | REAL | — | 5-byte palette entries |
| ods_first | 82-92 | REAL | F-002 | first-fragment only, 24-bit length |
| to_sup_framing | 95-113 | REAL | — | matches decoder's Sup framing (PTS+type+len); NOTE: real BD .sup carries PTS+DTS — decoder framing question for the pgs auditor, test is self-consistent |
| seed_display_set | 117-156 | REAL | — | PCS+WDS+PDS+2×ODS+END, mixed RLE codes |
| seed_two_display_sets | 161-170 | REAL | — | adds a Normal clear set |
| assert_invariants | 172-185 | REAL | — | cap+1 marker; rgba stride check |
| seeded_mutation_survival_stays_typed_and_bounded | 188-235 | REAL | F-001, F-003 | 400 rounds, 4 mutation kinds, 200 KB budget |
| truncation_sweep_never_panics_and_stays_typed | 238-269 | REAL | F-001 | every prefix × both framings; intact-seed clean + 2 objects + cues ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seeded_mutation_survival_stays_typed_and_bounded | YES (panic/cap/stride regressions) | real assertions per round — but blind to cue-timing regressions (F-001/F-003) |
| truncation_sweep_never_panics_and_stays_typed | YES | per-prefix invariants + clean intact seed + composition count |
The suite does NOT overclaim: header admits no oracle (lines 5-10). The only dishonesty risk
is implicit: passing in-dialect seeds cannot validate against real .sup captures.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO reference-encoder differential oracle; survival contract is absence-of-panic + bounded invariants" | lines 5-10 | REAL | read: suite matches exactly this |
| "damage ledger never exceeds MAX_DAMAGE_EVENTS + 1 (the overflow marker)" | lines 13-14 | REAL | assert_invariants:172-177 + executed |
| "every composed image's rgba.len() == w * h * 4" | lines 15-16 | REAL | assert_invariants:178-184 + executed |
| "the cue render never errors on a well-formed display set" | lines 16-17 | SHALLOW | pinned only for the intact seed (F-003) |
| (pgs/fuzz.rs) "cargo-fuzz target file belongs at fuzz/fuzz_targets/fuzz_sub_pgs.rs … Until the Lead wires it" | pgs/fuzz.rs:17-24 | REAL (admitted gap) | ls fuzz/fuzz_targets/: no fuzz_sub_pgs.rs — honestly labeled, unlike srt/ass/vtt which have targets |

## 7. Dead / duplicated / unreachable code
None in-file. The suite is the ONLY execution vehicle for `fuzz_sub_pgs` (no cargo-fuzz target
exists — see §6).

## 8. Dependencies & flags
subtitles::pgs only; std-only; no features; no unsafe. Prebuilt binary present in
target/debug/deps (no build needed for re-runs).

## 9. Open questions for the next agent
- pgs/mod.rs auditor: `cues` uses `Some(next - 1)` on a half-open span (mod.rs:609-611) — the
  same 1 ns dead-zone defect I executed in cc/mod.rs F-002; this suite cannot catch it (F-001).
- pgs/segment.rs auditor: is `PgsFraming::Sup` (4-byte PTS, no DTS) the real standalone .sup
  layout (PTS+DTS per BD spec/ffmpeg sup demuxer)? If not, both the framing and this test's
  `to_sup_framing` encode a non-existent format.
