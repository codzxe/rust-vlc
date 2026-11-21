# AUDIT: crates/demux/src/mkv/mod.rs#part1 (lines 1-579)
| Field | Value |
|---|---|
| Auditor | W05-A08 |
| Date | 2026-11-21 |
| Lines / functions | 579 audited (nominal part1 = 1-522; extended to 579 = end of `block_start_after_timecode`, the helper `load_next` calls; `next_block_packets` starts at 590 → A10's part2) / 20 |
| File verdict | REAL (with one UNREACHABLE accessor surface) |
| Reachable from shipped entry point? | YES for probe/new/open/load_next (call chain: `lumen open` → demux_host.rs:458 → apps/cli/src/containers/mod.rs:57 `"matroska" => mkv::open(stream, opts)` → `MkvDemuxer::new` + `start` → `open()`). NO for the issue-#34 metadata accessors (`title`/`track_uid`/`chapter_targets`/`attachments`/`tags`/`chapters`/`doctype`/`segment_info`/`cue_index`/`damage_bytes`/`pool_bytes`) — only callers outside the demux crate are its own tests (rg proof below). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `Read` over 1-650 (part1 + boundary); `rg -n "chapter_targets|track_uid|\.title\(\)|\.attachments\(\)|…"` across apps/ + crates/ (only shipped caller: apps/cli/src/containers/mkv.rs:199 `.track_entries()`; all other callers are crates/demux/tests/mkv_extras_wiring.rs); `rg -n "metadata|MediaFacts|fn open" apps/cli/src/containers/mkv.rs` (harvest consumes track facts only); `sed -n` on every quote; header.rs cross-checks (`time_base()` rejects scale 0 via TimeBase::new zero check; parse_info accepts scale 0 but open():440 catches it → no div-by-zero in chapter_targets); baseline log lines 2089-2153 (5 mkv test binaries, all green). |

## 1. One-paragraph verdict
The MKV front half is a real, executed demuxer: the top-level walk (`open`) collects Info/Tracks/Cues plus the issue-#34 extras (Tags/Attachments/Chapters) with typed errors and honest damage counters, `load_next`/`block_start_after_timecode` set up bounded cluster windows, and the whole lane is proven by five green test binaries in the baseline — including in-tree generated fixtures checked against ffprobe packet counts and bit-exact payloads (`mkv_fixtures.rs` 9/9) and the wiring suite driving tags/attachments/chapters/title/uid through `start` (`mkv_extras_wiring.rs` 9/9). The structural discipline holds under reading: unknown-size Segment children are handled or honestly terminated, declared ends are clamped to segment ∩ stream, progress in every loop is guaranteed, and the chapter-clock division is guarded (a `TimestampScale` of 0 is accepted by the Info parser but rejected by `time_base()` before any accessor can divide). The chunk's real defect is architectural: the entire issue-#34 metadata surface — media title, track UIDs, chapter seek targets, attachments, tags — is parsed, retained, and test-executed, but nothing in any shipped build reads it: the CLI host harvests only `track_entries()`, admits "MediaFacts carries no chapter vocabulary", and skips attachments entirely, so a shipped `lumen open` on an MKV surfaces none of it (and unlike MP4's cover-art lane, MKV cover art stays invisible). Two S4 hygiene notes: the unknown-size-child comment overstates which masters tolerate it, and multi-Segment files silently play only the first.

## 2. What actually works (verified)
— Full MKV lane executed in the baseline on in-tree fixtures (independent of the absent corpus): `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` lines 2140-2153 — `valid_fixtures_match_ffprobe_packet_counts … ok`, `audio_payload_is_bit_exact_vs_ffmpeg … ok`, `probe_matches_the_ebml_magic … ok`, `unknown_doctype_is_the_typed_refusal … ok`, `hand_built_laced_fixture_decodes_all_three_codings … ok`, `malformed_derivatives_are_typed_or_recover_never_panics … ok` (9/9) over `tests/fixtures/generated/valid/mkv_*.mkv` (on disk, verified with ls).
— The issue-#34 retention paths of `open()` executed: same log lines 2125-2138 — `tags_attachments_chapters_wire_through_start_retained … ok`, `unknown_size_tags_master_parses_and_playback_reaches_eos … ok` (the unknown-size branch at mod.rs:384-404), `chapter_targets_seek_lands_at_or_before … ok`, `title_falls_back_to_the_global_title_tag … ok`, `track_uids_are_stable_across_reopens … ok`, `damaged_block_recovers_once_and_still_reaches_eos … ok` (9/9).
— Damage/vandal resistance executed: `mkv_fixtures` `truncated_cluster_is_the_typed_truncation`, `zero_head_is_the_typed_bad_vint`; `mkv_editions_tags` 14/14 including `tag_bomb_ceilings_are_the_typed_resource_limits`, `chapter_bomb_ceiling_is_the_typed_resource_limit`, `atom_nesting_depth_guard_is_typed` (log lines 2105-2123).
— Div-by-zero guard in `chapter_targets` verified by tracing: `header.rs:117-120` `time_base()` → `TimeBase::new(timestamp_scale, 1_000_000_000)` errors when either component is 0 (media-types time.rs: "construction errors … when either component is zero"); `open()` calls it at mod.rs:440 and propagates — so `scale ≥ 1` holds for every `chapter_targets` call.
— Loop progress verified by reading: `open()`'s child walk advances `child_pos = child_end` with `child_end ≥ child_pos + hdr_len` (parse_header_at guarantees a non-advancing element is a typed refusal); `block_start_after_timecode` advances `at += hdr_len + size` with `hdr_len ≥ 2` (EBML id vint + size vint); `load_next` re-loops only after `self.walk = end` with `end > walk`.

## 3. Findings

### F-001 [S3] The entire issue-#34 metadata surface is unreachable from any shipped build — tests are its only consumers
Where: crates/demux/src/mkv/mod.rs:249-254 (title), 262-264 (track_uid), 276-313 (chapter_targets), with the admission at 270-274
Code:
```
    /// ticks). `[PROPOSAL: the chapter-navigation command
    /// (feature-matrix `playback-chapter-title`, phase 4) consumes
    /// this surface — `seek(target.span)` + `title` for the UI; the
    /// command layer does not exist yet, so nothing downstream reads
    /// it today]`.
```
What is wrong: `rg` across the whole repo for callers of `title()`, `track_uid()`, `chapter_targets()`, `attachments()`, `tags()`, `chapters()`, `doctype()`, `segment_info()`, `cue_index()`, `damage_bytes()`, `pool_bytes()` finds exactly one non-test caller: apps/cli/src/containers/mkv.rs:199 `.track_entries()` — the host's `harvest_facts`. The CLI host's own doc states the consequence: "**Chapters/Editions/ordered-chapters**: demuxer-side metadata only — `MediaFacts` carries no chapter vocabulary … **Attachments** are skipped by the demuxer (no track minted, no packet)" (mkv.rs:112-116). So the shipped binary opens an MKV and shows no media title, no chapters, no cover art, no tags — the code's own `[PROPOSAL]` comment admits "nothing downstream reads it today" (mod.rs:273-274).
Why it matters: the feature-matrix row `container-matroska-webm` (docs/feature-matrix.toml:132) advertises "Demux Matroska and WebM: tracks, chapters, attachments, cues-based seeking" with status implemented; the chapters/attachments half is implemented only down to a demuxer-internal seam that no shipped surface reaches. Cross-format inconsistency: MP4 cover art DOES surface as a still-image video track (mp4/mod.rs:75-80, verified by `cover_art_surfaces_as_a_still_image_video_track`), MKV cover art surfaces nowhere. The row's notes are partially honest ("chapter-nav command layer stays phase 4"), which keeps this at S3 rather than FALSE.
What correct looks like: either a shipped consumer (UI metadata/chapter list, `OpenedMedia` metadata map) or an explicit "retained, not surfaced" status in the matrix row instead of a bare "implemented".
Confidence: HIGH — Verification: rg caller census + host doc read + code comment read; tests executing the seam cited from baseline (lines 2125-2138).

### F-002 [S4] `open()`'s unknown-size-child comment overstates: Info/Tracks/Cues are NOT tolerated
Where: crates/demux/src/mkv/mod.rs:385-403
Code:
```
                    // Unknown-size child inside the Segment: the
                    // streaming form. A Cluster carrying it is the
                    // walk's cluster anchor; the metadata masters
                    // tolerate it too (their parsers clamp to the
                    // bound); anything else cannot be skipped past —
```
What is wrong: the comment says "the metadata masters tolerate it too", but the `match` below handles only `ID_CLUSTER`, `ID_TAGS`, `ID_ATTACHMENTS`, `ID_CHAPTERS`; an unknown-size `Info`, `Tracks`, or `Cues` falls to `_ => {}` and then `break` — the walk ends, `entries` stays empty, and the user gets the generic `no_tracks`/`no_clusters` typed error instead of a parse of the streaming-form master. (The Matroska spec only blesses unknown size for Segment/Cluster, so such files are rare — but the comment claims a tolerance the code does not have.)
Why it matters: misleading comment on a damage path; the next maintainer will trust it.
What correct looks like: route `ID_INFO`/`ID_TRACKS`/`ID_CUES` through their parsers with the segment-end clamp (their parsers already take a bound), or fix the comment.
Confidence: HIGH — Verification: read (control flow at 392-404).

### F-003 [S4] Multi-Segment files: every Segment after the first is silently ignored
Where: crates/demux/src/mkv/mod.rs:351-358
Code:
```
        let mut segment: Option<ElementHeader> = None;
        while pos < length {
            let Some(el) = ebml::parse_header_at(&mut self.reader, pos, length)? else {
                break;
            };
            if el.id == ebml::ID_SEGMENT {
                segment = Some(el);
                break;
```
What is wrong: the Segment search takes the FIRST `Segment` and never returns; the subsequent walk is bounded by `self.segment_end` (374), so a chained-segments file (legal Matroska — each Segment a self-contained movie) plays only its first segment, with no damage note, no event, and no doc mention of the limit anywhere in the module docs.
Why it matters: silent truncation of legal input; niche in practice (rarely muxed), hence S4.
What correct looks like: at minimum a damage note ("further segments ignored") surfaced at start, or the limit recorded in the module docs.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part1 range; `next_block_packets` 590+ → A10)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe` | 90-93 | REAL | — | 4-byte EBML magic 0x1A45DFA3; executed (`probe_matches_the_ebml_magic` ok, baseline 2149). |
| `MkvDemuxer::new` | 163-190 | REAL | — | No I/O; 1/1 placeholder base overwritten by open(); pool 2×MAX_PACKET_BYTES. |
| `doctype` / `segment_info` / `track_entries` / `cue_index` | 192-215 | REAL / UNREACHABLE (shipped) | — | Only `track_entries` has a shipped caller (mkv.rs:199); rest test-only. |
| `tags` / `attachments` / `chapters` | 217-242 | REAL / UNREACHABLE (shipped) | S3 | F-001; retention itself executed (wiring suite green). |
| `title` | 249-254 | REAL / UNREACHABLE (shipped) | S3 | Info Title → global TITLE tag fallback; F-001. |
| `track_uid` | 262-264 | REAL / UNREACHABLE (shipped) | S3 | `entries.get(handle)` — out-of-range → None; F-001. |
| `chapter_targets` | 276-313 | REAL / UNREACHABLE (shipped) | S3 | Div-by-zero impossible (scale validated via time_base() at open:440 — traced); ns→ticks conversion refuses non-fitting starts instead of clamping; hidden atoms skipped per spec; executed in tests only. F-001. |
| `damage_bytes` / `pool_bytes` | 316-325 | REAL / UNREACHABLE (shipped) | — | Honesty counters; test-consumed only. |
| `open` (the start walk) | 340-466 | REAL | S4 | Executed (wiring + fixture suites); unknown-size handling overstated (F-002); first-Segment-only (F-003); O(#clusters) header reads at start documented and deliberate (334-339); TimestampScale=0 accepted by parse_info (header.rs:411-418) but caught at 440 — error offset blames the Info element, acceptable. |
| `collect_tags` | 471-475 | REAL | — | TagList has no damage field (tags.rs:268-271) — nothing dropped. |
| `collect_attachments` / `collect_chapters` | 480-495 | REAL | — | Damage rows folded into damage_total AND kept on the lists. |
| `load_next` | 499-550 | REAL | — | Bounded window (min(declared end, walk+4MiB) clamped to segment); unknown-size non-cluster skips to segment end (its bytes were parsed at start — correct for trailing Tags); executed via EOS-reaching wiring tests. |
| `block_start_after_timecode` | 557-579 | REAL | — | Walks window to Timecode end; unknown-size child → typed bad_structure; no-Timecode fallback to cluster head; progress guaranteed (hdr_len ≥ 2); a Timecode overrunning its cluster silently empties the cluster (damage not counted — noted, edge-of-malformed). |

## 5. Test honesty (tests live in crates/demux/tests/mkv_*.rs — other agents' files; assessed for part1 coverage)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `probe_matches_the_ebml_magic` | YES | Magic pinned both ways (accept/reject). |
| `valid_fixtures_match_ffprobe_packet_counts` / `audio_payload_is_bit_exact_vs_ffmpeg` | YES | Real files, per-track counts + byte-exact payload vs an external truth. |
| `unknown_doctype_is_the_typed_refusal` | YES | open():343-346 refusal pinned. |
| `unknown_size_tags_master_parses_and_playback_reaches_eos` | YES | Drives open():384-404's unknown-size branch AND load_next's skip-to-segment-end arm to EOS. |
| `tags_attachments_chapters_wire_through_start_retained` | YES | Retention through start pinned. |
| `chapter_targets_seek_lands_at_or_before` | YES | chapter_targets' clock conversion + seek landing pinned. |
| `track_uids_are_stable_across_reopens` / `title_falls_back_to_the_global_title_tag` | YES | track_uid/title behavior pinned. |
| `damaged_block_recovers_once_and_still_reaches_eos` | YES | The mid-cluster-damage livelock fix (matrix notes: "fixed with the wiring (#34)") — the window-restore contract documented at mod.rs:584-589 (A10's range, cross-ref). |
| NO test covers: unknown-size Info/Tracks (F-002), multi-Segment files (F-003) | NO | Both paths unasserted. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "VP8/VP9 and AV1 WebM video are demuxed (packets + timestamps + seek) but NOT decodable (typed `Unsupported` at the decode seam, never a silent skip)" | mod.rs doc 19-23 | REAL-UNPROVEN (this chunk) | Demux half proven by fixtures; the decode-seam refusal is the codec layer's (corroborated by baseline mp4-smoke-open-session-hevc.txt's decode.unsupported shape for the analogous HEVC case). |
| "Cluster payloads are walked in bounded windows; packet bytes land in a 2-slot pool … larger payloads spill unpooled but stay block-bounded" | mod.rs doc 27-31 | REAL | MAX_CLUSTER_WINDOW/MAX_PACKET_BYTES/MAX_BLOCK_BYTES at 72-80; window logic at 499-550 (spill path is part2's emit — A10). |
| "chapter times are ns by spec — they do NOT scale with `TimestampScale`" | mod.rs doc 236-238 | REAL | chapter_targets (276-313) converts ns→ticks with explicit division; matches the Matroska spec's ns chapter times. |
| matrix row `container-matroska-webm`: "tracks, chapters, attachments, cues-based seeking" implemented; "chapter seek targets exposed" | docs/feature-matrix.toml:129-140 | SHALLOW (the chapters/attachments half) | Parsed+retained+test-executed, but no shipped consumer (F-001); row notes admit "chapter-nav command layer stays phase 4". Cues-based seeking IS shipped-reachable (host start-time lane seeks via `demuxer.seek(target)` at mkv.rs:167). |
| "Mid-cluster-damage livelock fixed with the wiring (#34)" | docs/feature-matrix.toml:139 | REAL | Fix documented at mod.rs:584-589 (A10 range); `damaged_block_recovers_once_and_still_reaches_eos` green in baseline (2129). |

## 7. Dead / duplicated / unreachable code
- F-001's accessor surface: `pub` API, test-consumed only, compiled into the shipped binary with no caller — the repo's largest reachable-but-unreached surface in this chunk. Everything else in part1 is wired.
- No duplication found (the module correctly delegates to ebml/header/cluster/cues/tags/attachments/chapters).

## 8. Dependencies & flags
- Std-only; reuses `crate::avi::riff::PayloadSlots` (mod.rs:58) like the MP4 demuxer; no feature gates (demux crate has no [features]) — everything ships.
- Downstream: `media_types::time::TimeBase/TimeSpan/Timestamp`, `plugin_api::demux::Demuxer` — all workspace crates.

## 9. Open questions for the next agent
- A10 (part2, 580-1045): (a) the window-restore contract in `next_block_packets` (584-589) is the #34 livelock fix — audit every exit path restores the window; (b) does `emit_block`'s spill path actually bound by MAX_BLOCK_BYTES as the module doc claims; (c) check `start()` propagates `open()`'s error before any accessor use.
- Matrix agent: `container-matroska-webm` row — decide whether "chapters, attachments" under status implemented needs a surfaced-consumer caveat (F-001) and whether MP4-vs-MKV cover-art asymmetry is a documented decision.
- Wave 4 (command layer): chapter_targets is the seam to consume — the [PROPOSAL] at mod.rs:270-274 is the checklist.
