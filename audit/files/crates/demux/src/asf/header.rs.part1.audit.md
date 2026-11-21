# AUDIT: crates/demux/src/asf/header.rs#part1 (lines 1–542)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 542 audited (1-542; chunk boundary at the nominal 542/543 split cuts `parse_at` mid-body — that function starts :495 and completes at :701 in part2/A06's range; I audited its :495-542 head for coherence and hand the rest to A06 with traced observations in §9) / 19 items fully in range + `parse_at` (partial) |
| File verdict | UNREACHABLE (real, test-executed code; no shipped entry point constructs the ASF demuxer) |
| Reachable from shipped entry point? | NO. Proof: `rg "AsfDemuxer\|asf::" apps/ --type rust` → zero hits; the open path dispatch `apps/cli/src/containers/mod.rs:50-75` has no `asf` arm (15 wired ids: adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav — `unwired_refusal` is the default); `rg "asf\|wmv" crates/media-probe/src` → zero hits — no probe signature exists, so the probe cannot even name the format (the corpus files probe-refuse with `probe.no-demuxer`, the row's own recorded compat-matrix note). Only `crates/demux` tests construct it: all 37 `asf::*` tests pass in baseline (orchrun-test-workspace-nofailfast.log.txt:1555-1589, :1953 — incl. `fixture_family_cross_checks_against_live_ffprobe`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Full read of the file (:1-1086 — needed for chunk-boundary coherence; findings filed only against 1-542, cross-boundary observations handed to A06 in §9) · `rg "AsfDemuxer\|asf::" apps/` (zero — unreachability proof) · Read `apps/cli/src/containers/mod.rs:1-120` (registry arm list, no asf) · `rg "asf\|wmv" crates/media-probe/src -i` (zero — no signature) · Read `crates/demux/src/asf/guid.rs` u16le/u32le/u64le/Guid::read (:184-215, :36-50 — all bounds-checked, defensive) · `rg -n "asf" docs/feature-matrix.toml` + Read row `container-asf-wmv` (:207-217) · `rg -n "asf::" orchrun…log.txt` (37/37 pass) · Read `crates/demux/src/asf/mod.rs:100-130` (constructor docs) · `rg "AsfDemuxer" crates/demux/src/lib.rs asf/mod.rs` (lib exports `pub mod asf` :45 — library-reachable only). |

## 1. One-paragraph verdict
Part 1 is the ASF header walk's vocabulary and entry: nine explicit memory/count ceilings (MP §12 style), ten plain data records whose field maps are documented with unusual honesty — the module docs open by admitting the ASF spec was unacquirable and that every layout is byte-verified against ffmpeg 7.1.5 output with `[VERIFY-P0]` pins — plus the GUID→registry `codec_descriptor` map and the `parse` entry that anchors the Header Object at offset 0 or the broadcast `+12` prefix shape. The code quality is high: every read goes through `SourceReader`'s clamp primitives and every helper in `asf::guid` is bounds-checked (a defensive standard the IVF sibling does not meet), the duplicate-FPO/duplicate-stream policies are damage-noted first-wins, and the whole family is executed by 37 baseline tests including a live-ffprobe fixture cross-check. The chunk's own defects are small (the damage log accumulated before a fatal parse error is dropped on the `Err` path). The structural problem is reachability: none of this can be reached from the shipped binary — no probe signature names ASF, no container-host arm dispatches to it — while the feature matrix still carries `windows = "implemented", macos = "implemented"` for `container-asf-wmv`, cells that the same platform-independent facts (and the row's own linux compat-matrix downgrade to `partial`) contradict.

