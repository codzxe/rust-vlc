# AUDIT: crates/codecs/src/video/mpeg12/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 185 / 8 items (enum + Display + Error impl, const, `accepts`, `decode_refusal`, Detail carrier ×3 items) + 4 tests |
| File verdict | UNREACHABLE (code real and correct; two headline doc claims FALSE) |
| Reachable from shipped entry point? | NO. Chain: `error.rs` items ← `facts.rs:19` (`use super::error::{accepts, decode_refusal}`) + bitreader/headers (`Mpeg12Error`) ← `Mpeg12Facts::scan` (facts.rs) ← **only** `crates/codecs/tests/mpeg12_corpus.rs` + `#[cfg(test)]` modules. `rg "Mpeg12Facts"` outside `crates/codecs/**` → no hits. The user-facing mpeg2video refusal is the CLI's own: `apps/cli/src/containers/ps.rs:845-848` asserts `cli.codec_unavailable` and `unreachable!("no mpeg2video decoder exists in this build")`. Matches W06-A04's own headers.rs#part1 conclusion and the facts.rs audit (W06 sibling). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:4 |
| Commands run | Read full file (185 lines); `rg "decode_refusal\|DESCRIPTORS\|mpeg12::error\|accepts\("` repo-wide (callers: facts.rs:19/261/288 + own tests; the `es.rs:456/486/655` hits are an unrelated local CLI test helper); `rg "Mpeg1video\|mpeg2video"` over `crates/demux/src` (TS :351-352 ✓, PS :771-784 ✓, MKV/MP4/AVI absent — F-002 evidence); `sed -n` of mkv/header.rs:775-830 (`codec_descriptor_for` — no V_MPEG1/V_MPEG2 arm, raw pass-through), mp4/mod.rs:437-476 (`codec_of` — no mpeg12 fourcc, `None` → trak skipped), avi/header.rs:217-222 (`StreamFormat::codec_descriptor` — MotionJpeg|Pcm only); `rg -A30 "pub enum H264Error"` (shape comparison, F-004); `rg "struct Detail\(String\)\|struct FeatureDetail"` (13 carrier sites, F-005); `rg "out of range \(\|marker bit violation\|truncated: needed"` (Display-string assertions: only 2 of 4 arms, F-006); `sed -n '840,850p' apps/cli/src/containers/ps.rs` (the real shipped refusal); `sed -n` quote verification on error.rs:1-11/70-73/81-91, ts/mod.rs:349-353; baseline test-workspace.log.txt:748-751 (4/4 tests ok), allfeatures :711-714, nofailfast :711-714; grep media-types lib.rs:47 (`ErrorKind` root re-export — doc link :8 resolves). |

## 1. One-paragraph verdict
The executable code here is small, clean, and does what it says: a 4-variant typed parse error (`Truncated`/`Marker`/`OutOfRange`/`Invalid`) with correct Display arms, a two-string descriptor gate (`accepts`), and a `decode_refusal` factory that builds a typed `ErrorKind::Unsupported` @ `mpeg12.decode.blocked` with the human reason in a source-carrier — all panic-free, all executed by 4 green baseline tests. But the module is unwired: nothing outside `crates/codecs` calls any of it, and the mpeg2video refusal users actually see is the CLI's own `cli.codec_unavailable`, not this one. On top of that, the module doc carries two FALSE headline claims: (1) that "the codec-api registry receives" this refusal and that "the sequence layer under this module is what probe/track-info consume" — no registry, no probe, no track-info touches this tree; (2) that "TS stream_type 0x01/0x02, PS/AVI/MP4/MKV mappings all normalize to these two strings" — true for TS and PS, false for AVI (MJPEG+PCM vocabulary only), MP4 (no mpeg12 fourcc; trak skipped), and MKV (no V_MPEG1/V_MPEG2 arm; raw CodecID pass-through). Plus four S4 nits: a broken intra-doc link, the "H.264 shape verbatim" overclaim, the 13th copy of the `Detail(String)` carrier, and a Display test that covers only 2 of 4 arms.

