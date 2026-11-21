# AUDIT: crates/demux/src/wav/error.rs (197 lines, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 197 / 13 items (mint + 10 constructors + 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`apps/cli/src/containers/mod.rs:73` `"wav" => wav::open(...)`; constructors minted on live paths — `unsupported_format` called at wav/format.rs:402, 474, 486) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | read whole file; grep nofailfast log → `wav::error::tests::*` 2/2 ok (1910-1911); grep `payload_too_large` across demux (zero wav call sites; avi crate-root + aiff copies exist); grep `unsupported_format(` call sites (format.rs ×3); sed -n on all quotes |

## 1. One-paragraph verdict
Same disciplined pattern as the IVF error module: ten constructors, one `mint`, stable
`demux.wav.*` codes at stage Demux, no secrets, exhaustive code/Display table test — both
module tests green in the nofailfast baseline, and the interesting constructors
(`unsupported_format`, `truncated`, `bad_format_params`) have live minting sites in
wav/format.rs. Two real defects: `payload_too_large` is dead vocabulary by the module's
own admission — zero production minting sites, kept for hypothetical future callers and
tested only by its own table (the identical dead copy exists in aiff/error.rs) — and
`unsupported_format(_offset)` takes an offset parameter it silently drops while its
sibling `bad_format_params` carries the same offset into the error detail, so the
signature promises location information the error never contains.

## 2. What actually works (verified)
— 2/2 module tests ok in baseline (log 1910-1911): the 10/10 code+Display table and the
64×0xAB sniff cap (≤16 rendered).
— Live minting sites verified: `unsupported_format(offset)` called at wav/format.rs:402
(WAVE_FORMAT_EXTENSIBLE without a ≥40-byte body), 474, 486 — the doc's three documented
conditions map to real parse branches.
— `no_data`'s shape is honest (`InvalidStructure { offset: None }` — unlike IVF's
fabricated `Truncated{1,0}` flagged in the ivf/error.rs audit this wave).
— `bad_format_params`'s doc records the demux/decode consistency rationale (accepting a
bad `nBlockAlign` would mint a stream the PCM decoder always rejects) — a real layering
argument, and the PCM decoder re-check is pinned at codecs (not re-verified here).

## 3. Findings

### F-001 [S3] `payload_too_large` is dead vocabulary — zero minting sites, admitted in its own doc, duplicated dead in aiff
Where: crates/demux/src/wav/error.rs:147-162
Code:
```
/// A payload ask exceeds the demuxer's packet ceiling. Vocabulary
/// without a minting site today: the packetizer never asks above
/// [`crate::wav::MAX_PACKET_BYTES`], and larger chunk payloads are
/// positionally packetized instead of read whole (the
/// `avi::payload_too_large` precedent — the typed flavor is kept for
/// callers that choose to fail fast).
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: grep over the demux crate finds zero call sites for
`wav::error::payload_too_large` outside its own test table (error.rs:180) — the doc
admits it ("without a minting site today"). The same dead constructor, with the same
admission-style doc, is copy-pasted in `crates/demux/src/aiff/error.rs:139-141`. Dead
public API kept for callers that do not exist, tested only against itself.
Why it matters: dead error vocabulary inflates the typed-refusal surface an auditor or
UI must map; a future refactor that breaks it is invisible; and its presence implies a
memory-safety refusal exists in the WAV path when the actual protection is the
positional packetizer (nothing ever mints this).
What correct looks like: delete it until a real caller exists (git history keeps the
precedent), or wire it at the actual ceiling check in the packetizer.
Confidence: HIGH — grep-verified zero call sites; doc admits it.

### F-002 [S4] `unsupported_format(_offset)` drops the offset its signature promises
Where: crates/demux/src/wav/error.rs:92-102
Code:
```
pub fn unsupported_format(_offset: u64) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("wav".to_string()),
                sniffed: Vec::new(),
