# AUDIT: crates/codecs/src/video/hevc/hw.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 403 / 21 items in table (5 enums/structs of data, 6 feed types, 2 error enums + 2 Display impls, 2 traits, HevcHwPath + Debug + 5 methods) |
| File verdict | UNREACHABLE (a REAL, well-documented contract seam — zero shipped callers, zero in-tree session implementations) |
| Reachable from shipped entry point? | NO. `rg -rn "HevcDecoder"` over crates/+apps/ → definition (hevc/decode.rs), re-export (hevc/mod.rs:52-53), and `crates/codecs/tests/hevc_hw_path.rs` ONLY — the test FakeBridge/FakeSession. `apps/cli/src/pipeline_host.rs` has no hevc/hev1/hvc1 dispatch arm; product.toml:168 `hw-decode = false` + :191 `hw_decode_policy = "software-only"` keep the whole hw family off. The module's own doc admits the adapters are "*not built here*" (hw.rs:7-8) and that the only in-tree session is "the fake session of the integration suite" (hw.rs:243-245). Baseline behavioral proof of unreachability: `sample-hevc.mp4` → Opening → **Error** → Idle (INDEX.md corrected smoke matrix — no HEVC decoder of any kind answers). The seam's tests (17, incl. the fake session driving full AUs) pass in baseline: test-workspace.log.txt:1088-1104. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | full-file read; `rg -rn "HevcDecoder"` over crates/+apps/ (zero production callers); `rg -n "HevcHwPath\|HevcHwBridge\|HevcHwDecoder"` (definition + mod.rs re-exports + tests only); `rg -n "hevc\|hev1\|hvc1"` over apps/cli/src (no dispatch arm); sed of docs/feature-matrix.toml:435-443 (HEVC row: status partial, honest notes "hw seam delivered (5.G2), software decode absent"); baseline log sed :1088-1104 (hevc_hw_path 17/17 ok); doc-vs-code cross-check of every trait contract claim against decode.rs/tests. |

## 1. One-paragraph verdict
This file is the HEVC hardware-decode contract: parameter-set/access-unit feed types, a minimal synchronous session trait, a bridge-discovery trait, and a 12-line `HevcHwPath::open` walk — competently designed, honestly documented (ADR-001's "no software decode, never a fake success" posture is stated in the module header and enforced by the typed `NoAccelerator`/`NoBridge` errors), and mirroring the H.264 5.F2 seam W06-A07 audited. But it is scaffolding for adapters that do not exist: no bridge or session implementation lives in the tree (the only impls are the integration suite's fakes), no shipped build constructs `HevcDecoder` (the sole consumer of this seam), and the shipped binary answers an HEVC track with a typed Error (baseline smoke). The feature-matrix row is honest about this ("partial … hw seam delivered, software decode absent"). Findings: a structural one — the seam carries raw-NAL-plus-parsed-facts duplication with zero adapters to validate the shape against (design risk frozen into "stable" API before first use), the one-field `HevcHwDecoded` wrapper, the `first_vcl` helper with no in-tree caller outside tests, and a doc reference typo. No correctness defects: there is almost no executable code here, and what exists (`open`, `first_vcl`, Debug) is trivially correct.

