# AUDIT: crates/codecs/src/video/hevc/decode.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 765 / 21 (no tests in-file; exercised by tests/hevc_hw_path.rs) |
| File verdict | REAL parser + AU assembler + hw-session seam; NO decode; UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — no `"hevc"` arm in apps/cli/src/pipeline_host.rs:241+ nor apps/web/src/host.rs:590+; `HevcDecoder` constructed only by crates/codecs tests (rg: 0 external refs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg (reachability, bridges); `./target/debug/deps/hevc_hw_path-c0de0d66a701dd51 --nocapture` → 17 passed (this file is the code under test); baseline mp4-smoke.md:57 hevc refusal |

## 1. One-paragraph verdict
Direct answer to the hostile question: there is NO HEVC decoder here —
parser-only plus typed refusal, exactly as the module doc admits ("no
software decode, by design and by ADR-001", :1-3; "zero adapters are
installed in-tree", :157). `HevcDecoder::configure` parses extradata
(hvcC/Annex-B), opens a hardware session through the `HevcHwBridge` plug,
and fails typed (`hevc.hw.required` / `hevc.hw.no_bridge`) when none exists
— which is always, today, because zero bridge impls exist outside test
fakes. `decode` is a real §7.4.2.4.4 access-unit assembler (picture-key
comparison + non-VCL-intervened boundary + delimiter/end-of-sample close)
feeding opaque AUs to the session; frames exist only if a session mints
them. The code is competent and its self-description is truthful. But none
of it is reachable: the shipped CLI refuses `hevc` one level earlier
(`cli.codec_unavailable` — baseline mp4-smoke.md:57), so this entire
typed-refusal architecture never executes in a shipped build.

## 2. What actually works (verified)
— verified by `./target/debug/deps/hevc_hw_path-c0de0d66a701dd51 --nocapture`
→ 17 passed / 0 failed (exit 0): configure-time hvcC seeding, in-band
activation, AU boundary detection (incl. all-IDR same-key streams), stale
drop, counted damage, typed hw failure mapping — all of it through this
file's code paths.

## 3. Findings

F-001 [S3]
Where: crates/codecs/src/video/hevc/decode.rs:363-373
Code:
```rust
    /// The "active" SPS for surface negotiation: the one referenced
    /// by the lowest-id known PPS (single-parameter-set streams —
    /// the whole corpus and virtually every real file — have exactly
    /// one; multi-PS streams re-sync on their in-band activations).
    fn active_sps(&self) -> Option<&HevcHwSps> {
        let pps = self
            .pps
            .values()
            .min_by_key(|p| p.pps.pic_parameter_set_id)?;
        self.sps.get(&pps.pps.seq_parameter_set_id)
    }
```
What is wrong: "active" SPS = SPS referenced by the lowest-id PPS. In a
multi-PPS/multi-SPS stream (resolution switching, layered encodes — real in
broadcast HEVC), the display size and pixel format negotiated with the
session are those of the lowest-id PPS's SPS, not of the picture actually
being decoded. "Re-sync on in-band activations" does not fix it: activation
calls `sync_hw_config`, which again picks `active_sps` by the same lowest-id
rule; the picture→PPS→SPS chain parsed per slice is never consulted.
Why it matters: wrong geometry/format fed to a real hw session on a
realistic stream class; the doc comment acknowledges the heuristic rather
than the per-picture truth.
What correct looks like: resolve the SPS from the AU's first VCL's
`slice_pic_parameter_set_id` when closing the picture.
Confidence: HIGH — Verification: read (no multi-SPS fixture exists to
execute; UNVERIFIED by execution).

F-002 [S3]
Where: crates/codecs/src/video/hevc/decode.rs (whole file) + baseline
Code (audit/runs/baseline/mp4-smoke.md:57):
```text
error: [decode.unsupported]: unsupported (cli.codec_unavailable): no decoder for `hevc` in this build (landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)
```
What is wrong: the entire Decoder impl — including its much-advertised
typed `hevc.hw.required` refusal — is dead code in shipped lanes. The CLI
host's `decoder_for` has no `"hevc"` arm (apps/cli/src/pipeline_host.rs
:241-346), the web host none either, so `HevcDecoder` is never constructed;
the user-facing refusal comes from the host fallthrough, not from this
module's carefully typed errors. 765 lines + the whole parser tree compile
into release binaries that can never call them.
Why it matters: dead weight; the "honest matrix row" this file implements is
superseded by a cruder provider-level refusal; every claim about typed
`hevc.*` errors is untestable from the product.
Confidence: HIGH — Verification: read + rg + baseline citation.

F-003 [S4]
Where: crates/codecs/src/video/hevc/decode.rs:683-694
Code:
```rust
                        let key = PictureKey::of(&slice);
                        self.pending_key = Some(key);
                        self.non_vcl_since_vcl = false;
                        if self.pending.len() < MAX_PENDING_UNITS {
                            self.pending.push(HevcHwUnit {
```
What is wrong: the pending picture's key advances even when the unit is
dropped at the `MAX_PENDING_UNITS` cap (the `else { self.skipped += 1 }`
branch). A first segment dropped at the cap still sets `pending_key`, so
(a) a later first segment with the identical key does NOT open a new
picture (boundary test at :663-668 compares keys only), and (b) the AU can
eventually close carrying continuation segments with no first segment.
Requires >4096 units in one picture (adversarial), but the damage model
then mis-buckets subsequent units instead of resetting.
Confidence: MEDIUM — Verification: read (mutation reasoning; not executed).

F-004 [S4]
Where: crates/codecs/src/video/hevc/decode.rs:440 and :748
Code:
```rust
                format: self.format.unwrap_or(media_types::PixelFormat::Nv12),
```
What is wrong: defensive default labels an un-negotiated frame Nv12. In the
current state machine `format` is always `Some` by the time frames emit
(`sync_hw_config` errors out on un-negotiable formats at :392, and a VCL
cannot be pending without an SPS having synced), so this is a dead branch —
but if the invariant ever breaks, a 4:4:4/10-bit frame would be silently
mislabeled Nv12 instead of refusing.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/codecs/src/video/hevc/decode.rs:473-486
Code:
```rust
    fn parse_vcl(&mut self, nal: &[u8]) -> Option<SliceSegmentHeader> {
        let mut candidates: Vec<(u8, SliceSegmentSpsInfo, SliceSegmentPpsInfo)> = Vec::new();
        for (pps_id, pps) in &self.pps {
```
What is wrong: per-VCL-NAL heap allocation and re-derivation of every PPS's
`SliceSegmentSpsInfo::from_sps` (which re-does CTB-grid arithmetic) on every
slice segment of every picture — O(PPS × slices) re-parsing per frame in the
hot path. Single-PPS streams (the norm) pay one small Vec + two seam
rebuilds per slice; acceptable but needless once contexts are static (they
change only on store_*).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_PENDING_UNITS / HW_COLOR consts | 86-93 | REAL | - | 4096-unit cap verified enforced at :457 and :686 |
| unsupported() | 96-107 | REAL | - | typed Unsupported with hevc FormatInfo |
| surface_format() | 112-122 | REAL | - | 8-bit 420→Nv12, 422→Yuyv422, mono→Nv12; 444/10-bit→None (typed refusal) |
| PictureKey::of | 139-151 | REAL | - | §7.4.2.4.4 comparison fields incl. poc-lsb/pic_output/no_output |
| Debug/Default | 196-235 | REAL | - | Debug exposes pending_units (unasserted by tests) |
| new / with_bridges | 241-253 | REAL | - | with_bridges only used by tests today |
| accepts | 257-263 | REAL | - | hevc/hvc1/hev1 + dotted forms |
| seed_extradata | 266-301 | REAL | - | hvcC sniff→parse→activate; Annex-B CodecPrivate walk |
| store_vps/sps/pps | 307-361 | REAL | - | byte-identity dedup; re-offer on change |
| active_sps | 367-373 | REAL (heuristic) | S3 | F-001 lowest-PPS-id rule |
| sync_hw_config | 376-409 | REAL | - | surface refusal `hevc.hw.surface_format`; conf-window typed |
| close_picture | 413-453 | REAL | - | non-VCL-only pending carries over; hw None guarded |
| push_opaque | 456-467 | REAL | - | bounded, sets non_vcl_since_vcl |
| parse_vcl | 473-501 | REAL | S4 | ambiguity refusal correct; per-call alloc (F-005) |
| parse_error / hw_error | 507-535 | REAL | - | DeviceLost→HardwareLost mapping verified by test |
| Decoder::configure | 538-578 | REAL | - | full state reset; typed refusals :570-571 |
| Decoder::decode | 580-731 | REAL | - | framing split, AU assembly, damage counting; F-003 cap wrinkle |
| Decoder::flush | 733-764 | REAL | - | close + session drain; Drained when empty |

## 5. Test honesty
No tests in-file; see tests/hevc_hw_path.rs audit (17/17 ran live; corpus
row skip-no-op; one tautological assert).
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no software decode ... typed Unsupported configure error ... never a silent skip" | decode.rs:1-15 | REAL | :570-571, :392; hevc_hw_path tests 17/17 |
| "zero adapters are installed in-tree" | decode.rs:157 | REAL | rg: no HevcHwBridge impl outside tests |
| "an adversarial stream must not grow the accumulator unboundedly" (MP §12) | decode.rs:86-88 | REAL (code) / UNPROVEN (test) | cap enforced :457/:686; test asserts only `contains("HevcDecoder")` |
| "Samplings with no vocabulary ... are refused with typed hevc.hw.surface_format" | decode.rs:61-65 | REAL | :392; no test exercises a 10-bit/4:4:4 SPS through configure (UNVERIFIED by execution) |
## 7. Dead / duplicated / unreachable code
Entire file unreachable from shipped entry points (F-002). `with_bridges`
has no production caller. `unwrap_or(Nv12)` branches are dead-defensive.
## 8. Dependencies & flags
Depends on codec-api, media-types, in-house parser modules; unconditional
compile (no feature gate) — dead weight in every shipped binary.
## 9. Open questions for the next agent
- sps.rs (next): does `cropped_size()` overflow-check conformance-window
  arithmetic (sync_hw_config maps its Err — verify the error is reachable)?
- Would wiring HevcDecoder behind the CLI "hevc" arm be a small card, or is
  the provider-level refusal the intended product posture (feature-matrix
  row says hw-only)?
