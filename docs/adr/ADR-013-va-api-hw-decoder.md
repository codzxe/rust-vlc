# ADR-013: VA-API Hardware Decode — the Linux lane (draft; decision pending)

- **Status:** **proposed** (drafted 2026-11-09, issue #2 / wave 15 — this file
  does NOT decide anything; it is the decision-ready design brief for the
  Linux hardware-decode lane). Decision due **before any VA-API adapter code
  lands** (Phase-7 cards 7.B1–7.B3, doc 13; the H.264 adapter 7.B1 is the
  sequence anchor — the HEVC row is conditional on it).
- **Deciders:** Lead (ADR-001's per-API audit mandate makes each OS decoder
  API its own gate; AGENTS §3.3 makes the unsafe adapter ADR-gated). The
  user is informed, not deciding: no MP §15 escalation here (R-03/ADR-010
  own the distribution question and are cited, not re-decided).
- **Evidence:** this file + `docs/dependencies/records/h264-software-decoder-audits.md`
  (same wave — the audited VA-API access-path set in its R2/R10);
  ADR-001 (accepted: hw-first for H.264/HEVC, per-API audit required);
  doc 04 §4.2/§4.3 (no software decoder ⇒ hw lane carries v1);
  `docs/inventory/platform-linux.md` + `platform-synthesis.md` (VLC baseline
  ships `vaapi`/`vdpau_avcodec` modules; VA-API covers 8/10/12-bit +
  4:2:2/4:4:4 on Intel); `docs/feature-matrix.toml` rows
  `platform-hwdecode-h264`/`-hevc` (both `not_started` — zero adapters
  exist) and `codec-video-h264`/`-hevc` (`partial` on seam-only evidence,
  cards 5.F2/5.G2); R-01/R-02 (doc 14 §2.1/§2.2, exposure 9 — the exposure
  this lane mitigates), R-15 (VA-API driver/device fragmentation, exposure 6),
  R-25 (hw-truthfulness), platform-synthesis SV-28 ("VA-API E2
  classification = open Lead question" — this ADR is that question's
  answer vehicle); AGENTS §3.2/§3.3 (dependency + unsafe-adapter rules).
- **Supersedes:** nothing — fills the Linux third of ADR-001's per-API
  audit mandate (Media Foundation and VideoToolbox get their own ADRs or
  amendments when their cards dispatch).
- **Consequence for risks:** R-01/R-02 mitigation evidence improves (lane
  designed, substrate audited) but neither closes — exposure stays 9 until
  an adapter decodes real media in CI; R-15 stays open (driver variance is
  runtime reality, not design); R-03 untouched (ADR-010 Q2/Q3 still gate
  distribution of any hw-mediated decode).

## Context

ADR-001 (accepted 2026-10-19) fixed the posture: H.264/HEVC decode is
**hardware-first** via OS decoders, admitted only after a per-API audit, as
a documented OS-ABI exception, with truthful reporting of when hardware is
active. What exists today is only the **seam**: `crates/codecs` ships the
clean-room H.264/HEVC parsers (5.F1/5.G1) plus the hw decode-path seams
(5.F2/5.G2) whose `Decoder` impls fail configure with the typed
`h264.hw.required` / `hevc.hw.required` `Unsupported` — honest by design.
**Zero adapters have landed** (matrix rows `platform-hwdecode-*`:
`not_started` on every OS), so on Linux today H.264/HEVC files do not
play. That is R-01/R-02 exposure 9 in its purest form: the two heaviest
P0/P1 video codecs are parser-only on the flagship free-desktop platform.

Why VA-API and not VDPAU/other: it is the pinned VLC 3.0.23 baseline's
Linux hw path (`vaapi` + legacy `vdpau_avcodec` modules; the inventory's
`avcodec ffmpeghw` priority-10001 census entry), it is the only lane that
covers the HEVC matrix claim (8/10/12-bit, 4:2:2/4:4:4 on Intel), and it
is the API the audited Rust access paths (below) actually reach.

The 2026-11-09 throwaway audits (`/tmp/w14dd-a` `pe`/`pf`, recorded in
`records/h264-software-decoder-audits.md` R2/R10) established the Rust
access-path set — none admitted, none in the workspace:

| Path | Shape | Build chain | Note |
|---|---|---|---|
| `libva` 0.1.5 (cros-libva lineage, BSD-3, MSRV 1.88) | safe bindings, **default `dlopen`** (libloading 0.9) | bindgen 0.72 + pkg-config + regex | heaviest surface, but the dlopen default means no link-time C; bindgen runs at build time against system libva headers |
| `libva-sys` 0.1.2 | raw `-sys` (libc) | cc + cfg-if + pkg-config | thin; leaves every invariant to us |
| `va` 0.0.1 | — | — | **false friend**: Vapory-blockchain language stub, not VA-API (dead end, recorded) |

SV-28's open classification question is therefore concrete: VA-API is a
**userspace C library over a kernel/driver service** — an E2-class
OS-service binding (doc 11 §1.2) on the "system service" reading, with the
complication that the service quality is **driver-dependent** (R-15:
Intel/AMD/Mesa variance, DMA-BUF interop). It is not vendorable pure Rust
and can never be: the only pure-Rust layer possible is the seam we already
ship.

## The lane design [PROPOSAL]

