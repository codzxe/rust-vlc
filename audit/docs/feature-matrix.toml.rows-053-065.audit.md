# CLAIM AUDIT: docs/feature-matrix.toml#rows-053-065 (audio-codec tail 53–56 + subtitle-format 57–65, of 252)
| Field | Value |
|---|---|
| Auditor | W18-A07 |
| Date | 2026-11-24 |
| Claims extracted | 13 matrix rows (52 platform statuses) + 8 note/evidence sub-claims audited in depth |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE (row verdicts) | REAL 0 / TRUE-honest 8 (blocked x3, not_started x5 — verified negative space) / UNREACHABLE 4 (subtitle-srt, subtitle-vtt, subtitle-pgs, subtitle-dvb-subs: real executed code, no shipped consumer) / UNREACHABLE-partly 1 (subtitle-ass-ssa, partial) |
| Internal contradictions found | 1 (status `implemented` ×9 platform cells for a crate no shipped binary compiles) |

## 1. Blunt verdict
The blocked/not_started half of this chunk (rows 053–056, 060–063) is the most honest
ledger text in the matrix: I verified the negative space — no speex, musepack, MIDI-synth,
g711/aes3/uleaddvaudio/ATRAC, microdvd, subviewer, sami or TTML code exists anywhere in
crates/ or apps/ (µ-law/A-law are typed refusals in the demuxers, not decoders). The
subtitle half (057–059, 064–065) inverts the problem: the code is real, heavily tested
(267/0 lib tests in the orchestrator baseline; 32/0 pgs and 6/0 srt differential executed
by W09) — and **no shipped entry point compiles the subtitles crate at all**. The only
workspace dependency on `subtitles` is `fuzz/Cargo.toml:50`; `use subtitles::` appears
nowhere outside the crate's own tests (my rg, W09-A03's grep). Every `implemented`
platform cell in rows 057/059/064/065 describes a library no player can load: no sidecar
load, no sub track wiring, no render path exists in lumen-cli or desktop. `implemented` on
windows/linux/macos for code that cannot execute in any shipped build is status inflation —
the row text never claims playability, so nothing is fabricated, but the ledger's
headline purpose ("status across platforms") is hollow for the entire subtitle category.

