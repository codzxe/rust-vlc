# ADR-010: Codec Patent & Redistribution Policy — the shipping matrix (draft brief; decision deferred to user + counsel)

- **Status:** **proposed** (drafted 2026-11-09, issue #24 / task 7.w2h — this file
  does NOT decide anything; it is the decision-ready record: evidence, options,
  one recommended posture). Decision due **before any binary-distribution plan**;
  hard trigger = Phase-8 packaging cards 8.E1–8.E3 (doc 13 §7), and per R-03
  (doc 14) the gate binds "before **any** public binary/artifact" — which also
  covers a publicly shipped web bundle (card 7.K1) if that precedes Phase 8.
- **Deciders:** user + counsel (doc 14 §6 escalation row: "Patent-risk
  acceptance or build-splitting … deadline: before any public binary/artifact").
  The Lead and agents assemble evidence; **nobody in this repo may make a legal
  determination** — every patent-status line below is either cited to a repo doc
  or tagged `[VERIFY: counsel]`.
- **Evidence:** the implementation inventory + patent-posture table in this
  file (per-codec in-repo file citations); doc 11 §8.3 (the codec patent risk
  register this ADR operationalizes — refreshed by the same change);
  doc 04 §3.1/§4.1 (codec status tables); `docs/feature-matrix.toml` codec
  rows; `docs/formats/*.md` patent notes (audio-ac3, audio-dts, video-hevc,
  video-mpeg4asp, video-ffv1, video-wmv-vc1, disc-dvd-video);
  `docs/dependencies/records/*.md` (admission records incl. the three BLOCKED
  validate-then-admit candidates of the issue-#7 admissions wave:
  oxideav-ac3, oxideav-mpeg4video, truehd); MP §14; R-01/R-02/R-03 (doc 14);
  D-009 (ROADMAP §3 — MIT license, patent-grant trade-off).
- **Supersedes:** nothing — fills the ADR-010 slot registered as *planned* at
  doc 01 §7 since Phase 0 ("assess patent risk separately from source licenses
  (MP §14); define the shipping matrix"; options listed there: ship everything
  / split builds / regional builds).
- **Consequence for risks:** R-03 stays **open** (exposure 9) until counsel
  sign-off; this file moves it from "ADR-010 not opened" to "drafted, evidence
  complete, awaiting the user+counsel gate". R-01/R-02 mitigation lines cite
  this file for the distribution half of their story.

## Context

MP §14 fixes the separation this ADR owns: *"Codec patent and redistribution
risk is assessed separately from source licenses."* Doc 11 §8.3 states the
mechanism: source licenses ≠ patent licenses; a permissively licensed (or
clean-room self-built) decoder can still embody patented techniques, and
distributing a decoder can trigger pool licensing. R-03 (doc 14, exposure 9)
names the consequence: a binary containing H.264/HEVC/AC-3/DTS-class decoders
"may be undistributable in some jurisdictions regardless of how clean the code
is" — a legal ceiling on *distribution*, not a missing feature.

What the repo's clean-room rule (AGENTS.md §3.1) does and does not buy: it is
evidence **for no copyright contamination** (independent implementation from
public specifications and observed behavior — doc 11 §8.2: "copyright covers
expression, not functionality"). It says **nothing about patents** — doc 11
§8.3's opening rule applies regardless of how the decoder came to exist ("a
permissively licensed decoder can still embody patented techniques;
distributing a decoder can trigger pool licensing"), and the format records
state it plainly: "Pure Rust does not clear patents (MP §14)"
(`docs/formats/audio-dts.md`). The §9 dependency audit is likewise
orthogonal: it governs language and dependency hygiene, not patent exposure.

Current distribution state (2026-11-09): **no binaries are distributed**. The
repo is source-only (MIT, ADR-006/D-009); CI artifacts are internal evidence;
no installer, app bundle, AppImage or published web bundle exists (Phases 7–8
not started, ROADMAP header). Nothing has needed this decision yet — which is
why it could stay a `[PROPOSAL]` register row through Phases 0–6. That grace
period ends at Phase 8 (cards 8.E1–8.E3), and doc 11 §9.3.5 already obligates
the release checklist to attach "ADR-006/010 shipping matrix" to release
notes.

## The evidence base

### 1. Implementation inventory — what a build contains today (file evidence)

`crates/codecs` (L3, doc 01 §2.5) compiles **unconditionally today** — per-codec
Cargo features "arrive with the registration card" (its `Cargo.toml` header;
module docs in `src/video/mod.rs` + `src/audio/mod.rs`). Everything below ships
in *every* current build — `apps/cli` directly, `apps/desktop` transitively
via `playback-engine` (all three Cargo.tomls carry the edge):

**Software decoders implemented (decode real media):**

| Codec | Implementation (in-repo evidence) | Provenance |
|---|---|---|
| Motion-JPEG / baseline JPEG | `crates/codecs/src/video/mjpeg/**` (card 2.B4; in-house per D-018) | in-house, clean-room |
| AV1 | `crates/codecs/src/video/av1/**` + `rav1d =1.1.0` asm-off (card 5.E1; `docs/dependencies/records/rav1d-1.1.0.md`) | admitted dep (BSD-2) + our OBU framing |
| VP9 | `crates/codecs/src/video/vp9/**` + `rusty_vp9 =0.1.3` (card 6.M-vp9; `docs/dependencies/records/rusty_vp9-0.1.3.md`) | admitted dep + our contract surface |
| DNxHD/DNxHR (VC-3) | `crates/codecs/src/video/dnx/**` + `filmcraft-dnx =0.9.0` (card 6.M-dnx + 6.VTA-DNX-RATIFY; `docs/dependencies/records/filmcraft-dnx-0.9.0.md`) | admitted dep (MIT OR Apache-2.0) |
| PCM family | `crates/codecs/src/audio/pcm/**` (card 2.B5; in-house per D-018) | in-house |
| ADPCM ×7 (MS/IMA-WAV/Yamaha/OKI/SWF/QT/Westwood) | `crates/codecs/src/audio/adpcm/**` (cards 5.J2a/5.J2b; `docs/formats/audio-adpcm.md`) | in-house |
| MP3 (+MP2 engine) | `crates/codecs/src/audio/mp3/**` + `symphonia-bundle-mp3 =0.6.1` (card 5.A3; record `symphonia-mp3-0.6.1.md`) | admitted dep (MPL-2.0) + our glue |
| MP2 | `crates/codecs/src/audio/mp2/**` (card 5.J3b; same engine) | admitted dep + our Layer II gate |
| AAC-LC | `crates/codecs/src/audio/aac/**` + `symphonia-codec-aac =0.6.1` (card 5.C2a; record `symphonia-aac-0.6.1.md`) | admitted dep + our ADTS parse |
| FLAC | `crates/codecs/src/audio/flac/**` + `claxon =0.4.3` (card 5.A1; record `claxon-0.4.3.md`) | admitted dep (Apache-2.0) |
| ALAC | `crates/codecs/src/audio/alac/**` + `symphonia-codec-alac =0.6.1` (card 5.J3a; record `symphonia-alac-0.6.1.md`) | admitted dep + our cookie parse |
| Vorbis | `crates/codecs/src/audio/vorbis/**` + `symphonia-codec-vorbis =0.6.1` (card 5.B2; record `symphonia-vorbis-0.6.1.md`) | admitted dep + our headers/seek |
| Opus | `crates/codecs/src/audio/opus/**` + `ruopus =0.1.2` (card 6.M-opus; record `ruopus-0.1.2.md`) | admitted dep + our OpusHead surface |

*Matrix drift noted honestly:* the `codec-audio-opus` and `codec-video-vp9`
rows in `docs/feature-matrix.toml` still read `blocked` (and the
ProRes/DNxHD row `not_started`) — last refreshed at the Phase-5 exit gate
5.R2-V, commit 3116aec — while the 6.M admissions above landed after — the
file evidence in this table is the ground truth; the matrix refresh belongs
to the admissions-wave follow-up (issue #7), not this ADR.

**Parsers / seams without software decode (no decoder binary surface, but
present in every build):**

| Codec | In-repo surface (evidence) | Decode posture |
|---|---|---|
| H.264/AVC | `crates/codecs/src/video/h264/**` — NAL/SPS/PPS/slice/SEI parser (5.F1) + hw seam (5.F2); the one `Decoder` impl fails configure with typed `h264.hw.required` `Unsupported` when no accelerator | **no software decode** (`h264/mod.rs` honesty boundary; doc 04 §4.2) |
| HEVC/H.265 | `crates/codecs/src/video/hevc/**` — VPS/SPS/PPS parser (5.G1) + hw seam (5.G2); typed `hevc.hw.required` | **no software decode** (`hevc/mod.rs`; doc 04 §4.3) |
| MPEG-1/2 video | `crates/codecs/src/video/mpeg12/**` — sequence-layer facts only (5.H1); decode typed-refused | no decode (doc 04 §4.1; `docs/formats/video-mpeg12.md`) |

**Hardware decode path (ADR-001):** H.264/HEVC (and VC-1/WMV, MPEG-2 where
the OS exposes them) are *planned* to decode via OS adapters (Media
Foundation / VideoToolbox / VA-API) in Phase-7 cards 7.B1–7.B3 — **no adapter
has landed**; the feature-matrix hw rows are `partial` on seam-only evidence.

**Pending admissions (the issue-#7 wave — none admitted, all currently
BLOCKED at the gate):** AC-3/E-AC-3 (`oxideav-ac3 0.0.11` — differential
gate fails, `docs/dependencies/records/oxideav-ac3-0.0.11.md`), MPEG-4 ASP
(`oxideav-mpeg4video 0.1.8` — differential gate fails,
`records/oxideav-mpeg4video-0.1.8.md`), TrueHD (`truehd 0.7.2` — compile
block, `records/truehd-0.7.2.md`). DTS has no candidate
(`docs/formats/audio-dts.md`). H.264/HEVC software candidates (rusty_h264 /
rust_h265 families) sit behind ADR-001's conformance-gated track
(`docs/feasibility/sections/video-codecs.md` §2). Of these families, the
AC-3 and MPEG-4 ASP format records already name **this ADR's patent gate as
a precondition of admission** ("ADR-010 patent registration first",
`docs/formats/audio-ac3.md` §Next-actions; "ADR-010 patent gate before
the first public binary", `docs/formats/video-mpeg4asp.md` §Plan); the
TrueHD record gates on its technical block only — its patent duty rests on
doc 11 §8.3's register row ("register before any self-build lands"), which
this ADR now extends to any admission, same as AC-3.

### 2. Per-codec patent posture — repo-asserted statements ONLY

Every line below either cites the repo doc that asserts it or is
`[VERIFY: counsel]` because the repo is silent. Nothing here is a legal
conclusion; the register itself carries doc 11 §8.3's banner: "every
pool/status line is `[VERIFY: current pool membership, rates, and jurisdiction
reading with counsel]`; this is a flag-for-legal-review, not a legal opinion."

| Family | Repo-asserted posture (citation) | Proposed default-build class [PROPOSAL] |
|---|---|---|
| H.264/AVC | multiple pools cover *implementers/distributors* of decoders (doc 11 §8.3) — full detail with counsel | excluded (hw path if counsel clears Q3) |
| HEVC | "notoriously fragmented (3+ pools + individual licensors)… worse than AVC"; distributing an HEVC decoder *at all* is a §14 item (doc 11 §8.3; doc 04 §4.3) | excluded |
| MPEG-2 | pool "largely expired (2019+)" `[VERIFY]` (doc 11 §8.3) | excluded until Q4 |
| MPEG-4 ASP | "ASP pool status varies" `[VERIFY]` (doc 11 §8.3); no royalty-free pledge exists (`docs/formats/video-mpeg4asp.md`) | excluded until Q5 |
| AAC | "active pools (Via Licensing AAC) covering decoders" `[VERIFY]` (doc 11 §8.3) — row already orders "flag before first public binary release" | excluded until Q6 |
| MP3 | "principal patents expired (~2017)" `[VERIFY: tail patents]` (doc 11 §8.3) | included, pending Q7 |
| Opus | "royalty-free: IETF RFC 6716; contributors granted royalty-free licenses; Cisco/MS patent commitments" `[VERIFY]` (doc 11 §8.3) | included, pending Q8 |
| VP8/VP9 | "Google's irrevocable royalty-free patent pledge" `[VERIFY: scope]` (doc 11 §8.3) | included, pending Q9 |
| AV1 | "AOMedia member royalty-free grant with defensive-termination clause" `[VERIFY]` (doc 11 §8.3) | included, pending Q10 |
| FLAC / Vorbis | "no known patent claims (Xiph's stated position)" `[VERIFY]` (doc 11 §8.3) | included, pending Q11 |
| AC-3/E-AC-3 | "Dolby/DTS licensing regimes" `[VERIFY]` (doc 11 §8.3); "core AC-3/A/52 patents have largely expired while E-AC-3 carries later filings" — widely reported, not verified by us (`docs/formats/audio-ac3.md`) | excluded until Q12 |
| DTS | same register row; "no expiry analysis for DTS has been done — the honest posture is encumbered until legal review says otherwise" (`docs/formats/audio-dts.md`) | excluded until Q13 |
| TrueHD/MLP | same register row (doc 11 §8.3) | excluded until Q14 |
| DNxHD/HR (VC-3) | **repo silent** `[VERIFY: counsel]` — the admission record covers license/purity only (`records/filmcraft-dnx-0.9.0.md`) | excluded until Q15 |
| MJPEG/JPEG | **repo silent** `[VERIFY: counsel]` | included, pending Q16 |
| ADPCM family | **repo silent** `[VERIFY: counsel]` | included, pending Q17 |
| ALAC | **repo silent** `[VERIFY: counsel]` | included, pending Q18 |
| PCM/uncompressed | **repo silent** `[VERIFY: counsel]` (no technique assertion expected; confirm the negative) | included, pending Q19 |
| FFV1 | "RFC 9043 informational, believed unencumbered" `[VERIFY at ADR-010]` (`docs/formats/video-ffv1.md`) | not implemented; Q20 before any admission |
| VC-1/WMV | "VC-1 pool / MPEG-LA exposure is an ADR-010 checklist item" `[VERIFY at ADR-010]` (`docs/formats/video-wmv-vc1.md`) | not implemented; Q21 |
| DVD CSS | "CSS descrambling adds regional circumvention-law exposure (ADR-010 input)" (`docs/formats/disc-dvd-video.md`) | out of scope here — feeds ADR-008; Q23 |

The "proposed default-build class" column is **[PROPOSAL] in every row** — it
is the engineering reading of what the repo docs assert, not counsel's answer.
Tier A = repo-asserted royalty-free/expired/silent-with-no-known-claims
(Opus, VP8/VP9, AV1, FLAC, Vorbis, MP3, MJPEG, ADPCM, PCM, ALAC);
Tier B = repo-flagged encumbered-or-unclear (H.264, HEVC, AAC, AC-3/E-AC-3,
DTS, TrueHD, MPEG-2, MPEG-4 ASP, VC-1, DNxHD pending Q15). Counsel may move
any row; the build system (below) makes each row independently switchable so
the final matrix is a configuration decision, not a code rewrite.

## Options (the doc 01 §7 set + R-03's contingency)

**(a) Split builds — default = Tier A only; Tier B behind optional features /
plugin builds.** Default distributed binaries compile only Tier A codecs;
Tier B codecs sit behind non-default Cargo features (self-build from source)
and/or separately distributed builds. Build system: exactly the per-codec
feature work the repo already plans — doc 01 §2.5(a) "one module per codec
behind Cargo features"; doc 06 §4.1's `#[cfg(feature = …)]` registration
pattern with `plugins/builtin/*.toml` manifest bijection (the
`tools/feature-audit` bijection checker is still `[PROPOSAL]`/unbuilt —
status note 2026-11-09 — and lands with the registration card). Disabled
codecs surface as typed errors, never silent skip — the established pattern:
`h264.hw.required`/`hevc.hw.required` `Unsupported` (crates/codecs), the
always-refusing `open_webview()` → `HostError::WebviewPending` (ADR-004),
audio's `audio.os_backend_pending` (ADR-005). Phase 8: 8.E1–8.E3 package the
default set only; §9.3.5's release-notes matrix states exactly which decoders
the artifact contains (the parity tool's honest-claims rule, doc 10 §7).
User impact: default download plays the Tier A set; Tier B media requires an
optional build or plugin — feature-matrix rows say `blocked: not in default
build (ADR-010)` with the reason.

**(b) Region-gated distribution.** Ship Tier B only in jurisdictions counsel
clears (doc 11 §8.3: "regional variance recorded"). Build system: same
feature split as (a) plus a region matrix (build targets × feature sets ×
distribution channels) and per-region release notes; doc 11 §8.3's
jurisdiction warning (exhaustion, offering-for-sale, utility-model regimes)
means every region is its own legal read (Q22). Phase 8: packaging matrix
multiplies (8.E1–8.E3 × regions). User impact: fragmented download pages,
geo-fencing, higher confusion and support cost.

**(c) Source-only posture.** Distribute no binaries at all; users build from
source with the feature set they choose (R-03's contingency: "source-only
distribution with build-it-yourself codec crates"). Phase 8 packaging shrinks
to checksums + reproducible-build docs (doc 11 §9.3.6's long-term
reproducible-builds line). User impact: maximal friction — contradicts the
product goal of an installable desktop player; kept as the fallback if counsel
cannot clear even Tier A or the org accepts no patent risk.

**(d) Ship everything in one binary.** The status-quo build (all admitted
codecs unconditional) distributed as-is. Rejected as a *recommendation* by
the repo's own register: R-03's mitigation is "no public binary distribution
until ADR-010 passes", and doc 11 §8.3's AAC row already flags "before first
public binary release". Retained as an option only because doc 01 §7 lists
it — it requires counsel to accept pool-licensing exposure for every Tier B
codec in every target jurisdiction (Q22) before a single artifact ships.

## Decision [PROPOSAL — pending user + counsel sign-off]

**Option (a), split builds with a Tier-A default**, with (b)'s regional
variance recorded per market **only if** counsel returns jurisdiction-split
answers (Q22), and (c) held as the standing contingency. Justification from
the repo's own architecture, not from any legal view:

1. **It is the build-system shape the repo already committed to.** Doc 01
   §2.5(a) and doc 06 §4.1 define per-codec Cargo features +
   manifest-registry registration as *the* codec onboarding pattern; the only
   work ADR-010 adds is choosing which features are default-on in distributed
   builds. The registration card is already owed before Phase 8 (R3, per
   `crates/codecs/Cargo.toml` header).
2. **It preserves the honest-reporting spine.** MP §5 ("feature exposure
   lists only implementations that exist"), MP §17 blocked-item rule, and the
   typed-refusal precedents (ADR-004/005, the h264/hevc hw-required errors)
   give disabled codecs a truthful, testable user-facing story — the parity
   report states exactly which decoders the artifact contains (doc 11
   §9.3.5).
3. **It matches how the risk is shaped.** The repo's flag list concentrates
   Tier B exposure in codecs that are *already* excluded from software decode
   (H.264/HEVC hw-only) or *not yet admitted* (AC-3/DTS/TrueHD/ASP) — the
   only implemented Tier B codec today is **AAC-LC** (plus DNxHD pending
   Q15). The default-build switch is therefore small, incremental, and
   reversible per codec as counsel answers arrive.
4. **It keeps Phase 8 unblocked on engineering.** 8.E1–8.E3 can package the
   Tier A default as soon as this ADR is accepted; no regional
   infrastructure is on the critical path, and each counsel answer (Q-ladder
   below) can flip one feature flag without re-opening the ADR.

**This ADR is NOT accepted.** Until the sign-off gate below passes, the
interim posture stands unchanged and binding (R-03): **no public binary
distribution of any build, Tier A included.**

## Consequences

- The sign-off gate (below) becomes a Phase-8 entry condition; card 8.E1's
  first AC references this ADR's accepted state.
- The per-codec feature split (registration card R3 + doc 06 §4.1 bijection
  tooling) gains an ADR-010 acceptance criterion: default feature set ==
  the accepted Tier A; every Tier B codec compiles only behind an explicit
  non-default feature; a CI check asserts no Tier B codec is reachable from
  the default feature set `[PROPOSAL: extend tools/feature-audit scope]`.
- `docs/feature-matrix.toml` gains a distribution column (or per-row note)
  recording default-build membership per codec once accepted
  `[PROPOSAL]` — owned by the matrix-maintenance card, not this ADR.
- Doc 11 §8.3 (same change) now points here and refreshes its exposure
  column to the 2026-11-09 implementation reality (AAC-LC, Opus, VP9, DNxHD
  implemented; AC-3/DTS/TrueHD/ASP still unadmitted).
- R-03 remains open; on acceptance it moves to mitigating (the matrix being
  the mitigation), and closes only when counsel clears the final ledger.
- If counsel moves a codec across tiers, the change is a feature-flag +
  matrix-row edit + ADR amendment (like ADR-005's per-target substrate
  amendments), not a new ADR.

## The counsel question ledger (the [VERIFY] set — 24 open items)

Every question maps to a repo citation or a repo silence; none is answered
here.

| # | Question for counsel | Source |
|---|---|---|
| Q1 | H.264/AVC pool status: current membership, rates, jurisdiction reading | doc 11 §8.3 |
| Q2 | HEVC: pool fragmentation; is distributing *any* HEVC decoder (even hw-mediated) licensable, and on what terms | doc 11 §8.3; doc 04 §4.3 |
| Q3 | HW-mediated decode: does OS-decoder use shift distributor exposure to the OS vendor, per jurisdiction | doc 11 §8.3 H.264 row |
| Q4 | MPEG-2 pool expiry confirmation | doc 11 §8.3 |
| Q5 | MPEG-4 ASP pool status | doc 11 §8.3 |
| Q6 | AAC pool status (Via Licensing AAC) incl. LC-only scope | doc 11 §8.3 |
| Q7 | MP3 tail patents beyond the ~2017 principal expiry | doc 11 §8.3 |
| Q8 | Opus royalty-free commitments: scope of RFC 6716 + Cisco/MS commitments | doc 11 §8.3 |
| Q9 | VP8/VP9: scope of Google's royalty-free pledge | doc 11 §8.3 |
| Q10 | AV1: AOMedia grant scope + defensive-termination effect on us | doc 11 §8.3 |
| Q11 | FLAC/Vorbis: confirm no known claims (Xiph position) | doc 11 §8.3 |
| Q12 | AC-3/E-AC-3: Dolby regime; verify the "AC-3 largely expired / E-AC-3 later filings" split | doc 11 §8.3; `docs/formats/audio-ac3.md` |
| Q13 | DTS: regime + expiry analysis (none done in-repo) | `docs/formats/audio-dts.md` |
| Q14 | TrueHD/MLP: Dolby regime | doc 11 §8.3 |
| Q15 | DNxHD/HR (SMPTE ST 2019-1 VC-3): any pool/claims (repo silent) | repo silence; `records/filmcraft-dnx-0.9.0.md` |
| Q16 | MJPEG/baseline JPEG: any live claims (repo silent) | repo silence |
| Q17 | ADPCM variants: any live claims (repo silent) | repo silence |
| Q18 | ALAC: any live claims (repo silent) | repo silence |
| Q19 | PCM/uncompressed: confirm the negative (repo silent) | repo silence |
| Q20 | FFV1 (RFC 9043): confirm "believed unencumbered" | `docs/formats/video-ffv1.md` |
| Q21 | VC-1: pool status | `docs/formats/video-wmv-vc1.md` |
| Q22 | Jurisdiction framework: exhaustion, offering-for-sale, utility models; does source-only distribution reduce exposure and by how much; which regions differ | doc 11 §8.3 |
| Q23 | DVD CSS descrambling: circumvention-law exposure by region (feeds ADR-008) | `docs/formats/disc-dvd-video.md` |
| Q24 | MIT's lack of an explicit patent grant: does our exposure profile warrant adding Apache-2.0 (D-009's recorded revisit trigger) | D-009 (ROADMAP §3); ADR-006 |

## Sign-off gate (what "accepted" requires)

1. Counsel answers the ledger (Q1–Q24) or the user accepts residual risk per
   row, recorded in this file's tier table.
2. The user picks the option (a)–(d) and the final Tier A/B assignment.
3. The Lead flips this ADR's status to **accepted** with the decision date,
   and updates ROADMAP §6, doc 11 §8.3/§10, and the R-03 register row in the
   same change.
4. No public binary artifact ships before 1–3 complete (R-03's hard rule).

## Revisit triggers

- Any codec admission or hw-adapter landing (7.B1–7.B3, the issue-#7 wave)
  re-checks its tier before it joins a distributed build — the admission
  records already carry the precondition.
- A counsel answer that moves a codec's tier → feature-flag + matrix-row edit
  + ADR amendment (per-Consequences).
- Any pool/licensing event named by counsel (new pool, rate change, expiry)
  re-opens the affected rows only.
- First regional-distribution plan (if option (b) layers in) re-runs Q22 per
  region.
