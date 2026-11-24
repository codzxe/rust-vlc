# CLAIM AUDIT: docs/formats/sub-vobsub.md
| Field | Value |
|---|---|
| Auditor | W19-A06 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL 7 / REAL-UNPROVEN 1 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest sibling of the DVB record, and every checkable claim lands byte-exact: module
layout, 15-test census, 128 KiB SPU cap, (date<<10)/90 timing, 0xBD/0x20..=0x3F
carriage, and the matrix row that quotes this record and its honesty caveats verbatim.
The vobsub-crate-not-taken decision is documented on both sides (record + matrix notes).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Row subtitle-vobsub backfilled to implemented by 5.Q2-V with honest gap "NO docs/formats record" (G-1) — this file closes it | :7-9 | REAL | docs/feature-matrix.toml:1043 status implemented x3/web not_started; :1044 evidence "crates/subtitles/src/vobsub/** (card 5.N2, commits ab696b4+93a53ec: idx/spu parse, 128 KiB cap, grayscale fallback, damage ledger; 230/0 gates); record docs/formats/sub-vobsub.md (G-1 closed, 2026-10-26)". |
| 2 | self_built = crates/subtitles/src/vobsub/** (mod/idx/spu/fuzz) | :19 | REAL | ls crates/subtitles/src/vobsub -> fuzz.rs idx.rs mod.rs spu.rs — exactly the four named files. |
| 3 | Unit suites: 15 tests across mod/idx/spu | :47 | REAL | rg -c '#\[test\]' -> mod.rs 5, idx.rs 3, spu.rs 7 = 15 exactly. |
| 4 | Multi-PES SPUs bounded by the entry's byte span and a 128 KiB cap | :31-32,:53 | REAL | spu.rs:103 `pub const SPU_MAX_SIZE: usize = 128 * 1024;` (byte-exact). |
| 5 | Timing: (date << 10) / 90 ms, half-open [t0,t1); SPU without end date open-ended | :37-41 | REAL | spu.rs:189 `0x01 => start_ms = Some((date << 10) / 90),`; mod.rs:32 and spu.rs:29 doc comments carry the same formula verbatim. |
| 6 | Carriage: PS demuxer routes 0xBD substream 0x20..=0x3F as "dvd_subtitle" | :42-43 | REAL | spu.rs:369 `find_start_code(sub, p, 0xBD)`, :414 `if (0x20..=0x3F).contains(&substream)`, :8-9 comment "demuxer (5.I2a) already routes `0xBD` + substream `0x20..=0x3F` payloads as `"dvd_subtitle"`" — the claim is quoted 1:1 in code comments. |
| 7 | vobsub 0.2.3 (CC0) crate verified as an alternative path, deliberately not adopted | :4-6,:59-60 | REAL | Matrix notes :1045 repeat it: "vobsub 0.2.3 crate (CC0) path verified ... 5.Q2-V backfill: implemented (5.N2) — same no-differential honesty as DVB; record owed." |
| 8 | Gates at 5.N2 card: 230/0 across cargo test -p subtitles; current tree re-verified green | :73-74 | REAL-UNPROVEN | Historical gate count (same 230 figure as the DVB/SRT records — cross-consistent); baseline cargo-test-workspace.txt subtitles package green; not re-run (load discipline). |
| 9 | SVCD/CVD siblings NOT implemented — separate grammars; tracked by the matrix row's scope note | :64-66 | REAL | Matrix row description "Parse VobSub (IDX/SUB) and SVCD/CVD bitmap subtitles" with vlc_modules svcdsub/cvdsub, while the record scopes itself to IDX/SUB only — the gap is disclosed on both sides, and rg finds no svcd/cvd module in crates/subtitles/src/ (only srt/dvb/vobsub/cc/vtt/pgs families). |

## 3. Contradictions
None. The record, matrix row, and the DVB sibling record agree on the shared
fuzz-survival file, the 230/0 gate figure and the no-differential honesty.

## 4. Fabrication / inflation indicators
None — same disclosure pattern as sub-dvb.md (no differential, web not_started,
SVCD/CVD explicitly out of scope).

Commands run: ls vobsub module; rg -c '#\[test\]' per file (5/3/7=15); rg -n -A12 matrix row; rg SPU_MAX_SIZE/0xBD/0x20..=0x3F/<<10 in spu.rs; rg -n svcd/cvd absence check.
