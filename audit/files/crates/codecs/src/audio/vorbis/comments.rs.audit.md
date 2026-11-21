# AUDIT: crates/codecs/src/audio/vorbis/comments.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Lines / functions | 379 / 19 (14 production fns/items + 4 tests + 1 test helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.ogg` → apps/cli/src/containers/mod.rs "ogg" arm → demux ogg harvests the header chain into facts → pipeline_host.rs:283-289 `"vorbis" => self.seeded(track, config, VorbisDecoder::new)` → `VorbisDecoder::configure` → `VorbisHeaders::from_extra_data` (headers.rs:222-224) → `comments::header_len` + `comments::parse`). Executed: baseline test-workspace.log.txt:498-501 (all four comment tests `ok`) and :1267 `tagged_fixture_exposes_its_vorbis_comments` `ok`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -n "comments::\|parse_comment_body\|VorbisComments" crates/codecs/src apps/cli/src` (callers: headers.rs:30,174,222,224; mod.rs:70 — no other callers, parse_comment_body has zero FLAC callers today); `rg -n "MAX_HEADER_BYTES" crates/demux/src/ogg/mod.rs` (95: `256 * 1024`, 1274: enforced on the header budget); `sed -n '210,240p' crates/codecs/src/audio/vorbis/headers.rs` (error propagation: `?` — comment failure kills the whole chain); `rg -n "vorbis" audit/runs/baseline/test-workspace.log.txt` (executed evidence); Read on the full file. |

