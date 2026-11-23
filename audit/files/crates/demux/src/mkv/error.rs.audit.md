# AUDIT: crates/demux/src/mkv/error.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-23 |
| Lines / functions | 284 / 14 constructors + `mint` (+ 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — constructors are called from the MKV demuxer chain wired at apps/cli/src/containers/mod.rs:57 `"matroska" => mkv::open(stream, opts)`; verified call sites: header.rs (`not_matroska` :280, `unsupported_ebml_limits` ×3), mod.rs (`unknown_doctype` :345, `no_tracks` :438, `no_clusters` :435, `scan_too_slow` :867, `not_started` ×3, `timecode_overflow` :818, `truncated`/`bad_structure` multiple), cluster.rs (`timecode_overflow` ×4, `bad_structure` ×6), cues.rs (`cue_table_too_large` ×3), editions.rs/attachments.rs (`bad_structure`/`truncated`) — every one of the 14 constructors has ≥1 live call site (rg count per name) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b mkv::error --test-threads=2` → 3 passed / 0 failed, exit 0; `sed -n` quote verifications; `rg` caller census |

## 1. One-paragraph verdict
A genuine typed-error constructor module: 13 of 14 constructors mint stage-`Demux` errors with stable `demux.mkv.*` codes and correct taxonomy kinds, every constructor is reachable through the CLI-wired MKV demuxer, and the 3 tests assert stage/code/Display shape and kind mapping for the full constructor set (I ran them: 3/3 pass). Two hygiene defects: `timecode_overflow` accepts an offset parameter it silently discards (callers pass real block offsets that never reach the error), and the module header's "every constructor … `demux.mkv.*`" claim is contradicted by `not_started()`'s `demux.not-started` code. Nothing here is fake, dead, or unreachable. Notably, the sniffed-bytes and doctype payloads are bounded at the caller side (4-byte head read in header.rs:278-280; DocType capped at `MAX_TEXT_FIELD` = 4 KiB in header.rs:56-60/314-321), so the error path cannot be turned into an unbounded allocation.

## 2. What actually works (verified)
- `./target/debug/deps/demux-5e0c7194d741938b mkv::error --test-threads=2` → `test result: ok. 3 passed; 0 failed` — `errors_carry_demux_stage_and_stable_codes` (all 14 codes + `[demux] ` prefix + `(code)` suffix), `kinds_map_to_the_documented_taxonomy` (exact ErrorKind matches incl. `CorruptDetail::Truncated{expected,found}` and `ResourceLimit{Memory}`), `not_matroska_sniff_capped_at_sixteen_bytes_by_display` (64 sniffed bytes → ≤16 in Display).
- Caller census (rg): all 14 constructors used; no orphan constructors.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/mkv/error.rs:120
Code:
```
pub fn timecode_overflow(_offset: u64) -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Timing,
        },
        "demux.mkv.timecode-overflow",
    )
}
```
What is wrong: the `_offset` parameter is accepted and discarded — `CorruptDetail::Timing` has no offset slot, so the block position callers compute and pass (cluster.rs:431 `error::timecode_overflow(offset)`, :433, mod.rs:818 `error::timecode_overflow(block.payload_offset)`) never reaches the error. The signature promises positional context the error cannot carry; every other `bad_*` constructor in this file embeds its offset. Why it matters: error triage of a corrupted timeline loses the "where" that sibling errors provide, and the API invites callers to pass information that is dropped. What correct looks like: drop the parameter, or thread the offset into the context (`ErrorContext` supports structured detail — as `bad_vint`/`bad_structure` do via `InvalidStructure{offset}`). Confidence: HIGH — Verification: read + rg callers.

F-002 [S4]
Where: crates/demux/src/mkv/error.rs:10-11 vs :174
Code:
```
//! Every constructor mints stage [`PipelineStage::Demux`] with a stable
//! `demux.mkv.*` context code.
```
vs
```
pub fn not_started() -> EngineError {
    mint(ErrorKind::Unsupported { format: None }, "demux.not-started")
}
```
What is wrong: the module-header claim is false for 1 of the 14 constructors — `not_started` mints `demux.not-started`, not `demux.mkv.*`. The deviation is deliberate and locally documented at :169-172 ("mirrored here so the mkv zone stays self-contained" — it intentionally aliases `crate::error::not_started`'s code so the AVI vertical and MKV zone mint identical not-started errors), but the blanket module claim still overstates. Same pattern exists in ps/error.rs:103-104. Blast radius: doc precision only. Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 27-29 | REAL | — | shared minter, stage const |
| not_matroska | 35-45 | REAL | — | sniffed bytes bounded by 4-byte head read (header.rs:278) |
| unknown_doctype | 51-61 | REAL | — | doctype bounded ≤4 KiB by MAX_TEXT_FIELD (header.rs:56-60, :314-321) |
| unsupported_ebml_limits | 66-71 | REAL | — | 3 call sites in header.rs |
| truncated | 77-84 | REAL | — | CorruptDetail::Truncated{expected,found} |
| bad_vint | 90-99 | REAL | — | offset embedded |
| bad_structure | 105-114 | REAL | — | offset embedded; heaviest use (editions/attachments/cluster/mod/header) |
| timecode_overflow | 120-127 | REAL | S4 | `_offset` discarded (F-001) |
| no_tracks | 131-141 | REAL | — | mod.rs:438 + header.rs:228 |
| no_clusters | 146-153 | REAL | — | mod.rs:435; InvalidStructure{offset:None} |
| cue_table_too_large | 158-167 | REAL | — | ResourceLimit::Memory; cues.rs ×3 |
| not_started | 173-175 | REAL | S4 | code outside the documented `demux.mkv.*` namespace (F-002) |
| seek_out_of_range | 179-184 | REAL | — | mod.rs:850/883/893 |
| scan_too_slow | 189-198 | REAL | — | ResourceLimit::CpuTime, budget/attempted in bytes (mod.rs:867) |
| tests (3) | 204-283 | REAL | — | would fail on code/kind/Display regressions; sniff-cap test pins the no-secrets claim |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_stable_codes | YES | asserts exact stage, code, `[demux] `-prefix and `(code)`-suffix for all 14 constructors |
| kinds_map_to_the_documented_taxonomy | YES | exact ErrorKind equality (not `matches!` alone) incl. payload fields |
| not_matroska_sniff_capped_at_sixteen_bytes_by_display | YES | 64 input bytes, asserts ≤16 survive rendering — pins the no-secrets claim of :15-18 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every constructor mints … a stable `demux.mkv.*` context code" | error.rs:10-11 | FALSE for not_started | :174 mints `demux.not-started` (F-002) |
| "Constructors take byte counts, offsets and format names only — never paths, URIs or file content" | error.rs:15-18 | REAL | read of all 14 signatures; sniffed bytes capped in Display (test 3) |
| "the index is refused (the seek falls back to the bounded cluster scan)" | error.rs:155-157 | REAL-UNPROVEN (caller side) | cues.rs has the 3 refusal sites; fallback behavior lives in mod.rs seek — not executed by me (cargo-build-budget), unit suite in baseline covers seek paths |

## 7. Dead / duplicated / unreachable code
None: every constructor has live call sites (rg census in the reachability row). `_offset` (F-001) is the only ignored input.

## 8. Dependencies & flags
Depends only on media-types taxonomy types (`ErrorKind`, `CorruptDetail`, `LimitKind`, `ErrorContext`, `FormatInfo`). No feature gates, no unsafe, no allocation beyond the two bounded `to_vec`/`to_string` copies (F-002-adjacent bounds verified at callers).

## 9. Open questions for the next agent
- mkv/mod.rs + cluster.rs (not my files): whether `scan_too_slow`'s `SeekFailed{TooSlow}` recovery contract ("session still at its old position and playable", :187-188) is actually honored by the host after the error — the error constructor only carries budget/attempted.
- The `demux.not-started` code aliasing between crate::error, ps::error and mkv::error is intentional; if the R3 hoist (error.rs:8) happens, the module-header claim can become true — re-check after any merge.
