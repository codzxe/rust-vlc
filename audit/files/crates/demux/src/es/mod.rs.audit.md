# AUDIT: crates/demux/src/es/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 50 / 0 (module root: 7 pub submodules + cfg(test) tests; all logic lives in the children) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (3 of 4 lanes): lumen-cli `apps/cli/src/containers/mod.rs:63-66` routes `hevc`/`h264` → `es::open_annexb` (annexb.rs) and `mp2`/`mp3` → `es::open_mpeg_audio` (mp3.rs). The `aac` lane is NOT on the shipped path — see F-001. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=1 'es::'` -> 13 es:: tests + ts::pes matches, all ok, exit 0; rg sweeps: error codes, unwrap/panic in es sources (none), CLI routing, es::aac consumers; sed quote verification |

## 1. One-paragraph verdict
A module root whose every doc claim checks out against the children: the
four lanes exist (h264/hevc are thin 29/30-line wrappers over the 539-line
shared annexb engine; mp3 is a real 690-line lane; aac is a pure re-export of
the adts demuxer, exactly as its own doc says), the three advertised typed
error codes exist in es/error.rs (:28, :36, :50) and are pinned by passing
tests (`h264_no_sync_is_typed_unsupported_after_scan`,
`mp3_truncated_final_frame_is_typed_sticky`, `next_event_before_start_is_typed`),
the house-rules claim (no unwrap/expect/panic in non-test code) holds over a
sweep of all six non-test child files, and 13 es:: tests pass live. The one
finding: the `aac` lane has no shipped caller — the CLI routes `.aac`
content through the `adts` container id, so es::aac is reachable only from a
corpus test that skips when the corpus is absent; the module doc's
"registry surface" framing does not say that.

## 2. What actually works (verified)
— verified by live run `es::` filter -> 13 tests ok (fuzz survival x2,
aac-lane identity, typed no-sync/truncation/not-started, AU-boundary rules
for both codecs, mp3 exact sample clock + resync). Error codes confirmed at
es/error.rs:28/36/50. House-rules sweep: rg over annexb.rs, mp3.rs, error.rs,
aac.rs, h264.rs, hevc.rs for unwrap/expect/panic/todo/unreachable -> zero
matches. mp3-twin rationale documented at mp3.rs:12-13 (layer ordering).
Baseline corroborates (cargo-test-workspace.txt:1891 es::fuzz ok).

## 3. Findings
F-001 [S4]
Where: crates/demux/src/es/mod.rs:17-18
Code:
```
//! - [`aac`] — raw AAC (`.aac`): the landed [`crate::adts`]
//!   demuxer, consumed as-is (this module is the registry surface).
```
What is wrong: the aac "lane" has no shipped caller. The CLI's container
dispatch (apps/cli/src/containers/mod.rs:52) routes format id `adts` to
containers/adts.rs (which constructs the adts demuxer directly); the probe
never emits an `aac` format id; and the only consumer of
`demux::es::aac::open` is tests/es_corpus.rs:235 — a test that early-returns
(SKIP) when /tmp/my-project/corpus-v4 is absent, i.e. in this environment it
never runs either. The re-export itself is honest (aac.rs documents itself as
the registry surface), but the module doc lists four lanes as peers without
marking that one of them is shipped-dead. Why it matters: a reader auditing
"raw ES file access" coverage would count four reachable lanes; there are
three. What correct looks like: a one-word qualifier ("registry surface —
the CLI routes .aac via the adts container") or routing `aac` through it.
Confidence: HIGH — Verification: read + rg of routing + CLI dispatch table.

## 4. Submodule inventory (file < 100 LOC; per-function table not required)
| Submodule | Lines | Verdict | Notes |
|---|---|---|---|
| aac.rs | 34 | REAL (UNREACHABLE from CLI) | pure re-export of adts; consumed by skipping corpus test only (F-001) |
| annexb.rs | 539 | REAL | shared H264/HEVC engine (not assigned to me) |
| error.rs | 78 | REAL | the three advertised codes exist (:28/:36/:50) |
| fuzz.rs | 130 | REAL | survival tests pass live |
| h264.rs | 29 | REAL | wrapper over annexb; routed by CLI |
| hevc.rs | 30 | REAL | wrapper over annexb; routed by CLI |
| mp3.rs | 690 | REAL | routed by CLI; documented table twin of codecs-side mp3 |
| tests.rs | 365 | REAL | 11 tests pass live |

## 5. Test honesty
No tests in mod.rs itself; the child suite (13 tests) is honest — typed
error pins with codes, AU-boundary rules, exact sample clock, resync
recovery. (es_corpus.rs shares the vacuous-skip pattern flagged for
mkv_editions_tags.rs.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| four lanes (h264/hevc/mp3/aac) | mod.rs:7-18 | REAL (aac shipped-dead — F-001) | submodules read; CLI routing verified |
| "demux.es.no-sync" / "demux.es.truncated" / "demux.es.not-started" typed paths | mod.rs:31-39 | REAL | error.rs:28/36/50 + passing pinned tests |
| "no unwrap/expect/panic! in non-test code" | mod.rs:22-23 | REAL | rg sweep of the six non-test files: zero matches |
| "audio lanes use exact sample arithmetic, never a guessed bit rate" | mod.rs:28-29 | REAL | `mp3_walks_frames_with_exact_sample_clock` passes live |
| mp3 tables are "a thin local twin ... layer ordering forbids sharing the crate" | mod.rs:13-16 | REAL | mp3.rs:12-13,124-125 documents the same rationale |

## 7. Dead / duplicated / unreachable code
- es::aac (F-001) — shipped-unreachable, test-only consumer (and that test
  skips without the corpus).
- The mp3 table duplication is documented debt with a stated architectural
  reason — acceptable, noted.

## 8. Dependencies & flags
None new (children use media_types/plugin_api per crate rules). No unsafe,
no features.

## 9. Open questions for the next agent
- es/annexb.rs (539 loc) and es/mp3.rs (690 loc) are the real engines behind
  this root — when assigned, audit the AU-boundary NAL rules and the
  ISO-table walk against the corpus lanes; this audit verified only the
  module-root claims and the passing test names.
