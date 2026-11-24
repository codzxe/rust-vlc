# CLAIM AUDIT: docs/formats/audio-opus.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 7 |
| REAL 5 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 1 |
| Internal contradictions found | 1 (era split, resolved by the matrix's own history) |

## 1. Blunt verdict
The decision record's plan was executed to the letter: the prescribed `ruopus = "=0.1.2"`
admission (its §Plan item 2) landed in crates/codecs/Cargo.toml:137 with the exact
default-features=false/features=["std"] pin, the decode glue and corpus differential
exist, and the matrix row flipped to implemented ×3 with evidence citing this lineage.
The record's "blocked" front-matter is its 2026-10-24 state, superseded by the row's own
issue-#7 flip note — an era split, not a false claim. Its honesty discipline is again
exemplary: every upstream/crate fact is dated, and every unvalidated claim is marked as
the README's, not theirs.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "no Opus decoder is registered in crates/codecs; the Ogg-family decoder refuses the `opus` descriptor with a typed error, test-pinned" | :92-95 | REAL-at-time (superseded) | vorbis/decoder.rs:1067 `assert!(!VorbisDecoder::accepts("opus"))` + :1121 "opus must be refused" — the pin exists and still passes; a decoder has since landed (see #2) |
| 2 | Plan item 2 (admission): `ruopus = "=0.1.2"` direct, glue ours, feature gate | :120-125 | REAL (executed) | crates/codecs/Cargo.toml:79 `codec-opus = ["dep:ruopus"]` + :137 `ruopus = { version = "=0.1.2", optional = true, default-features = false, features = ["std"] }` — exact pin shape; crates/codecs/src/audio/opus/ exists; tests/opus_corpus_differential.rs exists |
| 3 | matrix row codec-audio-opus blocked ×4 with empty evidence (this record is that evidence) | :6-8 | REAL-at-time (superseded) | matrix today (:738-748): implemented ×3 + web not_started, evidence "ruopus =0.1.2 (card 6.M-opus; record docs/dependencies/records/ruopus-0.1.2.md … opus_corpus_differential 2/2 pass)" — docs/dependencies/records/ruopus-0.1.2.md exists |
| 4 | carriage landed: Ogg maps OggCodec::Opus → descriptor `opus`; OpusHead/OpusTags detection; Clock::Opus48000 (960 samples = 20 ms) test-pinned | :88-91 | REAL | demux/src/ogg/mod.rs:153 (descriptor), :169 (OpusHead/OpusTags magic), :189 `Opus48000`, :204 TimeBase, :227 granule — all present |
| 5 | crates.io/repo facts (symphonia-codec-opus is a 1-byte placeholder; ruopus published 2026-06-29, MIT, unsafe-denied, Miri-checked, no_std) | :45-79 | UNVERIFIED | external web facts, fetched 2026-10-24 by its author; nothing in-repo can confirm or refute; the doc itself marks the conformance claims "the README's, not ours" |
| 6 | corpus: 3 Opus lanes (0047/0077 OGG mono/5.1 48kHz; 0078 vfr-av mkv vp9+opus) | :97-107 | REAL-in-kind / drift | the committed manifest carries opus lanes with different indices (vfr-av mkv vp9+opus confirmed at media/0081-vfr-av.mkv) — same lane structure as the record's corpus; numbering differs (different local generation, same as video-vp8-vp9.md's drift) |
| 7 | "monitor: re-query symphonia-codec-opus each wave" + doc-drift list for the Lead | :126-149 | REAL-UNPROVEN | the drift items it lists (audio-codecs.md row 10, matrix blocked_reason) were resolved by the later admission (matrix note now carries the full corrected reason) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| front-matter "status = blocked" (2026-10-24) | matrix row today: implemented ×3 | era split — the row's own notes record the flip ("Issue #7 flip (W16-EE, 2026-11-09): implemented"); the doc's plan was the path there. Not a living contradiction. |

## 4. Fabrication / inflation indicators
- None. It quotes upstream's placeholder README verbatim, refuses to adopt unverified
  conformance claims, and prescribes the exact pin that later shipped.

## 5. Matrix rows
Documents codec-audio-opus (row :738): its blocked-era record is now superseded by
implemented ×3 with in-tree evidence (differential test + dependency record + the
wrapper with honest gap notes: s16-LSB-equivalence class, gain≠0 unmeasured, wasm32
unmeasured → web not_started).

## 6. Open questions for the next agent
- web=not_started for opus: verify whether ruopus builds on wasm32 (its README claims
  wasm32 support; the repo has not measured it — the matrix note says "wasm32 not
  measured").
