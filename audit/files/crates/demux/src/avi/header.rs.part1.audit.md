# AUDIT: crates/demux/src/avi/header.rs#part1 (lines 1-443)
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 443 (chunk 1-443 of 866; part2 = 434-866 is A03's) / 14 items |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES — EXECUTED (call chain: `lumen` CLI → containers/mod.rs:68 `"avi" => avi::open` → `AviDemuxer` → `parse_hdrl`/`StreamDesc`; the AVI first-vertical lane is the one end-to-end-verified lane: baseline `mp4-smoke-bench-avi-mjpeg-pcm.txt` `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` and `avi-firstvertical-open-session.txt` full lifecycle) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg `avi::` in baseline log (85 tests ok, lines 1590-1674) · rg `damage.note` in avi/mod.rs (per-chunk call sites; per-note drain at mod.rs:409-414) · rg `events.len|truncate|MAX_DAMAGE` (no cap anywhere) · rg `coalesce` (idx1 coalesces per reason, index.rs:23; movi/hdrl paths do not) · rg `from_wave_tag|ticks_for_sample_size` (tests exist) · sed -n verification of every quote |

## Chunk boundaries
Part1 (1-443, this audit): module docs, `AviMainHeader`, `StreamFormat`, `PcmSample` (+`from_wave_tag`), `StreamDesc` (`time_base`/`track`/`ticks_per_chunk`), `ticks_for_sample_size`, consts (`MAX_STRN_BYTES`, `MAX_PCM_CHANNELS`, WAVE tags, SubFormat GUIDs, sizes), `DamageLog`, `HeaderList`, `zeroed_main_header`. Part2 (434-866, A03): `parse_hdrl`, `advance`, `peek_form`, `parse_avih`, `parse_strl`, `peek_size` — the parsing engine that consumes this vocabulary.

## 1. One-paragraph verdict
This chunk is the AVI header vocabulary — the PCM/WAVE tag matrix (tags 1/3 + the WAVEFORMATEXTENSIBLE GUIDs), the stream-descriptor record with exact scale/rate time bases, the chunk→tick ladder, and the damage ledger — and it is real and unusually well-tested: 85 avi tests executed green in the baseline, including a full `pcm_wave_tag_matrix`, GUID resolution, impossible-combination skips, negative-biHeight acceptance, and the misalignment-damage rule. The arithmetic is clean everywhere I traced it (`saturating_add` on the span end, `min(i64::MAX)` on tick counts, division guarded by the `sample_size == 0` branch). One defect: `DamageLog` has no bound on its `events` Vec and no coalescing seam — the movi walk drains per-note (`surface_damage`) and idx1 coalesces per reason (both elsewhere), but the `strl`/`hdrl` skip path accumulates one 16-byte event per ≥84-byte skipped stream list for the whole of `start()`, letting a hostile header inflate memory to roughly a quarter of the file size transiently, while the doc comment calls the ledger "bounded-recovery bookkeeping".

## 2. What actually works (verified)
- PCM vocabulary executed: baseline log 1645 `pcm_wave_tag_matrix_maps_every_width_and_kind ... ok`; 1673 `wave_format_extensible_resolves_subformat_guids ... ok`; 1635 `impossible_pcm_combinations_skip_the_stream_siblings_survive ... ok`; 1628 `float_pcm_without_sample_ladder_skips_the_stream ... ok`; codecs side 440-441 (`waveformatextensible_length_ladder…`, `…_resolves_guids_and_decodes ... ok`) — the descriptor-string seam (`pcm_u8`…`pcm_f64le`) round-trips demux→codec.
- Tick ladder executed: 1643 `packets_carry_the_avi_ladder_timing ... ok`; 1644 `pcm_misalignment_is_damage_not_silence ... ok` (pins `ticks_for_sample_size`'s `!aligned` flag); 1620 `block_align_disagreements_are_damage_notes_not_skips ... ok`.
- Containment rules executed: 1622 `channels_over_ceiling_skips_the_stream_siblings_survive` (MAX_PCM_CHANNELS), 1641 `negative_biheight_accepts_bottom_up_dib_with_damage`, 1663-1664 `short_strf`/`short_strh` siblings-survive, 1618 `all_streams_degenerate_is_typed_no_supported_streams` — the module doc's per-stream-containment model (lines 24-32) is test-pinned.
- End-to-end: the AVI lane is the shipped first vertical (baseline INDEX.md §contrast: 60/60 frames decoded, full open-session lifecycle).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/avi/header.rs:382-401 (this chunk); call-site evidence avi/mod.rs:409-414, 635, 696; index.rs:23
Code:
```
/// Damage observed while parsing (bounded-recovery bookkeeping — doc 03
/// §4.3(d): repairs are counted and surfaced, never silent).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DamageLog {
    /// One `(skipped bytes, reason)` pair per recovery decision; `0`
    /// bytes means "a decision, not a byte range" (e.g. a skipped
    /// unsupported stream).
    pub events: Vec<(u64, &'static str)>,
```
```
    pub fn note(&mut self, skipped: u64, reason: &'static str) {
        self.total = self.total.saturating_add(skipped);
        self.events.push((skipped, reason));
    }
```
What is wrong: The ledger has no event-count cap and no coalescing seam; `note` appends unconditionally. The two realistic storm paths are defended elsewhere: the movi walk drains after every note (`surface_damage`, avi/mod.rs:409-414, `mem::take(&mut self.damage.events)`), and idx1 damage is coalesced to one event per reason (index.rs:23 — the fix for a real 524k-event storm, test `idx1_damage_coalesces_per_reason` ok at log 1633). But the header-parse path (part2's `parse_hdrl`/`parse_strl`, which take `&mut DamageLog`) accumulates one event per skipped/unknown structure for the entire duration of `start()`: a hostile `hdrl` built from minimal degenerate `strl` lists (~84 bytes each: LIST+form+strh header+56-byte strh) mints ~24 bytes of ledger per ~84 bytes of file — ~28% of file size in RAM, transiently, before the first drain.
Why it matters: Unbounded allocation from file-controlled counts (checklist class); a 2 GB hostile header eats ~0.5 GB. Also the doc comment overclaims: "bounded-recovery bookkeeping" — the *scans* are bounded, the *ledger* is not.
What correct looks like: Either a `MAX_DAMAGE_EVENTS` cap in `note` (fold overflow into a single `damage-log-truncated` event), or a per-reason coalescing helper in `DamageLog` itself (the idx1 fix generalized) used by every parse phase.
Confidence: HIGH on the structure (no cap — rg `events.len|truncate|MAX_DAMAGE` over the crate: zero hits); HIGH on the drain/coalesce facts (quoted); MEDIUM on the exact memory ratio (84-byte minimal strl arithmetic estimated, not executed). Verification: read.

## 4. Per-function table (chunk 1-443)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-56 | REAL | — | Robustness-model claims test-pinned (see §2); claims about parse behavior are part2's to enforce (and are: lying_hdrl/lying_idx1 tests ok) |
| `AviMainHeader` | 67-82 | REAL | — | All-advisory posture (doc lines 51-56); zeroed when absent (`zeroed_main_header`) |
| `StreamFormat` | 91-117 | REAL | — | Two-arm v0 vocabulary (MJPEG / PCM); biHeight signedness documented at the field |
| `PcmSample::descriptor` / `bits_per_sample` / `bytes_per_sample` | 145-183 | REAL | — | Closed vocabulary; bits↔bytes consistent; names match the codec registry (tests ok) |
| `PcmSample::from_wave_tag` | 190-207 | REAL | — | Tag 1 → 8/16/24/32 int, tag 3 → 32/64 float; everything else typed-None (caller decides skip) — matrix test ok; tag 0xFFFE resolution lives in part2 via the GUID consts below |
| `StreamFormat::codec_descriptor` | 217-222 | REAL | — | `"mjpeg"` / pcm names — the L0 seam |
| `StreamDesc` | 226-258 | REAL | — | Advisory-vs-truth fields documented per field |
| `StreamDesc::time_base` | 267-269 | REAL | — | `TimeBase::new(scale, rate)` — one tick = scale/rate s; zero component → typed `bad_stream_params` (test `zero_rate_or_scale_skips_the_stream_not_the_file` ok, log 1674) |
| `StreamDesc::track` | 278-311 | REAL | — | `saturating_add` span end; `length == 0` → Missing end (never guessed); language: None with an honest LANGID note (295-297) |
| `StreamDesc::ticks_per_chunk` | 320-322 | REAL | — | Delegates; documented video/PCM split |
| `ticks_for_sample_size` | 329-337 | REAL | — | Div guarded by the zero branch; `min(i64::MAX)` cast clamp; `!aligned` damage flag pinned by `pcm_misalignment_is_damage_not_silence` |
| consts `MAX_STRN_BYTES` / `MAX_PCM_CHANNELS` / WAVE tags / GUIDs / sizes | 342-380 | REAL | — | GUIDs byte-verified against `{00000001-…}` / `{00000003-…}` KSDATAFORMAT values; ceiling tests ok |
| `DamageLog` | 384-401 | DEFECTIVE | S3 | F-001 — unbounded events, no coalescing seam |
| `HeaderList` | 404-430 | REAL | — | `filtered_ordinals` (W5-14 idx1 scope truth) + `resume_at` — both consumed by part2/mod walk |
| `zeroed_main_header` | 434-443 | REAL | — | Trivial; the no-avih path (test `missing_header_and_missing_movi_are_distinct_typed_errors` ok) |

## 5. Test honesty (avi tests live in avi/tests.rs; all 85 ok in baseline, log lines 1590-1674)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `pcm_wave_tag_matrix_maps_every_width_and_kind` (log 1645) | YES | Enumerates the full tag×width matrix incl. refusals — pins `from_wave_tag` |
| `wave_format_extensible_resolves_subformat_guids` (1673) | YES | 0xFFFE + both GUIDs → the right PcmSample |
| `impossible_pcm_combinations_skip_the_stream_siblings_survive` (1635) | YES | The typed-None path + containment |
| `packets_carry_the_avi_ladder_timing` (1643) | YES | `ticks_per_chunk`/`time_base` end-to-end on real packets |
| `pcm_misalignment_is_damage_not_silence` (1644) | YES | The `!aligned` flag becomes a damage note, not silent flooring |
| `channels_over_ceiling_skips_the_stream_siblings_survive` (1622) | YES | MAX_PCM_CHANNELS enforced |
| `idx1_damage_coalesces_per_reason` (1633) | YES | 1000 lying idx1 entries → ONE event — the storm fix my F-001 asks to generalize |
| Gap | — | No test for DamageLog growth itself (F-001): nothing pins a bound on `events.len()` for the hdrl/strl skip path |

## 6. Doc claims touching this chunk
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The PCM family recognized here is tag 1 (8/16/24/32-bit), tag 3 (32/64) and tag 0xFFFE resolving to either GUID" | header.rs:38-44 | REAL | from_wave_tag (190-207) + GUID consts (365-372) + part2 resolution; matrix/GUID tests ok |
| "Every impossible combination … skips the stream with a damage reason — never a panic, never the file" | header.rs:46-50 | REAL | `impossible_pcm_combinations…`, `short_strf…`, `short_strh…` all ok |
| "biHeight is read signed (D-9): a negative value … accepted with the absolute height and a damage note" | header.rs:33-37 | REAL | Field doc (97-101) + `negative_biheight_accepts_bottom_up_dib_with_damage` ok (1641); enforcement is part2's |
| "Advisory avih fields … are never trusted for control flow" | header.rs:51-56 | REAL | AviMainHeader all-advisory (67-82); part2's parse keeps `strl` as truth (`lying_hdrl_size_does_not_swallow_the_movi` ok, 1636) |
| "Damage observed while parsing (bounded-recovery bookkeeping …)" | header.rs:382-383 | OVERCLAIM | F-001: the ledger is unbounded; only the scans and (separately) the idx1/movi paths are bounded/drained |

## 7. Dead / duplicated / unreachable code
None in this chunk. All items are consumed by part2 (`parse_hdrl`/`parse_strl`) or the movi walk (`ticks_for_sample_size`, `DamageLog`); `PcmSample`'s vocabulary is mirrored codec-side by documented design (the descriptor string is the seam — boundaries.toml forbids the crate edge).

## 8. Dependencies & flags
No feature gates, no unsafe. `riff::{FourCC, JUNK, SourceReader}` from the sibling module (A02's file, riff tests ok 1596-1613). `boundaries.toml` discipline documented at 122-125 (the demux-must-not-depend-on-codecs rule that justifies the descriptor-string seam).

## 9. Open questions for the next agent
- A03 (part2): confirm `parse_strl` notes one damage event per skipped list (F-001's accumulation path) and whether any cap was intended; the generalized coalescing fix belongs in `DamageLog`.
- The `Language: None` posture (295-297) is honest — but note the baseline INDEX.md standing defect "TrackInfo language: Some("voe")" on MP4 tracks: that garbage default comes from the mp4 side, not AVI; keep the contrast when auditing that lane.