## 1. One-paragraph verdict
A clean, in-house Vorbis-comment (and future FLAC VORBIS_COMMENT) parser: every length is read through bounds-checked helpers, the lying-count clamp bounds the loop, UTF-8 validation is typed, and the four in-file tests assert exact error codes and would fail under real mutations. No panic path exists (all accesses via `data.get`/checked slices; I traced the truncated-vendor, lying-count, framing-clear, no-`=`, and count-overrun cases against the tests that pin them). All findings are hygiene: the `MAX_ONE_COMMENT` doc mis-states the scale it "mirrors" (16 MiB vs the Ogg demuxer's 256 KiB — 64× apart) and would reject legitimate >16 MiB cover art on the advertised future FLAC reuse; `header_len` and `parse_comment_body` duplicate the same walk with a copy-pasted clamp formula (divergence would desync the chain split); and a missing framing byte is reported as framing-bit-clear rather than truncation.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:498-501` → `comments::tests::*` (roundtrip+case-insensitive lookup, repeatable tags, typed damage × 5 classes, body-length measurement) all `ok` (baseline run #3); `:1267` → the tagged fixture path exposing real comments through the full header chain.
— Verified by trace (read): the lying-count clamp math — with `count = 1000` and 8 body bytes remaining, `min(1000, 8/4+1) = 3`, iteration 2 fails at `read_len` with `vorbis.comment.truncated` — matches the test's asserted code exactly.
— Verified by `rg`: `parse`/`header_len` are called from exactly one production site each (headers.rs:222,224) and agree on the walk (identical clamp formula at 123 and 177) — the chain split boundary (`IDENT_LEN + comment_len`) is consistent with `parse`'s consumption (pinned by test 370-378: `header_len == full.len()`).

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/vorbis/comments.rs:34-37
Code:
```
/// A sanity ceiling for one comment's length (a comment body beyond
/// 16 MiB is damage, not metadata `[PROPOSAL: mirrors demux::ogg's
/// MAX_HEADER_BYTES scale]`).
const MAX_ONE_COMMENT: usize = 16 * 1024 * 1024;
```
What is wrong: The comment claims the ceiling "mirrors demux::ogg's MAX_HEADER_BYTES scale" — ogg's is `256 * 1024` (ogg/mod.rs:95), 64× smaller. Two consequences: (a) on the only shipped Vorbis lane (Ogg), the demuxer's 256 KiB total-header budget rejects oversized headers long before this ceiling can bound anything real — the ceiling's only reachable effect there is a different diagnostic on hostile length fields; (b) the module doc (lines 8-11) advertises that FLAC reuse of `parse_comment_body` "is a two-line composition" — but FLAC containers carry no 256 KiB cap, and real-world hi-res `METADATA_BLOCK_PICTURE` cover art exceeds 16 MiB (base64-inflated); on that reuse the ceiling would reject a legitimate file's header chain with `vorbis.comment.truncated` (headers.rs:222-224 propagate with `?` → the file fails to open).
Why it matters: The stated rationale is numerically false, and the advertised future composition imports a behavior trap (legit-file rejection) that the comment presents as sanity.
What correct looks like: Either size the ceiling honestly for its purpose (e.g., the same 256 KiB scale on the Ogg lane, or a per-container parameter when the FLAC reuse lands) and fix the comment, or gate the FLAC reuse on a container-aware limit.
Confidence: HIGH — Verification: read + rg (ogg/mod.rs:95; no FLAC callers today).

### F-002 [S4]
Where: crates/codecs/src/audio/vorbis/comments.rs:119-127 (vs 168-193)
Code:
```
    let mut pos = 0usize;
    let vendor_len = read_len(&data[7..], &mut pos, "vendor")?;
    read_slice(&data[7..], &mut pos, vendor_len, "vendor")?;
    let count = read_len(&data[7..], &mut pos, "count")?;
    let count = count.min(data[7..].len().saturating_sub(pos) / 4 + 1);
    for _ in 0..count {
        let len = read_len(&data[7..], &mut pos, "comment length")?;
        read_slice(&data[7..], &mut pos, len, "comment")?;
    }
```
What is wrong: `header_len` (measurement) and `parse_comment_body` (parse) re-implement the identical walk — including the lying-count clamp formula, duplicated verbatim at 123 and 177. If one is ever changed (clamp, framing semantics, field order) without the other, `from_extra_data`'s chain split (measure) and `parse`'s consumption (parse) desync: the setup header would be sliced at the wrong offset and the stream fails with a misleading `vorbis.setup.magic` — or worse, on a crafted file, the split could shift the boundary in ways the framing byte incidentally tolerates.
Why it matters: Duplicated structure parsers are the classic drift-seam; this pair is load-bearing for the entire Vorbis open path.
What correct looks like: One walker parameterized by a sink (`FnMut(&[u8]) -> Result<(), _>` or an enum `WalkOutcome { Len(usize), Parsed(VorbisComments) }`) with `header_len` and `parse` as thin wrappers.
Confidence: HIGH — Verification: read (both bodies compared line by line).

### F-003 [S4]
Where: crates/codecs/src/audio/vorbis/comments.rs:194-203
Code:
```
    let framing = data.get(pos).copied().unwrap_or(0);
    if framing & 0x01 == 0 {
        return Err(corrupt(
            "vorbis.comment.framing",
```
What is wrong: An absent framing byte (pos == data.len()) is collapsed into `0` and reported as `vorbis.comment.framing` ("framing bit is not set") rather than `vorbis.comment.truncated` — conflating "the header ends before its framing byte" (a length problem, which `header_len` at 129-138 reports correctly as truncated) with "the framing byte is present but malformed".
Why it matters: Diagnostic precision for direct `parse` callers; on the shipped path `from_extra_data` slices exactly the measured length (which verified presence), so this only misreports for out-of-band callers of the public `parse` — but the function is `pub` and its doc (145-147) promises the truncated code for exactly this class.
What correct looks like: `match data.get(pos) { None => Err(truncated…), Some(b) if b & 1 == 0 => Err(framing…), … }`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `VorbisComments` struct | 40-47 | REAL | — | Case-preserved keys, ordered pairs — matches doc. |
| `get` | 52-58 | REAL | — | First case-insensitive match; linear scan, metadata-path, fine. |
| `get_all` | 63-69 | REAL | — | Multiplicity preserved (spec §5.2.2); allocates per call, non-hot path. |
| `title`/`artists`/`album`/`date` | 72-93 | REAL | — | Thin conveniences over get/get_all. |
| `eq_ignore_ascii_case` | 98-103 | REAL | — | Length-first + byte fold — correct ASCII rule; non-ASCII keys compare byte-wise (spec-compliant enough; key charset itself is not validated — lax acceptance, noted, not elevated). |
| `header_len` | 111-140 | REAL | F-002 | Prefix gate; walk via checked helpers; framing PRESENCE check only (bit check is parse's job — asymmetric but consistent within the pair today); returns 7+pos+1 — pinned by test 370-378. |
| `parse` | 149-159 | REAL | — | Prefix gate duplicated from header_len (same 3-condition shape) then delegates. |
| `parse_comment_body` | 167-205 | REAL | F-002, F-003 | Vendor→count(clamped)→comments→framing; UTF-8 typed; `split_once('=')` = first-'=' key rule (spec §5.2); empty key accepted (spec-silent, harmless); count-overrun → setup-header bytes read as lengths → ceiling/truncation refuses (traced: `\x05vor…` LE u32 ≈ 1.9 GB > 16 MiB → typed refusal, no over-read). |
| `read_len` | 208-211 | REAL | — | LE u32 via checked 4-byte slice; `as usize` safe (u32). |
| `read_slice` | 215-245 | REAL | F-001 | Ceiling checked BEFORE bounds (hostile giant length → ceiling error, otherwise bounds error — both typed truncated); `data.get(*pos..*pos+len)` — no panic path. |
| `decode_utf8` | 248-257 | REAL | — | `std::str::from_utf8` + typed corrupt. |
| test `comments_roundtrip_with_case_insensitive_lookup` | 279-292 | REAL | — | Would fail on key/value split change, case-fold change, or storage-order change. |
| test `repeatable_tags_keep_multiplicity` | 294-312 | REAL | — | Would fail if get_all collapsed duplicates. |
| test `damage_is_typed_never_panicking` | 314-368 | REAL | — | Five damage classes with exact codes; would fail (panic or wrong code) if bounds checks were removed or codes renamed. |
| test `body_length_measurement_feeds_the_chain_split` | 370-378 | REAL | — | Would fail if header_len's arithmetic drifted from the real packet length. |
| test helper `comment_header` | 264-277 | REAL | — | Hand-built wire form, correct field order. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| comments_roundtrip_with_case_insensitive_lookup (279) | YES | Exact values + case-preserved key + folded lookup asserted. |
| repeatable_tags_keep_multiplicity (294) | YES | `vec!["a","b"]` asserted. |
| damage_is_typed_never_panicking (314) | YES | Exact code per class incl. the lying-count clamp path (`vorbis.comment.truncated` at 367) — removing the clamp would NOT be caught by this test directly (the unclamped loop still errors at read_len), but the clamp's purpose is defense-in-depth, not a behavior the test can distinguish; noted, not a defect. |
| body_length_measurement_feeds_the_chain_split (370) | YES | `len == full.len()` — any walk drift breaks it. |
| External: tagged_fixture_exposes_its_vorbis_comments (tests/vorbis_bitexact.rs) | YES | Real ffmpeg-authored fixture through the full chain; green in baseline (:1267). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "This is OUR parser, not the engine's — symphonia-codec-vorbis does not expose the comment header" | module docs 5-7 | REAL (per this repo's usage) | symphonia's vorbis crate is used only as the decode engine in decoder.rs; comments parsed here. Engine internals not independently verified (external crate) — UNVERIFIED beyond the wiring. |
| "FLAC's VORBIS_COMMENT shares this wire form … a future FLAC reuse is a two-line composition" | module docs 8-11 | REAL-UNPROVEN (future) | `parse_comment_body` is field-generic as claimed (no vorbis magic inside); zero FLAC callers today (rg). See F-001 for the ceiling trap. |
| "Keys are ASCII case-insensitive per the spec" | module docs 15-16 | REAL | eq_ignore_ascii_case 98-103 + test 291. |
| "Nothing panics; every length is bounds-checked before use" | module docs 23-25 | REAL | Traced every access: `get`/checked slices only. |
| "`[PROPOSAL: mirrors demux::ogg's MAX_HEADER_BYTES scale]`" | 34-37 | FALSE (numerically) | ogg's is 256 KiB (ogg/mod.rs:95); this is 16 MiB — F-001. |

## 7. Dead / duplicated / unreachable code
- `parse_comment_body` is `pub` but has zero non-test callers besides `parse` (the doc-advertised FLAC reuse has not landed) — public seam for the future, not dead.
- Duplication: header_len vs parse_comment_body walk (F-002); the prefix gate appears twice (112-118 vs 150-157).
- The `MAX_ONE_COMMENT` ceiling branch fires only on hostile length fields in practice (F-001).

## 8. Dependencies & flags
No external deps in this file (pure in-house parser; symphonia not touched here). Module rides `codec-vorbis` feature (default-on, Cargo.toml:62-63). No unsafe.

## 9. Open questions for the next agent
- vorbis/headers.rs owner: if `from_extra_data` ever caches or re-slices, keep it calling `header_len` and `parse` on the SAME sub-slice (they agree today; F-002 is the drift seam).
- FLAC owner: if the advertised reuse lands, resolve F-001's ceiling first — a >16 MiB FLAC picture must not fail the file open.
