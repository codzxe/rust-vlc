# WAVE 09 SUMMARY — subtitles (SRT/VTT/ASS/PGS/DVB/CC), text rendering, compositing

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 119 files, 100% covered |
| Findings | S0:2 S1:3 S2:25 S3:50 S4:100 (approx from headers) |
| Tree purity | clean |

## DOMAIN VERDICT (the §8.9 hostile question answered)
The ENTIRE crates/subtitles (25,416 LOC) is UNREACHABLE from every shipped
binary: no app depends on it (sole dependent: fuzz/Cargo.toml:50); the CLI
refuses subtitle.* commands, deselects subrip tracks at open, and
Frame::Subtitle is a no-op. All matrix rows "subtitle-srt/vtt implemented"
are FALSE as user-visible features (library-island code). Parsers are
genuinely robust (BOMs, mutations, saturating math — 267 tests) but they
parse into a void. Text rendering: NO fonts are loaded anywhere (no
TTF/OTF parsing, no system fonts, swash/skrifa still [PROPOSAL]) — every
glyph renders as a solid block. Compositing: real alpha blending
((a*s+(255-a)*d+127)/255 verified) with exactly ONE consumer: its own test
surface. Overlays go NOWHERE.

## Worst findings
1. [S0, EXECUTED] cc/mod.rs PAC range bug: 608 PACs keyed on first bytes
   0x40-0x5F (text range) instead of 0x10-0x1F — real "HELLO WORLD!"
   renders as "D!"; "AB" vanishes; tests pass via in-dialect vectors
   (W09-A07). Matrix row subtitle-cc-608-708 "implemented" FALSE twice
   over: wrong code AND nobody feeds cc_data (codecs SEI discards type-4).
2. [S0] srt parser: matrix "implemented" x3 platforms vs shipped CLI
   no-ops subtitle frames entirely (W09-A07).
3. [S1, executed] ASS parser.rs:722 "attempt to add with overflow" panic
   on crafted PlayResX+\pos — "no panics on any input" contract FALSE
   (W09-A01).
4. [S1] DVB region-redefinition palette OOB panic (stale 8-bit CLUT
   indices; all profiles) (W09-A02/A08). [S1] PGS compose amplification:
   <=255 PCS refs x 16 MiB canvas ~= 4 GiB retained from ~3 KiB input
   (W09-A04). [S1] VobSub 67 MiB-per-cue RLE amplification (W09-A10).
5. [S2, executed] cc/render.rs vertical MIRRORING: anchor math bottom-up
   vs place_cue top-down — every CC caption vertically flipped; the test
   pins the wrong value (W09-A09).
6. [S2, executed] vtt/text.rs 954ms O(n^2) entity scan on 200KB bare-&
   cue (W09-A07).
7. [S2] pgs/rle runs clip at CANVAS not ROW -> cross-row smear,
   python-proven, "never smear" invariant false (W09-A09).
8. [S2] spu.rs coordinate check INVERTED — illegal x2<x1 accepted, legal
   y1==y2 rejected (W09-A09). layout.rs reference (0,0) div-by-zero;
   "center on extent" not implemented (W09-A09/A10).
9. [S2] CEA-708 SET_CURRENT_WINDOW op length 1 vs spec 2 -> walk desync;
   damage variants bypass the 256-event cap (W09-A05).
10. [S3 cluster] ASS tags beyond G-2 set dropped-but-counted;
    \an/\r/\k/\t/\fad/\frz/\p stripped; Secondary/Outline/Back colors
    silently skipped uncounted; SRT </font> clears color (no stack);
    "5ms vs 500ms" doc lie executed.
