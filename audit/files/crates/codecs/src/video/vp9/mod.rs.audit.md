# AUDIT: crates/codecs/src/video/vp9/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / functions | 72 / 0 functions (module root: 2 module decls + 1 re-export + 1 doctest) |
| File verdict | REAL — every claim verified except one miscited provenance note; the documented subsystem executed 9 tests live here (5 unit + 4 differential) |
| Reachable from shipped entry point? | YES — `pub mod vp9` behind default-on `codec-vp9`; shipped dispatch arm apps/cli/src/pipeline_host.rs:313 `"vp9" =>`; producers exist: demux ivf `VP90`→`vp9` (demux/src/ivf, unit-tested tests.rs:73-74) and mkv `V_VP9`→`vp9` (demux/src/mkv/header.rs:805, tested :980) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/vp9_differential-4ec38b32b2dc3e15` -> 4 passed / 0 failed (exit 0, 30.20s); `./target/debug/deps/codecs-bae0f1756cc0a809 vp9` -> 5 passed / 0 failed (exit 0); rg (pin, record sections, error codes, demux mappings, matrix row); sed quote verification |

## 1. One-paragraph verdict
A documentation-first module root whose claims I could check almost
entirely, and they hold: the `rusty_vp9 = "=0.1.3"` pin is verbatim at
crates/codecs/Cargo.toml:145 behind `codec-vp9` (:81, default-on); the
evidence record exists and contains exactly what is cited — the six-lane
bit-exactness differential vs ffmpeg libvpx-vp9 (record :141, lane table
:165-172 incl. lane F profile-2 10-bit 50/50 identical, matching
mod.rs's "lane F measured 10-bit bit-exact") and the hostile-input
no-panic battery (§H); the delivery plumbing is real — IVF VP90→`vp9`
mapping unit-tested, mkv V_VP9→`vp9` mapping exists at header.rs:805
(mkv-direct lanes remain unexercised per the record addendum, and mod.rs
honestly scopes them to "the parallel container cards' surface, reconciled
by the Lead"); the v0 boundary is exactly as documented — `vp9.bitdepth`
(decoder.rs:187) and `vp9.layout` (:199) typed refusals, proven live by
`tenbit_lane_is_a_typed_refusal_not_a_misdecode`. I executed the
subsystem: 4/4 differential tests (30.20s, real decode + live ffmpeg
oracle + garbage/truncation sweeps) and 5/5 vp9 unit tests. The one defect
is a miscitation: "(record honest gap)" points at a threading-gap
statement the record does not contain.

## 2. What actually works (verified)
— verified by `./target/debug/deps/vp9_differential-4ec38b32b2dc3e15` ->
`differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok`,
`tenbit_lane_is_a_typed_refusal_not_a_misdecode ... ok`,
`hostile_garbage_is_recovered_and_the_stream_survives ... ok`,
`off_by_one_truncation_sweep_is_panic_free ... ok` (4 passed, 30.20s,
exit 0); `./target/debug/deps/codecs-bae0f1756cc0a809 vp9` -> 5 passed
incl. `wrong_descriptor_is_a_typed_refusal` (the doctest's exact claim,
proven by the unit test).

## 3. Findings

F-001 [S4]
Where: crates/codecs/src/video/vp9/mod.rs:43
Code:
```rust
//! refuses to reinterpret what it cannot name. The engine is
//! single-threaded (record honest gap).
```
What is wrong: the parenthetical attributes the single-threaded honesty
note to the admission record, but the record's own "## G — Honest gaps"
section (rusty_vp9-0.1.3.md:211-230) lists profile-3/4:4:4/SVC/mkv-direct
gaps, throughput, upstream conformance claims, and maintenance risk — NO
threading statement anywhere (full-text search for
thread/single/threaded/mt/parallel/rayon: zero relevant hits). The
single-threaded FACT is true and stated at the real site
(decoder.rs:290-291, which cites the record the same way and adds the
`vp9-mt` fork detail), but the record never recorded that gap, so the
"record honest gap" pointer is a dead citation for anyone following the
evidence trail.
Why it matters: the repo's evidence discipline rests on citations being
resolvable; this one resolves to a record that is silent on the claim.
What correct looks like: add the threading gap to record §G, or cite
decoder.rs's own statement instead of the record.
Confidence: HIGH — Verification: read (record §G + full-text search).

## 4. Per-function table (file <100 LOC, declarations only)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header doc block | 1-66 | REAL | S4 | every claim verified except F-001's citation |
| doctest (wrong-descriptor refusal) | 47-66 | REAL-UNPROVEN (executed) | - | could not run doctests here (cargo-build-budget); its exact assertion is unit-tested at decoder.rs:490-506 and passed live this session |
| `pub mod decoder;` | 68 | REAL | - | audited separately (vp9/decoder.rs.audit.md) |
| `mod error;` | 69 | REAL | - | `vp9.*` taxonomy verified (not_configured, bitdepth, layout, codec_descriptor) |
| `pub use decoder::Vp9Decoder;` | 71 | REAL | - | exists; constructed by shipped pipeline_host.rs:313 arm |

## 5. Test honesty
N/A in-file beyond the doctest. The referenced suites' honesty verified by
execution: 4/4 differential (live ffmpeg oracle, bit-exact MD5 gates,
panic sweeps) + 5/5 unit (incl. wrong-descriptor and bad-state refusals).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`rusty_vp9 = "=0.1.3"`, default features (an empty set) — prometheus-telemetry OFF" | :9-10 | REAL | codecs/Cargo.toml:145 (no default-features override needed — empty set); record title "rusty_vp9 0.1.3 (default features)" |
| "six-lane bit-exactness differential against ffmpeg libvpx-vp9 ... is docs/dependencies/records/rusty_vp9-0.1.3.md" | :12-14 | REAL | record :141/:165-172 (six lanes incl. 10-bit lane F 50/50) |
| "VP90 fourcc → descriptor vp9" | :28 | REAL | demux/src/ivf/tests.rs:73-74 asserts codec()==Some("vp9") |
| "Matroska/WebM V_VP9 tracks map to the same descriptor" | :30 | REAL | demux/src/mkv/header.rs:805 + test :980; mkv-direct lane unexercised (record addendum) — mod.rs scopes it honestly |
| "10/12-bit and 4:2:2/4:4:4 are typed Unsupported (vp9.bitdepth / vp9.layout)" | :36-38 | REAL | decoder.rs:187/:199; `tenbit_lane_is_a_typed_refusal_not_a_misdecode` passed live |
| "lane F measured 10-bit bit-exact at the engine level" | :40-41 | REAL | record :172 (50/50 identical, MD5 YES) |
| "The engine is single-threaded (record honest gap)" | :43 | HALF-TRUE | fact true (decoder.rs:290-291); record carries NO such gap — F-001 |

## 7. Dead / duplicated / unreachable code
None. Reachable end-to-end: ivf demux → descriptor → shipped dispatch arm
→ decoder (the vp9_differential suite drives exactly this class of path;
matrix row codec-video-vp9 :480 status windows=implemented, linux=partial
per the 2026-11-10 compat-matrix triage in its notes — a cross-file state
worth watching, not this file's defect).

## 8. Dependencies & flags
Behind default-on `codec-vp9` (codecs/Cargo.toml:81); engine exact-pinned,
empty default features. Doctest uses codec-api prelude + media-types ids.

## 9. Open questions for the next agent
- Baseline staleness observed: audit/runs/baseline/cargo-test-workspace.txt
  lists doctests as `video/mod.rs - video::av1 (line 76)` /
  `video::vp9 (line 87)`, but the current tree's video/mod.rs is 42 lines
  and the doctests live in av1/mod.rs:53 / vp9/mod.rs:45 — the baseline
  predates a refactor. Anyone citing the baseline for doctest inventory
  (or for per-file test counts around moved code) should re-check against
  the current tree. UNVERIFIED: current doctest execution (cargo-build-
  budget); behavior covered by unit tests as noted.
- When the mkv-direct lane lands (record §G follow-up), does the byte-
  for-byte passthrough claim (:28-29) hold for laced Matroska blocks?
