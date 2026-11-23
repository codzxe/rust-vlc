# AUDIT: crates/subtitles/src/vtt/text.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 178 / 8 (4 prod incl. 2 private helpers, 4 tests) |
| File verdict | SHALLOW — correct happy-path semantics with executed proof of a quadratic CPU path on untrusted cue text and cross-line tag stripping that contradicts the module's own spec note |
| Reachable from shipped entry point? | NO — crate unlinked (parser.rs F-001); matrix row subtitle-vtt claims "implemented" ×3 platforms (docs/feature-matrix.toml:926) on the same unwired crate |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rustc probe (audit/runs/probe_vtt.rs vs prebuilt rlib, exit 0 — timings + outputs below); rg feature-matrix vtt row (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
The exactly-once tag-then-entity decode order is right and test-pinned, the five named
entities are exactly the WebVTT spec set, and malformed references pass through literally —
good, honest, small code. But I executed two hostile inputs the in-crate fuzz corpus does not
cover: (1) a single 200 KB cue of bare ampersands takes 954 ms to decode — `decode_one`'s
`find(';')` rescans the whole remaining cue per `&`, quadratic on input with no `;` — while
the "large_document_under_200kb_is_survivable" test only feeds `&amp;` (semicolon close by)
and `<`×199000, so the suite's "must not hang" claim was never exercised against the one
byte that makes it quadratic; (2) a stray `<` on one cue line strips through the newline into
later lines up to a distant `>`, destroying their text — the file's own doc says "Tags cannot
span lines in WebVTT". Both executed; both real.

## 2. What actually works (verified)
— probe_vtt (exit 0): `&lt;i&gt;real&lt;/i&gt;`-order semantics confirmed by in-crate tests
  (baseline pass) and my probe's clean cases; 200 KB `<`-only cue parses fast (strip_tags
  returns on first unterminated `<`, O(n)).
