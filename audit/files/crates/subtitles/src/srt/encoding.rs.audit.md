# AUDIT: crates/subtitles/src/srt/encoding.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Lines / functions | 389 / 7 fns + 9 tests |
| File verdict | REAL (pure, correct, damage-tolerant decoder) — inside the UNREACHABLE island crate |
| Reachable from shipped entry point? | NO (in-crate caller `srt::parser` only; `subtitles` compiled by no shipped host) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | prebuilt `subtitles-70f89de07410626f srt::encoding` -> 9 passed / 0 failed (exit 0); python3 cp1252 reference cross-check (table matches); sed quote verification |

## 1. One-paragraph verdict
This is the cleanest file of the batch: a std-only, panic-free byte decoder with a fully
correct Windows-1252 table (cross-checked against Python's cp1252 codec — all 32 high-half
entries match, including the five undefined bytes), correct UTF-16 surrogate-pair handling
and odd-tail damage, correct BOM dispatch, and a correct CR/CRLF normalizer. Damage events
are recorded for every degradation. Tests are real and pass (9/9 executed). Defects are
hygiene-grade only: double UTF-8 validation, a UTF-16 double-BOM inconsistency, and damage
events that all carry position 0.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f srt::encoding` ->
`test result: ok. 9 passed; 0 failed` — BOM matrix, UTF-16 LE/BE roundtrip of a
non-ASCII sample, unpaired-surrogate + odd-tail degradation, CP1252 fallback with
smart-quote roundtrip, undefined-byte U+FFFD, BOM-committed lossy UTF-8, line endings.
— verified by python3 cp1252 cross-check: `CP1252_HIGH` matches the reference codec
entry-for-entry (incl. 5 Nones at 0x81/0x8D/0x8F/0x90/0x9D).

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/srt/encoding.rs:105-108
Code:
```
        _ if std::str::from_utf8(bytes).is_ok() => DecodedSource {
            // Reuse the lossy path purely for its stray-BOM strip
            // (the validation above guarantees no replacement).
            text: decode_utf8(bytes, damage),
```
What is wrong: the no-BOM path validates UTF-8 twice (`from_utf8` here, then again inside
`decode_utf8` at :123). Wasted O(n) pass; the comment admits the reuse but not the cost.
Why it matters: minor perf, hot for every parsed SRT. What correct looks like: a
`decode_utf8_validated` that skips re-validation. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/subtitles/src/srt/encoding.rs:192-194 vs :129-131
Code:
```
    if out.starts_with('\u{FEFF}') {
        out.replace_range(0..'\u{FEFF}'.len_utf8(), "");
    }
```
What is wrong: the CP1252 path strips a leading U+FEFF, but `decode_utf16` (:201-242)
does NOT — a double-BOM'd UTF-16 file (`FF FE FF FE ...`) keeps a stray U+FEFF as text,
polluting the first index line exactly the way the UTF-8 path guards against. Inconsistent
dirty-world handling between paths. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/subtitles/src/srt/encoding.rs:112, :124, :183, :228, :231, :239
Code:
```
            damage.record(SrtDamageKind::Windows1252Fallback, 0);
```
What is wrong: every damage event records byte position 0 — all six call sites hardcode
`0`, so damage reports never say WHERE the bad byte was. Diagnostic value of the ledger is
halved. Confidence: HIGH — Verification: read (all `record(` calls in file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SourceEncoding::name | 51-60 | REAL | — | trivial const map |
| detect_bom | 74-85 | REAL | — | 3 BOMs; prefix-confusion case tested |
| decode | 88-119 | REAL | S4 | dispatch policy; double-validation (F-001) |
| decode_utf8 | 121-133 | REAL | — | lossy + stray-FEFF strip; InvalidUtf8 damage |
| decode_windows1252 | 176-196 | REAL | S4 | table 100% correct (python cross-check); FEFF strip (F-002 inconsistency) |
| decode_utf16 | 201-242 | REAL | S4 | surrogate join/unpaired/odd-tail all correct; no FEFF strip |
| normalize_line_endings | 247-263 | REAL | — | CR/CRLF→LF; "\r\r\n"→"\n\n" correct |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| bom_detection_matrix | YES | pins all 3 BOMs + 2-byte non-BOM |
| utf8_bom_is_stripped | YES | BOM gone, zero damage |
| utf16_le_and_be_decode_identically | YES | non-ASCII roundtrip both endians |
| unpaired_surrogates_and_odd_tail_degrade | YES | exact FFFD + damage-kind order |
| invalid_utf8_falls_back_to_windows1252 | YES | 0xE9/0x93/0x94 mappings pinned |
| windows1252_undefined_bytes_degrade | YES | 0x81/0x8D → FFFD x2 |
| valid_multibyte_utf8_stays_utf8 | YES | no CP1252 misdetection |
| bomd_utf8_invalid_payload_stays_lossy_utf8 | YES | BOM commits to UTF-8 |
| line_endings_normalize | YES | all 3 line-ending shapes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the five undefined bytes (0x81/0x8D/0x8F/0x90/0x9D) degrade to U+FFFD" | :16-19 | REAL | python3 cp1252 cross-check + test pass |
| "BOM-less UTF-16 ... degrades into garbage-block damage events" | :23-26 | REAL (pipeline, not this file) | damage kind exists (srt/damage.rs:41) and is recorded by the parser (srt/parser.rs:181); this module itself emits no damage for valid-UTF-8 NUL-interleaved text — the claim is about the srt::parse pipeline, accurate in that context |
| "`[VERIFY]` against a chardet-grade corpus once one is licensed in" | :20-21 | honest open limit | no corpus in tree |

## 7. Dead / duplicated / unreachable code
`char::from_u32(u as u32).unwrap_or('\u{FFFD}')` (:235) — fallback arm unreachable for
non-surrogate u16 (always Some); harmless dead defense. Whole module reachable only via
`srt::parser` (:78, :95) -> `srt::parse` -> crate root -> nothing shipped.

## 8. Dependencies & flags
Std-only. No unsafe, no features, no deps beyond `crate::srt::damage`.

## 9. Open questions for the next agent
- Does the parser tolerate a leading U+FEFF from the UTF-16 double-BOM case (F-002), or
  does the first cue get dropped as a garbage block?
- srt/damage.rs `DamageSink` cap behavior (MAX_DAMAGE_EVENTS) — unbounded file → bounded
  ledger? (damage.rs is another agent's file.)
