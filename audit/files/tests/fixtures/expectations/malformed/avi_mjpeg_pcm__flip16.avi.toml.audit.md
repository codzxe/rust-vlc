# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__flip16.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A08) |
| Date | 2026-11-24 |
| Lines / functions | 31 lines (schema/fixture/sha/derivation/error keys) |
| File verdict | REAL (pin verified by execution-grade reproduction) |
| Reachable from shipped entry point? | As evidence: consumed by media-probe fixtures_matrix.rs + demux fixtures_class_b.rs (both green in baseline); playback-engine soak runner; NOT a shipped artifact |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256sum + stat (pin match); python3 bit-diff vs parent; rg consumers; baseline citations |

## 1. One-paragraph verdict
The pin is exact and the derivation record is truthful: sha256 2e08b021…753823b and size 425,516
reproduced byte-for-byte against the fixture on disk; the parent diff is exactly 16 single-bit
flips, all offsets (2,7,10,15,17,18,29,30,33,47,52,58,59,60,62,63) inside the declared 64-byte
RIFF header extent — matching `[derivation] flip_bits = 16, flip_extent = 64`. Consumers exist
and ran green in the orchestrator baseline. The one real defect is the `[error]` block's union
contract: a specific outcome (typed probe.unsupported refusal) is known and recorded elsewhere,
but the TOML only pins the vague union.

## 2. What actually works (verified)
— verified by EXECUTION-grade reproduction:
`sha256sum tests/fixtures/generated/malformed/avi_mjpeg_pcm__flip16.avi` →
`2e08b0219bcc8025e29a87e973a803b631c8a98e46cdfee6e66b1122d753823b` (== pin); `stat -c%s` →
`425516` (== pin); python3 diff → 16 bits / 16 bytes / extent ≤ 63 (== derivation).
Consumer runs: fixtures_matrix.rs reads the RECORDED TOMLs ("not a transcription", :246) —
baseline cargo-test-workspace.txt:3670-3680 `…avi_class_b_derivatives_are_classed_per_their_nature
… ok` (8 tests); demux fixtures_class_b.rs — :2242+ `flip16_broken_magic_is_a_typed_parse_error
… ok`; codecs fuzz_survival.rs:23 lists this fixture — :1115+ 6/6 ok. CORPUS.md's table row
carries the same sha/size.

## 3. Findings
F-001 [S3]
Where: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__flip16.avi.toml:24-26
Code:
[error]
# v0: the union contract — the specific flavor (typed-error |
# bounded-recovery | rejected) is pinned by demuxer conformance
# (cards 2.B2/2.B3).
expected = "typed-error-or-bounded-recovery"
What is wrong: the expectation permits THREE different outcomes ("typed-error |
bounded-recovery | rejected") — a test asserting this file can pass while the fixture's actual
behavior drifts between flavors. The specific truth IS known and pinned elsewhere:
tests/integration/first-vertical/tests/recovery_matrix.rs:21 records "avi flip16 | typed open
refusal: `probe.unsupported` with the sniffed bytes named". Why it matters: union contracts
weaken regression detection (a bounded-recovery → typed-error flip passes silently); the TOML
is the machine-readable contract and it is vaguer than the repo's own knowledge.
What correct looks like: pin `expected = "typed-error"` (with the stage/code), leaving the
union only for genuinely-unstable cases.
Confidence: HIGH — Verification: read (TOML + recovery_matrix.rs:21).
Note: recovery_matrix itself is `#[ignore]`d tier-2 (LUMEN_CORPUS-gated, :1330) — not run in
the standard baseline; the union contract is thus also rarely exercised.

## 4. Per-function table
N/A (data/evidence file): keys schema/fixture/class/parent/sha256/size/license/derivation/error
— all verified except the error union (F-001).

## 5. Test honesty
The consuming suites read the RECORDED TOML (fixtures_matrix.rs:246 comment) — hashes and
derivation fields are cross-checked, not transcribed: a fixture/TOML mismatch fails loudly.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seeded distinct bit positions within the header extent" | :27 | REAL | 16 distinct single-bit offsets ≤ 63 |
| CORPUS.md row (class B, flip16, size, sha) | CORPUS.md | REAL | identical sha/size reproduced |

## 7. Dead / duplicated / unreachable code
None. (recovery_matrix tier-2 consumer is ignored-by-design, documented at :1319.)

## 8. Dependencies & flags
None (TOML). Env gate for tier-2: LUMEN_CORPUS.

## 9. Open questions for the next agent
When the union→specific migration happens (v1 expectation schema?), flip16's sibling TOMLs
(flip1/flip4/garbage0/trunc*) share the defect — migrate as a set.