## 2. What actually works (verified)
- 4/4 tests executed ok — baseline test-workspace.log.txt:748-751 (`video::mpeg12::error::tests` ×4 ok; also allfeatures :711-714 and nofailfast :711-714).
- `accepts` gates exactly `mpeg1video`/`mpeg2video` — read + content-sensitive test (:134-140, five foreign refusals incl. case-sensitivity `"MPEG2VIDEO"`).
- `decode_refusal` produces `ErrorKind::Unsupported` with context `mpeg12.decode.blocked` and a downcastable `Detail` source — read of :91-112 + tests :143-165.
- The two descriptors match the two demux lanes that really do normalize to them: TS stream_type 0x01/0x02 (ts/mod.rs:351-352) and the PS 64-byte peek (ps/mod.rs:771-784).
- The doc-04 §4 characterization ("no validated pure-Rust engine exists; self-build is an L-sized … queued Phase 6+") matches the actual row: `| MPEG-1/2 video | P1 | missing | none (libmpeg2 is C) | everything (DVD/DVB workhorse) | L | 6 |` (docs/04-codec-format-roadmap.md §4 table, sed-verified).
- The `[`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)` doc link resolves — media-types/src/lib.rs:47 re-exports `ErrorKind` at root.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/mpeg12/error.rs:6-11 and :85-86
Code:
```
//! The one engine-facing conversion is the **decode-refusal gate**:
//! [`Mpeg12Error::decode_refusal`] builds the typed
//! [`ErrorKind::Unsupported`](media_types::ErrorKind::Unsupported)
//! that the codec-api registry receives for `mpeg1video` /
//! `mpeg2video` descriptors while software decode is `blocked` —
```
and
```
/// queued Phase 6+). The sequence layer under this module is what
/// probe/track-info consume; decode itself refuses loudly.
```
What is wrong: both consumer claims are phantoms. `rg` repo-wide: `decode_refusal`/`accepts` are called only from `facts.rs:19,261,288` (inside the same dead tree) and the module's own tests; `Mpeg12Facts` (the sole parent) has zero production callers. No codec-api registry, no probe, no track-info path references `mpeg12` at all. The refusal a shipped binary actually produces for an MPEG-1/2 track is the CLI's generic one — `apps/cli/src/containers/ps.rs:845-848`:
```
        match decoder_for(&opened, &video) {
            Err(err) => {
                assert_eq!(err.context.code.as_str(), "cli.codec_unavailable", "{err}")
            }
            Ok(_) => unreachable!("no mpeg2video decoder exists in this build"),
```
so "an honest, observable refusal" (:11) is observable only under `cargo test`.
Why it matters: the file's stated reason to exist — "the one engine-facing conversion" — is an integration that does not exist. A future agent wiring MPEG-1/2 support will start from a doc that mis-describes where the refusal surfaces and who consumes the facts.
What correct looks like: same one-sentence honesty fix as the rest of this tree: "unwired as of this card — the shipped refusal is the CLI's generic `cli.codec_unavailable`; this gate activates when the sequence layer is registered."
Confidence: HIGH — Verification: rg callers + ps.rs read + facts.rs/headers.rs sibling audits (independently re-verified by my own rg).

### F-002 [S3]
Where: crates/codecs/src/video/mpeg12/error.rs:70-72
Code:
```
/// The codec descriptors this module recognizes (the demuxers'
/// registry vocabulary: TS stream_type 0x01/0x02, PS/AVI/MP4/MKV
/// mappings all normalize to these two strings).
```
What is wrong: the claim is FALSE for 3 of the 5 named containers. Verified per mapping table:
- TS ✓ — `0x01 => "mpeg1video"`, `0x02 => "mpeg2video"` (ts/mod.rs:351-352).
- PS ✓ — `probe_video_codec` returns both strings (ps/mod.rs:771-784).
- AVI ✗ — the AVI demuxer's whole stream vocabulary is `StreamFormat::MotionJpeg | Pcm`; `codec_descriptor` (avi/header.rs:217-222) can emit only `"mjpeg"` or a `pcm_*` name. No MPEG-1/2 video mapping exists.
- MP4 ✗ — `codec_of` (mp4/mod.rs:437-476) has no `mpg1`/`mpg2`-class fourcc arm; unknown entries return `None` and "the trak is skipped with a damage note".
- MKV ✗ — `codec_descriptor_for` (mkv/header.rs:775-830) has no `V_MPEG1`/`V_MPEG2` arm; unknown ids "pass through **raw**", so a Matroska MPEG-2 track carries descriptor `V_MPEG2`, not `mpeg2video`, and this gate would refuse it as foreign.
Why it matters: the doc asserts a normalization contract that three demuxers violate; anyone relying on it (tests, wiring, UI descriptor display) gets raw/skipped tracks where the doc promises registry vocabulary.
What correct looks like: "TS stream_type 0x01/0x02 and the PS peek normalize to these two strings; AVI/MP4/MKV have no MPEG-1/2 mapping yet (raw/skip paths)."
Confidence: HIGH — Verification: sed of all three mapping tables.

### F-003 [S4]
Where: crates/codecs/src/video/mpeg12/error.rs:7
Code:
```
//! [`Mpeg12Error::decode_refusal`] builds the typed
```
What is wrong: broken intra-doc link. `decode_refusal` is a free function of this module (:91), not an associated function of the `Mpeg12Error` enum — the enum (read in full) has no impl block beyond `Display`/`Error`. The associated function the author meant is `Mpeg12Facts::decode_refusal` (facts.rs:260). Rustdoc reports `broken_intra_doc_links`.
Why it matters: the module doc's lead sentence points at a path that does not resolve — the same link-rot class as hevc/mod.rs:40 (W06-A04 file 8, F-002).
What correct looks like: `[`decode_refusal`](self::decode_refusal)` or `[`Mpeg12Facts::decode_refusal`](super::facts::Mpeg12Facts::decode_refusal)`.
Confidence: HIGH — Verification: full file read (no `impl Mpeg12Error` block) + facts.rs:260.

### F-004 [S4]
Where: crates/codecs/src/video/mpeg12/error.rs:2-4
Code:
```
//! (card 5.H1: no panics, no damage smoothing — every failure names
//! its rule; the H.264 [`H264Error`](crate::video::h264::parser::
//! H264Error) shape verbatim).
```
What is wrong: two overclaims. (1) "every failure names its rule" — `Truncated { needed, have }` (:21-26) carries no rule code; its Display is "truncated: needed N bits, have M" (:57). 3 of 4 variants carry a dotted `code`; this one names none. (2) "the H.264 H264Error shape verbatim" — the shapes differ in every load-bearing detail: `Truncated` fields are `u32` here vs `u64` in H264Error (nal.rs:33-38); `OutOfRange` carries `code: &'static str` here vs `max: u64` in H264Error (nal.rs:52-59); H264Error has `ForbiddenZeroBit`/`Emulation`/`Unsupported` variants this enum lacks, and this enum adds `Marker`; even the Truncated Display differs ("…needed {needed} bits…" vs h264's "…needed {needed}…", nal.rs:79). "Modeled on" would be true; "verbatim" is not.
Why it matters: "verbatim" invites copy-paste trust in a diff review ("nothing to check, it's the same shape") — exactly when the payload semantic changed from `max` (the spec bound, machine-checkable) to `code` (a prose rule).
What correct looks like: "the H.264 H264Error shape, adapted: Marker replaces the NAL-layer's ForbiddenZeroBit/Emulation; OutOfRange carries the rule code instead of the spec max."
Confidence: HIGH — Verification: side-by-side read of both enums + Display arms (sed/rg outputs above).

