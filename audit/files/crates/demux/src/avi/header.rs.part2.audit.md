# AUDIT: crates/demux/src/avi/header.rs#part2 (lines 434-866)
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Chunk boundaries | part2 = lines 434-866 of 867 (part1 = 1-433 → W04-A05 per assignments.tsv). The boundary is clean: part2 = `zeroed_main_header`, `parse_hdrl`, the strl chunk-id constants, `advance`, `peek_form`, `parse_avih`, `StrlParts`, `parse_strl`, `peek_size`. I read part1 (types/constants/DamageLog) for context; findings scoped to 434-866. |
| Lines / functions | 433 / 9 production functions |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (AVI is the first vertical: `lumen open` → containers avi → `parse_hdrl` on every AVI open; baseline `avi-firstvertical-open-session.txt` full lifecycle + `mp4-smoke-bench-avi-mjpeg-pcm.txt` 60/60 frames decoded; rc-smoke 6/6 engine checks) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | full read; `sed -n` on every quoted range; `rg -n "samples_per_sec"` (never compared with strh rate); `rg -n "negative-biheight|strh-too-short|duplicate-avih|hdrl-extends|not-honored|strl-extends|without-strh" crates/demux/src/avi/tests.rs crates/demux/tests/` (coverage map); baseline nofailfast lines 1590-1663 (58 avi tests ok) |

## 1. One-paragraph verdict
The `hdrl`/`strl` parser is the most battle-tested code in my assignment set: 58 AVI tests
pass in the baseline, the truncation-vs-damage taxonomy is engineered (the
declared/physical-cut gate at 520-523 is correct under case analysis), per-stream
containment is real (degenerate `strh`/`strf` skip the stream with 20+ distinct damage
reasons, several pinned by tests), and the WAVEFORMATEXTENSIBLE GUID resolution has the
right offsets and the right skip-vs-truncate distinctions. One genuine behavioral hole:
the strh clock (`dwRate`/`dwScale`, the PTS-ladder base) and the strf clock
(`nSamplesPerSec`, the decoder config) are **never cross-checked** — the function
damage-notes every other disagreement pair (sample_size vs block_align, block_align vs
channels×bytes) but a rate disagreement silently produces wrong-speed playback with the
two clocks disagreeing between demuxer and decoder. Remaining findings are hygiene:
strf payload reads are not clamped to the declared strl extent, a declared-short `avih`
is fatal to the file while every sibling degeneracy is containment damage, and six
damage paths in this range have zero test coverage.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` lines 1590-1663: 58 `avi::` tests ok, incl. `channels_over_ceiling_skips_the_stream_siblings_survive` (1622), `block_align_disagreements_are_damage_notes_not_skips` (1620), `float_pcm_without_sample_ladder_skips_the_stream` (1628), `truncated_head_is_typed_at_the_boundary` (a cut sweep, 1586-1608 in tests.rs).
— verified by baseline INDEX.md §"contrast: AVI first-vertical": `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` on the AVI fixture — parse_hdrl feeds the one lane that actually plays end-to-end in the shipped build.
— verified by read: the truncation gate condition (520-523) fires `truncated` exactly when the clamp was the *physical* stream end (`bound == reader.length() && bound < declared_end`) and falls to the `hdrl-declared-size-not-honored` damage note when the RIFF size lied — the taxonomy distinction the AIFF parser gets wrong (aiff/mod.rs audit F-001) is done right here.
— verified by read: WAVEFORMATEXTENSIBLE GUID at strf bytes 24..40 (line 792: `strf_offset + 8 + 24`), correct per the 18-byte WAVEFORMATEX + 6-byte extension layout; declared-short extensible blobs are skipped as *structural* damage ("extensible-strf-truncated", 788-790), not misreported as physical truncation.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/avi/header.rs:837-842 (checks present) vs 683-684 + 771 (the unchecked pair)
Code:
```rust
            if sample_size != u32::from(block_align) {
                damage.note(0, "sample-size-disagrees-with-block-align");
            }
            if block_align != channels.saturating_mul(sample.bytes_per_sample()) {
                damage.note(0, "block-align-disagrees-with-format");
            }
