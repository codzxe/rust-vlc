# Subtitle record — WebVTT (parse, settings subset)

Status: **implemented** (parse, card 5.M3). Feature-matrix row
`subtitle-vtt` was `not_started` ×4 with empty evidence at wave-9
open — stale vs the landed parser; backfilled by the 5.Q2-V records
audit (this file is that evidence). Front-matter keys per doc 04 §6:

- id = `subtitle-vtt` (`docs/feature-matrix.toml`)
- status = implemented (parse + model projection); platform-
  independent code, evidence Linux-only
- phase = 5 (landed, wave 6)
- fuzz_target = `fuzz_sub_vtt` (doc 04 §8.5 planned name; in-crate
  `vtt` fuzz harnesses — cargo-fuzz absent, Lead-owned wiring)
- crates = [] — zero external deps
- self_built = [`crates/subtitles/src/vtt/**` (mod/damage/settings/
  text/timestamp)]
- last_audit = 2026-10-25 (5.Q2-V; engine evidence 5.M3)

## Scope landed (5.M3)

- Block-level walk: BOM skipped, CRLF normalized; header `WEBVTT` +
  optional header text; the ONLY fatal is a missing header (typed
  `VttError::MissingHeader`, non_exhaustive). NOTE blocks skipped
  whole; STYLE blocks collected tolerantly (raw CSS + `::cue` rule
  count — styling DEFERRED onto model TextStyle by a later card);
  REGION blocks typed-unsupported notes.
- Cues: optional id line (numeric → CueId verbatim; else sequential
  auto ids from 1; all-digit overflow → damage + auto); `-->` with
  optional whitespace; `HH:MM:SS.mmm` + `MM:SS.mmm` greedy
  timestamps (u128 intermediate, saturating i64::MAX µs; sec ≤ 59;
  min ≤ 59 only when hours present — spec-permitted).
- Settings subset → model: `line:N%|N[,start|center|end]`,
  `position:N%[,left|center|right|line-left|line-right]`,
  `align:start|center|end|left|right`; `vertical:`/`size:` =
  typed-unsupported damage notes (never silently applied); unknown
  names typed (forward-compat); first-valid-wins on duplicates;
  merged onto bottom-center default.
- Cue text → CueText::Plain: ALL tags stripped (`<v>`, `<i>`,
  `<c.cls>`, karaoke stamps; unterminated `<` truncates tail), then
  one-pass entity decode (the six spec-named + numeric dec/hex;
  `&amp;lt;`→`&lt;` exactly once). `&quot;` stays literal (spec
  defines exactly six named escapes; ffmpeg emits it anyway —
  recorded in the AC truth).
- Damage ledger: 10 typed kinds with 1-based line numbers
  (BadTimestamp, ReversedInterval, TimingLineNotSecond, OrphanBlock,
  BadSetting, UnknownSetting, UnsupportedSetting, UnsupportedBlock,
  IdOverflow, …); never panics.

## Corpus / differential evidence (5.M3)

| Fixture | Origin | Result |
|---|---|---|
| basic.vtt | ffmpeg 7.1.5 `-c:s webvtt` (SRT→webvtt) | counts/times/text/settings equal hand-derived truth |
| long.vtt | ffmpeg 7.1.5 (>1 h → long timestamp form) | PASS (both spellings exercised by real encoder output) |
| styled.vtt | ffmpeg 7.1.5 (ASS alignments + `{\an4}`) | PASS — ffmpeg's webvtt ENCODER drops ALL positioning (their gap, recorded) |
| settings_heavy.vtt | handcrafted (BOM+CRLF, NOTE/STYLE/REGION, both id kinds, every supported setting, vertical/size, reversed, 120%) | PASS; damage ledger asserted EXACTLY (5 events, exact line numbers) |

- Fuzz-lite: 4000 seeded mutations of a 64-cue realistic doc + 3000
  random printable-biased buffers + 160 KB well-formed + 199 KB
  hostile unterminated-tag cue — ALL <200 KB: zero panics, only
  typed failure is MissingHeader, invariants hold, deterministic
  re-parse equality.
- Gates at card: 92/0 (20 new vtt unit + 4 acceptance + 3 fuzz);
  fmt + clippy clean. Current tree (5.N2 session, re-verified
  5.Q2-V): 230/0 with `vtt_acceptance` 4 + `vtt_fuzz` 3 green.

## Honest gaps (D-10)

- HLS `X-TIMESTAMP-MAP` arrives as tolerated header text; offsets
  UNAPPLIED (Phase-6 HLS card seam).
- `vertical:`/`size:` typed-unsupported; STYLE/REGION styling
  deferred onto model TextStyle (later card).
- `&quot;` display parity vs browser players unmeasured
  (spec-literal chosen, recorded).
- VLC 3.0.23 oracle PENDING-INFRA (ffmpeg 7.1.5 is the always-on
  oracle); wasm32 untested (web column honestly not_started).

## Next steps

Phase-6 network-hls consumes the parser for segment-side WebVTT;
the styling card consumes collected `::cue` CSS.
