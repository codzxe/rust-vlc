# Format record — Ogg Kate (karaoke-overlay subtitle codec)

Status: **blocked (v1) — decision record (card 5.P2b-I part B)**, per
feasibility subsystems §7.3 μADR-A recommendation: **drop from v1 via
typed-unsupported track** (option b). Self-build M (option a) was
considered and rejected on value: Kate's usage share is believed
near-zero outside Ogg-advocate archives `[VERIFY — doc 04 §8.6 item 7
data]`; its motion/scroll/karaoke effects would degrade exactly like
the ASS styling subset's unsupported tags (doc 04 §5). Full-block
(option c, not even recognizing the mapping) was rejected by the μADR
in favor of honest typed refusal.

- Date of evidence: **2026-10-24** (crates.io API, this card).
- Supersedes-cites: doc 04 §2 Ogg paragraph + §5 row "Ogg Kate P3
  missing, no pure decoder found [VERIFY-P0]", feasibility subtitles §1
  row 13 + §3.368 + subsystems §7.3 μADR-A (2026-10-19), feature-matrix
  row `subtitle-kate` (blocked ×4).

## Parse / Decode / Encode

Nothing implemented. **Honest landed-state finding (this card):** the
matrix's Ogg-container note claims the demuxer "carries
Vorbis/Opus/Theora/Kate/Speex/FLAC-in-Ogg mappings" — **stale for
Kate**: `crates/demux/src/ogg/mod.rs` `OggCodec` has no Kate variant;
a Kate logical stream (magic `\x80kate\0`) falls to
`OggCodec::Unknown` → descriptor "ogg-opaque" (opaque passthrough, no
clock). That is *not* the μADR-A option-(b) typed refusal — the user
sees a generic opaque track, not an error naming Kate. **Next step
(when the Ogg/Kate card is scheduled): add the `\x80kate\0` magic to
`identify()` with a `Kate` variant → typed `unsupported-decoder`-class
error naming "kate"** (S, test-pinned like the opus-refusal precedent
in codecs). Doc-drift flagged for the Lead (matrix Ogg note);
deliberately not edited here (investigation scope).

## Evidence (what was checked, 2026-10-24)

- **crates.io direct names:** `kate`, `libkate`, `ogg-kate`, `kate-rs`,
  `katesub` — **all NOT_FOUND**; keyword search "kate" surfaces no
  subtitle-relevant crate (top hits are zk-SNARK/LSP/completion
  crates). Confirms doc 04 §5's [VERIFY-P0] and feasibility subtitles
  §1 row 13 (2026-10-19) — still zero pure-Rust Kate decoders.
- **Reference implementation:** libkate is C (BSD) — forbidden as a
  dependency; usable only as behavioral oracle. The Kate bitstream is
  publicly documented (wiki.xiph.org/OggKate), which keeps the M-sized
  self-build option alive if share data ever surprises.
- **VLC handling (oracle):** VLC ships its own `kate` spu decoder
  (modules-linux.json spu-decoder list, 23 registrations); formats.json
  `subtitle-census-tail` is census-only (`not-probe-verified`) — no
  probe fixture exists, so even the oracle's Kate behavior is
  unverified `[VERIFY if a differential is ever wanted]`.

## Bit depths / layouts

Text-with-styles plus an optional bitmap-canvas mode (feasibility
subtitles §1 row 13); bitmap mode would additionally need the
renderer-wgpu overlay path (doc 04 §5 bitmap-compositing contract) —
another reason the M estimate is honest only with the overlay contract
landed.

## Tracks / Seeking / Timestamps

Rides Ogg logical-stream mechanics (granule = milliseconds since start
of the Kate event per the Ogg Kate mapping); container-side handling
is the Ogg demuxer's. Nothing Kate-specific implemented.

## Corrupt-input behavior

N/A (no parser ships).

## Limits / Platforms

Matrix: blocked ×4, unchanged by this card.

## Corpus

None — no .ogv-with-Kate lane in corpus-v4; acquisition would need a
libkate-encoded fixture (C toolchain, oracle-side only) if ever
prioritized.

## Results

None (no code). Next step: (1) μADR-A text rides ADR-008's subtitle
annex or stands alone in the blocked-registry (ADR-008 not yet
written — honest gap, Lead owns); (2) the S-sized `identify()`-level
typed refusal above is the only in-tree work item, foldable into any
future Ogg-polish card; (3) recheck crates.io at Phase 7 R-10 —
currently zero candidates, so the trigger is usage-share data, not a
crate.
