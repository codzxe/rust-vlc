# AUDIT: crates/demux/src/mp4/mod.rs#part1 (lines 1-599)
| Field | Value |
|---|---|
| Auditor | W05-A08 |
| Date | 2026-11-21 |
| Lines / functions | 599 audited (nominal part1 = 1-556; extended to 599 = natural boundary, end of `esds_oti`; `impl Mp4Demuxer` starts at 601) / 15 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open`/`lumen decode-bench` → host `open_source` → apps/cli/src/demux_host.rs:458 → apps/cli/src/containers/mod.rs:67 `"mp4" => mp4::open(stream, opts)` → `Mp4Demuxer::new`/`start`. EXECUTED: audit/runs/baseline/mp4-smoke-open-session-420.txt (4:2:0 H.264+AAC → tracks 2 ready → Playing, EXIT=0), mp4-smoke-open-session.txt, mp4-smoke-open-session-aac.txt, mp4-smoke-decode-bench.txt) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `Read` over lines 1-599 + 600-659 boundary; `rg -n "SAMPLE_IS_NON_SYNC|unpack_language|esds_oti|…"` (call sites: 713, 851, 916…1117, 1170-1179, 1760, 2169); `rg -n "language|voe|und"` across mp4/{tests,test_support} (NO language assertion anywhere; fixture writes 0x55C4 `// "und"`); `rg -n "esds"` in tests+test_support (no builder, no 0x6b/0x6c assertions); python3 decode check (0x55C4 → "voe" bugged / "und" correct); baseline log greps (mp4::tests 1764-1801, mp4_fixtures 2155-2162, fixtures_parent 905-916); `ls tests/fixtures/generated/valid/` (fixtures on disk). |

