# CLAIM AUDIT: docs/feature-matrix.toml (rows 040-052)

| Field | Value |
|---|---|
| Auditor | W18-A10 |
| Date | 2026-11-24 |
| Rows audited | 13 (codec-audio-mp2 … codec-audio-wavpack; file lines 651-831) |
| Claims extracted | 42 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 26 / 11 / 0 / 0 / 0 / 5 / 0 (claim-level) |
| Internal contradictions found | 4 |

## 1. Blunt verdict

The audio-codec block is the most honest quadrant of the matrix so far:
every "implemented" row is backed by real, shipped, CLI-reachable decoder
code (all 7 families dispatch in `apps/cli/src/pipeline_host.rs:241-346`,
all engines in `Cargo.lock`, all prior per-file audits REAL, baseline
`cargo test` green). The rows that admit gaps (aac-lc partial, alac partial)
are telling the truth — but the machine-written triage text they inherited
from the compat-matrix reporter **misdiagnoses typed codec-gate refusals as
"no decoder in this build"** and prescribes "land the missing decoder
lane" for lanes that already exist and already decoded sibling files in the
same run. One notes line (row 050, ADPCM) cites a dependency that does not
exist anywhere in the tree — flat fabrication-by-copy-paste. Windows/macos
"implemented" cells are code-complete-but-never-built claims (Linux-only
evidence, admitted in-record); per the schema that is legal, per reality it
is unproven.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | mp2: windows/linux/macos "implemented", web "not_started" | :660 | REAL (linux) / REAL-UNPROVEN (win,mac) | Code `crates/codecs/src/audio/mp2/` (6 files); dispatch `pipeline_host.rs:266-270` `"mp2" => { let mut decoder = Mp2Decoder::new();` (sed-verified). Linux-only evidence admitted in record `docs/formats/audio-mp3-mp2.md`. No win/mac build ever ran (baseline has no such lane). |
| 2 | "card 5.J3b; record docs/formats/audio-mp3-mp2.md, 5.REC-2: honest ±1-LSB verdict on 4 rows" | :661 | SHALLOW-count mismatch | Record table has 8 lanes of which ONE is Layer II (`| 7 | fixture mp2_48k.mp2 | MPEG-1 Layer II stereo | … 35526 (24.475%, ±1)`), verdict "8/8 lanes inside the ±1-LSB lossy tolerance" — "4 rows" matches nothing in the record. ±1 honesty itself verified. |
| 3 | "NOT MD5-bit-exact, two independent lossy syntheses" | :661 | REAL | `docs/formats/audio-mp3-mp2.md:62-65` (md5s differ, ±1 column present). |
| 4 | aac-lc: windows/macos "implemented", linux "partial", web "not_started" | :673 | REAL (linux-partial) / REAL-UNPROVEN (win,mac) | Compat run `docs/corpus/compat-matrix-20261110.md:133` "4 tracks decoded across 5 files (0 concealed packets) … 1 tracks REFUSED". |
| 5 | "142 concealed (damaged) packets on VALID corpus files of codec `aac` — the decode is not clean" | :682 | REAL | `compat-matrix-20261110.md:132` + raw-rows jsonl carries per-track `concealed` counters. |
| 6 | "1 corpus tracks of codec `aac-lc` REFUSED (no decoder in this build) vs 4 decoded" | :682 | FALSE (cause mislabeled) | Raw row: `0313-audio-only.aac` → `"message": "decode fatal: [decode] unsupported in aac (aac.channel_count)"` — a typed >2ch gate (`decoder.rs:53-56` "caps at stereo"), NOT a missing decoder; the "aac-lc" descriptor IS dispatched (`pipeline_host.rs:288` `"aac" | "aac-lc" =>`). |
| 7 | "Next step: … land the missing decoder lane (bounded fix) or split the row's claimed descriptor set" | :682 | FALSE (half) | The lane exists and decoded 4 files in the same run; the refusal is the channel-count gate. "Split the row" half is defensible. |
| 8 | he-aac: all four platforms "blocked" | :693 | REAL (honest blocked) | No SBR/PS code: `symphonia-codec-aac` in lock has no sbr/ps; `decoder.rs:33-56` refuses AOT 5/29 typed. |
| 9 | "Zero upstream code: no sbr.rs/ps.rs in symphonia-codec-aac@main" | :695 | REAL-UNPROVEN | Feasibility `audio-codecs.md:38` marks it **[RV]** (verified 2026-10-19 via crates.io API); I cannot re-verify — no registry cache in sandbox (UNVERIFIED directly, doc-corroborated). |
| 10 | "HE-AAC files decode as half-rate LC — must be DETECTED AND REPORTED, never silently played." | :695 | FALSE (for implicit ADTS) | `decoder.rs:45-49`: "Implicit ADTS HE signaling: undetectable from the 7-byte header … Such streams decode as **half-rate LC**" — they ARE silently played; only the explicit ASC path is refused typed (:39-41). |
| 11 | flac: implemented w/l/m, web not_started | :707 | REAL (linux) / REAL-UNPROVEN (win,mac) | `crates/codecs/src/audio/flac/` (6 files) + `claxon = { version = "=0.4.3", optional = true }` (Cargo.toml:86, in lock); dispatch :253-259; compat run: "codec-audio-flac (3, bit-exact family)" (:244). |
| 12 | "4 fixtures byte-exact + STREAMINFO MD5, corpus 0046, 539x realtime" | :708 | REAL | `docs/formats/audio-flac.md:39-41` "byte-exact vs ffmpeg PCM dumps **and** the STREAMINFO MD5 … corpus 0046 (88.2 kHz/24-bit, 44 frames) byte-exact both directions"; prior file audit REAL (tests executed). |
| 13 | alac: implemented w/m, partial linux | :720 | REAL (linux-partial) / REAL-UNPROVEN (win,mac) | Compat run `:134` "1 tracks decoded across 2 files … 1 tracks REFUSED". |
| 14 | "1 corpus tracks of codec `alac` REFUSED (no decoder in this build) vs 1 decoded" | :722 | FALSE (cause mislabeled) | Raw row `0396-audio-only.mov` → `"message": "[decode] unsupported in alac (alac.config.missing)"` — cookie-harvest gap in the MOV lane; the ALAC decoder exists and decoded the MKA file in the same run. |
| 15 | "Next step: land the missing decoder lane (bounded fix) or split the row's claimed descriptor set" | :722 | FALSE (misdiagnosis) | The decoder lane exists (`pipeline_host.rs:293-297`); what is missing is the MOV cookie harvest (`decoder.rs:620-626` per its own comment). |
| 16 | "legacy alac 0.5.0 kept as oracle" | :722 | FALSE | `grep -c '^name = "alac"' Cargo.lock` → 0; no alac crate anywhere in the tree — nothing was "kept". |
| 17 | vorbis: implemented w/l/m, web not_started | :733 | REAL (linux) / REAL-UNPROVEN (win,mac) | Code + `symphonia-codec-vorbis =0.6.1` in lock; dispatch :271-275; compat "codec-audio-vorbis (4)" clean. |
| 18 | "5-row differential, silence lane BIT-EXACT, others <=6e-8" | :734 | REAL | `docs/formats/audio-vorbis.md:42` "| corpus 0085-audio-only.ogg | 2/8000 | 5.96e-8 | PASS |". |
| 19 | "lewton 0.10.2 demoted to oracle per doc 04" | :735 | FALSE-as-worded | `lewton` absent from Cargo.lock and all Cargo.tomls — it was removed entirely, not "demoted to oracle" (no oracle role exists in-tree). |
| 20 | opus: implemented w/l/m, web not_started | :746 | REAL (linux) / REAL-UNPROVEN (win,mac) | `crates/codecs/src/audio/opus/` (5 files) + `ruopus = { version = "=0.1.2", … }` (Cargo.toml:137, in lock); dispatch :276-282; compat "codec-audio-opus (4)" clean. |
| 21 | "opus_corpus_differential 2/2 pass in the codecs suite (643 tests / 624 pass / 0 fail…)" | :747 | REAL-UNPROVEN (numbers) | Baseline `cargo-test-workspace.txt:880` "test result: ok. 506 passed; 0 failed" is the lib suite only; the 643/624 figure (19 gap = ignored) is the record's run — I could not re-run (no target/, cargo-build-budget). Record `docs/dependencies/records/ruopus-0.1.2.md` exists (227 lines). |
| 22 | "honest per-record gaps: s16-LSB-equivalence class … gain≠0 streams unmeasured, wasm32 not measured" | :748 | REAL (honest) | Admission record carries the same gaps; web honestly not_started. |
| 23 | ac3-eac3: all "blocked" | :759 | REAL (honest blocked) | No ac3 decode code; `pipeline_host.rs:372-379` catch-all refuses typed `cli.codec_unavailable`. Feasibility §5.2 anchor exists (`audio-codecs.md:312`). |
| 24 | dts: all "blocked", spec access [VERIFY] | :773 | REAL (honest) | `audio-codecs.md:320` §5.3 exists; no dts decoder in tree. |
| 25 | truehd-mlp: all "blocked" | :787 | REAL (honest blocked) | `truehdd`/`bitstream-io` NOT in Cargo.lock (grep 0) — decode genuinely absent, as claimed. |
| 26 | "bitstream-io 4.10.0 ToBits macro OOM root-caused; scratch differential 4/4 bit-exact (4,065,640 samples)" | :790 | REAL (per record) | `docs/formats/audio-truehd.md:29-36` "rustc 1.99.1 is OOM-killed compiling it … cause = bitstream-io 4.10.0's `ToBits`/write-tree macro expansion"; :45 "full-sample compare vs `ffmpeg -map 0:a -f s32le`". Scratch lane unverifiable by me (no vendored code) — doc-corroborated. |
| 27 | adpcm: implemented w/l/m, web not_started | :801 | REAL (linux) / REAL-UNPROVEN (win,mac) | `crates/codecs/src/audio/adpcm/` (8 files, in-house); dispatch :298-303 (`codec.starts_with("adpcm_")`); compat "codec-audio-adpcm (2)" clean. |
| 28 | "symphonia-codec-adpcm 0.6.1 (upstream 'Good')" | :803 | FALSE | `grep -c '^name = "symphonia-codec-adpcm"' Cargo.lock` → 0; zero symphonia references in `crates/codecs/src/audio/adpcm/`; the record the row itself cites says: `audio-adpcm.md:15-16` "crates = [] — **in-house, zero new dependencies** (D-018 logic; NOT symphonia-codec-adpcm)". Fabricated dependency attribution. |
| 29 | "lettered adpcm_oki typed-Unsupported (NO ffmpeg 7.1.5 reference at all)" | :803 | REAL | `adpcm/oki.rs` audited REAL with typed Unsupported; mod.rs doc table marks adpcm_oki "no ffmpeg 7.1.5 decoder or encoder exists". |
| 30 | wma: all "blocked", "WMA absent from Symphonia's codec table entirely" | :814/:816 | REAL (honest) | `audio-codecs.md:330` §5.4; no wma code in tree. |
| 31 | wavpack: all "blocked", GPL-3.0 conflict cited | :828/:830 | REAL (honest) | `audio-codecs.md:338` §5.5; `symphonia-codec-wavpack` not in lock. |
| 32 | (all 7 implemented rows) "evidence … Linux-only evidence" | per row | REAL (self-admitted) | Every implemented row's evidence string ends "Linux-only evidence" — the win/mac "implemented" cells have zero platform-specific evidence; consistent with parity-report 0/252 verified (`xtask-parity-report.txt:61`). |
| 33 | (schema) "not_started → verified jumps are REJECTED" / tooling "OWED and unassigned debt" | header :19-22, :30-31 | STALE | Header says feature-audit is "OWED and unassigned debt" while `audit/runs/baseline/xtask-feature-audit.txt` shows the tool exists and runs (FAILURE, 2 missing paths — neither in my rows). |