## 2. Claim ledger (sharpest sub-claims)
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "status = { windows = "blocked", … }" + "No pure crate (speex NOT_FOUND 2026-10-19)" (row 053) | feature-matrix.toml:838-846 | TRUE-in-tree / UNVERIFIED-crate-search | rg speex over crates/ apps/ → only ogg *mapping* (`crates/demux/src/ogg/mod.rs:156 OggCodec::Speex => "speex"`); no decoder code. The 2026-10-19 crates.io NOT_FOUND result is not re-runnable offline: UNVERIFIED. |
| 2 | "status … blocked" + "No pure crate (musepack NOT_FOUND 2026-10-19)" (row 054) | :847-859 | TRUE-in-tree / UNVERIFIED-crate-search | rg musepack/mpc → zero code in crates/ apps/. |
| 3 | "rustysynth 1.3.7 (MIT, pure, std-only …); oxisynth 0.1.0 is LGPL-2.1" (row 055 blocked_reason) | :860-873 | UNVERIFIED | External-crate facts; no network. In-tree absence verified: rg fluidsynth/rustysynth/oxisynth → zero. |
| 4 | "g711 … aes3 … uleaddvaudio, ATRAC1/3/X" not_started (row 056) | :874-891 | TRUE | rg g711/aes3/uleaddvaudio/atrac over crates/ apps/ → zero decoder code; µ-law/A-law are explicit typed refusals (`crates/demux/src/aiff/error.rs:81-82: "…`ulaw`, `alaw`, `ima4` and friends have no decoders"). |
| 5 | "267/0 lib tests incl. per-tag suites (1bd5519)" (row 058 evidence) | :910 | REAL | `audit/runs/baseline/cargo-test-workspace.txt:5739-5740`: `Running unittests src/lib.rs (…subtitles-70f89de07410626f)` / `running 267 tests` — then 267 passed 0 failed. Matches exactly. |
| 6 | "6/6 ffmpeg 7.1.5 differential incl. UTF-16 both endians" (row 057 evidence) | :898 | REAL-UNPROVEN-differential / REAL-tests | Baseline `cargo-test-workspace.txt:6097-6104`: srt_canonical_differential 6 tests, real names (canonical_out1_styled_tags_survive …), all ok. The ffmpeg-7.1.5-authored fixture provenance itself is UNVERIFIED offline. |
| 7 | "32 unit tests + fuzz_sub_pgs survival harness, all green in this tree 2026-10-26" (row 064) | :990 | REAL | W09-A04 executed: `./target/debug/deps/subtitles-70f89de07410626f pgs::` → 32 passed 0 failed; baseline `:6078` pgs_fuzz_survival 2 passed. |
| 8 | "Row was STALE not_started x4 with empty evidence while 5.N1 shipped — flipped 2026-10-26 as a RECORDS CORRECTION only (no new code this session)" (row 064 notes) | :995 | TRUE (admitted staleness) | Confirms the matrix's own history of rows lagging code by weeks; the flip direction (under-claim → truth) is the honest direction. |
| 9 | subtitle rows' platform statuses `implemented` on win/linux/macos (057/059/064/065) | :893-, 917-, 982-, 995- | UNREACHABLE | rg `subtitles` over all Cargo.tomls → sole dependent `fuzz/Cargo.toml:50`; rg `subtitles::` over apps/ crates/ (excluding the crate itself) → zero consumers (W09-A03 lib.rs audit: "sole external consumer is fuzz/Cargo.toml:50"). No CLI/desktop/web wiring exists — `lumen --help subtitles` → "unknown settings area". |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Rows 057/059/064/065: `implemented` on windows/linux/macos ("code complete on that platform") | The subtitles crate is not a dependency of lumen-cli, desktop, or web; only fuzz links it; zero `use subtitles::` outside the crate (rg, this session). | TRUE that the code is complete; FALSE as a platform-delivery claim — the code is unreachable from every shipped binary, so no platform has anything. The status vocabulary has no way to say "real but unserved". |
| Chunk header (rows 1-13 file): "Config cross-check (R7 …): no enable-without-implementation, ever" | Row 064's own notes admit the row sat not_started while code shipped for weeks (records correction 2026-10-26). | Matrix rows can drift either direction; the R-gates validate shape, not truth (also xtask feature-audit: "2 evidence paths missing", parity 0/252). |

## 4. Fabrication / inflation indicators
- Status inflation without lying evidence: four `implemented` ×3-platform rows for a crate
  with zero shipped consumers. Evidence strings cite only in-crate unit/fuzz tests —
  correct, executed, and completely silent about the fact that no product path exists.
- The `partial` row (058 ASS/SSA) is the only row in the chunk that under-claims honestly
  (styling subset, degrade counters, "libass oracle impossible").
- Blocked rows (053–055) cite external crate-search results dated 2026-10-19 that are
  unfalsifiable offline — plausible, but the ledger's evidentiary standard (R4 for
  `verified`) does not apply to `blocked_reason`, so these strings are unchecked forever.
- Positive: this chunk contains no fabricated execution claims — every number I could
  check (267/0, 32/0, 6/0, 2/0) matches a real test binary run in the baseline.

## 5. Matrix rows
| Row | id | Statuses claimed (w/l/m/web) | Evidence string (gist) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 053 | codec-audio-speex | blocked x4, blocked_reason: no pure crate | "" (empty) | NONE — only the ogg demuxer maps the speex NAME (`ogg/mod.rs:156`); no decoder | rg (negative space) | TRUE/blocked (in-tree absence verified; crate-search claim UNVERIFIED offline) |
| 054 | codec-audio-musepack | blocked x4, no pure crate | "" | NONE | rg → zero | TRUE/blocked (same caveat) |
| 055 | codec-audio-midi-synth | blocked x4, rustysynth/oxisynth rationale | "" | NONE (no fluidsynth/rustysynth/oxisynth code) | rg → zero | TRUE/blocked; external-crate facts UNVERIFIED offline |
| 056 | codec-audio-census-tail | not_started x4 | "" | NONE — µ-law/A-law = typed demuxer refusals, not decode | rg → zero; aiff/error.rs:81-82 quote | TRUE/not_started (honest; "uLaw breadth [VERIFY]" flags real) |
| 057 | subtitle-srt | impl / impl / impl / not_started | crates/subtitles/src/srt/** + record sub-srt.md; "6/6 ffmpeg differential … 230/0" | crates/subtitles/src/srt/{mod,parser,encoding,tags,damage,render,fuzz}.rs present; record docs/formats/sub-srt.md (101 lines) present | Not by me; baseline: srt_canonical_differential 6/0, srt_fuzz_survival 4/0; W09-A07 facade audit | REAL code + REAL tests, **UNREACHABLE from shipped entry points** (crate unlinked: only fuzz/Cargo.toml:50). Platform cells are fiction-by-omission: nothing can load a .srt in any shipped build. |
| 058 | subtitle-ass-ssa | partial x3 / web not_started | crates/subtitles/src/ass/** + G-2 override-tag engine; "267/0 lib tests" | ass/{parser,render,layout,tags,fuzz}.rs present; record sub-ass-ssa.md (99 lines) | Baseline: subtitles lib 267/0 (cargo-test-workspace.txt:5739-5740); W09 audits of parser/render/tags | REAL code, `partial` status honest (subset + degrade counters), **UNREACHABLE from shipped entry points** — same unlinked-crate defect |
| 059 | subtitle-vtt | impl / impl / impl / not_started | crates/subtitles/src/vtt/**; "7003 fuzz inputs zero panics, 92/0" | vtt/{mod,settings,text,timestamp,damage,fuzz}.rs present; record sub-vtt.md (81 lines) | Baseline: vtt tests among the 267/0; W09 audits | REAL code + tests, **UNREACHABLE** (same class) |
| 060 | subtitle-microdvd | not_started x4 | "" | NONE — no microdvd dir (ls crates/subtitles/src) | ls | TRUE/not_started (honest) |
| 061 | subtitle-subviewer | not_started x4 | "" | NONE | ls | TRUE/not_started |
| 062 | subtitle-sami | not_started x4 | "" | NONE | ls | TRUE/not_started |
| 063 | subtitle-ttml | not_started x4 | "" | NONE | ls | TRUE/not_started |
| 064 | subtitle-pgs | impl / impl / impl / not_started | crates/subtitles/src/pgs/**; "32 unit tests + fuzz_sub_pgs … all green" | pgs/mod.rs (1092 LOC, 19 prod fns) present; record sub-pgs.md (62 lines, card 8.G1) | W09-A04 executed: `subtitles-70f89de07410626f pgs::` → 32/0; baseline pgs_fuzz_survival 2/0 | REAL code + REAL tests, **UNREACHABLE from shipped entry points**; row history admits weeks of staleness (records-correction note). Also note: "docs/formats record owed" is STALE — sub-pgs.md now exists (card 8.G1). |
| 065 | subtitle-dvb-subs | impl / impl / impl / not_started | crates/subtitles/src/dvb/**; "230/0 subtitles gates incl. dvb_vobsub_fuzz_survival"; record sub-dvb.md | dvb/{segment,clut,rle,fuzz}.rs present; record sub-dvb.md (81 lines) | Baseline: dvb_vobsub_fuzz_survival 2/0 (:6060); W09 audits of segment/rle/clut | REAL code + tests, **UNREACHABLE from shipped entry points**; honest gap admitted in notes (no reference-encoder differential, no corpus lane) |

Schema check executed this session (python3 tomllib): rows 53–65 ids/categories/prefixes
consistent (codec-audio-* in audio-codec, subtitle-* in subtitle-format); blocked rows
carry blocked_reason (R5 ok); the partial row carries notes (R6 ok); no `verified`
statuses exist in the chunk, so R4 evidence resolution is not exercised.

Commands run (this agent): rg sweeps (speex/musepack/fluidsynth/rustysynth/oxisynth/
g711/aes3/uleaddvaudio/atrac; `subtitles::` consumers; Cargo.toml dependents), ls of
crates/subtitles/src + docs/formats, baseline line citations (5739-5740, 6060, 6097-6104),
python3 tomllib chunk validation, `bin/lumen --help subtitles` (unknown area).