## 2. What actually works (verified)
— Bounds discipline: `MAX_HEADER_BYTES` (4 MiB) gates the whole-header read before allocation (checked in `parse_at`'s head, :513 — the gate itself is in part2's lines but the constant and its doc are part1's); `MAX_TSD_BYTES`, `MAX_CODEC_LIST_ENTRIES`, `MAX_METADATA_*`, `MAX_EXTENSION_CHILDREN`, `MAX_TOP_OBJECTS` all exist and are consumed (call sites verified at :737, :864, :937, :994, :1024).
— `parse` (:471-492): 16-byte GUID anchor at 0, the +12 broadcast-prefix retry (so the FPO gate — not a bare `not-asf` — can refuse broadcast files typed), defensive `Guid::read` on short prefixes (zero-GUID fallback ≠ HEADER ⇒ typed `not_asf` with sniffed bytes). Traced on sub-16-byte files: no panic, typed refusal.
— `codec_descriptor` (:381-452): total map (audio tag→pcm/adpcm/wma names incl. per-bits PCM; video fourcc→mjpeg/wmv/vc1 names); every unknown passes through raw (`asf-audio-tag-{tag:04x}`, `asf-fourcc-{name}`) for a typed downstream codec refusal — the mkv unknown-codec precedent, never a silent guess. `CodecDescriptor::new` failure maps to a typed error at the call site (:757-762).
— `utf16_lossy` (:361-373): `as_chunks::<2>` pairs, odd tail dropped (lossy by contract), trailing NULs stripped (ASF lengths include the terminator — measured, documented).
— Data records (:114-356): field-for-field consistent with the documented byte maps; `FileProperties::is_broadcast`/`declares_seekable` bit extraction matches the documented flags cross-check (broadcast fixture flags 3 vs normal 2).
— Test execution: 37/37 `asf::*` tests pass in baseline (orchrun :1555-1589), including `header_above_memory_ceiling_refuses_typed`, `broadcast_fixture_refuses_typed_where_ffprobe_refuses`, `fixture_family_walks_match_the_measured_truth`, `unknown_waveformatex_tags_pass_through_raw`, and the live-ffprobe cross-check (:1953). The demux library surface is real and exercised; only the shipped wiring is absent.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/asf/header.rs:471-491
Code:
```
    pub fn parse(reader: &mut SourceReader) -> Result<AsfHeader, media_types::EngineError> {
        let mut damage = DamageLog::default();
        let mut skipped: Vec<(Guid, u64)> = Vec::new();
```
(and the two `Self::parse_at(reader, …, &mut damage, &mut skipped)` tail calls at :485/:490)
What is wrong: `damage` and `skipped` are threaded mutably into `parse_at`, which accumulates advisory notes (duplicate objects, skipped children, count mismatches) across the walk — but every fatal `Err` return from `parse_at` (bad object size, truncation, broadcast, no-streams) bubbles through `parse` and drops the accumulated log: the caller receives the terminal error with zero damage context.
Why it matters: The damage notes are exactly the diagnostic that explains *why* a hostile or damaged file died where it did (e.g. three skipped children before the fatal one); losing them on the error path makes the typed error the only observable. Advisory-only impact — but this file's whole posture is honest observability, and the log survives only on the success path.
What correct looks like: Either return the damage log with the error (an error struct carrying `DamageLog`), or expose `AsfHeader::parse_with_damage(reader) -> (Result<…>, DamageLog)` so hosts can log the walk even on failure.
Confidence: HIGH — Verification: read; all `Err` exits of `parse_at` traced (they return `Err(...)` directly with no side channel).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (field-map provenance, spec-unavailable disclosure) | 1-57 | REAL | — | Every layout claim marked verified-vs-recalled; the [VERIFY-P0] posture is the most honest provenance section in the crate. Fixture closures cross-checked by the executed fixture tests. |
| `use` set | 58-69 | REAL | — | Reuses `avi::header::DamageLog` + `avi::riff::SourceReader` (cross-family, documented); `super::guid` table. |
| 9 bound constants | 71-106 | REAL | — | All consumed (see §2); PROPOSAL-tagged with sane figures. |
| `FileProperties` struct | 114-147 | REAL | — | Field map matches the documented 80-byte layout (parse at :705-719, part2). |
| `FileProperties::is_broadcast` / `declares_seekable` | 151-161 | REAL | — | Bit masks per the documented flags cross-check. |
| `WaveFormatEx` struct | 167-184 | REAL | — | 18-byte base + cbSize tail map (parse :779-806, part2). |
| `VideoInfo` struct | 188-204 | REAL | — | BITMAPINFOHEADER facts + extradata_len. |
| `VideoInfo::fourcc` | 209-221 | REAL | — | Lossy ASCII render, observability only (raw bytes are the truth — documented). |
| `StreamRecord` struct | 228-261 | REAL | — | Carries verbatim `tsd_bytes` as the decoder-config seam (`SharedBytes`); flags_raw honest ("semantics unpinned"). |
| `CodecEntry` / `MetadataPair` structs | 265-287 | REAL | — | Advisory records; disagreement policy documented (SP wins). |
| `TopObject` / `DataObjectLocation` structs | 290-317 | REAL | — | Location facts only; `reserved_matches` honest about the unverified `01 01` constant. |
| `AsfHeader` struct | 320-356 | REAL | — | The walk's product; `std::mem::take` for damage/skipped at construction (:696-699, part2). |
| `utf16_lossy` | 361-373 | REAL | — | Verified §2. |
| `codec_descriptor` | 381-452 | REAL | — | Verified §2; total map, raw passthrough for unknowns. |
| `AsfHeader::parse` | 471-492 | REAL | S4 | F-001 (damage dropped on Err). Anchor + broadcast-prefix retry traced sound. |
| `AsfHeader::parse_at` (starts :495; :495-542 in this chunk) | 495-701 | REAL (partial) | — | Head half traced: 30-byte head length-checked (:506), `header_size` ceiling + floor + end-vs-stream checks before the whole-header read (:513-537) — the length-check-after-read discipline IVF lacks is present here. Children loop + per-object dispatch + gates (broadcast/no-streams/codec-list cross-check) complete in part2 (A06). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `not_asf_bytes_refuse_typed` / `truncated_head_is_truncation_not_not_asf` | YES | Part-1 surface (parse's anchor + prefix retry): magic vs truncation discrimination pinned. |
| `broadcast_flags_refuse_typed` / `broadcast_fixture_refuses_typed_where_ffprobe_refuses` | YES | FPO flag gate pinned against a real ffmpeg broadcast fixture. |
| `header_above_memory_ceiling_refuses_typed` | YES | MAX_HEADER_BYTES gate (parse_at head — this chunk's half). |
| `unknown_waveformatex_tags_pass_through_raw` / `pcm_descriptor_maps_by_bits` | YES | `codec_descriptor`'s passthrough + per-bits map pinned. |
| `fixture_family_walks_match_the_measured_truth` + `fixture_family_cross_checks_against_live_ffprobe` (:1953) | YES | Full-tree walks against measured ffmpeg/ffprobe truth — the strongest evidence; would fail on any field-map drift. |
| 37 total | — | All executed, 0 failed (orchrun :1555-1589, :1953). Detailed suite audit belongs to asf/tests.rs's assignee (A02). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| `container-asf-wmv` status `windows = "implemented"`, `macos = "implemented"` | docs/feature-matrix.toml:212 | **FALSE** | Platform-independent facts: no ASF signature in media-probe (rg zero hits), no `asf` arm in the container registry (containers/mod.rs:50-75). The open path cannot reach this code on ANY OS. The row's own notes record the linux compat-matrix downgrade to `partial` for exactly this gap (probe.no-demuxer) — windows/macos were not re-run but share the identical code path, so their `implemented` cells contradict the same evidence. |
| `container-asf-wmv` status `linux = "partial"` | same row | REAL | The honest cell — matches the recorded compat-matrix refusal. |
| "36-test asf unit suite … ffmpeg 7.1.5 sha256-pinned fixture differential incl. live ffprobe" (evidence string) | same row :216 | REAL | 37 `asf::*` tests executed in baseline (orchrun :1555-1589) incl. the live-ffprobe cross-check (:1953). The crate-level work is real; the over-claim is only the win/macos status cells. |
| "Every layout below is byte-verified against ffmpeg 7.1.5 output … closes each object exactly" | module docs :9-18 | REAL (fixtures) / PARTIAL (hostile) | Fixture closure pinned by executed tests; but the SP object's ECD length is parsed then discarded unvalidated (`let _ = ecd_len`, :748 — part2 range, handed to A06), so "closes exactly" is enforced for TSD only, not the full SP payload. |
| "spec text not acquirable … [VERIFY-P0]" | module docs :9-18 | REAL | Honest disclosure; nothing in the code claims spec authority. |

## 7. Dead / duplicated / unreachable code
- The entire module is UNREACHABLE from shipped entry points (see header row) — dead-in-product, alive-in-library (`pub mod asf` exported from the demux crate; consumed by tests only).
- No internal dead code found in 1-542.

## 8. Dependencies & flags
- Depends on `avi::header::DamageLog` and `avi::riff::SourceReader` — the ASF card building on the AVI card's reader (same cross-family pattern as IVF; documented at the use site).
- `media_types::packet::SharedBytes` for `tsd_bytes`.
- No feature gates; compiled unconditionally into the demux lib.

## 9. Open questions for the next agent (A06 — part2, :543-1085)
Handed over with exact sites, already traced (I read the whole file for boundary coherence; verify and file under your own chunk):
1. **S3 candidate — advisory objects are fatal on internal size lies.** `parse_extended_content_description` returns `Err(error::bad_object_size(0))` when an entry's name-length (:939-941) or type+value head (:949-951) runs past the payload, and `walk_header_extension` returns `Err` when `22 + data_size > payload.len()` (:988-990). Both objects are advisory (metadata text / capture-and-ignored children), and the sibling `parse_codec_list` handles the identical shape with damage-notes + break (:866-869, :895-898) per the file's own doctrine "advisory text, never a parse failure" (:881-884). A corrupt metadata block kills the whole open; ffmpeg tolerates. Today unreachable (see verdict), so S3 not S2.
2. **S4 candidate — `let _ = ecd_len`** (:748): SP's error-correction length parsed then discarded unvalidated, no damage note for a lying ECD length; the "closes each object exactly" doc claim is enforced for TSD only.
3. **S4 candidate — silent skip:** `parse_codec_list` with `payload.len() < 20` returns `Ok((empty, 0))` (:858-860) — the only skip in the file with no damage note.
4. **Refuted panic (do not re-chase):** the codec-list cross-check's `guid::u16le(&entry.info, 0)` (:666) cannot panic on empty `info` — `asf::guid::u16le` is bounds-checked and returns 0 (guid.rs:184-189, verified verbatim). An empty `info` merely produces tag 0 ⇒ a spurious `codec-list-disagrees` damage note — cosmetic.
5. Confirm the children-walk guards I traced in the :495-542 half (head length check :506-511, ceiling/floor/end checks :513-537) hold through the rest of `parse_at` — the sites I read are consistent.