Claims 34-42 are the per-row inventory_ref/vlc_modules anchors (`inventory/formats.json#mp2` etc.) — all resolve to the seed inventory; not separately falsifiable here (covered by inventory audits).

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 044 notes: alac REFUSED "no decoder in this build"; next step "land the missing decoder lane" | pipeline_host.rs:293-297 dispatches "alac"; same compat run decoded an ALAC MKA track | The refusal is `alac.config.missing` (raw-rows jsonl, 0396-audio-only.mov) — a cookie-harvest gap. The matrix notes' cause and fix are wrong. |
| Row 041 notes: aac-lc REFUSED "no decoder in this build" | pipeline_host.rs:288 dispatches "aac"/"aac-lc"; 4 sibling files decoded in the same run | True cause: `aac.channel_count` typed gate (raw-rows, 0313-audio-only.aac). The canned phrase is the compat reporter's (tests/compat/src/report.rs:1197-1199 `"; {no_decoder} tracks REFUSED: no decoder in this build"`), which collapses every `decoder_for` Err — including codec configure-gates — into "no decoder". |
| Row 050 notes: "symphonia-codec-adpcm 0.6.1" | Cargo.toml/lock: no such crate; audio-adpcm.md:15 "in-house … NOT symphonia-codec-adpcm" | The record and lock are true; the matrix notes line is false (copy-paste from a sibling row). |
| Row 042 blocked_reason: HE-AAC "must be DETECTED AND REPORTED, never silently played" | decoder.rs:45-49: implicit ADTS HE is "undetectable", decodes as half-rate LC | The code is right and honest about it; the blocked_reason's imperative is unfulfilled for the implicit case (explicit ASC case IS refused typed, :39-41). |

