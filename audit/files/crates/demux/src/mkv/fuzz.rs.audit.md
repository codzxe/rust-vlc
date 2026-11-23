# AUDIT: crates/demux/src/mkv/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2025-06-27 |
| Lines / functions | 84 / 1 harness + 1 const + 2 tests |
| File verdict | REAL (thin: 52 fixed stable inputs; no stable mutation loop) |
| Reachable from shipped entry point? | NO (by design — fuzz surface): wired to cargo-fuzz target fuzz/fuzz_targets/fuzz_demux_mkv.rs:19 (registry-verified); the demuxer it exercises IS shipped (apps/cli containers/mod.rs:57 `"matroska" => mkv::open`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p demux --lib mkv::fuzz` (exit 0, 2 passed); `bash scripts/fuzz_registry_check.sh` (exit 0, "35 targets, diff empty, corpora present"); `sed -n` quote verification; ls of fuzz/ tree + malformed fixtures |

## 1. One-paragraph verdict
The harness is real, correctly budgeted (at most 200_000 `next_event` calls
per pass, check-before-increment), registered in the fuzz target table, and
its two survival tests pass. But "proven on stable by the deterministic
sweeps" (fuzz.rs:16-17) is inflation: the sweeps are 52 fixed inputs — empty,
a 4-byte EBML marker, 2 x 4096-byte constant walls, and 48 short
deterministic patterns of length <= 48 — with zero mutation over real seeds,
while the AVI zone ships a full seeded xorshift mutation loop (6 flavors over
real fixtures, tests/fuzz_loop_stable.rs) and the TS zone ships LCG mutations
(tests/ts_fuzz_survival.rs). The harness never probes seek (the MP4 twin does
3 data-derived probes per pass), and the "no unbounded allocation" contract
leg is unobservable — the harness adds no accounting and rests on demuxer
ceilings that have the per-`Tags`-element budget holes found in my tags.rs
audit. The campaign-wiring docs contradict each other (README "NOT WIRED"
vs audit.yml's nightly campaign tier).

## 2. What actually works (verified)
— verified by `cargo test -p demux --lib mkv::fuzz` -> 2 passed (empty,
marker, 0xFF wall, 0x00 wall, 48 patterns). — `bash scripts/
fuzz_registry_check.sh` -> exit 0: "fuzz-registry: CLEAN — 35 targets, diff
empty, corpora present" (the mkv target's shim really calls this harness;
seed corpus fuzz/corpus/fuzz_demux_mkv has 3 seeds: seed_mkv_flip1.mkv,
seed_mkv_h264_aac.mkv, seed_webm_vp9_opus.webm). — Stable-side survival is
additionally pinned by the checked-in malformed derivatives (18 files:
trunc0-4/flip1/4/16/garbage0 x mkv+webm, tests/mkv_fixtures.rs:345-405,
baseline cargo-test-workspace.txt:2358 ok; files verified present in
tests/fixtures/generated/malformed/). — Budget loop is correct: check
`drained >= FUZZ_EVENT_BUDGET` precedes increment+call, so exactly
<= 200_000 `next_event` calls per pass; `Err(_) | EndOfStream => break`.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/mkv/fuzz.rs:12-19 and 67-83
Code:
```
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_mkv.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's integration corpora. Seek probing rides the
//! mkv integration suites (cues-indexed); this pass covers the full
//! header/cluster/block walk, which is where untrusted bytes land.
```
and the entirety of the "sweeps":
```
    #[test]
    fn empty_and_marker_inputs_survive() {
        fuzz_demux_mkv(&[]);
        fuzz_demux_mkv(b"\x1A\x45\xDF\xA3");
    }

    #[test]
    fn hostile_wide_inputs_survive() {
        // All-0xFF (EBML maximal varint false positives) and
        // all-0x00 (zero-length ID/size fields).
        fuzz_demux_mkv(&vec![0xff; 4096]);
        fuzz_demux_mkv(&vec![0x00; 4096]);
        for length in 1..=48usize {
            let data: Vec<u8> = (0..length).map(|i| (i * 53 % 256) as u8).collect();
            fuzz_demux_mkv(&data);
        }
    }
```
What is wrong: "proven on stable" overstates 52 fixed inputs, none longer
than 4096 bytes and none a mutation of a real container. The AVI zone's
stable equivalence runs N seeded xorshift64* inputs in 6 flavors including
real-fixture bit-flips, truncations and head+tail splices
(tests/fuzz_loop_stable.rs); the TS zone runs LCG mutations over a synthetic
multi-program stream plus real corpus bytes (tests/ts_fuzz_survival.rs).
The MKV zone has no runtime-mutation sibling — only pre-generated fixed
derivatives. Additionally the harness never calls `seek` (MkvDemuxer::seek
exists, mod.rs:1003): the delegated "seek probing" suites seek only VALID
corpus files (mkv_corpus.rs:228 `corpus_matroska_seek_roundtrip`) and
hand-built chapter fixtures — never a malformed derivative — while the MP4
twin harness does 3 data-derived seek probes per pass (mp4/fuzz.rs,
FUZZ_SEEK_PROBES = 3). A seek-path panic on corrupted cues is invisible to
every runnable test today. What correct looks like: an mkv
fuzz_loop_stable twin (the generator is documented as "a near-verbatim
twin" pattern) + seek probes in the harness.
Confidence: HIGH — Verification: read + ran (all named tests/files checked).

F-002 [S3]
Where: crates/demux/src/mkv/fuzz.rs:14-15 vs fuzz/README.md "Status" block
vs .github/workflows/audit.yml:325-351
Code (fuzz.rs):
```
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_demux_mkv.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
```
(fuzz/README.md): "## Status: SCAFFOLDED, NOT WIRED — one Lead decision
outstanding" ... "UNTIL: the decision lands, nothing in the local gate or CI
builds this directory *in campaign mode*."
(audit.yml):
```
  fuzz:
    if: github.event.schedule == '0 2 * * *' || (github.event_name == 'workflow_dispatch' && inputs.tier == 'fuzz')
```
which installs nightly + cargo-fuzz and runs
`bash scripts/quality/25_fuzz_campaign.sh 180 "${MINE[@]}"` over 5 shards.
What is wrong: the docs contradict the CI. fuzz/README says no CI builds the
campaign; audit.yml schedules a nightly 35-target campaign. The campaign
script's own header agrees with neither ("wires none of them into a gate").
No campaign artifacts are committed (fuzz/artifacts does not exist), so
whether a campaign ever ran is UNVERIFIED (CI history not in the sandbox).
Why it matters: whether the mkv fuzz target has ever executed under
coverage guidance is unknowable from the repo, and the wiring-status docs
give three different answers. What correct looks like: one authoritative
status line, updated when audit.yml landed.
Confidence: HIGH (contradiction verified) — campaign execution UNVERIFIED.

F-003 [S4]
Where: crates/demux/src/mkv/fuzz.rs:21-25 and 35-37
Code:
```
//! # Bounds (MP §12)
//!
//! Event budget: hard cap below; the walk's positions are monotone.
//! Payload memory: the demuxer's own ceilings (`MAX_*` in the module)
//! apply — this harness adds none.
```
and
```
/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates without a ceiling — by construction
/// of the demuxer and the budget above.
```
What is wrong: the harness has no allocation accounting, so the
"never allocates without a ceiling" leg is asserted, not tested; it rests
entirely on demuxer ceilings — which have the per-`Tags`-element budget
holes documented in my tags.rs audit (value budget excludes
name/language/ietf bytes; budgets reset per `Tags` master while
`collect_tags` merges unboundedly). A crafted many-`Tags` file retains
file-scale memory and no stable test can see it (the 52 inputs are tiny;
libFuzzer's rss_limit would only catch it in the never-verified campaign).
Confidence: HIGH — Verification: read (harness body) + tags.rs audit
findings F-001/F-002.

F-004 [S4]
Where: crates/demux/src/mkv/fuzz.rs:39
Code:
```
    let Ok(stream) = crate::avi::test_support::mem_stream(data.to_vec()) else {
```
What is wrong: the MKV harness reaches into `avi::test_support` for its
memory stream — cross-family test-support reuse (the mp4 twin uses
`super::test_support::mem_stream`). Works (avi test_support is compiled
into the crate), but it is a layering smell that would break silently if
the AVI module ever tightens its test_support visibility.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file <100 LOC — table included anyway)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FUZZ_EVENT_BUDGET (const) | 31-33 | REAL | - | 200_000; budget check correct |
| fuzz_demux_mkv | 38-61 | REAL | S3/S4 | budgeted drain; no seek probes (F-001); no alloc accounting (F-003); avi test_support reuse (F-004) |
| tests::empty_and_marker_inputs_survive | 67-71 | REAL | - | survival-only, 2 inputs |
| tests::hostile_wide_inputs_survive | 73-83 | REAL | - | survival-only, 50 inputs |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | only for panics on those 2 inputs | survival contract; no assertion beyond not-panicking |
| hostile_wide_inputs_survive | only for panics/hangs on those 50 inputs | same; a hang would surface as a test timeout, a panic as a failure — but 52 inputs prove survival for 52 inputs, not "never panics" |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| cargo-fuzz target is fuzz/fuzz_targets/fuzz_demux_mkv.rs | fuzz.rs:14-15 | REAL | file exists, calls this harness (line 19); registry gate CLEAN (ran) |
| "Lead-owned nightly wiring" (pending) | fuzz.rs:15 + README "NOT WIRED" | FALSE vs audit.yml | nightly campaign tier exists (.github/workflows/audit.yml:325-351) — F-002 |
| "harness logic is meanwhile proven on stable by the deterministic sweeps" | fuzz.rs:16-17 | SHALLOW | 52 fixed inputs, no mutation loop (F-001); 18 fixed derivatives + corpus add real but static coverage |
| "Never panics, never hangs, never allocates without a ceiling" | fuzz.rs:35-37 | REAL-UNPROVEN | budget verified by read; alloc leg unobservable (F-003); campaign execution UNVERIFIED |
| "Seek probing rides the mkv integration suites" | fuzz.rs:17-18 | SHALLOW | those suites seek valid files/fixtures only, never malformed (F-001) |

## 7. Dead / duplicated / unreachable code
None in this file. (Dead-code context: the fuzz campaign targets exist but
their execution is unproven — F-002.)

## 8. Dependencies & flags
`plugin_api` (Demuxer trait) + `super::MkvDemuxer` + `crate::avi::
test_support::mem_stream`. The cargo-fuzz workspace (fuzz/Cargo.toml) pins
libfuzzer-sys 0.4.13 — separate workspace, not built by the release lane.
No feature gates on this module.

## 9. Open questions for the next agent
- Did the nightly audit.yml fuzz tier ever run (CI history)? If yes, commit
  campaign artifacts; if no, the README/audit.yml contradiction (F-002)
  should be resolved and the "SCAFFOLDED, NOT WIRED" banner updated.
- Would `corpus_matroska_seek_roundtrip` survive a trunc-mutated corpus
  file? Nobody seeks over malformed bytes today (F-001) — a directed
  seek-over-derivative test is cheap to add.
