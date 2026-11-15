# ADR-001: Hardware Video Decoding — hw-first for H.264/HEVC, software-first for everything else

- **Status:** accepted (2026-10-19, Phase 0 gate)
- **Deciders:** Lead (per MASTER-PROMPT §2 default + §16 Phase 0 mandate; user
  informed 2026-10-19, no tightening applied)
- **Evidence:** `docs/feasibility/sections/video-codecs.md` (0.C1-I),
  `docs/inventory/platform-{windows,macos,linux}.md` + `platform-synthesis.md`
  (0.C5), `docs/inventory/modules-linux.json` (baseline priorities: avcodec
  ffmpeghw 10001, dav1d 10000), `docs/dependencies/preaudit-foundation.md`
- **Supersedes:** open question in doc 01 §7 / doc 02 §6
- **Consequence for risks:** R-01/R-02 mitigated-not-closed; ADR-010 patent
  matrix still gates distribution

## Context

MASTER-PROMPT §2 fixes the default: software codecs are written in Rust; OS
decoders (VideoToolbox, Media Foundation, VA-API and similar) may be used only
after audit, as a documented OS-ABI exception, with truthful reporting of when
they are active. Phase 0's codec audit (0.C1-I) found:

1. **H.264/HEVC**: doc 04's "no production pure-Rust decoder" finding is stale
   but only barely — a 2026 wave of young, self-validated, tarball-scanned
   pure-Rust decoders exists (rusty_h264-decoder 0.16.1, rust_h264 0.4.0,
   rusty_h265, rusty_vp9). None has independent conformance validation.
   The pinned VLC baseline itself prefers hardware for these (ffmpeghw
   priority 10001).
2. **All other families**: credible pure-Rust paths exist (MJPEG clean; AV1 =
   rav1d with asm-off, statically proven cfg-gated; VP9/MPEG-1/2/4 candidates;
   audio via Symphonia et al. per 0.C2-I).
3. **OS-decoder Rust access** is policy-clean via metadata-generated bindings:
   windows-rs (Media Foundation), objc2-video-toolbox 0.3.2 (VideoToolbox);
   VA-API on Linux has an open classification question (system graphics stack
   vs third-party C library — libva is userspace, drivers are the OS stack).

## Decision

1. **H.264 and HEVC are hardware-first** via the E2 OS-ABI exception, with
   truthful active-decoder reporting (a UI/engine query that states which
   decoder produced the output — a feature-matrix and conformance-tested
   contract). Machines without OS-decoder support get an honest
   `unsupported-software-decode-pending` error, never a fake success.
2. **A conformance-gated software-adoption track** is opened for the 2026
   pure-Rust decoder wave: rusty_h264-decoder et al. may enter ONLY after
   passing our corpus + fuzz + differential-vs-oracle conformance (doc 06 §7
   kit), and only behind the same plugin API. Until then they stay out.
3. **H.264/HEVC bitstream parsers are self-built in Rust regardless** (needed
   for probe/track info and any future software path).
4. **Everything else stays software-first in Rust** (per §2): MJPEG (Phase 2,
   D-001), AV1 via rav1d asm-off (Phase 5, proof plan P5.1–P5.7), VP9,
   MPEG-1/2/4, Theora, and all audio.
5. **VA-API classification**: treated as an **E2 OS-service binding** (system
   graphics stack component), conditional on each driver being an OS/distro-
   shipped component — NOT on bundling any third-party codec library. Access
   via audited Rust bindings only; no dlopen of unpackaged binaries. If a
   stricter reading is later imposed, Linux hw decode flips to blocked and the
   feature matrix says so.
6. **WebCodecs on web** (ADR-008 territory) is the browser-API analog —
   allowed as a browser service, not a bundled codec, pending its own audit.

## Consequences

- Parity ceiling for H.264/HEVC = hardware-equipped machines; the honest
  matrix rows carry `hw-first` semantics with the active-decoder report.
- R-01/R-02 downgraded from existential to bounded (software track may
  eventually close them; no date promised).
- ADR-010 (patents) becomes the dominant legal risk for hw-first flagship
  codecs (unchanged by this ADR).
- Phase 7 cards 7.B (per-OS hw integration) proceed; Phase 5 keeps the
  conformance-gated software cards conditional.

## Revisit triggers

- A pure-Rust H.264/HEVC decoder passes our conformance kit → reconsider
  default software-first for those codecs.
- VA-API classification overruled → Linux row flips to blocked.
- ADR-010 outcome forbids shipping hw-decode binaries → feature matrix marks
  hw rows blocked-by-legal and the software track priority rises.

## Addendum (2026-11-15): the software track's first trigger fires — H.264

The first revisit trigger fired. `rust_h264` 0.6.1 — the issue-#2 admission
audits' recommended candidate — passed the conformance kit's requirements on
the Lead's instruction: the differential suite
(`crates/codecs/tests/h264_sw_differential.rs`) measured **bit-exact against
ffmpeg 7.1.5** on six libx264-authored lanes (Constrained Baseline CAVLC,
Main CABAC, Main/High with B-frames, odd-size SPS crop, single-IDR still) on
BOTH entry forms (Annex-B ES and MP4/AVCC), plus the hostile battery
(garbage recovery, truncation sweep, garbage-then-real) and the engine-level
e2e lane (`mp4_h264_family_lane`: h264+aac plays to `Ended`, exit 0).
Admission record: `docs/dependencies/records/rust_h264-0.6.1.md`.

Decision consequences, recorded:

1. **The software track is the shipped-default H.264 path** (decision 2's
   gate passed; the `codec-video-h264-software` matrix row flips
   `blocked` → `implemented`). The CLI provider dispatches
   `sw::H264SwDecoder` for `h264`/`avc1` under the existing
   `playback.hw-decode-policy = "software-only"` default — the policy's
   meaning is now literally true.
2. **The hw-first structure is unchanged**: `decode::H264Decoder` (the
   parser → AU → hw-session lane) stays for `prefer-hardware`/
   `hardware-only` consumers, with its typed `h264.hw.required` refusal
   when no accelerator is present; the capability registry's H.264
   fallback target flips `UnsupportedPending` → `Software` (the truthful
   record). HEVC keeps `UnsupportedPending` until its own trigger fires.
3. The boundary stays ours: NAL framing, EPB unescape, `avcC` parse and
   damage accounting ride the in-house 5.F1 parser — the admitted engine
   is fed through that bridge, never its own splitters (the rusty_vp9
   admission's rule, applied again).
4. Same session (the color-converter extension): the CLI present limb
   converts I420 BT.601/BT.709 × full/limited — closing the
   `cli.i420_color_unsupported` parking that kept vp9/av1/h264 sessions
   in `Error` (the known W1-06-adjacent gap). BT.2020 still refuses
   typed (no landed lane carries it).
