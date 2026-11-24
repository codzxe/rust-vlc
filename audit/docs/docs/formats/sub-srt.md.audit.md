# CLAIM AUDIT: docs/formats/sub-srt.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A09) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 5 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The SRT subtitle record (5.M1 + 5.Q2-V backfill + 8.G1 cross-link). Matrix agreement is
exact on both rows it owns: subtitle-srt implemented x3 (the claimed backfill landed),
sibling subtitle-encoding-detection partial x3 — precisely the split this record
prescribes (this module's encoding.rs IS the sibling's landed subset). Module set and
fuzz shim verify. Differential/fuzz run claims not re-runnable. No fabrication.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "subtitle-srt was not_started ×4... backfilled by the 5.Q2-V records audit" | :4-6 | REAL at date; landed | feature-matrix.toml:892 subtitle-srt now implemented x3, web not_started |
| 2 | "sibling row subtitle-encoding-detection (partial ×3 desktop / not_started web)" | :7-8 | REAL | feature-matrix.toml:1085 status partial x3 + web not_started — exact match |
| 3 | "self_built = crates/subtitles/src/srt/** (encoding/parser/tags/damage)" | :27-28 | REAL | ls -> damage.rs, encoding.rs, fuzz.rs, mod.rs, parser.rs, render.rs, tags.rs |
| 4 | "fuzz/fuzz_targets/fuzz_sub_srt.rs shim... Lead-owned wiring" | :21-25 | REAL | fuzz/fuzz_targets/fuzz_sub_srt.rs exists |
| 5 | "BOM-less INVALID UTF-8 falls back to Windows-1252... typed damage; BOM-less UTF-16 deliberately NOT sniffed" | :36-43 | REAL-UNPROVEN | encoding.rs present; the fallback matrix is its documented behavior (per baseline + 5.Q2-V); not re-executed |
| 6 | "6/6 canonical-differential asserts EQUAL; 37 committed mutants; every-prefix truncation sweep — zero panics" | :62-77 | UNVERIFIED | not re-runnable (cargo budget); consistent with the record's own gates |
| 7 | "cargo test -p subtitles... 230/0 with srt_canonical_differential 6 + srt_fuzz_survival 4 green" | :78-82 | UNVERIFIED | cargo forbidden; not in baseline granularity |
| 8 | "The plugin-api SubtitleParser seam is NOT wired" | :88-90 | REAL | honest-gap statement; consistent with the plugin-api seams audit lineage (boundaries.toml class) |

## 3. Contradictions
None. Both owned rows agree with the record today — this is the no-drift case for a
backfilled row family.

## 4. Fabrication / inflation indicators
None. The record pins ffmpeg's own limitations as asserted behavior (VTT→SRT drops
<font> wrappers) rather than hiding them; the web column stays honestly not_started.

## 5. Notes for next agents
- The record's honest-gap "VLC 3.0.23 oracle PENDING-INFRA" matches the family-wide
  baseline gap (rc-smoke/PERF-VS-VLC context: no VLC binary in sandbox sessions).
- subtitle-encoding-detection partial (not implemented) is the correct reading for a
  module that implements BOM/UTF-8/CP1252 but no subsdec heuristics — do not "upgrade"
  it when citing.