```
What is wrong: The function damage-notes two WAVEFORMATEX internal disagreements
(`dwSampleSize` vs `nBlockAlign`, `nBlockAlign` vs channels×bytes) but never compares
the strh clock with the strf clock: `rate`/`scale` (strh bytes 20/24, lines 683-684)
become the track time base (`StreamDesc::time_base`) and drive the PTS ladder
(`ticks_per_chunk`), while `samples_per_sec` (strf byte 4, line 771) rides the decoder
config (`StreamFormat::Pcm`). A file with strh `dwRate=44100, dwScale=1` but strf
`nSamplesPerSec=48000` (corrupt or hostile) is accepted with **zero damage note** —
the demuxer advances time at 44.1 kHz while the decoder consumes 48 kHz samples:
silent playback-speed/duration drift between the two halves of the same engine.
Why it matters: This is the exact class of disagreement the surrounding code
exists to surface ("repairs are counted and surfaced, never silent" — DamageLog
doc, 382-383); every sibling field pair is checked, this one is not, and its failure
mode is user-visible (wrong speed, A/V desync on muxed files) rather than cosmetic.
What correct looks like: `if u64::from(rate) != u64::from(samples_per_sec) ||
scale != 1 { damage.note(0, "strh-rate-disagrees-with-strf"); }` (or prefer strf
for PCM and note the override, matching what the decoder actually consumes).
Confidence: HIGH — Verification: read (`rg -n "samples_per_sec"` → only stored, never compared; `rate` used only in time_base/ticks)

### F-002 [S4]
Where: crates/demux/src/avi/header.rs:727-729 (also 761-762, 792)
Code:
```rust
            let Some(bytes) = reader.read_exact_or_end_at(40, strf_offset + 8)? else {
                return Err(error::truncated(strf_offset + 8 + 40, reader.length()));
            };
```
What is wrong: The strf payload reads (40 bytes BITMAPINFOHEADER for video, 16 bytes
WAVEFORMATEX and 16 bytes GUID for audio) are absolute-position reads bounded only by
the physical stream — never clamped to the enclosing `strl`'s declared end. A lying
strf size (e.g. 4 GiB) makes the strl walk exit immediately (advance jumps past
`end`), but the deferred payload read then parses 40 bytes from wherever the strf
header sat, potentially *past the strl extent* — bytes belonging to the next chunk.
Contrast the AIFF parser, which clamps every payload read by the FORM/extent
intersection (`want.min(extent_end - payload_start)`, aiff/mod.rs:334-336).
Why it matters: Structure-bounded reads are the house rule ("nothing trusts a
declared size before it has been ceiling-checked", riff.rs:13-14); here a declared
extent is trusted for the walk but not for the read. Blast radius is small (wrong
width/height/GUID parsed from neighbor bytes, per-stream contained, no panic).
What correct looks like: Clamp the strf read to `min(strf_size, end - (strf_offset+8))`
exactly like the AIFF COMM read, and treat the clamped-short case as structural
damage (skip stream) rather than reading foreign bytes.
Confidence: HIGH — Verification: read (all three strf read sites lack extent clamps; strl `end` computed at 607-608 but not applied)

### F-003 [S4]
Where: crates/demux/src/avi/header.rs:569-571
Code:
```rust
    if declared_size < 56 {
        return Err(error::bad_stream_params(chunk_pos));
    }
```
What is wrong: A declared-short `avih` aborts the whole open (`Err` propagates through
`parse_hdrl`), while every other degenerate header piece in this file is containment
damage: a short `strh` skips the stream ("strh-too-short", 623-628), a short extensible
strf skips the stream, a missing avih is *fine* (zeroed, `main.unwrap_or_else
(zeroed_main_header)`, 528). The module doc itself declares every avih field advisory
("Advisory `avih` fields … are never trusted for control flow", lines 51-56) — yet the
chunk's size is trusted enough to kill the file.
Why it matters: Strictness inconsistency: an advisory-only structure is the one
structure whose corruption is fatal. A hand-muxed file with a 40-byte avih is refused
outright where VLC synthesizes defaults.
What correct looks like: `damage.note(size, "avih-too-short")` + keep the zeroed
header (the already-existing `zeroed_main_header` path), matching the doc.
Confidence: MEDIUM — Verification: read (severity tempered: the refusal is typed and
honest; the defect is the inconsistency, not a lie)
### F-004 [S4]
Where: crates/demux/src/avi/header.rs:465, 481, 525, 610, 678, 670 (the untested damage sites)
Code:
```rust
    if declared_end > bound {
        damage.note(declared_end - bound, "hdrl-extends-past-riff");
    }