## 1. One-paragraph verdict
The MP4 demuxer's front half is real, executed, and disciplined: the probe/box-walk core (`probe`, `iter_boxes`, `read_top`) is bounds-checked with typed refusals for every lie a file can tell (64-bit sizes, size==0, child>parent, truncation), the codec vocabulary and ISO 23003-5 PCM mapping are pinned by executing table tests, and the whole thing ran green in the baseline on real ffmpeg-authored fixtures and in three shipped-binary smoke transcripts — the demux half is NOT the owner's MP4 blocker (per INDEX CORRECTION v2, 4:2:0 H.264+AAC plays end-to-end; the failures were decode-side yuv444p refusal and the hard-coded bench lane). But this chunk contains the traced root cause of a standing baseline defect: `unpack_language` has an off-by-one (`b'a' + bits` instead of `0x60 + bits`), so every MP4 track whose mdhd language is `und` (packed 0x55C4 — ffmpeg's default, i.e. nearly all of them) reports `language: Some("voe")` — exactly what all three baseline smoke transcripts show — and the `None`-for-"und" branch is unreachable for und files; no test in the repo asserts a track language, so the suite is blind to it. The esds mislabeled-codec quirks (0x6b/0x6c) are likewise assertion-free.

## 2. What actually works (verified)
— Probe + codec vocabulary EXECUTED in the shipped binary: mp4-smoke-open-session-420.txt:9 shows `codec: CodecDescriptor { description: Some("h264") }` / `Some("aac")` — the output of `codec_of` (avc1→h264, mp4a→aac) on the sample's stsd entries, and the open only happened because `probe` claimed the ftyp brand.
— Full MP4 lifecycle on 4:2:0: mp4-smoke-open-session-420.txt lines 4-15 (tracks 2 ready → Buffering → Playing → Stopped → Idle, EXIT=0) — the demuxer fed the pipeline end-to-end.
— `iter_boxes`/`read_top` bounds discipline verified by reading AND by executing tests: baseline no-fail-fast log lines 1766 (`child_beyond_parent_is_typed`), 1791 (`size_one_reads_64bit_largesize`), 1792 (`size_zero_convention_extends_to_stream_end`), 1798 (`truncated_box_header_is_typed_truncation`) — all ok.
— PCM mapping pinned: baseline log 1777-1780 (`ipcm_fpcm_pcmc_mapping_table`, `ipcm_fpcm_with_pcmc_demux_as_pcm_codecs`, `ipcm_pcmc_unmapped_width_notes_unknown_entry`, `ipcm_without_pcmc_defaults_to_entry_name_convention`) — all ok; the table test pins 10 mappings + 4 refusals + 2 defaults (tests.rs:801-848).
— Real-file fixture suite executed: baseline log 2155-2162 (`valid_mp4_fixtures_demux_fully`, `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed`, `malformed_mp4_fixtures_survive_the_fuzz_harness` — ok) over `tests/fixtures/generated/valid/mp4_*.mp4|mov` (present on disk, verified with ls).
— `SAMPLE_IS_NON_SYNC` (line 146, mask 0x8000): the dual-encoding concern in its comment is actually handled at the use site — mod.rs:1760 `sync: sflags & SAMPLE_IS_NON_SYNC == 0 && sflags & 0x1_0000 == 0` (part4, verified by grep).

## 3. Findings

### F-001 [S2] `unpack_language` is off by one: mdhd `und` (0x55C4) decodes as "voe" — the baseline's standing garbage-language defect, traced to this line
Where: crates/demux/src/mp4/mod.rs:534-539
Code:
```
/// Unpack the mdhd 3×5-bit language code; `None` for "und".
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
What is wrong: ISO 14496-12 §12.4.2 packs the language as 3×5-bit letters, each letter = **0x60 + bits** ('a' = bits 1). The code adds `b'a'` (0x61) — one too much. Verified arithmetic: packed 0x55C4 = bits (21, 14, 4) = "und" correctly, but the code yields 'a'+21='v', 'a'+14='o', 'a'+4='e' → `Some("voe")`. That string is not hypothetical: ALL THREE baseline MP4 smoke transcripts show it on every track — mp4-smoke-open-session-420.txt:9, mp4-smoke-open-session.txt:9, mp4-smoke-open-session-aac.txt:9 each carry `language: Some("voe")` (h264, aac, aac-only tracks), and INDEX.md's standing-defects list flags it as "garbage default (demux wave to trace)". Second-order defects: (a) the `tag == "und" → None` branch is unreachable for real `und` files — it fires only for packed 0x51A3, whose correct decode is "tmc" (a genuine tmc track would be misreported as `None`); (b) bits == 0 (invalid per spec) maps to 'a'.
Why it matters: wrong user-visible metadata on the owner's core format — virtually every ffmpeg-authored MP4 (no language metadata) reports a bogus language "voe" in `MediaInfoReady`. The chain executes: unpack_language (mod.rs:1117) → TrackState.language → `LanguageTag::parse` at mod.rs:880-884 (accepts any 3 letters) → TrackInfo. Not S0/S1: playback itself is unaffected (the 420 sample plays to Stopped).
What correct looks like: `(b'`' + ((packed >> shift) & 0x1F) as u8)` (0x60 base), plus an explicit `bits == 0 → invalid` handling, and one test asserting `unpack_language(0x55C4) == None` and e.g. 0x55C4-adjacent "eng" (packed (5,14,7) → 0x14C7) == Some("eng").
Confidence: HIGH — Verification: read + python3 decode of 0x55C4 both ways + the defect EXECUTED in three baseline transcripts (cited above); fixture author's own comment confirms intent — mp4/test_support.rs:124 `b.extend_from_slice(&u32b(0x55C4)); // "und"`.

### F-002 [S3] The esds mislabeled-codec quirks (OTI 0x6b→mp3, 0x6c→mjpeg) have zero assertion coverage
Where: crates/demux/src/mp4/mod.rs:572-576 (quirk contract), functions `esds_oti` (576-599) + `oti_of_esds` (542-570), called at mod.rs:1170/1173
Code:
```
/// Look for an `esds` child inside an stsd entry payload and read its
/// ObjectTypeIndication — the mislabeled-codec quirks: `Some(0x6b)`
/// means an `mp4a` entry actually carries MPEG-1 Layer III, and
/// `Some(0x6c)` means an `mp4v` entry actually carries M-JPEG.
```
What is wrong: no test in the repo constructs an `esds` child or asserts either quirk — `rg -n "esds"` over mp4/tests.rs and mp4/test_support.rs returns nothing (test_support has no esds builder), and the only `mjpeg` assertions (tests.rs:315, 392) are the cover-art tests, which never go through `esds_oti`. The real-file fixtures exercise `esds_oti` on their mp4a tracks, but with an AAC OTI (0x40), where `Some(0x40) != Some(0x6b)` and `None != Some(0x6b)` produce the identical outcome — a mutation that breaks `oti_of_esds` entirely (say, mis-walking the expandable length) passes the whole suite, and with it the mp4a→mp3 / mp4v→mjpeg reclassification silently dies.
Why it matters: two advertised quirk behaviors (the doc comment above) are unfalsifiable by the test suite — classic "name promises more than the body (of tests)".
What correct looks like: a test_support `esds_box(oti)` builder plus two tests: mp4a+OTI(0x6b) track asserts codec "mp3", mp4v+OTI(0x6c) asserts "mjpeg".
Confidence: HIGH — Verification: rg over both test files (no esds/0x6b/0x6c matches); read of the call sites.

### F-003 [S3] `pcm_codec_of_ipcm` fallback contradicts its own code: "f = little-endian" vs `let float = kind == b"fpcm"`; an `fpcm` entry without `pcmC` maps to an integer codec
Where: crates/demux/src/mp4/mod.rs:514-521 (fallback), with the contradiction at 482
Code:
```
    // No pcmC child: entry-name default per ISO/IEC 23003-5 §7.3.2
    // (i = big-endian, f = little-endian), 16-bit assumed (the
    // QuickTime convention).
    match kind {
        b"ipcm" => Some("pcm_s16be"),
        b"fpcm" => Some("pcm_s16le"),
        _ => None,
    }
```
What is wrong: three lines above (mod.rs:482) the same function interprets the entry names as integer-vs-float classes: `let float = kind == b"fpcm";` — 'i' = integer PCM, 'f' = float PCM (that is why the pcmC table maps fpcm 0x20/0x40 to `pcm_f32*`/`pcm_f64*`). The fallback comment's rationale ("i = big-endian, f = little-endian") contradicts that, and the fallback itself returns `pcm_s16le` — a 16-bit INTEGER codec — for a FLOAT entry with no pcmC child, i.e. the entry's class is dropped precisely in the one case where the name is the only surviving class information.
Why it matters: an `fpcm` stream without pcmC (malformed-but-recoverable per the function's own damage philosophy) would be decoded as integer PCM — noise, not audio. Blast radius is small (pcmC is written by real muxers; ffmpeg ≥ 7 writes it — tests.rs:858 comment) and the path is test-pinned as-is (tests.rs:844-848 asserts `fpcm` no-pcmC → "pcm_s16le"), so the defect is a pinned wrong behavior, not an accidental one.
What correct looks like: either `fpcm` no-pcmC → a float default (`pcm_f32le`) or the honest `None` (unknown-entry damage path) — and a comment that doesn't contradict line 482.
Confidence: MEDIUM — the ISO 23003-5 §7.3.2 default-when-pcmC-absent could not be verified in-sandbox (no spec copy); the internal contradiction and integer-codec-for-float-entry facts are read-verified.

### F-004 [S4] `probe` walks the ftyp minor-version field as if it were a brand, contradicting its own doc comment
Where: crates/demux/src/mp4/mod.rs:161-168
Code:
```
        // Brands: major(4) + minor(4) + compatible list.
        let mut i = 8;
        while i + 4 <= header.len() {
            let brand = [header[i], header[i + 1], header[i + 2], header[i + 3]];
            if KNOWN_BRANDS.iter().any(|b| **b == brand) {
                return true;
            }
            i += 4;
```
What is wrong: the loop starts at offset 8 and steps by 4 over major(8-11), **minor version(12-15)**, then the compatible brands. The minor-version field is an integer (typically 0 or 0x200), not a brand; the function's doc (lines 149-151: "an `ftyp` whose major **or** any compatible brand is known") and the inline comment ("major(4) + minor(4) + compatible list") both describe a two-part check, but the code probes all three. A file whose minor version happens to equal a known brand's bytes (e.g. 0x69736F6D) would be claimed. Never observed in practice; the doc-comment pair also says "A leading moov/wide/free" while the code matches `b"moov" | b"wide" | b"free" | b"skip"` (line 173, "skip" undocumented).
Why it matters: probe hygiene on the format the owner cares about; the cost is a theoretical false claim, the benefit of fixing is a probe that matches its contract.
What correct looks like: `let mut i = 16;` for the compatible-brand loop after checking `header[8..12]` as the major brand.
Confidence: HIGH — Verification: read (offsets counted against the ISO-BMFF ftyp layout: size(4) kind(4) major(4) minor(4) compat…).

## 4. Per-function table (part1 range; `impl Mp4Demuxer` methods start at 601 — part2/A07)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `probe` | 154-174 | REAL | S4 | Executed in shipped binary (smoke opens). Brand walk includes the minor-version field (F-004); <8 bytes refuse; QT heads incl. undocumented `skip`. |
| `u32be` / `u16be` / `u64be` / `i64be` | 318-340 | REAL | — | All bounds-checked via `data.get(..)?`; no panics; callers pass small offsets. |
| `iter_boxes` | 345-384 | REAL | — | Enforces header truncation, largesize ≥ 16, size==0 to parent end, checked_add overflow, child>parent (`box_overruns_parent`), non-advancing extents; verified by 4 executing tests (baseline log 1766/1791/1792/1798). |
| `read_top` | 390-433 | REAL | — | 16-byte prefix read, clamped position, largesize ≥ 16, checked_add; NOTE: does not itself clamp declared end ≤ stream length — by design ("callers … clamp first", line 388-389); part2/3 callers (713, 851, 2169) must be checked by A07/A10. |
| `codec_of` | 438-470 | REAL | — | 28-entry vocabulary, `None` for unknown (damage path, never a fake name); EXECUTED (h264/aac descriptions in all 3 smoke transcripts); `sowt→pcm_s16le` pinned by test (baseline 1793). |
| `pcm_codec_of_ipcm` | 481-522 | REAL | S3 | pcmC FullBox walk correct (body.get(4)/get(5)); 10 mappings + 4 refusals test-pinned; fallback contradiction F-003. |
| `kind_of` | 525-532 | REAL | — | vide/soun/sbtl/text/subt/clcp → kinds, else Data. Trivial, correct. |
| `unpack_language` | 535-539 | REAL (defective) | S2 | F-001 off-by-one; executed defect ("voe" ×3 transcripts); None branch dead for und files. |
| `oti_of_esds` | 542-570 | REAL-UNPROVEN | S3 | Expandable-length walk is correct by reading; quirk outcomes unasserted (F-002); executes on every mp4a/mp4v entry via fixtures/smoke. |
| `esds_oti` | 576-599 | REAL-UNPROVEN | S3 | Shotgun offsets [28, 76, 78] for audio/visual entry child walks; wrong candidate falls through safely; quirk outcomes unasserted (F-002). |
| `StickyShape::of` / `StickyShape::mint` | 251-263 | REAL | — | Replayable error shape; trivially correct (clone of stage/kind/context). |
| Constants `MAX_*`/`KNOWN_BRANDS`/`EXTENSIONS`/`SAMPLE_IS_NON_SYNC` | 109-146 | REAL | — | Ceilings documented with [PROPOSAL] markers; `SAMPLE_IS_NON_SYNC` dual encoding handled at use site (1760, verified); `MAX_SAMPLES` 48 MiB estimate arithmetically right (Sample ≈ 48 B). |

## 5. Test honesty (mp4 tests live in mp4/tests.rs — A04/A05 chunks; mp4_fixtures.rs — fixture suite)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `probe_accepts_known_brands_and_qt_heads` (tests.rs:92-113) | YES | Known major/compat brands accepted, unknown rejected, cut-short ftyp rejected, QT heads accepted — brand-table and kind mutations fail it. Would NOT catch F-004 (minor-version field untested). |
| `ipcm_fpcm_pcmc_mapping_table` (tests.rs:801-848) | YES | 10 positive mappings + 4 refusals + 2 defaults asserted against `pcm_codec_of_ipcm` directly. Pins F-003's wrong fallback as intended behavior. |
| `sowt_entry_maps_to_pcm_s16le` / `threegp_brand_and_samr_entry_open` | YES | codec_of rows pinned through full demux. |
| `size_one_reads_64bit_largesize`, `size_zero_convention…`, `child_beyond_parent_is_typed`, `truncated_box_header…` | YES | iter_boxes/read_top lie-detection pinned (green in baseline 1766-1798). |
| `valid_mp4_fixtures_demux_fully` (mp4_fixtures.rs) | YES (codecs/kinds/frag/chapters) | Real ffmpeg files, per-track codec+kind+chapter counts asserted. NO language assertion — F-001 invisible. |
| NOTHING covers `unpack_language` | NO | Zero assertions on track language anywhere in mp4 tests/fixtures (rg verified); test_support.rs:124 writes 0x55C4 labeled `// "und"` while the code turns it into "voe" — the suite blesses the wrong decode by silence. |
| NOTHING covers esds quirks | NO | No esds builder, no 0x6b/0x6c case (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "size == 1 reads the 64-bit largesize (which must be ≥ 16 …); size == 0 means 'to end of parent/stream'" | mod.rs doc 26-28 | REAL | iter_boxes:355-376 and read_top:403-427 implement exactly this; tests green in baseline (1791, 1792). |
| "every box extent is clamped to parent ∩ stream" | mod.rs doc 22-24 | REAL in iter_boxes (parent side); stream side is caller duty — read_top:386-389 comment admits it; part2/3 verification belongs to A07/A10. |  |
| "The whole first `stsd` entry payload rides `DecoderConfig::extra_data`" | mod.rs doc 84-86 | REAL-UNPROVEN (this chunk) | TrackState.extra (216-218) exists; the stsd harvest is at mod.rs:1148-1202 (part2). |
| matrix row `container-mp4-mov-3gp` "Demux ISO-BMFF family … fragmented MP4" status implemented | docs/feature-matrix.toml:142-152 | REAL for this chunk's share (probe/box-walk/codec vocab executed: 3 smoke transcripts + fixture suite) | Note the row's notes field still carries the stale "No MP4 demuxer code exists" sentence from before Phase 5, self-contradicted by its own later "2026-11-15: end-to-end VIDEO PLAYBACK proven" sentence — flagged for the matrix-rows agent. |
| "voe"/language garbage "demux wave to trace" (standing defect) | INDEX.md:140-142 + W04-A06/ids.rs/event.rs prior audits | TRACED HERE | Root cause = F-001 (mod.rs:536). The `.ok()` swallow at mod.rs:883 (`LanguageTag::parse`) flagged by the media-types auditors is part2's line, but note "voe" parses fine — the swallow is not what surfaces the garbage. |

## 7. Dead / duplicated / unreachable code
- `unpack_language`'s `None` branch: dead for its documented purpose (no packed value that correctly decodes to "und" can reach it; the only trigger, 0x51A3, is "tmc") — F-001.
- No other dead code in the chunk; every helper has call sites (verified by the rg call-site list in Commands run).

## 8. Dependencies & flags
- Reuses `crate::avi::riff::{PayloadSlots, SourceReader}` (mod.rs:97) — cross-format sharing of the read/pool machinery, no external deps (crate is std-only per Cargo.toml — no [features] section; everything ships in `lumen`).
- `media_types::ids::LanguageTag` is the downstream validator (mod.rs:882-883, part2) — accepts any 3-letter tag, so it cannot catch F-001.

## 9. Open questions for the next agent
- A07 (part2, 600-1112): (a) confirm `start()`'s moov read clamps the declared end to the stream length (read_top deliberately does not — line 388-389); (b) mod.rs:883 `.ok()` swallow of `LanguageTag::parse` errors — with F-001 fixed, decide whether a hard-invalid 3-letter tag should surface as damage instead of silent None; (c) the language chain 1117→880-888 is where a regression test for F-001 belongs (assert `tracks[0].language == None` for the 0x55C4 fixture).
- A06/A10 (parts 3/4): verify MAX_MOOV/MAX_MOOF ceilings are actually enforced where moov/moof bodies are read whole (doc 29-35), and that the fragment scan honors MAX_FRAGMENT_SCAN_BOXES.
- F-003's ISO 23003-5 §7.3.2 default (fpcm without pcmC) needs a spec-owning agent — sandbox has no spec copy.
