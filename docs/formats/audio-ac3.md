# Audio record — AC-3 / E-AC-3 (Dolby Digital / Digital Plus) decode

Decision record (card 5.K4-I). No code changed. Front-matter keys per
doc 04 §6:

- id = `codec-audio-ac3-eac3` (matches `docs/feature-matrix.toml`, which
  carries `status = blocked` ×4 with empty `evidence` — this record is
  that evidence)
- status = **blocked** (decode; carriage is landed, below)
- phase = 6 (decode admission window; matrix `phase_target = 6`)
- fuzz_target = `fuzz_codec_ac3` (doc 04 §3.1 row; does not exist until a
  decoder is admitted)
- crates = [] — **none admitted**; the 2026 candidate wave is recorded
  below, all below the validation bar
- self_built = [] — demux-side carriage only (PS private-1 + TS stream
  types, 5.I2 et al.), no bitstream contact
- last_audit = 2026-10-24 (this card's crates.io re-survey)

## Verdict (the card's question: passthrough policy vs self-build-L-split
vs blocked-with-record)

1. **Decode: BLOCKED ×4, upheld** — typed `unsupported-decoder` naming the
   missing decoder, stream-granularity playback semantics exactly as
   5.K3-I picked for WMA (demux the file, decode every stream we can,
   error the AC-3 track; never silent silence).
2. **The 0.C2-I "None found" premise is amended (this card's material
   finding):** a 2026 pure-Rust AC-3 wave exists on crates.io (below),
   missed by the 2026-10-19 survey because it queried only the exact name
   `ac3`. The Phase-6 self-build-L **plan is demoted to fallback of last
   resort**; the primary unblock becomes **validate-then-admit
   `oxideav-ac3`** — the exact 5.K2-I/ruopus pattern (S spike → M
   admission, Phase 6, Lead pre-approval, C5 glue-ours posture).
3. **Self-build split proposed (the L-split):** (a) **S — self-built
   sync-frame parser** (BSI/syncinfo: bsid, sr_codes, channel map,
   bitrate) for probe/track-info on raw `.ac3`, TS `0x81/0x84/0x87` and
   PS private-1 tracks + the raw-ES reader gap (doc 04 §2) — the ADR-001
   decision-3 analog ("bitstream parsers are self-built in Rust
   regardless", needed for probe/track info ahead of any decode); (b)
   **L (+M for E-AC-3) — the decoder itself**, only after the validation
   AND patent gates below. Reference exists (`veloster-ac3` 0.1.10,
   parser only, 26 dl) but the header is small enough that ours is the
   honest S.
4. **Passthrough policy (the ADR-001 analog, proposed not decided):**
   the hw-first pattern's audio translation is **not an OS decoder but an
   OS digital-output transport** — IEC 61937-packed AC-3/DTS handed to
   the audio device (S/PDIF/HDMI), the receiver's *licensed* decoder
   renders it, we perform zero decode → patent-exposure-neutral for us
   and honest (VLC's own pinned build did exactly this: `spdif`
   "passthrough packing", decode never exercised [V3 re-probe]).
   Proposal: an **S/M card — compressed-bitstream output mode on
   ADR-005's `AudioBackend` seam** (the trait is currently PCM-render
   only; the mode is an extension, not a rewrite; rides the ADR-005
   revisit-trigger admission gate). No OS ships a callable AC-3 decode
   API on macOS/Linux; **Windows MF's "Dolby AC-3 decoder" MFT is the
   one E2-class OS-decoder candidate** — availability + licensing terms
   `[VERIFY]`, folded into the hw-first platform notes the way 5.K3-I
   handled the WMA audio MFT question, NOT proposed for v1.

## Patent note (pure-Rust posture ≠ patent-clear)

MP §14 separates patent risk from source licenses; doc 11's patent
matrix row: "AC-3/E-AC-3, DTS, TrueHD | Dolby/DTS licensing regimes
`[VERIFY]` | decoders are `missing` today | **register before any
self-build lands**"; R-03 (risk 9): a binary containing an AC-3 decoder
"may be undistributable in some jurisdictions regardless of how clean
the code is" — ADR-010 `[PROPOSAL]` is still open and gates any
distribution. Widely reported (not verified by us): core AC-3/A/52
patents have largely expired while E-AC-3 carries later filings —
`[VERIFY: legal review at ADR-010, before ANY admission — oxideav or
self-build]`. Consequence recorded as policy: **patent gate applies
identically to admitting oxideav-ac3 and to a self-build**; validation
effort must not run ahead of the ADR-010 registration.

## Evidence (crates.io API, fetched this session 2026-10-24)

The 2026 pure-Rust AC-3 wave (all candidates, none admitted):

- **`oxideav-ac3`** 0.0.11 — "Pure-Rust AC-3 (Dolby Digital) audio
  decoder", repo `github.com/OxideAV/oxideav-ac3`, created 2026-04-23,
  last update 2026-09-01, 11 versions, **6 782 downloads** (the family
  leader; same OxideAV project as `oxideav-wma`, recorded by 5.K3-I).
  Pre-0.1, single-maintainer family, zero published conformance evidence,
  provenance unverified against our clean-room bar (ATSC A/52 is a
  public spec, so clean-room is *possible* here — unlike WMA — but
  unproven). **The S-spike candidate.**