## 2. What actually works (verified)
- `HevcHwPath::open` bridge walk (:356-371) — correct first-claim-wins iteration with the two documented typed refusals; verified against the 17 passing hevc_hw_path tests in baseline (test-workspace.log.txt:1088-1104: `alien_bridge_leaves_the_stream_unclaimed`, `accelerator_without_bridge_is_typed_no_bridge`, `no_accelerator_is_typed_unsupported_never_silent` — exactly this function's contract).
- The typed-error honesty boundary — verified behaviorally by the baseline smoke: `sample-hevc.mp4` → `Opening → Error → Idle`, EXIT 0 (INDEX.md corrected matrix); no fake success, matching the module doc's promise (hw.rs:10-13).
- The feed-shape documentation (raw NAL + parsed geometry, emulation prevention kept) cross-checked against the h264 hw seam's twin (hw/mod.rs — W06-A07) and the test session that consumes these types (tests pass).
- No unsafe, no panics, no arithmetic — the only executable logic is the 3-method delegating wrapper and the 12-line walk.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/hevc/hw.rs:239-247 (and the whole type surface :69-206)
Code:
```
/// **The 7.B adapter contract**: one hardware decode session for
/// HEVC, fed by the parser's output.
///
/// Implementations are per-OS adapter internals (VA-API,
/// VideoToolbox, Media Foundation — Phase 7.B); the in-tree proof
/// is the fake session of the integration suite. The trait is
/// deliberately minimal and synchronous: the engine's decode worker
```
What is wrong: a 403-line "stable for them to build against now" (:23) API surface has never been built against by anything real — the sole implementations are test fakes, and the sole consumer (`HevcDecoder`) is itself unreachable. The H.264 twin has the same shape but at least has a drafted VA-API adapter session (h264/hw/vaapi/session.rs — W06-A09's chunked file) exercising the analogous contract; HEVC has none.
Why it matters: the doc promises API stability for adapters while zero adapters validate the design — the classic frozen-seam risk (any first real adapter will find the shape wrong: e.g. no error channel for "frame not ready yet, retry" — `decode` returns `Ok(HevcHwDecoded)` with an empty vec or `Err`, nothing in between; no surface-format/stride in `HevcHwFrame` beyond `size`, which no real renderer can present from an opaque u64). The project pays 403 lines of maintenance for a promise Phase 7 may never collect (ADR-013 still "[proposed]" per the matrix notes).
What correct looks like: either land the first adapter (the Linux VA-API draft per matrix notes) before declaring stability, or mark the module `#[doc(hidden)]`/explicitly experimental so "stable" is not claimed in doc comments ahead of first use.
Confidence: HIGH — Verification: read + repo-wide caller/impl sweeps (see header).

### F-002 [S4]
Where: crates/codecs/src/video/hevc/hw.rs:199-206
Code:
```
pub struct HevcHwDecoded {
    /// Frames ready this call.
    pub frames: Vec<HevcHwFrame>,
}
```
What is wrong: a single-field wrapper struct around `Vec<HevcHwFrame>` — every producer/consumer site must say `.frames` to reach the only data it holds. The h264 twin's session returns the vec directly in the same position.
Why it matters: pure API noise on a seam already criticized for premature stability claims (F-001); a future second field is speculation (YAGNI).
What correct looks like: `type HevcHwDecoded = Vec<HevcHwFrame>;` (or return the vec) until a real second output exists.
Confidence: HIGH — Verification: read (sole field; all uses are `.frames` accesses in tests).

### F-003 [S4]
Where: crates/codecs/src/video/hevc/hw.rs:172-178
Code:
```
    /// The first VCL unit's slice facts, when present.
    #[must_use]
    pub fn first_vcl(&self) -> Option<&HevcHwVcl> {
        self.units.iter().find_map(|u| u.vcl.as_ref())
    }
```
What is wrong: `first_vcl` scans for the first unit that HAS a vcl — that is "first unit with VCL facts", which for a conforming AU is the first VCL unit, but the method's contract is defined by iteration order over a field any caller mutates freely (`units: Vec<HevcHwUnit>` is public); with zero in-tree production callers, the semantic is unpinned outside tests.
Why it matters: trivial correctness today (the tests build AUs in stream order), but a public helper whose only contract lives in its name on an unreachable surface is doc-debt the first adapter will trip on.
What correct looks like: a doc line stating the precondition ("assumes `units` in stream order; returns the first VCL unit's facts") or construction-time invariant.
Confidence: HIGH — Verification: read (callers: tests only — repo sweep in header).

### F-004 [S4]
Where: crates/codecs/src/video/hevc/hw.rs:332-334
Code:
```
/// The consumption point: an open hardware session fed by the
/// parser's output. Owns the [`HevcHwDecoder`] the bridge minted;
/// [`super::decode::HevcDecoder`] drives it.
```
What is wrong: the doc-link target is written as `super::decode::HevcDecoder`, but from this module's scope `super` is `hevc`, and the item is `hevc::decode::HevcDecoder` — however rustdoc resolves `super::decode` here, the path is inconsistent with the sibling references at :12 which say [`super::decode`] (module). Cosmetic doc-path wobble on an otherwise meticulously cross-referenced file.
Why it matters: doc-link drift is the thin end of the same honesty wedge the rest of this audit enforces; broken intra-doc links fail `cargo doc`'s linkcheck lane if one exists.
What correct looks like: `[`HevcDecoder`](super::decode::HevcDecoder)` consistently, or re-export once and link the re-export.
Confidence: MEDIUM — Verification: read (path-shape mismatch between :12 and :334; rustdoc resolution not run — no build allowed).
Note: a Bash `rg` display artifact rendered the same line as `super::decode::n` in one sweep; the Read tool shows the true bytes `super::decode::HevcDecoder` — quote above is from the Read tool, per the wave-5 lesson recorded in W05-A07's log.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum HevcFraming` | 72-83 | REAL | — | data; AnnexB vs LengthPrefixed{length_size} mirrors the h264 twin |
| `struct HevcHwVps/Sps/Pps` | 88-112 | REAL | — | data; raw NAL + parsed struct pairs (doc: framing stripped, EPB kept — the form OS APIs take) |
| `struct HevcHwStreamInfo` | 116-130 | REAL | — | data; Default derive; "most recent activation last" ordering contract is doc-only (nothing enforces it in-tree) |
| `struct HevcHwVcl` | 136-140 | REAL | — | single-field wrapper (nal projection folded into SliceSegmentHeader — documented HEVC-vs-H.264 difference, consistent) |
| `struct HevcHwUnit` | 143-152 | REAL | — | data |
| `struct HevcHwAccessUnit` + `first_vcl` | 157-178 | REAL | S4 | F-003 |
| `struct HevcHwFrame` | 189-197 | REAL | — | opaque u64 surface + size + pts; no format/stride (F-001's presentation-gap point) |
| `struct HevcHwDecoded` | 202-206 | REAL | S4 | F-002 one-field wrapper |
| `enum HevcHwError` + `Display` | 210-237 | REAL | — | three-case typed failure; honest Display strings |
| `trait HevcHwDecoder` | 248-277 | REAL (contract) | — | backend_id/configure/decode/flush; "stays usable after Decode error" contract doc-only; no in-tree impl beyond fakes |
| `trait HevcHwBridge` | 290-298 | REAL (contract) | — | open() → Option walk participant; None = not mine |
| `enum HevcHwOpenError` + `Display` | 302-330 | REAL | — | the two honest refusals; Display strings state the ADR-001 posture verbatim |
| `struct HevcHwPath` + `Debug` | 335-347 | REAL | — | Debug prints backend label (truthful-report convention) |
| `HevcHwPath::open` | 356-371 | REAL | — | 12-line bridge walk; first-claim wins; verified by 3 baseline tests |
| `HevcHwPath::backend_id` | 374-377 | REAL | — | delegate |
| `HevcHwPath::configure/decode/flush` | 384-402 | REAL | — | three delegates; zero logic |

## 5. Test honesty (no tests in this file; the seam's tests live in tests/hevc_hw_path.rs — assessed for coverage of THIS file's surface)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `no_accelerator_is_typed_unsupported_never_silent` / `accelerator_without_bridge_is_typed_no_bridge` / `alien_bridge_leaves_the_stream_unclaimed` | YES | pin `open`'s two refusals + the decline walk |
| `annexb_stream_feeds_aus_and_emits_opaque_frames`, `multi_segment_picture_is_one_au`, `orphan_continuation_segment_is_counted_damage`, `corpus_0044_annexb_full_stream_survives…` | YES (fake-session scope) | drive the feed types + the trait contract through a fake — but prove nothing about any real adapter (none exists) |
| `corpus_0044_annexb_full_stream…` | NOT IN THIS SANDBOX | corpus-v4 absent — see nal.rs#part3 F-001 (same skip pattern) |
Baseline: 17/17 ok (:1088-1104). No mocks-of-the-subject problem: the subject here IS the contract; the fake is the other side of it — legitimate.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No software decode lives here or anywhere under `hevc`" | hw.rs:10-13 | REAL | no bitstream-decoding code under hevc/ (decode.rs delegates to the hw session or refuses); smoke: sample-hevc.mp4 → Error |
| "a machine without an OS decoder gets a typed `unsupported` error … never a fake success" | hw.rs:11-13 | REAL | HevcHwOpenError::NoAccelerator Display states it; baseline smoke shows the Error transition |
| "This is the surface 7.B adapters implement; it is stable for them to build against now" | hw.rs:22-23 | SHALLOW | zero adapters exist to build against it (F-001) — "stable" is a promise, not a state |
| matrix row: status partial, "hw seam delivered (5.G2), software decode absent" | docs/feature-matrix.toml:439-443 | REAL | matches code exactly — the row is honest |
| wave-6 hostile question | task prompt | N/A for HEVC | HEVC has no shipped decode at all (software or hw); the H.264 answer is in sw.rs.audit.md |

## 7. Dead / duplicated / unreachable code
- The ENTIRE module is unreachable (see header) — compiled unconditionally (in-house hevc, no feature gate), consumed only by the equally-unreachable decode.rs and by tests.
- Near-duplication with the H.264 5.F2 seam (h264/hw/mod.rs types H264Framing/H264HwSps/…): same shapes field-for-field modulo the documented HEVC differences (VPS type, no interlaced flag, folded NAL projection). Deliberate mirror design, cited both ways in docs — acceptable, but any contract fix must be applied twice (F-001 compounds).
- `first_vcl` has no production caller (F-003).

## 8. Dependencies & flags
- codec-api (`HwAccelerator`), media-types (`Stamp`), the in-house parser types (via `super::parser`), std (`fmt`, `sync::Arc`). No external deps, no unsafe, no feature gate. Lint-clean per baseline clippy (#5/#6).

## 9. Open questions for the next agent
- decode.rs owner (same tree): confirm `HevcDecoder::with_bridges`'s bridge list is always empty in production (no installed_bridges equivalent of the h264 lane) — my sweep shows tests-only construction.
- If Phase 7.B ever lands, re-audit the "stable" claim (F-001) against the first real adapter's needs — especially `HevcHwFrame`'s missing format/stride and the empty-vec-vs-Err lag channel.