```
What is wrong: the three call sites (format.rs:402, 474, 486) all pass a real offset, and
the constructor discards it (`_offset`). The sibling `bad_format_params` (75-84) carries
its offset in `InvalidStructure { offset: Some(_) }`; this one cannot because `Unsupported`
/`FormatInfo` has no offset slot — so the parameter is pure signature noise that suggests
the error locates the problem when it never does.
Why it matters: an operator debugging "unsupported format" gets no position while the
adjacent "bad fmt" error does — inconsistent diagnostics from the same parser.
What correct looks like: drop the parameter (and fix the three call sites), or extend
FormatInfo with an optional offset so the hint is real.
Confidence: HIGH — read all sites.

### F-003 [S4] `mint` duplicated verbatim across the demux crate's error modules
Where: crates/demux/src/wav/error.rs:23-25 (also ivf/error.rs:20, es/error.rs:12, ogg/error.rs:19, aiff/error.rs, crate-root error.rs)
Code:
```
fn mint(kind: ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}
```
What is wrong: identical helper copy-pasted per module; the header doc defers
consolidation to R3 (lines 7-9) — same finding as the ivf/error.rs audit (F-003 there);
counted instances now six (wav, ivf, es, ogg, aiff, crate-root/avi).
Confidence: HIGH — grep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` | 23-25 | REAL | S4 | duplication F-003 |
| `not_wav` | 30-40 | REAL | — | sniffed magic; cap tested |
| `truncated` | 47-54 | REAL | — | honest expected/found; covers RIFF head/chunk header/fmt/data cut (doc) |
| `bad_structure` | 59-68 | REAL | — | InvalidStructure + offset |
| `bad_format_params` | 75-84 | REAL | — | carries offset; consistency rationale documented |
| `unsupported_format` | 92-102 | REAL | S4 | drops `_offset` (F-002); live call sites ×3 |
| `no_fmt` | 107-114 | REAL | — | offset None honest (no single position) |
| `no_data` | 119-126 | REAL | — | honest shape; "WAV twin of avi::no_movi" |
| `not_started` | 131-136 | REAL | — | operational shape |
| `seek_out_of_range` | 140-145 | REAL | — | doc lists three causes (before start / past end / absent stamp) — unlike IVF's contradicted doc, no contradiction traced here |
| `payload_too_large` | 153-162 | REAL (code) / DEAD (usage) | S3 | zero minting sites (F-001) |
| `errors_carry_demux_stage_and_wav_codes` | 168-189 | REAL | — | exhaustive 10/10 + Display shape |
| `not_wav_sniff_capped_at_sixteen_bytes_by_display` | 191-196 | REAL | — | cap proven |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_wav_codes | YES | every constructor's code + Display asserted |
| not_wav_sniff_capped… | YES | 64 bytes → ≤16 rendered |
| (payload_too_large row inside the table) | NO — it tests dead vocabulary against itself | F-001 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Vocabulary without a minting site today" | error.rs:148 | TRUE (self-indicting) | grep: zero call sites — F-001 |
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | error.rs:13-14 | REAL | read all ten; the one violation of *spirit* is the dropped offset (F-002) |
| "the PCM decoder re-checks on the facts blob" | error.rs:72-73 | REAL-UNPROVEN | layering claim about codecs/src/audio/pcm — not re-verified in this audit (out of my files) |
| "Sniffed magic bytes are capped by the taxonomy's Display" | error.rs:14-15 | REAL | tested, baseline ok |

## 7. Dead / duplicated / unreachable code
- `payload_too_large` — dead (F-001), duplicated dead in aiff/error.rs:141.
- `mint` ×6 (F-003).
- Nothing unreachable: the WAV lane is wired (`wav::open` arm).

## 8. Dependencies & flags
media_types only; no unsafe; no feature gates. Test block cfg(test) only.

## 9. Open questions for the next agent
1. aiff/error.rs (not my assignment) carries the same dead `payload_too_large` with the
   same admission — whoever audits aiff should cite this finding rather than re-derive.
2. If `FormatInfo` gains an offset slot (F-002 fix), IVF's `unsupported_fourcc` could
   carry the frame position too.