```
What is wrong: Six damage paths in part2 have zero test coverage anywhere in the repo
(verified `rg` over `crates/demux/src/avi/tests.rs` and `crates/demux/tests/`):
`hdrl-extends-past-riff` (465), `duplicate-avih` (481), `hdrl-declared-size-not-honored`
(525), `strl-extends-past-hdrl` (610), `strl-without-strh` (678), `unknown-chunk-in-strl`
(670). The tested reasons (strh-too-short, zero-rate-or-scale, negative-biheight,
extensible-strf-truncated, unsupported-video-handler, block-align disagreements,
channels-over-ceiling) prove the pattern works — the untested six are exactly the
hdrl-walk-level branches, including the early-stop/resume_at contract that recovers a
lying hdrl size (497-501).
Why it matters: The early-stop rule is the documented recovery mechanism for a
corrupted hdrl size ("This is what recovers a corrupted `hdrl` size that would
otherwise swallow the `movi` list", module doc 16-19) — an untested documented
recovery behavior.
What correct looks like: One test per branch: hdrl size lying past RIFF, duplicate
avih, hdrl tail shorter than a chunk header, strl size past hdrl, strl without strh,
unknown chunk inside strl — each pinning the damage reason and the surviving sibling
stream.
Confidence: HIGH — Verification: read + rg (no test constructs these shapes)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `zeroed_main_header` | 434-443 | REAL | — | the advisory-avih-absent default |
| `parse_hdrl` | 455-533 | REAL | F-004 | walk `pos+8 <= end`; early-stop on non-strl LIST/unknown chunk (the lying-hdrl recovery); truncation gate (520-523) verified correct by case analysis: fires only on physical cut (bound==length && bound<declared), falls to damage note when the RIFF size lied; `resume_at = pos` may exceed `end` after a lying inner size — the parent walk's own bounds absorb it |
| `STRH/STRF/STRN/STRD/VPRP` consts | 536-548 | REAL | — | vprp silent-skip carries the W3-16 corpus rationale (57 clean rows) |
| `advance` | 551-553 | REAL | — | `pos+8+size+pad`, no saturation — inputs bounded (pos ≤ length, size u32) so no overflow |
| `peek_form` | 556-561 | REAL | — | `Ok(None)` on a physical cut inside the form → parse_hdrl breaks → cut misfiled as structural damage, not truncation (2-4-byte edge; folded into notes, cosmetic) |
| `parse_avih` | 564-583 | REAL | F-003 | declared < 56 fatal (F-003); offsets verified against the avih layout (flags@12, totalFrames@16, streams@24, width@32, height@36) |
| `StrlParts` (struct) | 587-591 | REAL | — | deferred strf read (see F-002) |
| `parse_strl` | 600-861 | REAL | F-001/F-002 | containment model: 20+ damage reasons; strh floor 56; signed biHeight with bottom-up damage note; `height > i32::MAX` catches \|i32::MIN\|; extensible GUID resolution correct offsets; F-001 (no rate cross-check), F-002 (unclamped strf reads) |
| `peek_size` | 864-866 | REAL | — | trivial accessor over peek_chunk_header |

## 5. Test honesty
Tests live in `crates/demux/src/avi/tests.rs` (another agent's assignment) and the
integration suites; they exercise part2 heavily: 58 avi tests green in baseline
(1590-1663). Mutation reasoning (no edits): swapping the GUID read offset 24→22 would
misparse every extensible stream and fail `channels_over_ceiling…`-family fixtures
(scrambled-extensible fuzz also survives, 1592); deleting the `scale==0` guard fails
the `zero-rate-or-scale` pin (tests.rs:548). The six uncovered branches (F-004) would
NOT fail under mutation — that is the finding.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A degenerate `strl` skips its stream, never the file" | module doc 24-32 | REAL | parse_strl containment arms; tests `channels_over_ceiling_skips_the_stream_siblings_survive` (baseline:1622), `all_streams_degenerate_is_typed_no_supported_streams` (1618) |
| "biHeight is read signed … accepted with the absolute height and a damage note" | module doc 33-37 | REAL | lines 731-739; test pin `negative-biheight-bottom-up` (tests.rs:618) |
| "Advisory avih fields … are never trusted for control flow" | module doc 51-56 | REAL for fields / INCONSISTENT for the chunk size | F-003: the size gate is control flow |
| "every impossible combination … skips the stream with a damage reason — never a panic, never the file" | module doc 46-50 | REAL | from_wave_tag refusals 816-825; fuzz `scrambled_extensible_audio_headers_survive` (baseline:1592) |
| "repairs are counted and surfaced, never silent" (DamageLog) | 382-383 | FALSE for one pair | F-001: the strh/strf rate pair is silent |

## 7. Dead / duplicated / unreachable code
None found in part2. `zeroed_main_header` is reachable (avih-absent path, 528).

## 8. Dependencies & flags
Uses only `media_types`, `super::riff`, `crate::error` — no layering violations, no
features, no unsafe, no unwrap/expect in production. Constants carry PROPOSAL
rationales (MAX_STRN_BYTES 4 KiB, MAX_PCM_CHANNELS 64). The damage-reason strings are
the observability contract — 6 of them untested (F-004).

## 9. Open questions for the next agent
- Does any consumer cross-check `StreamFormat::Pcm.samples_per_sec` against the track
  base at wiring time (which would neutralize F-001)? Check the codec-config handoff in
  apps/cli containers/avi.rs + codecs::audio::pcm.
- `resume_at` past the RIFF bound after a lying inner chunk size: confirm the top-level
  walk (avi/mod.rs, another agent) clamps it before its own loop.
- W04-A05 (header.rs part1): `ticks_for_sample_size` (329-337) is in your range — note
  `sample_size` vs `block_align` divergence is damage-noted but the ladder still uses
  `sample_size` (the strh field) — consistent with F-001's strh-wins policy.