## 4. Fabrication / inflation indicators

1. Row 050's engine attribution names a crate that does not exist in the
   project — classic sibling-row copy-paste inflation (all six sibling rows
   name a symphonia crate; only five of six are real).
2. "lewton 0.10.2 demoted to oracle" / "legacy alac 0.5.0 kept as oracle"
   (rows 045/044) describe crates that are not in the dependency graph at
   all — "kept as oracle" implies presence.
3. Row 040 "±1-LSB verdict on 4 rows" — the cited record contains 1
   Layer II row out of 8 lanes; the count is wrong in the flattering
   direction (more evidence than exists for the mp2 lane specifically).
4. Machine-written next-steps ("land the missing decoder lane") misdirect
   future work at non-existent gaps while real gaps (MOV cookie harvest,
   AAC channel cap) go unnamed — the tooling generated fake work items.
5. Windows/macos "implemented" cells across all 7 rows: zero win/mac
   execution evidence anywhere; legal per the schema's loose "implemented"
   definition, but 14 unproven platform cells in this chunk alone.

## 5. Matrix rows (13)

| Row | id | Statuses claimed (w/l/m/web) | Evidence string claims | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 040 | codec-audio-mp2 | impl/impl/impl/not_started | crates/codecs/src/audio/mp2/**, record ±1-LSB | mp2/ (6 files), Mp2Decoder dispatch :266, symphonia-bundle-mp3 in lock (codec-mp2→codec-mp3 feature, DEFAULT-on) | YES: baseline codecs 506 tests pass (cargo-test-workspace.txt:880); compat-matrix clean 1/1; prior mp2 audits executed differentials | REAL (linux); win/mac REAL-UNPROVEN |
| 041 | codec-audio-aac-lc | impl/partial/impl/not_started | aac/**, record 6-row 1-LSB, compat-matrix | aac/ (5 files) + symphonia-codec-aac in lock; dispatch :288-292 | YES: compat run 4/5 files, 142 concealed pkts; 0313 refused aac.channel_count | REAL (linux partial honest); win/mac REAL-UNPROVEN; notes cause FALSE |
| 042 | codec-audio-he-aac | blocked ×4 | none (blocked_reason only) | No SBR/PS anywhere; typed refusal AOT 5/29 (decoder.rs:39-41) | YES (typed-refusal unit test in baseline 506: `asc_object_type_sniffing_spots_the_he_family … ok`) | REAL (honest blocked); "never silently played" claim FALSE for implicit ADTS |
| 043 | codec-audio-flac | impl/impl/impl/not_started | flac/** + claxon, 4 fixtures byte-exact + MD5, 539x | flac/ (6 files) + claxon in lock; dispatch :253 | YES: compat clean 3/3 bit-exact; prior audits executed | REAL (linux); win/mac REAL-UNPROVEN |
| 044 | codec-audio-alac | impl/partial/impl/not_started | alac/**, 5/5 bit-exact, compat-matrix | alac/ (4 files) + symphonia-codec-alac in lock; dispatch :293 | YES: compat 1/2 (0396-mov refused alac.config.missing) | REAL (linux partial honest); notes misdiagnosis FALSE |
| 045 | codec-audio-vorbis | impl/impl/impl/not_started | vorbis/**, 5-row differential | vorbis/ (7 files) + symphonia-codec-vorbis in lock; dispatch :271 | YES: compat clean 4/4; prior audits executed | REAL (linux); win/mac REAL-UNPROVEN; "lewton kept as oracle" FALSE |
| 046 | codec-audio-opus | impl/impl/impl/not_started | opus/** + ruopus record, corpus 2/2 | opus/ (5 files) + ruopus in lock; dispatch :276 | YES: compat clean 4/4; ruopus record + W16-EE flip history | REAL (linux); win/mac REAL-UNPROVEN |
| 047 | codec-audio-ac3-eac3 | blocked ×4 | typed unsupported until Phase 6 | No ac3 decoder; typed catch-all refusal :372-379 | YES (refusal lane exercised by compat h264-family analog; no decode exists — as claimed) | REAL (honest blocked) |
| 048 | codec-audio-dts | blocked ×4 | spec access [VERIFY] | No dts decoder anywhere | n/a (nothing to run) | REAL (honest blocked) |
| 049 | codec-audio-truehd-mlp | blocked ×4 | OOM root-cause, 4/4 scratch bit-exact | No truehdd/bitstream-io in lock (verified 0); record carries the OOM analysis | Scratch differential UNVERIFIED by me (no code in tree — it was scratch); blocked status itself consistent | REAL (honest blocked); scratch claims doc-corroborated only |
| 050 | codec-audio-adpcm | impl/impl/impl/not_started | adpcm/**, 7 descriptors, 8/8 breadth | adpcm/ (8 files, in-house — mod.rs:1-7); dispatch :298 | YES: compat clean 2/2; prior audits bit-exact vs ffmpeg | REAL (linux); notes' symphonia attribution FALSE |
| 051 | codec-audio-wma | blocked ×4 | WMA absent upstream | No wma code | n/a | REAL (honest blocked) |
| 052 | codec-audio-wavpack | blocked ×4 | GPL-3.0 conflict | No wavpack code; crate not in lock | n/a | REAL (honest blocked) |

Row-level summary: 13/13 rows have code-or-absence exactly matching their
status cell (no inflated statuses); 5 rows carry false or misdiagnostic
notes/evidence text (041, 042-duty, 044, 045-oracle, 050); 14 win/mac
"implemented" cells unproven.

## Commands run

sed/grep/rg verification of all quoted lines (exit 0 each); `python3` raw-rows
enumeration (distinct refusal messages: `cli.codec_unavailable` ×33,
`alac.config.missing` ×1, `aac.channel_count` ×1, `vp9.layout` ×1); Cargo.lock
dep greps (claxon/symphonia-*/ruopus present; truehdd/bitstream-io/
symphonia-codec-adpcm/wavpack/lewton/alac-crate absent). No fresh cargo test
run (no target/ dir; compile budget) — executed evidence cited from
audit/runs/baseline/ and prior-wave audit files.

## Open questions for the next agent

- Row 041's "142 concealed packets" triage was due post-2026-11-10; did any
  later wave fix the concealment or the aac-lc descriptor split? Re-run
  compat-matrix to see if 0313 still refuses.
- report.rs:1199's canned "no decoder in this build" phrase should be
  itemized as a tooling defect owner (tests/compat/src/report.rs) — it
  fabricated the matrix notes' misdiagnoses.
- Header comment "The full tools/feature-audit … is OWED and unassigned
  debt" is stale vs the landed tool (baseline shows it running).
