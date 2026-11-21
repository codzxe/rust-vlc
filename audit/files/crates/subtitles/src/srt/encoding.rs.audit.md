# AUDIT: crates/subtitles/src/srt/encoding.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-21 |
| Lines / functions | 389 / 9 production items + 1 test helper + 9 tests |
| File verdict | UNREACHABLE (real, test-executed in baseline — 9/9 unit + 6/6 differential — but no shipped entry point links the crate; the only consumers are the in-crate srt parser, the crate's own tests, and fuzz/Cargo.toml) |
| Reachable from shipped entry point? | NO. In-crate chain: `srt/parser.rs:78-79` `parse_bytes` → `encoding::decode` + `normalize_line_endings`; `srt/mod.rs:63` re-exports `SourceEncoding`. Crate consumed only by `fuzz/Cargo.toml:50` + its own tests. `apps/cli`/`apps/desktop-tauri` have no `subtitles` dependency (rg Cargo.tomls). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | Read tool (whole file); `rg -n "encoding::|decode\(|normalize_line_endings|detect_bom"` over crates/subtitles → production caller map (parser.rs:78/79/95, mod.rs:63, tests); CP1252_HIGH table checked entry-by-entry against the Windows-1252 standard (27 defined chars + 5 None holes at 0x81/0x8D/0x8F/0x90/0x9D); UTF-16 surrogate arithmetic hand-verified; baseline `orchrun-test-workspace-nofailfast.log.txt` → 9/9 `srt::encoding::tests` ok (:5269+ run), `srt_canonical_differential.rs` 6/6 ok (:5627, UTF-16 both endians through the full parse) |

## 1. One-paragraph verdict
The core of this file is verifiably correct: the Windows-1252 high-half table matches the standard slot for slot (including exactly the five undefined bytes the module doc names), the BOM matrix is right, the UTF-16 surrogate-pair arithmetic is the textbook formula with every malformed shape (unpaired high, lone low, odd tail) degraded to typed damage instead of a panic, and nine baseline-green tests pin each policy branch with ordered damage vectors. The defects are at the edges: the dirty-world "strip one stray leading U+FEFF" tolerance exists on the UTF-8 and CP1252 paths but is missing on the UTF-16 path (a double-BOMmed UTF-16 SRT — the same dirty-world case the code explicitly handles for UTF-8 — keeps a leading ZWNBSP that will corrupt the first cue), the CP1252 copy of that strip is provably dead code, the common no-BOM path scans the buffer three times against the file's own "one pass" philosophy, and the matrix evidence string mislabels the fallback policy.

## 2. What actually works (verified)
— CP1252_HIGH (:138-171) checked byte-for-byte against Windows-1252: 0x80→U+20AC € … 0x9F→U+0178 Ÿ, `None` exactly at 0x81/0x8D/0x8F/0x90/0x9D. Correct.
— UTF-16 pair math (:220): `0x1_0000 + ((u - 0xD800) << 10) + (v - 0xDC00)` — correct; `char::from_u32` fallback honestly commented as unreachable-by-construction (:221-225); lone-low and odd-tail both typed damage (:230-240).
— BOM dispatch (:75-85, :90-102): slicing `&bytes[3..]`/`&bytes[2..]` is safe because detect_bom proved the prefix length; partial-prefix `EF BB` → None pinned by test (:294).
— Baseline execution: 9/9 `srt::encoding::tests` ok in the 267-test lib run; `srt_canonical_differential.rs` 6/6 ok (:5627) incl. `canonical_utf16_variants_parse_identically` — this file's decode ran on real UTF-16LE/BE fixtures end-to-end in the gate.
— `normalize_line_endings` (:248-263): CR/CRLF/lone-CR all → LF; the `"\r\r\n"` → `"\n\n"` edge pinned (:386).

## 3. Findings

### F-001 [S3]
Where: crates/subtitles/src/srt/encoding.rs:201-242 (contrast 127-131)
Code:
```
    let mut out = String::with_capacity(bytes.len() / 2 + 1);
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let u = unit([bytes[i], bytes[i + 1]]);
```
```
    // A stray leading U+FEFF (double BOM, dirty world) pollutes the
    // first index line — strip one.
    if text.starts_with('\u{FEFF}') {
        text.replace_range(0..'\u{FEFF}'.len_utf8(), "");
```
What is wrong: `decode_utf16` has no stray-BOM strip. A double-BOMmed UTF-16 file (`FF FE FF FE …` / `FE FF FE FF …`) — the exact "dirty world" case the UTF-8 path handles at :127-131 — decodes with a leading U+FEFF (ZWNBSP) character in the text, because after the caller strips the first two BOM bytes (:96/:100) the remaining payload begins with a second BOM code unit.
Why it matters: the leading ZWNBSP pollutes the first SRT index line (the very pollution the UTF-8 strip's comment describes); `srt/parser.rs`'s digit-only index check will reject the first cue — a double-BOMmed UTF-16 SRT silently loses its first cue while a double-BOMmed UTF-8 SRT parses clean. Inconsistent dirty-world policy across the three codecs in one file. Blast radius is one edge input class, hence S3 (the S2 case would be a common input shape).
What correct looks like: after the decode loop, `if out.starts_with('\u{FEFF}') { out.replace_range(0..3, ""); }` (mirroring :129-131), plus a test pinning `FF FE FF FE` → text without ZWNBSP. No test today covers any double-BOM form for any encoding.
Confidence: HIGH — Verification: read (all three decode paths traced; U+FEFF as a leading code unit reaches the plain `else` push at :233-236).

### F-002 [S4]
Where: crates/subtitles/src/srt/encoding.rs:192-194
Code:
```
    if out.starts_with('\u{FEFF}') {
        out.replace_range(0..'\u{FEFF}'.len_utf8(), "");
    }
```
What is wrong: dead code. This strip sits in `decode_windows1252`, whose output alphabet is Latin-1 (U+0000–U+00FF) plus the CP1252 high table (U+0152, U+0153, U+0160, U+0161, U+0178, U+017D, U+017E, U+0192, U+02C6, U+02DC, U+2013–U+203A, U+20AC, U+2122) — no mapping produces U+FEFF, and the function is only reached when `detect_bom` returned None (:111-117), so the input cannot begin `EF BB BF` either. The branch can never fire.
Why it matters: dead defensive code that implies a tolerance the CP1252 codec cannot need — a reader concludes all three paths share the stray-BOM policy, which F-001 shows they do not (the one path that needs the strip lacks it; the one that has it cannot use it).
What correct looks like: delete the strip here; add it to `decode_utf16` (F-001).
Confidence: HIGH — Verification: read (full table alphabet enumerated; call-site precondition traced).

### F-003 [S4]
Where: crates/subtitles/src/srt/encoding.rs:105-108 and 122-126
Code:
```
        _ if std::str::from_utf8(bytes).is_ok() => DecodedSource {
            // Reuse the lossy path purely for its stray-BOM strip
            // (the validation above guarantees no replacement).
            text: decode_utf8(bytes, damage),
```
```
    if std::str::from_utf8(bytes).is_err() {
        damage.record(SrtDamageKind::InvalidUtf8, 0);
    }
    let mut text = String::from_utf8_lossy(bytes).into_owned();
```
What is wrong: the no-BOM valid-UTF-8 path scans the full buffer three times (`from_utf8` at :105, again at :123, `from_utf8_lossy` at :126). The comment at :106-107 acknowledges the double work as intentional for the strip, but the file's own philosophy is "one pass, one output allocation" (:175, :199-200).
Why it matters: hygiene/perf inconsistency on the most common path; SRT sidecars are small so this is not a hot-path defect — S4.
What correct looks like: pass the validated `&str` down (`decode_utf8_str`) or have `decode_utf8` return early on prevalidated input; at minimum drop one of the two identical `from_utf8` scans.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/subtitles/src/srt/encoding.rs:14-15 and docs/feature-matrix.toml row `subtitle-encoding-detection` evidence
Code:
```
//! * no BOM, INVALID UTF-8 → **Windows-1252** (the common 8-bit
//!   fallback for legacy SRTs — ffmpeg's `sub_charenc` behaviour).
```
What is wrong: two doc-precision defects. (a) The matrix evidence string says "BOM matrix UTF-8/UTF-16LE/UTF-16BE + BOM-less UTF-8 lossy fallback + damage" — the actual BOM-less policy is: valid UTF-8 → UTF-8 (default, no fallback); invalid UTF-8 → Windows-1252. "BOM-less UTF-8 lossy fallback" names neither branch correctly. (b) The module doc attributes the CP1252 fallback to "ffmpeg's sub_charenc behaviour" — ffmpeg's `sub_charenc` is an explicitly user-supplied encoding option, not an automatic CP1252 fallback (the auto-detecting heuristic is VLC subsdec's). UNVERIFIED: no ffmpeg source in the sandbox to cite.
Why it matters: the row's notes field is accurate ("BOM-class detection landed; BOM-less UTF-16 deliberately NOT sniffed; full subsdec-class heuristics not started" — all true in code), so the row's "partial" status is honest; but the evidence phrase and the ffmpeg attribution misdescribe the shipped policy to anyone who reads only the matrix.
What correct looks like: evidence = "BOM matrix UTF-8/UTF-16LE/UTF-16BE + BOM-less valid-UTF-8 default + invalid-UTF-8→CP1252 fallback + typed damage"; module doc cites VLC subsdec (or drops the attribution).
Confidence: MEDIUM — Verification: read (code policy verified); ffmpeg/VLC behavior UNVERIFIED (no sources in sandbox, no network).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-29 | REAL | S4 | policy doc matches code except the ffmpeg attribution (F-004); "BOM-less UTF-16 not sniffed" policy + NUL-collision argument is sound (CP1252 never emits NUL) |
| `SourceEncoding` + `name` | 34-61 | REAL | — | 5 variants, stable diagnostic names |
| `DecodedSource` | 63-71 | REAL | — | plain data |
| `detect_bom` | 73-85 | REAL | — | 3 BOMs + None; partial prefixes rejected (test-pinned :294); arm order safe (prefixes disjoint) |
| `decode` | 87-119 | REAL | S4 | dispatch correct; triple validation on common path (F-003); CP1252 fallback records one damage event |
| `decode_utf8` | 121-133 | REAL | — | lossy + InvalidUtf8 damage + stray-BOM strip (UTF-8 double-BOM only — correct) |
| `CP1252_HIGH` | 135-171 | REAL | — | verified byte-for-byte vs Windows-1252; 5 None holes exactly at 0x81/8D/8F/90/9D |
| `decode_windows1252` | 173-196 | REAL | S4 | correct mapping; dead strip (F-002); one pass, one alloc as claimed |
| `decode_utf16` | 198-242 | REAL | S3 | correct pair math + all malformed shapes typed; missing stray-BOM strip (F-001) |
| `normalize_line_endings` | 244-263 | REAL | — | CR/CRLF/lone-CR → LF; "\r\r\n" edge pinned; O(n), single alloc |
| tests ×9 + `decode_parts` | 265-389 | REAL | — | all baseline-green; ordered damage vectors; gaps: double-BOM any form, ASCII-no-BOM trivial case |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bom_detection_matrix | YES | all 3 BOMs + 2 non-BOM shapes incl. partial prefix |
| utf8_bom_is_stripped | YES | encoding + first char + empty damage |
| utf16_le_and_be_decode_identically | YES | real sample (é, →, ünïcode) roundtripped through BOTH endians with exact text equality |
| unpaired_surrogates_and_odd_tail_degrade | YES | exact text + ordered 2-entry damage vec |
| invalid_utf8_falls_back_to_windows1252 | YES | 0xE9/0x93/0x94 mapped to é/“/” + single fallback damage |
| windows1252_undefined_bytes_degrade | YES | 0x81/0x8D → U+FFFD + ordered 3-entry damage |
| valid_multibyte_utf8_stays_utf8 | YES | multibyte UTF-8 not misread as CP1252 |
| bomd_utf8_invalid_payload_stays_lossy_utf8 | YES | BOM commits to UTF-8, no CP1252 switch |
| line_endings_normalize | YES | 3 shapes incl. the \r\r\n trap |
| (absent) double-BOM UTF-16 | — | NO test — would fail today (F-001) |
| (absent) UTF-8 double-BOM strip | — | the :129-131 strip is unpinned (EF BB BF EF BB BF case) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "BOM matrix UTF-8/UTF-16LE/UTF-16BE + BOM-less UTF-8 lossy fallback + damage" | feature-matrix.toml `subtitle-encoding-detection` evidence | SHALLOW (imprecise) | Actual: no-BOM valid→UTF-8 default, invalid→CP1252 fallback (:105-117). Notes field of the same row is accurate |
| row status partial x3 / web not_started | same row | REAL (honest) | BOM-class only; no BOM-less UTF-16 sniffing (deliberate, :23-29); no chardet heuristics — all true |
| "6/6 ffmpeg 7.1.5 differential incl. UTF-16 both endians" (srt row, this file's lane) | `subtitle-srt` evidence | REAL | baseline :5627 srt_canonical_differential 6/6 ok incl. `canonical_utf16_variants_parse_identically` |
| "all std-only (C5), all damage-tolerant, never panicking" | module doc :4-5 | REAL | std-only confirmed; panic-free trace (all slices prefix-proven by detect_bom; table index `b - 0x80` bounded by match 0x80..=0x9F) |

## 7. Dead / duplicated / unreachable code
F-002 dead strip in decode_windows1252. No duplication otherwise. Crate unreachable from shipped binaries (header chain).

## 8. Dependencies & flags
std-only; no unsafe; no features; damage sink from `crate::srt::damage` (another agent's file). `#[must_use]` on public fns.

## 9. Open questions for the next agent
- srt/parser.rs owner: how does the parser treat a leading U+FEFF mid-text (F-001's symptom) — typed damage on the first cue, or first-cue skip? Determines F-001's user-visible blast radius precisely.
- Anyone with ffmpeg/VLC sources: confirm `sub_charenc` is opt-in (F-004b attribution).
- srt/damage.rs owner: does `DamageSink` cap event count (repeated `UndefinedWindows1252Byte` on a large CP1252 file — unbounded Vec or capped ledger?).
