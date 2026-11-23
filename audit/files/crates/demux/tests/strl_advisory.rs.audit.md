# AUDIT: crates/demux/tests/strl_advisory.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 364 / 13 (4 #[test]], 1 builder family, drivers) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test; runs in the default lane (baseline cargo-test-workspace.txt:2399-2405: all 4 tests ok, none ignored). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | baseline citation; sed comparison of builder vs idx1_stream_scope.rs (duplicate); rg for strn-title coverage in in-crate tests. |

## 1. One-paragraph verdict
A real regression suite for the W4-R1 vprp fix: the exact ffmpeg strl shape
(strh+strf+JUNK(4120)+vprp(68)) must walk with zero damage, the full advisory
set (strd/strn/JUNK/vprp) must be silently skipped, an unknown child must be
damage-counted exactly once with its byte size and the damage-record-first
ordering, and vprp must not mask unknown neighbors. All asserts are exact
(reasons vectors, ledger totals, byte-exact six-frame rows). The only defect
is the duplicated AVI builder shared (copy-pasted) with idx1_stream_scope.rs.
One claim is unverifiable: the VPRP_FFMPEG bytes purport to be copied from
corpus-big's 0021-avi-av.avi, which is regenerated, not committed.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:2402-2405: all four tests "ok" in the
  default lane.
- read: the STRN advisory test (278-297) exercises the header.rs STRN branch
  (my header.rs audit F-004's cap is NOT hit here — 28-byte name); the
  strn-title-surfacing behavior itself is pinned in-crate at
  avi/tests.rs:415 (baseline ok).

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/strl_advisory.rs:36-96 vs idx1_stream_scope.rs:39-118
Code:
```
fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
```
What is wrong: the builder family (chunk/end_sized/avih/strh/strf) is a
verbatim near-copy of the idx1_stream_scope.rs builder — two independently
drifting copies of the RIFF byte rules; tests/common already exists as the
shared home. (Same finding as idx1_stream_scope F-001; filed against both
since neither owns the shared version.)
Confidence: HIGH — Verification: read + sed diff.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| chunk / end_sized / avih / strh_video / strf_video | 36-96 | REAL | S4 | F-001 duplication; `as u32` truncation same as sibling file |
| frame_payload | 100-102 | REAL | — | varying per-frame digests |
| VPRP_FFMPEG / junk_ffmpeg | 104-132 | REAL | — | measured-shape constants (see doc claims) |
| build_avi_with_strl_children | 140-190 | REAL | — | movi-relative idx1, patched sizes |
| demuxer_over_bytes / walk / canonical_rows | 196-245 | REAL | — | fail-loud drivers |
| clean_ffmpeg_strl_shape_walks_with_zero_damage | 255-271 | REAL | — | the fix's positive pin |
| full_advisory_set_is_silently_skipped | 277-297 | REAL | — | strd/strn/JUNK/vprp |
| unknown_strl_child_is_damage_counted_not_fatal | 309-340 | REAL | — | 37-byte odd payload (pad path), damage-first |
| vprp_does_not_mask_unknown_neighbors | 345-364 | REAL | — | exact single note |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all four | YES | exact reason/ledger/row asserts; regression of the vprp skip or the catch-all fails loudly |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg authors strh(56)+strf(40)+JUNK(4120)+vprp(68) on every clean file — measured 6/6 corpus-big" | strl_advisory.rs:5-7 | REAL-UNPROVEN | corpus regenerated, not committed; sample count 6/6 not re-executable |
| VPRP_FFMPEG "copied from corpus-big's 0021-avi-av.avi — our own generated corpus" | strl_advisory.rs:104-108 | REAL-UNPROVEN | source file not in tree; bytes are non-degenerate (real-looking field values), so the stand-in-coincidence defense holds, but provenance is unverifiable |
| pre-fix "recovered=1/damage=68 on 57 clean rows" | strl_advisory.rs:8-12 | REAL-UNPROVEN | historical worklog claim; pre-fix behavior not re-run |

## 7. Dead / duplicated / unreachable code
F-001 duplication only. Nothing dead.

## 8. Dependencies & flags
Integration test using demux::avi public API + test_support::mem_stream. No
feature gates, no external binaries.

## 9. Open questions for the next agent
- The strn advisory test uses a 28-byte name; the MAX_STRN_BYTES=4096 cap
  path (header.rs F-004) has no test in either suite — an over-cap strn
  walks silently truncated today.