- `vuio-codec-ac3` 0.0.1 (2026-10-19, 51 dl) — "SIMD-accelerated AC-3
  **and E-AC-3** decoder and encoder in pure Rust. **Forked from
  oxideav-ac3**" — day-one fork, no independent body of evidence.
- `filmcraft-ac3` 0.9.0 (2026-10-23, 11 dl, 1 version) — "**Clean-room**
  AC-3 and E-AC-3 … ATSC A/52 audio decoder" — claims the right words
  but records **no repository URL on crates.io** → provenance
  unverifiable, one day old. Watch-list only.
- `soundkit-ac3` 0.13.3 (2026-09-12, 45 dl, wavey-ai/soundkit) — "raw
  AC-3 decoder **wrapper** for soundkit" — wrapper surface, unclear what
  it wraps `[VERIFY if ever promoted]`.
- `veloster-ac3` 0.1.10 (2026-10-11, 26 dl) — **sync-frame parser
  only**, not a decoder; reference for the S-parser split above.
- Confirms 0.C2-I: exact-name `ac3` remains NOT_FOUND; `liba52` is C
  (forbidden, doc 04 §1.2); Symphonia has no AC-3 crate (absent from
  the upstream table).

## Landed state in this tree (no code changed by this card)

- **PS (5.I2):** private-stream-1 `0x80..=0x87` → `PrivateKind::Ac3` →
  descriptor `ac3` (substream byte + 3 filler stripped, bit-exact vs
  `ffmpeg -c copy -f ac3`); fixtures `ps-vob-ac3.vob` (dvd_nav +
  mpeg2video + ac3) pinned in `crates/demux/tests/ps_fixtures.rs`. DTS
  rides the same private-1 table (`0x88..=0x8F`).
- **TS:** stream types `0x81 → "ac3"`, `0x84 | 0x87 → "eac3"`,
  `MediaKind::Audio` (`crates/demux/src/ts/mod.rs`).
- **Raw ES: no `.ac3` reader yet** (`crates/demux/src/es/` = aac /
  annexb / h264 / hevc / mp3) — the S-parser split fills this gap.
- Decode refuses at the registry level (no AC-3 decoder registered) —
  the typed-refusal status quo.

## Corpus (corpus-v4, ffprobe 7.1.5 this session)

| Lane | Shape | Notes |
|---|---|---|
| `0001-audio-only.ac3` (1.11 s) | ac3, stereo, 44.1 kHz, 192 kb/s | raw bare stream — exercises the future ES reader + syncinfo probe |
| `crates/demux/fixtures/ps/ps-vob-ac3.vob` | mpeg2video + ac3 (+ nav) | committed 5.I2 fixture, ffmpeg-authored, payload bit-exact |

Honest gap: **no ac3-in-TS lane exists in corpus-v4** (all 5 `.ts` files
probed: h264-only or aac — checked 2026-10-24), no `.ps/.mpg/.vob` in
the media dir (PS coverage is the committed fixtures), no E-AC-3 lane at
all. The PS/TS demuxers EXPOSE ac3/eac3 tracks today (decode-pending);
an E-AC-3 + TS-embedded acquisition item belongs on the Phase-6 corpus
plan alongside C-V6.

## VLC oracle posture

Pinned baseline chose `spdif (passthrough packing)` with a dummy aout —
"decode NOT exercised" (formats.json `ac3-eac3` row, ac3_raw.log /
eac3_raw.log `[V3 re-probe]`). A decode-side oracle run (force
avcodec/a52) is a prerequisite for any admission differential.

## Next actions

1. **S — validation spike (Phase 6 window):** oxideav-ac3 0.0.11 into a
   scratch harness (NOT the workspace graph): ATSC A/52 conformance
   vectors, differential vs ffmpeg 7.1.5 on 0001 + ps-vob-ac3.vob (+
   E-AC-3 + TS lanes once acquired), purity pre-checks (tarball scan,
   license, unsafe census), **and the ADR-010 patent registration first**
   (doc 11: "register before any self-build lands" — extended by this
   record to any admission).
2. **S — self-built AC-3 sync-frame parser + raw `.ac3` ES reader**
   (probe/track-info; ADR-001 decision-3 analog; patent-light header
   parse, still flagged to ADR-010).
3. **M — admission** (only after 1 + ADR-010): direct pin, glue ours,
   `fuzz_codec_ac3`, record flips to implemented-with-evidence.
4. **S/M card proposal (for the Lead): IEC 61937 passthrough output
   mode** on the ADR-005 seam — AC-3 and DTS ride it; patent-neutral
   playback path while decode stays blocked.
5. **L fallback:** self-build A/52 (+M E-AC-3) only if oxideav-ac3
   fails validation and ADR-010 permits a decoder at all.
6. Monitor: re-query the wave each phase (filmcraft-ac3 gaining a repo,
   vuio diverging, Symphonia adding an AC-3 crate would all re-rank).

## Doc drift for the Lead (out of this card's scope)

- doc 04 §3.1 AC-3 row "none (crates.io ac3 NOT_FOUND…)" — stale since
  2026-04-23 (oxideav-ac3); §7 gap-7 framing "AC-3/DTS: L self-builds
  Phase 6" should carry the validate-then-admit primary.
- feasibility audio-codecs §1 row 12 + §5.2 — same correction.
- `feature-matrix.toml` `codec-audio-ac3-eac3.blocked_reason` → amend to
  name the candidate wave + patent gate; backfill `evidence` to this
  record.