— Baseline: 4/4 text tests + 3/3 vtt_fuzz tests pass (cargo-test-workspace.txt:5965-5970
  region and vtt_fuzz block at 6135-6141).

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/vtt/text.rs:61-73 with 82-84
Code: (verbatim)
```
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        match decode_one(after) {
```
with
```
    let semi = s.find(';')?;
    let name = &s[..semi];
```
What is wrong: for every `&`, `decode_one` scans forward to the next `;` (or the end of the
whole cue text when none exists). A cue of n bare ampersands performs n scans of length
n, n−1, … — O(n²). Executed (probe_vtt, release-opt probe): one 200 KB cue of `&` →
**954 ms**; `&x`-repeated 200 KB → **480 ms**. A subtitle file is untrusted input parsed on
the load path; a crafted file with a handful of such cues stalls the loader for seconds
(20 cues ≈ 18 s). The in-crate survival suite claims "large document under 200 KB is
survivable" but its corpus only contains `&amp;` (scan stops at the near `;`) and
`<`×199_000 (strip_tags' O(n) path) — vtt_fuzz.rs:161-183 never feeds semicolon-free
ampersands, so the quadratic path is untested. What correct looks like: bound the entity
scan (e.g. cap `find(';')` at the next `&`/newline/32 bytes — WebVTT entity names are ≤ 6
chars) or pre-scan for `;` once. Confidence: HIGH — Verification: executed (timings above).
F-002 [S3]
Where: crates/subtitles/src/vtt/text.rs:27-29 vs 36-45
Code: (verbatim, doc)
```
/// Remove every `<...>` tag. Tags cannot span lines in WebVTT, but we
/// tolerate the whole document being one string: an unterminated `<`
/// truncates the remainder (documented tolerance, never a panic).
```
vs (verbatim, code)
```
        match after.find('>') {
            Some(gt) => rest = &after[gt + 1..],
```
What is wrong: `after` spans ALL remaining lines of the cue (the caller joins lines with
`\n`), so a stray `<` on line 1 strips through the newline up to a `>` on a later line —
exactly the tag-spanning-lines behavior the doc says WebVTT forbids. Executed (probe_vtt):
cue text `a < b\nc > d` → decoded text `Plain("a  d")` — the newline and `c ` are silently
destroyed. Real WebVTT parsers treat the malformed tag as line-scoped. What correct looks
like: search for `>` only within the current line (`find(['>', '\n'])`). Confidence: HIGH —
Verification: executed.
F-003 [S4]
Where: crates/subtitles/src/vtt/text.rs:9-11
Code: (verbatim)
```
//! non-recursive pass, HTML semantics). A `<` with no closing `>` on
//! the line is malformed; the tolerant choice is to drop the rest of
//! the payload from there (per spec it cannot be cue text).
```
What is wrong: "no closing `>` on the line" is not what the code checks (it checks the whole
remaining payload), and "drop the rest of the payload" silently destroys every subsequent
line of a multi-line cue for one malformed line — executed: `keep <oops dropped\nsecond
line` → `Plain("keep ")`, "second line" gone. The tolerance is documented, but the
line-vs-payload wording misleads; per spec only the malformed line should be affected.
Confidence: HIGH — Verification: executed.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| decode_cue_text | 22-25 | REAL | — | tags → entities order (verified by tests) |
| strip_tags | 30-49 | REAL | F-002, F-003 | O(n); unterminated-< truncation executed |
| decode_entities | 55-77 | REAL | F-001 | quadratic on semicolon-free ampersands (executed 954 ms/200 KB) |
| decode_one | 81-114 | REAL | F-001 | unbounded `find(';')`; named set exactly the 5 WebVTT entities; control/BOM refused; overflow tolerated |
| tests ×4 | 120-177 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| strips_common_tags | YES | v/c/ruby/karaoke-stamp cases |
| decodes_named_and_numeric_escapes | YES | exact values incl. hex + emoji |
| escapes_decode_exactly_once_and_order_is_tags_first | YES | the &amp;lt; / &lt;i&gt; order proofs |
| malformed_input_is_tolerated | YES | bare &, bad numeric, surrogate, NUL, unterminated tag, multi-line, empty |
Honest suite for the semantics it covers; the coverage GAP (semicolon-free `&` floods,
cross-line `>`) is what let F-001/F-002 through — the fuzz suite shares the gap
(vtt_fuzz.rs:161-183).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "strip tags first, decode escapes second … exactly once" | lines 6-9 | REAL | tests 150-156 (baseline pass) |
| "A `<` with no closing `>` on the line is malformed; … drop the rest of the payload" | lines 9-11 | FALSE wording | code scans whole payload, not the line (F-002/F-003, executed) |
| "Tags cannot span lines in WebVTT, but we tolerate the whole document being one string" | lines 27-28 | FALSE for the code | code lets tags span lines up to a later `>` (F-002, executed) |
| "Styling is DEFERED, like the ASS route" | lines 13-18 | REAL | decode returns plain text; matrix notes "tags degrade to plain runs" (docs/feature-matrix.toml:928) — honest |
| matrix: subtitle-vtt "implemented" win/linux/macos | docs/feature-matrix.toml:926 | FALSE at product level | crate unlinked from shipped apps (parser.rs F-001) |

## 7. Dead / duplicated / unreachable code
None in-file. Module is pub(super)-private to vtt; the crate is unlinked from shipped
entry points (see parser.rs F-001).

## 8. Dependencies & flags
Std-only, no unsafe, no features, no deps. All functions allocation-light except the output
String (single with_capacity).

## 9. Open questions for the next agent
- vtt/mod.rs + timestamp.rs auditors: same `find`-rescan pattern anywhere else? (My probe
  only exercised the cue-text path.)
- If F-001 is fixed by bounding the scan, re-run probe_vtt to confirm the 954 ms figure
  drops to <5 ms.
