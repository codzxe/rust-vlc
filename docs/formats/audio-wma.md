# Codec record — WMA 1 / 2 / Pro / Lossless (decode)

Status: **blocked — decision record (card 5.K3-I)**. Verdict:
**blocked on all platforms, honestly typed** — an ASF file (once the
Phase-6 container lands) must demux, list its tracks, play every stream we
*can* decode, and surface the WMA audio track as a typed
`unsupported-decoder` error. **Full-blocking the file was considered and
rejected** (rationale below). No code exists or is scheduled before the
Phase 7 decision point (doc 14 R-10).

- Date of evidence: **2026-10-24** (crates.io API re-checks, this card).
- Supersedes-cites: doc 04 §3 row "WMA 1/2 / Pro" (missing), feasibility
  audio-codecs §2.4 + §5.4 (0.C2-I, 2026-10-19), feature-matrix row
  `codec-audio-wma` (blocked ×4).

## Parse / Decode / Encode

Nothing implemented, nothing scheduled in v1. Decode of WMAv1 (tag 0x160),
WMAv2 (0x161), WMA Pro (0x162), WMA Lossless (0x163) is refused at the
codec-registry level with a typed `unsupported-decoder`-class error naming
the missing decoder (MP §17 blocked-item rule; doc 02 §4: blocked rows
carry typed errors, "no silent silence anywhere"). Encode: never planned.

## Evidence (what was checked, 2026-10-24)

- **crates.io direct names:** `wma` — **NOT_FOUND** (re-confirmed;
  was also NOT_FOUND at the 2026-10-19 feasibility pass). `wmav2` —
  NOT_FOUND. Name-collision disambiguation: **`wma-rs` exists but is a
  "Weighted Moving Average" trading-indicator crate** (0.1.0, 2020) —
  not the codec; any future search must not be fooled by it.
- **NEW since the 2026-10-19 verification (the material finding of this
  card):** search surfaces **`oxideav-wma`** — "Pure-Rust Windows Media
  Audio (WMA v1 + v2 baseline; later Pro + Lossless) decoder", MIT,
  part of the OxideAV project family. Versions: 0.0.1/0.0.2 (2026-05-03,
  both **yanked**), 0.0.3 (2026-08-30), **0.0.4 (2026-09-01)**;
  399 downloads total. Assessment: **pre-0.1, zero published conformance
  evidence, provenance unverified against our clean-room bar** — ffmpeg's
  WMA decoder is itself reverse-engineered and "cannot be consulted —
  clean-room" (feasibility §5.4); whether oxideav-wma's implementation
  derives from that lineage is exactly what a future audit must establish
  `[VERIFY: provenance + conformance vs a real corpus before any
  adoption]`. **Not a candidate dependency today**; recorded as the
  **recheck trigger for the Phase 7 decision point**.
- **Symphonia:** WMA is absent from the upstream codec table entirely —
  no `symphonia-codec-wma` crate or workspace member (feasibility §2.4,
  re-verified 2026-10-19). Unchanged.
- **Spec:** Microsoft published no open WMA *decoder* bitstream spec
  (feasibility §5.4); self-build "L" was always the optimistic number.
- **VLC coverage (what we cannot mirror):** VLC decodes WMA2 via
  `avcodec` (docs/inventory/formats.json, `asf-wmv` row: decoders
  `["avcodec (WMA2)"]`; probe evidence "WMAv2 only" [V7] — WMA1/Pro
  variants unprobed). The ffmpeg C dependency is forbidden in our tree,
  so VLC's WMA half is **not** a path for us, only a behavioral oracle.

## Bit depths / layouts

N/A (blocked). For the record: WMAv2 commonly 16-bit, 1–2 ch, 8–48 kHz;
Pro extends to 24-bit/6 ch (per ffmpeg's format surface — cited as
observable behavior only, not as implementation input).

## Tracks / Seeking / Timestamps

Codec-level record; container-side handling is `container-asf.md`'s
scope. The Phase-6 container card must carry the WMA tracks through as
**typed-unsupported** streams, not drop them.

## Corrupt-input behavior

N/A (no parser ships). The typed refusal path itself is panic-free by
construction (registry-level, no bitstream contact).

## Limits / Platforms

Matrix: `blocked` on windows/linux/macos/web — unchanged by this card.
No platform has an OS-provided WMA software decoder path we may call
(Windows has in-box WMV-family MFTs *for video*; the audio MFT question
is folded into the hw-first `codec-video-wmv-vc1` row's platform notes
and is not proposed for the audio row `[VERIFY if a WMA audio MFT is
ever exposed via our hw-decoder ADRs — currently out of scope]`).

## Corpus (honest gap)

**No WMA test material exists in-tree** beyond the Phase-0 probe lane's
WMAv2 8 kHz mono track inside the `asf_wmv2_wmav2` fixture
(formats.json; ffmpeg-generated). corpus-v4/media has no WMA lanes
(checked 2026-10-24). Acquisition item C-V6 (corpus-plan.md: "truehdd /
wma / vc1 curated sources exist & allowlisted", Phase 6) is the
prerequisite for any future audit — including oxideav-wma's.

## Results

None to report (blocked). This record is the decision evidence.

## Verdict rationale — blocked-honest vs full-blocked (the 5.K3-I pick)

**Picked: blocked + typed-unsupported playback semantics** (demux the
file, decode what we can, error the WMA track). Rejected: refusing the
whole ASF file. Rationale:

1. **Stream-granularity policy:** the engine already plays containers
   with one blocked track (e.g. MKV carrying a blocked TrueHD track still
   plays its video). Full-blocking ASF would be inconsistent and would
   retroactively devalue the Phase-6 container work.
2. **Real playback surface exists:** on Windows/Linux the hw-first
   `codec-video-wmv-vc1` path can decode WMV video; audio-only refusal
   would turn those files silent-but-watchable → correctly reported, not
   hidden.
3. **Honesty rule:** typed errors name the missing decoder (R-10's own
   mitigation line); a file-level refusal hides *which* half failed.

## Matrix-row proposal (for the Lead; feature-matrix.toml not touched by
this investigation card)

- `codec-audio-wma`: status stays `blocked` ×4; `phase_target` stays 7;
  `evidence` ← "docs/formats/audio-wma.md (5.K3-I, 2026-10-24)";
  `blocked_reason` updated to: "No spec-grade pure-Rust WMA decoder:
  `wma`/`wmav2` NOT_FOUND (2026-10-24), `wma-rs` is a same-name trading
  crate; Symphonia has no WMA crate; `oxideav-wma` 0.0.4 (MIT, 2026-09)
  exists but is pre-0.1 with unverified provenance/conformance — the
  Phase-7 recheck trigger (doc 14 R-10). Full record: docs/formats/
  audio-wma.md. ASF files demux + play other streams with this track
  typed-unsupported (5.K3-I)."

## Next actions

1. Phase 7 decision point (R-10): re-audit oxideav-wma — provenance vs
   clean-room bar, conformance on C-V6 curated corpus, license confirmed
   MIT, maintenance cadence. Pass → ADR-gated adoption proposal; fail →
   blocked stands.
2. Phase 6 container cards (`container-asf.md` split proposal) must
   implement the typed-unsupported WMA track surface.
3. C-V6 corpus acquisition (with vc1/truehdd) before any WMA audit.
