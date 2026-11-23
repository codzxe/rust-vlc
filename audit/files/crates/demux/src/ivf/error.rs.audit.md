# AUDIT: crates/demux/src/ivf/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-23 |
| Lines / functions | 215 / 13 (10 constructors + mint + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:55 (`"ivf" => ivf::open(...)`) -> containers/ivf.rs:82 (`IvfDemuxer::new`) -> ivf/mod.rs mints these errors (e.g., `no_frames` call site). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (callers, baseline), sed -n quote verification. No cargo runs. |

## 1. One-paragraph verdict
A clean, complete typed-error module: ten constructors covering every refusal the IVF demuxer can produce (magic, truncation, bad header, fourcc gate, no-frames, frame-beyond-file, memory ceiling, pts overflow, not-started, seek range), all minted at stage `Demux` with stable `demux.ivf.*` codes, with zero payload/path leakage (sniffed bytes capped by the taxonomy's Display — tested). The in-file tests pin stage+code for all ten, the Display shape (`[demux] ... (code)`), the sniff cap, and the kind taxonomy. The single defect is cosmetic-but-real: `no_frames()` fabricates a `Truncated { expected: 1, found: 0 }` detail for a file that is NOT truncated (it is complete but empty), misreporting the failure mode in any error surface that renders the detail.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt lines 1904-1906: all three tests `... ok` (`errors_carry_demux_stage_and_ivf_codes`, `kinds_map_to_the_documented_taxonomy`, `not_ivf_sniff_capped_by_display`).
— read: every constructor passes byte counts/offsets/bare fourcc only (no paths/URIs/payloads) — the module-doc "No secrets" claim (lines 9-13) holds.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ivf/error.rs:83-97
Code (verbatim):
```
pub fn no_frames() -> EngineError {
    mint(
        ErrorKind::Corrupt {
            detail: CorruptDetail::Truncated {
                expected: 1,
                found: 0,
            },
        },
        "demux.ivf.no-frames",
    )
}
```
What is wrong: a zero-frame IVF file is complete, not truncated — the stream ended exactly where its length field said. Stuffing `Truncated { expected: 1, found: 0 }` fabricates an expectation of "1" that nothing in the format ever promised; any UI or log rendering the detail reports a truncation that did not happen. The doc comment (83-86) honestly records the DIVERGENCE from ffmpeg (which opens empty files cleanly) but not the kind-mismatch. The stable code `demux.ivf.no-frames` is fine; the kind/detail is the wrong vocabulary slot (an "empty" variant is what the taxonomy needs). Blast radius: error-display quality only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 20-22 | REAL | — | stage+kind+code assembly |
| not_ivf | 27-37 | REAL | — | Unsupported + FormatInfo{ivf, sniffed}; capped by Display (test 185-189) |
| truncated | 41-48 | REAL | — | Corrupt::Truncated, honest expected/found |
| bad_header | 55-64 | REAL | — | InvalidStructure{offset} |
| unsupported_fourcc | 71-81 | REAL | — | AV01/VP90 gate; bare 4-byte tag, no payload |
| no_frames | 87-97 | REAL | S4 | fabricated Truncated{1,0} (F-001) |
| frame_beyond_file | 103-110 | REAL | — | Truncated, sticky; ffmpeg divergence documented |
| frame_too_large | 115-124 | REAL | — | ResourceLimit::Memory before allocation — the MP §12 cap's typed voice |
| pts_overflow | 128-137 | REAL | — | hostile-u64 guard |
| not_started | 140-145 | REAL | — | contract order |
| seek_out_of_range | 149-154 | REAL | — | bare refusal; no target context (minor) |
| tests::errors_carry_demux_stage_and_ivf_codes | 161-182 | REAL | — | all 10 codes + stage + Display suffix pinned |
| tests::not_ivf_sniff_capped_by_display / kinds_map... | 185-214 | REAL | — | 16-byte cap; kind taxonomy equality |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_ivf_codes | YES | exact stage+code pairs for all 10 constructors + Display ends_with code |
| not_ivf_sniff_capped_by_display | YES | 64 hostile bytes -> <= 16 hex-pair occurrences |
| kinds_map_to_the_documented_taxonomy | YES | full ErrorKind equality incl. nested detail fields |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Constructors take byte counts, offsets and the bare fourcc only — never paths, URIs or payload content" | error.rs:10-12 | REAL | verified by read of all 10 signatures |
| "sniffed bytes ... capped by the taxonomy's own Display (16)" | error.rs:12-13 | REAL | test 185-189 + baseline ok |
| "we refuse loudly instead" (empty file) | error.rs:83-86 | REAL | no_frames refusal; kind mismatch is F-001 |

## 7. Dead / duplicated / unreachable code
None. All constructors have call sites in ivf/mod.rs (rg-verified for no_frames; the module is the demuxer's only error source).

## 8. Dependencies & flags
media-types only (error taxonomy imported, never redefined). No features, no unsafe, no external deps. Pattern mirrors adts::error per the header.

## 9. Open questions for the next agent
- media-types auditors: is there (or should there be) an `Empty`/`NoData` CorruptDetail variant so no_frames (and the AVI no-movi / ADIF no-data precedent it cites) can stop abusing Truncated?
- Note for the CLI auditor: apps/cli/src/containers/ivf.rs:576 calls `demux::ivf::test_support::ivf_header` — another test-support seam inside the shipped CLI binary (same pattern as avi's demux_host.rs:739).
