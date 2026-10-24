# Audio record — DTS (Coherent Acoustics) decode

Decision record (card 5.K4-I). No code changed. Front-matter keys per
doc 04 §6:

- id = `codec-audio-dts` (matches `docs/feature-matrix.toml`: `blocked`
  ×4, empty `evidence` — this record is that evidence)
- status = **blocked** (decode; carriage is landed — PS private-1 and
  TS `0x82` both expose the descriptor)
- phase = 7/8 (decision point; matrix `phase_target = 8`)
- fuzz_target = `fuzz_codec_dts` (doc 04 §3.1 row; planned name only)
- crates = [] — **none admitted**; the two existing pure-Rust candidates
  are recorded below, both far below the bar
- self_built = [] — demux-side carriage only
- last_audit = 2026-10-24 (this card's crates.io re-survey)

## Verdict (blocked-with-record vs passthrough-only vs self-build-L)

1. **BLOCKED ×4 — upheld, unchanged in direction.** DTS decode ships as
   a typed `unsupported-decoder` track error with stream-granularity
   playback semantics (5.K3-I precedent: the file plays, the DTS track
   errors by name — never silent silence).
2. **The blocker remains spec availability BEFORE engineering**
   (feasibility §5.3, upheld): DTS Coherent Acoustics is not freely
   published the way ATSC A/52 is; ETSI TS 102 114 covers broadcast
   parts only, obtainability/terms `[VERIFY]` (feasibility [VERIFY]
   item 9). A clean-room build without a citable spec "is a research
   project, not a roadmap item" — so **self-build L is NOT scheduled**,
   unlike AC-3.
3. **New since 2026-10-19 (recorded, verdict moved in nuance only):**
   a pure-Rust DTS Core decoder exists — **`oxideav-dts` 0.0.2**
   ("decoder and Core encoder, Core profile, with EXSS/XCH/XXCH/X96/XLL
   extensions **planned**"), created 2026-05-03, updated 2026-10-06,
   2 versions, **68 downloads**, repo `github.com/OxideAV/oxideav-dts`
   — same family as `oxideav-wma` (5.K3-I) and `oxideav-ac3` (this
   wave's AC-3 record). Its existence implies *someone* had a source;
   provenance vs our clean-room bar is unverified (the WMA question,
   not the ATSC-question). Core-only also means the HD extensions that
   Blu-ray DTS-HD MA / DTS-HD HR lanes actually need are absent. Plus
   `vuio-codec-dts` 0.0.1 (2026-10-19, 43 dl) — "**Forked from
   oxideav-dts**", Core profile. **Not candidates today; recorded as
   the named Phase-7/8 recheck trigger** (the oxideav-wma slot for
   DTS, R-10).
4. **Passthrough policy: DTS rides the same card as AC-3** — IEC 61937
   packed DTS Core over S/PDIF/HDMI to the receiver's licensed decoder
   is *the* classic passthrough case (older than AC-3's). The S/M
   proposal in `audio-ac3.md` §Next actions 4 covers both codecs; no
   separate DTS output work exists or is needed. VLC's pinned build
   itself ran DTS as `spdif (passthrough packing)` only — and was
   configured `--disable-dca` (formats.json dts row rationale), i.e.
   **the oracle build has no DTS PCM decode either**; spdif-only
   evidence (dts_raw.log, "same passthrough caveat as ac3-eac3
   `[VERIFY]`").
5. **No OS-decoder path on any platform** (the ADR-001 analog answer for
   DTS): macOS/Linux expose digital-output transports only; no
   MF/CoreAudio DTS decode API is recorded in the inventories
   `[VERIFY if one is ever surfaced via the hw-decoder ADRs — same
   posture as the WMA audio MFT note in 5.K3-I]`.

## Patent note

Doc 11's matrix row names "Dolby/**DTS** licensing regimes `[VERIFY]`"
alongside AC-3/TrueHD with the same rule: "register before any
self-build lands". R-03 (risk 9) lists DTS decoders among the
distribution blockers pending ADR-010 `[PROPOSAL]`. Pure Rust does not
clear patents (MP §14); unlike AC-3 (largely-expired core patents,
`[VERIFY]`), no expiry analysis for DTS has been done — the honest
posture is "encumbered until legal review says otherwise". The
passthrough mode avoids the question entirely (we transport; the
receiver's licensed silicon decodes).

## Landed state in this tree (no code changed by this card)

- **PS (5.I2):** private-stream-1 `0x88..=0x8F` → DTS routing with the
  substream+filler strip, payload bit-exact vs `ffmpeg -c copy` —
  fixture `ps-vob-dts.vob` (dvd_nav + mpeg2video + dts; 94 PES) pinned
  in `crates/demux/tests/ps_fixtures.rs`.
- **TS:** stream type `0x82 → "dts"`, `MediaKind::Audio`
  (`crates/demux/src/ts/mod.rs`) — carriage is exposed alongside AC-3's
  `0x81`/`0x84|0x87`.
- **Raw ES:** no `.dts` reader (`es/` = aac/annexb/h264/hevc/mp3) —
  would be an S parser (sync words 0x7FFE/0x8001, core frame header)
  if ever prioritized; not proposed this card (no corpus demand below).
- Decode refuses at the registry level — typed-refusal status quo.

## Corpus (honest gap)

**No DTS material exists anywhere in corpus-v4** (checked 2026-10-24:
no `.dts` bare stream, no DTS-in-TS/MKV/PS lane in media/); the only
DTS bytes in-tree are the committed 5.I2 VOB fixture. An acquisition
item (DTS-CD bare stream + a TS/Blu-ray-remux-style Core lane) belongs
on the Phase-6/7 corpus plan before any recheck audit of oxideav-dts.

## Parse / Decode / Encode

Nothing implemented, nothing scheduled before the Phase-7/8 decision
point. Encode: never planned (oxideav-dts's "Core encoder" half is
irrelevant to us). The typed refusal path is panic-free by
construction (registry-level, no bitstream contact).

## Next actions

1. **Phase 7/8 decision point (R-10) stands**, trigger now named:
   re-audit `oxideav-dts` (provenance vs clean-room bar + what spec it
   was built from; conformance on an acquired DTS corpus; Core-vs-HD
   coverage statement) and `vuio-codec-dts` (fork divergence). Pass →
   ADR-gated adoption proposal (with the AC-3 patent-registration
   precondition); fail → blocked stands with this record as evidence.
2. Ride the AC-3 record's actions: IEC 61937 passthrough card (both
   codecs), ADR-010 patent registration before any decoder lands.
3. Corpus: acquire DTS lanes (above) before the recheck.
4. Optional S (only if prioritized by Phase-0 share data): DTS sync
   parser for probe/track-info parity on `.dts`/TS lanes.

## Doc drift for the Lead (out of this card's scope)

- `feature-matrix.toml` `codec-audio-dts.blocked_reason` → append the
  2026-10-24 finding (oxideav-dts 0.0.2 + vuio fork exist, Core-only,
  unvalidated/unverified provenance; named recheck trigger) and
  backfill `evidence` to this record.
- feasibility audio-codecs §1 row 13 "None … `dts` = unrelated
  tooling" — stale since 2026-05-03 (oxideav-dts); §5.3's spec blocker
  stands but should cite the candidate.