### F-005 [S4]
Where: crates/codecs/src/video/mpeg12/error.rs:114-126
Code:
```
/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern: the reason rides in `source`, never in
/// `Display`'s no-secrets surface).
#[derive(Debug, Clone)]
struct Detail(String);
```
What is wrong: this is one of **13** verbatim copies of the private error-carrier pattern in `crates/codecs` — `rg "struct Detail\(String\)\|struct FeatureDetail"` returns 12 `Detail(String)` sites (aac, mp3, mp2, opus, adpcm, pcm, alac, flac, vorbis, dnx, mpeg12, vp9 error modules) plus mjpeg's `FeatureDetail(String)`. Each copy re-implements the same 10-line Display+Error pair privately. (My own vp9/error.rs audit called vp9's the "8th verbatim copy" — that count undercounted; the recorded basis is unknown. Current grep count: 13 sites.)
Why it matters: 13 private re-implementations of one trait-shaped concept with no shared type — a `codec_api::detail::Detail` (or a `thiserror`-style carrier) would delete ~130 lines and make the pattern testable once.
What correct looks like: one shared carrier type in codec-api/media-types; per-module `From` impls if naming matters.
Confidence: HIGH — Verification: rg across crates/codecs (site list above).

### F-006 [S4]
Where: crates/codecs/src/video/mpeg12/error.rs:167-184 (test `parse_errors_render_their_rule`)
Code:
```
        assert_eq!(
            Mpeg12Error::Truncated {
                needed: 12,
                have: 3
            }
            .to_string(),
            "truncated: needed 12 bits, have 3"
        );
```
What is wrong: the Display test pins only the `Truncated` and `Marker` arms. `rg "out of range \(|invalid:"` across the workspace shows zero assertions on the `OutOfRange` ("{field} {value} out of range ({code})", :61) and `Invalid` ("invalid: {code}", :63) renderings. Mutation check (reasoned, not executed): swap or garble those two format strings → every test in the workspace stays green.
Why it matters: the test's name promises "parse errors render their rule"; two of the four rule renderings are untested — the exact arms whose payloads differ from the H.264 original (F-004).
What correct looks like: extend the test with `OutOfRange { field: "frame_rate_code", value: 9, code: "mpeg12.seq.table6-4" }` and `Invalid { code: "mpeg12.ext.id" }` exact-string assertions.
Confidence: HIGH — Verification: rg of the rendered strings repo-wide (only :175/:182 assert) + read of all four Display arms.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc block | 1-11 | FALSE (consumer claims) | S3 | F-001, F-003, F-004 |
| `enum Mpeg12Error` (4 variants + field docs) | 18-51 | REAL | — | correct payloads; `Truncated` lacks a rule code (F-004); derives Debug/Clone/PartialEq/Eq — used by bitreader/headers `?`/match paths |
| `impl Display for Mpeg12Error` | 53-66 | REAL | S4 | 4 arms correct; only 2 tested (F-006) |
| `impl std::error::Error for Mpeg12Error` | 68 | REAL | — | trivial marker |
| `pub const DESCRIPTORS` | 73 | REAL | S3 | doc claim false for AVI/MP4/MKV (F-002) |
| `accepts` | 76-79 | REAL | — | exact two-string gate; `#[must_use]`; test content-sensitive |
| `decode_refusal` | 91-112 | REAL (unreachable) | S3 | correct typed Unsupported @ `mpeg12.decode.blocked` with Detail source; phantom registry consumer (F-001); foreign branch honest |
| `struct Detail` + `Display` + `Error` | 117-126 | REAL | S4 | 13th carrier copy (F-005) |
| tests ×4 | 133-184 | REAL | S4 | executed :748-751; Display coverage 2/4 (F-006) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `descriptor_gate_accepts_exactly_the_two_ids` (:134-140) | YES | exact 2 acceptances + 5 refusals incl. case-sensitive `"MPEG2VIDEO"` and `""` |
| `decode_refusal_is_typed_unsupported_for_both_descriptors` (:143-153) | YES | pins `ErrorKind::Unsupported` + context `mpeg12.decode.blocked` for both ids |
| `decode_refusal_names_foreign_descriptors` (:156-165) | YES (foreign branch) | downcasts the source to `Detail` and asserts "neither"; NOTE: the blocked-branch wording ("software decode is blocked…") is never asserted — deleting that sentence keeps the suite green |
| `parse_errors_render_their_rule` (:168-184) | PARTIAL | 2 of 4 Display arms pinned; OutOfRange/Invalid untested (F-006) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every failure names its rule" | error.rs:2 | FALSE (1 of 4 variants) | `Truncated` has no `code` (F-004) |
| "the H.264 H264Error shape verbatim" | error.rs:3-4 | FALSE (in detail) | field types, OutOfRange payload, variant sets, Display all differ (F-004) |
| "the codec-api registry receives [the refusal] … an honest, observable refusal" | error.rs:6-11 | FALSE (wiring) / REAL (library behavior) | zero production callers; CLI's `cli.codec_unavailable` is the shipped refusal (F-001) |
| "The sequence layer under this module is what probe/track-info consume" | error.rs:85-86 | FALSE | facts.rs has no probe/track-info caller (F-001) |
| "TS stream_type 0x01/0x02, PS/AVI/MP4/MKV mappings all normalize to these two strings" | error.rs:70-72 | FALSE for AVI/MP4/MKV; REAL for TS/PS | three mapping tables read (F-002) |
| "no validated pure-Rust engine exists; self-build is an L-sized … queued Phase 6+" | error.rs:83-85 → doc 04 §4 row | REAL | doc 04 §4 table row sed-verified (`missing`/`none (libmpeg2 is C)`/`L`/`6`) |
| matrix row `codec-video-mpeg12*` blocked | feature-matrix.toml (sibling audits) | REAL | consistent with the honest refusal design |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable (header, F-001 chain). The `es.rs:456/486/655` `decode_refusal` hits in apps/cli are an unrelated same-named local test helper — a naming collision worth renaming on one side (noted, not counted).
- `Detail` duplication: F-005 (13 sites).

## 8. Dependencies & flags
- Depends on `media_types::error` (EngineError/ErrorKind/ErrorContext/FormatInfo/PipelineStage) — workspace-internal, no external deps; `media_types::ErrorKind` root re-export makes the :8 doc link resolve.
- No feature gates: mpeg12 is unconditional (`video/mod.rs:36-39`), so this file compiles into every shipped binary and executes never.

## 9. Open questions for the next agent
- If the sequence layer ever gets wired to probe/track-info (the doc's phantom consumer), the PS demuxer's 64-byte `probe_video_codec` peek (ps/mod.rs:761-784, defaults to "mpeg2video" on read error) is the incumbent to replace — strictly weaker than the full walk (facts.rs audit F-002).
- MKV `V_MPEG2` pass-through produces a raw descriptor that `accepts` would refuse as "a registry wiring bug" per :88-90 — the intended UX for that lane deserves a decision when MPEG-1/2 lands anywhere.
- mjpeg's `FeatureDetail` vs everyone else's `Detail` (F-005): one shared carrier would also settle which name is canonical.