1. **Trait seam (already shipped, unchanged).** The adapter plugs the
   5.F2/5.G2 decode-path seam: parser → access-unit → adapter, frames back
   through the existing planar-YUV frame model. The typed
   `h264.hw.required` refusal flips to a real configure attempt only when
   an adapter registers — the audio ADR-005 seam/adapter split, applied to
   video. The software-decode lane (issue #2's other record) independently
   fills the hw-less fallback behind the same seam; the two lanes never
   share code below it.
2. **Unsafe boundary discipline (AGENTS §3.3).** All FFI lives in **one
   L5 platform adapter crate** (`platform-linux`-class, never imported by
   L0–L4): the chosen access path is the crate's *only* unsafe dependency;
   the adapter exposes safe, typed, fallible APIs; documented invariants
   name ownership/lifetime of surfaces and buffers (VA-API returns
   driver-owned memory — copies or explicit DMA-BUF import contracts, never
   borrowed pointers crossing the boundary); a fuzz/dirty-world harness at
   the parse→adapter edge (D-4); the adapter carries its own conformance
   and leak tests, mirroring the ADR-005 safety-review pattern per doc 11
   §1.2 E2.
3. **Substrate choice = the open decision.** `libva` 0.1.5 (dlopen
   default, safe wrappers upstream) vs `libva-sys` 0.1.2 + our own thin
   safe layer (smaller trusted surface, more invariants we own) vs deferring
   the lane. The bindgen-at-build chain of `libva` is the same
   consideration ADR-005's cpal record flagged for coreaudio-sys: CI needs
   libva headers, and generated bindings must be reviewed like source.
4. **CI conformance.** Admission evidence before any `partial` claim: (i)
   the H.264/HEVC hw-path corpus decoded through the adapter on a
   GPU-carrying runner (or honestly recorded as untested — MP §5); (ii)
   differential frame-hash comparison vs the software oracle lane (the
   rust_h264 admission plan's ffmpeg differential, giving us a
   hw-vs-sw cross-check for free); (iii) capability probe reporting —
   per-driver profile coverage surfaced through the truthful-reporting
   contract (R-25: the UI/matrix badge must say hw when hw is active, per
   MP §2); (iv) driver-variance ledger kept per R-15 (Intel/AMD at
   minimum, NVIDIA noted as its own revert-class question).

## Options

**(a) `libva` 0.1.5 as substrate, dlopen default.** Pros: safe wrappers
exist upstream; dlopen keeps link-time C out of our build; active cros-libva
lineage (ChromeOS). Cons: bindgen build-dep chain (CI header dependency);
invariants written by others; API breadth we do not control.

**(b) `libva-sys` 0.1.2 + our own safe layer.** Pros: minimal trusted
surface, every invariant ours and documented in the adapter ADR — the
closest match to AGENTS §3.3's discipline. Cons: we re-write what (a)
already has; cc/pkg-config build chain still present; more unsafe for us
to own, not less.

**(c) Direct VA-API without a crate.** Not viable: the API is a C ABI over
driver .so's; no metadata-generated binding path exists (unlike Windows'
`windows`-crate route in ADR-001).

**(d) Defer VA-API; land Media Foundation/VideoToolbox first (7.B1
sequence argument inverted).** Pros: the Windows lane has the
metadata-bindings posture ADR-001 already blessed; defers SV-28. Cons:
Linux is the platform where the hw-only posture bites hardest
(no hw ⇒ no H.264/HEVC at all, since the software lane is also
unadmitted); R-01/R-02 stay at full exposure on the free-desktop flagship.

## Decision [PROPOSAL — pending Lead sign-off]

**No decision is made in this file.** The recommended shape, for the Lead
to accept or amend when card 7.B1/7.B2 dispatches: **option (a)** as the
working substrate candidate — `libva` 0.1.5, default features (dlopen),
admitted only into the L5 adapter crate behind the seam per the design
above — **with option (b) retained as the fallback** if the bindgen chain
or upstream-invariant review fails at admission time, and the audit
preconditions of `records/h264-software-decoder-audits.md` (R9 tarball
sha256, RustSec, publisher pins) re-run for whichever crate is chosen.
Nothing here authorizes importing either crate today.

## Consequences

- On acceptance, the adapter card (7.B1/7.B2 Linux half) dispatches with
  this ADR as its safety-review template; `platform-hwdecode-h264` /
  `-hevc` Linux cells move `not_started → in_progress`, and only reach
  `partial` on CI conformance evidence (never on "it compiled").
- SV-28 resolves as: **VA-API = E2 OS-service exception, adapter-gated**
  (userspace-library-over-driver reading, documented here).
- R-15's driver-variance ledger becomes a named deliverable of the adapter
  card; per-GPU CI divergence is its standing trigger.
- The hw lane and the software lane (records/h264-software-decoder-audits.md)
  are complementary mitigations of R-01/R-02 — neither closes either row
  alone; ADR-010 Q2/Q3 still gate any *distribution* of the resulting
  binary.
- ADR-001 is amended by reference only (its per-API audit list gains a
  pointer here); Media Foundation / VideoToolbox lanes get their own
  briefs — this file owns Linux only.

## Sign-off gate (what "accepted" requires)

1. Lead picks the substrate option and the adapter crate boundary; records
   it in this file's Decision section with the date.
2. The admission record for the chosen crate lands in
   `docs/dependencies/records/` (full R1–R10, from the admission-time
   lockfile — the 2026-11-09 throwaway graphs are evidence, not the
   admission).
3. deny.toml + workspace edges updated in the same change that lands the
   adapter crate — never before the ADR is accepted.

## Revisit triggers

- Any `libva`/`libva-sys` release that changes the build chain (bindgen
  removal or addition) re-opens the substrate question.
- NVIDIA-on-VA-API (or a VDPAU revival need) re-opens the lane scope.
- Driver-variance ledger divergence beyond Intel/AMD envelopes re-scores
  R-15.
- The software lane's admission (rust_h264 track) changes the fallback
  calculus — hw-less Linux stops being "no playback", which may reorder
  the 7.B sequence but does not remove the truthful hw-reporting duty.
