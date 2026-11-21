# AUDIT: crates/codecs/src/video/mpeg12/headers.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 1–686 of 1106 audited (chunk part1 of 2; nominal 1–553, extended to 686 = natural boundary: end of production code; the `#[cfg(test)] mod tests` at 688–1106 is part2/W06-A06) / 26 items |
| File verdict | UNREACHABLE (real, spec-shaped, differentially tested code — but no shipped entry point reaches it; see F-001) |
| Reachable from shipped entry point? | NO. Call chain dead-ends: `headers::*` ← `facts.rs` (`Mpeg12Facts::scan`, facts.rs:85/127/142/185/211/219) ← **only** `crates/codecs/tests/mpeg12_corpus.rs` + facts.rs's own `#[cfg(test)]` module. `rg -n "mpeg12" apps/` → one comment in apps/web/Cargo.toml:94 (no code). `rg -n "Mpeg12Facts" --glob '!crates/codecs/**'` → no hits. The CLI refuses mpeg2video with its OWN error (`apps/cli/src/containers/ps.rs:843-847` `cli.codec_unavailable`), never routing through `mpeg12::error::decode_refusal`. W05-A06 independently reached the same conclusion ("rg Mpeg12Facts (test-only)"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 2 |
| Commands run | `Read` headers.rs 1–786 (chunk + boundary); `grep -n "pub fn\|impl\|mod tests"` (structure map, exact lines); `rg -n "Mpeg12Facts\|decode_refusal\|descriptor_gate"` crates+apps (facts consumers: facts.rs internal + tests/mpeg12_corpus.rs only); `rg -n "mpeg12" apps/` (Cargo comment only); `rg -n "vbv_buffer_bytes\|variable_bitrate\|time_code()\|letter()"` crates (no production callers); `grep -n "video::mpeg12" audit/runs/baseline/test-workspace.log.txt` (742-770: 29 tests ok); `sed -n '1142,1150p' orchrun-test-workspace-nofailfast.log.txt` (mpeg12_corpus 3/3 ok); `sed -n` + `od -c` on quoted ranges (verified byte-exact; note: shell rendering ate a literal `[m` in one `#[must_use]` — od confirmed the real bytes). |

## 1. One-paragraph verdict
This is a competent, spec-shaped clean-room MPEG-1/2 sequence-layer parser — bit layouts verified field-by-field against 11172-2/13818-2 syntax (sequence/GOP/picture/extension/slice), the frame-rate and Table B-1 VLC tables are correct to the letter, and it is pinned by an executed ffprobe differential over real ffmpeg-authored bytes (baseline `mpeg12_corpus` 3/3 ok: width/height/profile/level/chroma/frame-rate/interlace on raw ES + TS + PS lanes). But none of it is reachable: `Mpeg12Facts::scan` has no production caller anywhere — the CLI demuxes mpeg2video and refuses decode with its own error without ever entering this module — so ~2.3k lines (per the worklog) of parser ship as dead weight in the `lumen` binary, a "hw-feed seam" and "probe facts" whose consumers exist only in the future tense. On the dead path sit four genuine defects: a reserved profile id mapped to "4:2:2" in direct contradiction of the function's own doc, a misimplemented `extra_bit_slice` loop that desynchronizes the bit position when the flag is set, a VBV-bytes helper whose scale and +1 I could not reconcile with the spec's 16·1024-bit units and which nothing consumes or differentially tests, and a set of struct-doc-declared "forbidden" field values that parse never checks.

## 2. What actually works (verified)
— All 22 headers tests ran ok in baseline (`test-workspace.log.txt` lines 758-770), incl. `frame_rate_table_matches_spec`, `sequence_header_rejects_forbidden_codes`, `sequence_header_marker_violation_is_typed`, `mb_increment_table_covers_one_through_thirty_three`.
— Real differential vs ffprobe executed: `orchrun-test-workspace-nofailfast.log.txt` lines 1142-1149 — `fixture_raw_es_facts_match_ffprobe`, `fixture_transport_lanes_ts_and_ps`, `corpus_mpeg2video_lanes_field_assertions` all ok (facts produced by THIS file's parsers, compared against measured ffprobe truth on committed fixtures + corpus lanes, incl. a 4:2:2 lane that exercises `profile_name`'s escape arm 0x85).
— Bit-layout order verified by me against the spec syntax sections: sequence_header (§6.2.2.1: 12+12+4+4+18+marker+10+1+1[64×8]+1[64×8]) at 160-201; sequence_extension (§6.2.2.3: id4+8+1+2+2+2+12+marker+8+1+2+5) at 284-312; GOP (drop+5h+marker+6m+6s+6p+closed+broken) at 380-387; picture_header (10+3+16, P: +1+3, B: +1+1+3) at 447-476; picture_coding_extension (§6.2.3.1: id 1000, 4×f_code, 2+2+1×10) at 533-556.
— Table B-1 macroblock_address_increment (599-636): all 33 (code,length) pairs match ISO 13818-2 Table B-1 (1='1', 2='011', … 16='0000010111'(10 bits), 26-33=11 bits); prefix-free, so the greedy 1..=11-bit walk (668-678) is a correct decoder.
— `frame_rate_with_ext` (333-351): frame_rate×(n+1)/(d+1) with Euclid reduction matches §6.3.3; the doc's `24/12→2/1` differential case is pinned by test `frame_rate_extension_refines` (baseline line 759).

## 3. Findings

### F-001 [S3] The entire headers module is dead code in every shipped binary — "probe facts" and "hw-feed seam" consumers do not exist
Where: crates/codecs/src/video/mpeg12/headers.rs:1-686 (module; consumer graph)
Code:
```
//! - probe/track-info facts: resolution, frame rate,
//!   profile/level, chroma, progressive/interlaced, GOP timecode,
//!   picture-type census, motion-vector syntax presence;
//! - the hw-feed seam: ADR-001's Phase-7 adapters will hand
```
(mod.rs:23-28, the module's stated purposes)
What is wrong: `Mpeg12Facts::scan` — the sole production consumer of every parser in this file — is called only from `crates/codecs/tests/mpeg12_corpus.rs` and facts.rs's own test module. No app, no probe path, no registry path references `mpeg12` (rg over `apps/`: one Cargo.toml comment). The mpeg2video decode refusal the user actually hits comes from the CLI's own `cli.codec_unavailable` (ps.rs:843-847), not from `mpeg12::error::decode_refusal`.
Why it matters: The module's reason to exist ("probe/track-info facts") is unrealized; ~2.3k lines compile into the shipped `lumen` binary (and `apps/web/Cargo.toml:93-95` even claims they "tree-shake out", which is a wasm-specific argument — in the native CLI they are linked and unreferenced). Dead parser code with a ffprobe differential is honest dead code, but it is still maintenance surface that tests gate on while shipping nothing.
What correct looks like: Either wire `Mpeg12Facts::scan` into the probe/track-facts path it was built for (the facts are already ffprobe-parity proven), or mark the module `#[cfg(any(test, feature = "mpeg12-facts"))]` so the shipped binary does not carry it.
Confidence: HIGH — Verification: rg call graphs (listed above); W05-A06's independent conclusion cross-cited.

### F-002 [S3] `profile_name` maps reserved id 7 to "4:2:2" — contradicting its own doc line and ffmpeg's vocabulary
Where: crates/codecs/src/video/mpeg12/headers.rs:73-82 (doc at 56)
Code:
```
    match (profile_and_level >> 4) & 0x7 {
        1 => Some("High"),
        2 => Some("Spatial"),
        3 => Some("SNR"),
        4 => Some("Main"),
        5 => Some("Simple"),
        6 => Some("Multiview"),
        7 => Some("4:2:2"),
        _ => None,
    }
```
What is wrong: The doc (line 56) says "Un-modeled/reserved ids return `None`". Profile nibble `0b111` is reserved in Table 8-2 (the 4:2:2 profile's indication is 0x85 = escape+000, handled in the escape arm at 67-72 — that arm IS ffprobe-verified via the corpus 4:2:2 lane). ffmpeg's MPEG-2 profile list ends at 6 (Multiview); id 7 prints "unknown" in ffprobe. This code would report "4:2:2" for a reserved id no reference decoder names — a fabricated fact on exactly the ffprobe-parity surface this module exists to provide. No test covers nibble 7 (the part2 test at 848 pins only "Main").
Why it matters: If/when the facts path is wired (F-001), a hostile or merely exotic stream with profile nibble 7 gets a wrong profile string; today it is a mapping that lies about its own doc contract.
What correct looks like: `7 => None` (reserved), matching the doc and ffmpeg; keep the escape-arm 0x85 → "4:2:2".
Confidence: MEDIUM (Table 8-2 text not consultable in-sandbox; ffmpeg's profile enum ends at Multiview=6 — high confidence) — Verification: read + ffmpeg public profile list knowledge; UNVERIFIED against the ISO table's reserved-111 row by direct citation.

### F-003 [S3] `SliceHeader::parse` misimplements the `extra_bit_slice` group — no loop, no terminating bit read; bit position desynchronizes when the flag is set
Where: crates/codecs/src/video/mpeg12/headers.rs:653-662
Code:
```
        let mut intra_slice_flag = false;
        if r.read_bit()? == 1 {
            intra_slice_flag = r.read_bit()? == 1;
            if intra_slice_flag {
                // intra_slice + reserved bits (§6.2.4): consumed,
                // not modeled further.
                r.read_bit()?;
                r.read_bits(7)?;
            }
        }
```
What is wrong: §6.2.4's slice syntax is a `while (extra_bit_slice == '1')` group: after the optional `intra_slice_flag`/`intra_slice`/`reserved_bits` fields, the bitstream contains ANOTHER `extra_bit_slice` bit terminating the loop. This code reads the first `extra_bit_slice` and the intra group, then walks straight into the macroblock-address VLC loop (668-678) without consuming the terminating bit — so on any stream with `extra_bit_slice == 1`, every subsequent bit (including `first_macroblock_address_increment`) is read one bit early, i.e. misparsed or reported `None`.
Why it matters: The module doc claims "clean-room from the two specs' syntax sections". Real encoders always write `extra_bit_slice = 0` (both specs: "shall be 0" in this version), so today's blast radius is zero — but the module's stated destiny (F-001: hw-feed substrate) inherits a silent desync on the one path that exercises the group.
What correct looks like: `while r.read_bit()? == 1 { … }` with the second read inside the loop, per Table 6-15 — or at minimum consume and require the terminating 0 bit after the group.
Confidence: MEDIUM — Verification: read; spec Table 6-15 syntax recalled, not consultable in-sandbox (no network/ISO copy); the structural asymmetry (one conditional group, no terminator) is visible in the code itself.

### F-004 [S3] `vbv_buffer_bytes` scale is suspect (16·1024 bytes + `+1`) and nothing tests it against anything but itself
Where: crates/codecs/src/video/mpeg12/headers.rs:241-245
Code:
```
    /// VBV buffer size in 16·1024-byte units (the spec's scale).
    #[must_use]
    pub fn vbv_buffer_bytes(&self) -> u32 {
        (self.vbv_buffer_size_value + 1) * 16 * 1024
    }
```
What is wrong: The spec's VBV size formula is expressed in 16·1024-BIT units (2048 bytes), not 16·1024-byte units, and has no `+1` — so this helper, if my reading of 11172-2 §2.4.2.2 / 13818-2 §6.2.2.1 is right, returns ~8× too large a byte count and adds a spurious increment. The only test (line 748: writer puts 19, asserts `(19+1)*16*1024`) is self-referential — it pins the implementation, not the spec; the ffprobe differential deliberately compares only "width, height, profile, level, chroma/pix_fmt, frame rate, interlace" (mpeg12_corpus.rs:22-24), so no external truth ever touches this value.
Why it matters: A helper named for a spec quantity whose unit is off by 8× is a landmine for the future facts/hw-feed consumer; and its "the spec's scale" comment asserts verification that does not exist.
What correct looks like: Either `2048 * value` bytes (if the 16·1024-bit-unit reading is confirmed) with a differential pin, or delete the helper until a consumer exists.
Confidence: MEDIUM — Verification: read; UNVERIFIED against the ISO text (not available in this sandbox); no in-repo differential or doc pins the scale (rg vbv across crates/docs).

### F-005 [S4] Field docs declare values "forbidden" that `parse` never checks
Where: crates/codecs/src/video/mpeg12/headers.rs:583 (quantiser "1..=31, 0 forbidden"), 132-135 (sizes, spec "shall not be zero"), 501 (picture_structure "1 top, 2 bottom, 3 frame" — 0 reserved), 359-366 (timecode fields 6-bit, spec ranges 0..59)
Code:
```
    /// `quantiser_scale_code` (5 bits; 1..=31, 0 forbidden).
    pub quantiser_scale_code: u32,
```
What is wrong: `SliceHeader::parse` (652) stores `quantiser_scale_code` without rejecting 0; `SequenceHeader::parse` accepts 0×0; `PictureCodingExtension::parse` accepts picture_structure 0; GOP fields accept 63 minutes. The struct docs themselves document the forbidden ranges — the parsers don't enforce them. The module's damage model ("no damage smoothing") argues these flow on as facts, but the docs and the code disagree.
Why it matters: Doc-code drift on a parsing module whose only value proposition is spec fidelity; garbage-tolerant by design is fine, but then the docs should not claim the ranges are meaningful.
What correct looks like: Typed `OutOfRange` errors for the forbidden values (consistent with the aspect/frame-rate/chroma gates that DO exist at 163-177, 293-304), or docs downgraded to "raw 6-bit field".
Confidence: HIGH — Verification: read of each parse body.

### F-006 [S4] Dead API surface + one false doc claim on Table B-1
Where: crates/codecs/src/video/mpeg12/headers.rs:236-245, 400-409, 108-115, 595-597
Code:
```
/// The macroblock_address_increment VLC (13818-2 Table B-1, identical
/// in 11172-2): (code, length) → increment. 1..=33 modeled; the
```
What is wrong: (a) `variable_bitrate`, `vbv_buffer_bytes`, `GopHeader::time_code`, `PictureCodingType::letter` have zero production callers (rg: tests only) — dead surface on an already-unreachable module. (b) "identical in 11172-2" is false for increments 17..=33 — 11172-2's Table B-1 stops at 16 (17-33 are MPEG-2 additions), so an MPEG-1 stream carrying those codes is misclassified here as a legal increment rather than an error.
Why it matters: Minor: dead code and an over-broad doc claim on the same table the code gets otherwise exactly right.
What correct looks like: Delete or wire the dead helpers; scope the doc claim to "13818-2 Table B-1 (11172-2 caps at 16)".
Confidence: HIGH for (a) (rg); MEDIUM for (b) (spec recollection, not consultable).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-10 | REAL | — | "clean-room, no foreign code" — unverifiable but consistent with field-level syntax correctness |
| `frame_rate_table` | 17-30 | REAL | — | matches Table 6-4/6-5 exactly (24000/1001, 24, 25, 30000/1001, 30, 50, 60000/1001, 60); pinned by `frame_rate_table_matches_spec` (baseline 760) |
| `ChromaFormat`/`name` | 33-53 | REAL | — | ffprobe spellings yuv420p/422p/444p; corpus differential covers 420 + 422 lanes |
| `profile_name` | 65-83 | REAL | S3 | escape arm 0x85 → "4:2:2" ffprobe-verified via corpus; nibble-7 mapping fabricated → F-002 |
| `level_value` | 87-90 | REAL | — | `& 0x0F` matches ffprobe level (0x48→8=Main); corpus-pinned |
| `PictureCodingType::{letter,has_motion_vectors}` | 93-127 | REAL | S4 | correct I/P/B/D letters; dead API surface (F-006a) |
| `SequenceHeader` struct | 130-150 | REAL | — | field widths documented correctly |
| `SequenceHeader::parse` | 152-213 | REAL | S4 | layout exact; aspect 1..=4 + frame-rate gates; no zero-size gate → F-005 |
| `SequenceHeader::aspect_ratio` | 215-225 | REAL | — | 1:1, 4:3, 16:9, 2.21:1 per Table 6-2 |
| `SequenceHeader::frame_rate` | 227-232 | REAL | — | table lookup, `unwrap_or((0,0))` unreachable post-gate |
| `SequenceHeader::variable_bitrate` | 234-239 | REAL | S4 | 0x3FFFF VBR marker correct; no production caller |
| `SequenceHeader::vbv_buffer_bytes` | 241-245 | REAL | S3 | scale suspect + self-referential test → F-004 |
| `SequenceExtension` struct | 251-273 | REAL | — | MPEG-2 geometry fields |
| `SequenceExtension::parse` | 275-325 | REAL | — | id-nibble gate 0001 ✓, chroma 00 rejected ✓, layout exact; marker enforced |
| `SequenceExtension::frame_rate_with_ext` | 327-351 | REAL | — | (n+1)/(d+1) + Euclid reduction per §6.3.3; corpus-pinned (0130 lane) |
| `GopHeader` struct + `parse` | 354-397 | REAL | S4 | marker after hours ✓; no 0..59 range gates → F-005 |
| `GopHeader::time_code` | 399-409 | REAL | S4 | HH:MM:SS:PP format; dead API |
| `PictureHeader` struct | 412-429 | REAL | — | MPEG-1 predictor fields kept for MPEG-2 back-compat ✓ |
| `PictureHeader::has_motion_vectors` | 431-438 | REAL | — | P/B only ✓ (card 5.H1 probe fact) |
| `PictureHeader::parse` | 440-487 | REAL | — | coding types 1-4, 0/5-7 typed refusal ✓; P/B conditional fields ✓ |
| `PictureCodingExtension` struct | 493-524 | REAL | — | full §6.2.3.1 field set |
| `PictureCodingExtension::parse` | 526-572 | REAL | S4 | id 1000 gate ✓; picture_structure 0 unvalidated → F-005; composite_display tail not consumed (harmless: borrowed payload) |
| `SliceHeader` struct | 578-593 | REAL | — | honest `Option` for the VLC miss |
| `decode_macroblock_address_increment` | 598-636 | REAL | — | all 33 entries match Table B-1; prefix-free |
| `SliceHeader::parse` | 638-685 | REAL | S3 | slice 0 rejected ✓; `extra_bit_slice` group misimplemented → F-003; quantiser 0 unvalidated → F-005; greedy VLC walk correct |

## 5. Test honesty (tests live in part2, 688-1106 — W06-A06's range; executed evidence cited from baseline)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_raw_es_facts_match_ffprobe (corpus, ok) | YES | real ffmpeg bytes vs measured ffprobe truth on w/h/profile/level/chroma/framerate/interlace — the strongest possible pin; would catch any field-order or table error in this chunk |
| frame_rate_table_matches_spec (ok, 760) | YES | hard-coded expected table |
| sequence_header_rejects_forbidden_codes (ok, 767) | YES | aspect 7 / frame-rate 9 typed refusals asserted |
| sequence_header_marker_violation_is_typed (ok, 766) | YES | marker bit 0 → typed error |
| mb_increment_table_covers_one_through_thirty_three (ok, 762) | YES | iterates 1..=33 asserting Some |
| vbv assertion (line 748, inside roundtrip test) | NO (against spec) | writer writes 19, test asserts (19+1)·16·1024 — pins the implementation, not the spec scale → F-004 |
Mutation check (reasoned): flipping `7 => Some("4:2:2")` to `None` changes no test outcome (no coverage) — that is precisely how F-002 survived; swapping two entries of Table B-1 fails the 762 table test and the corpus.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Un-modeled/reserved ids return `None`" | headers.rs:56 | FALSE | line 80 returns Some("4:2:2") for reserved 7 → F-002 |
| "clean-room from the two specs' syntax sections, no foreign code consulted" | headers.rs:9-10 | REAL-UNPROVEN | field layouts verified correct by me; authorship process unverifiable |
| "ffprobe decodes it [0x85] to "4:2:2" — measured on this tree's fixtures" | headers.rs:61-63 | REAL | corpus 4:2:2 lane passed in baseline (orchrun nofailfast 1142-1149) |
| "probe/track-info facts … hw-feed seam" (module purpose) | mpeg12/mod.rs:23-28 | PAPER-ONLY | no production consumer exists → F-001 |
| "Table B-1 … identical in 11172-2" | headers.rs:595-596 | FALSE | 11172-2's table stops at 16 → F-006b |
| worklog: "Mpeg12Facts::scan: one-pass, total, damage-tolerant" | docs/worklog/archive-2026-10.md:9611 | REAL | facts tests 752-757 ok (facts.rs is W06-A05's file; tests executed) |

## 7. Dead / duplicated / unreachable code
- The whole file (see F-001): compiled, exported, never called from shipped code.
- Dead helpers even within the dead module: `variable_bitrate`, `vbv_buffer_bytes`, `time_code`, `letter` (F-006a).
- `bit_rate_value` stored raw (no ×400 bits/s scale, no extension merge) — same dead-field class.

## 8. Dependencies & flags
- No feature gates: mpeg12 is an unconditional in-house module of `codecs` (crates/codecs/Cargo.toml comment: in-house decoders/parsers stay unconditional), so it compiles into `lumen` (native CLI) and every default-features build; only apps/web's opt-in wasm lane can omit it (and there the comment claims tree-shaking, F-001).
- Depends on `super::bitreader` (MSB-first reader, another agent's file — its `read_bits`/`read_marker`/`read_bit` semantics are load-bearing for every layout claim above; its baseline tests 742-747 ok) and `super::error` (my file #8).

## 9. Open questions for the next agent
- part2 (W06-A06): does any test in 688-1106 cover `profile_name` nibble 7, the `extra_bit_slice == 1` path, or vbv scale? (I found none — those gaps are F-002/F-003/F-004.)
- facts.rs owner (W06-A05): confirm `Mpeg12Facts::scan` tolerates `quantiser_scale_code == 0` (unvalidated here) without producing a misleading fact.
- Whoever wires probe facts: the ffprobe differential exists and passes — the wiring is the missing 10% that would flip this file's verdict from UNREACHABLE to REAL.
