# AUDIT: crates/codecs/src/audio/mp3/id3.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 621 / 17 (9 prod incl. 2 types + `Id3Tag::text`, 8 tests + 1 test helper) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point calls it — see §1) |
| Reachable from shipped entry point? | NO for the shipped `lumen` CLI/tauri packet path. Compiled in (feature `codec-mp3` default-on; `apps/cli` deps `codecs` with defaults), but `parse_id3v2` callers are only `mp3::decode_stream` (crates/codecs/src/audio/mp3/decoder.rs:193), `mp2::decode_stream` (mp2/decoder.rs:131), `is_mp2_stream` (mp2/header.rs:307) — all test/fuzz-only (rg over `apps/`+`crates/` shows no other caller). The shipped MP3 lane (`apps/cli/src/pipeline_host.rs:261-265` → `Mp3Decoder::decode`) never parses ID3: the demux-side `skip_id3` (crates/demux/src/es/mp3.rs:352-377) does its own header-only skip. |
| Findings | S0: 0 S1: 1 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "parse_id3v2\|id3v2_len" crates apps` → callers listed above; `sed -n` on all quoted ranges; `od -c` on mp2/header.rs:302 (display-mangle check); `rg -n '"mp3"\|"mp2"' crates/demux/src` → ES demuxer mints mp3/mp2 tracks (demux-side ID3 skip is separate code) |

## 1. One-paragraph verdict
This is a genuine, substantially correct ID3v2.2/2.3/2.4 parser with honest typed error paths for oversized tags, frames past the tag end, and bad encodings — not a stub. It carries one real defect: `parse_picture` indexes `payload[i..]` with an `i` that can exceed the payload length, panicking on any APIC/PIC frame whose declared size is shorter than the mime/type/description skeleton (a 0–4 byte picture frame), directly violating the crate's documented "no panics on any input" convention (crates/codecs/src/lib.rs:49-52) on the exact surface the registered fuzz target `fuzz_codec_mp3` hammers. Second, the v2.3 frame-flag bits are wrong (encryption checked at v2.4's bit position), so encrypted/grouped v2.3 frames and v2.4 data-length-indicator frames decode as garbage instead of being refused. None of it is reachable from the shipped binary's decode path because the demux layer skips ID3 with its own separate parser; the code is reachable only through the public `decode_stream` helpers, the fuzz target, and in-tree tests.

## 2. What actually works (verified)
— `id3v2_len` / `parse_id3v2` round-trip v2.3, v2.4+footer+syncsafe frame sizes, v2.2 3-char ids, global unsync reversal, and the three typed refusal paths: verified by reading the test bodies (lines 449-620) and tracing each assertion against the code (no cargo runs; per-instruction ban). The v2.3 unsync semantics (tag size = physical, frame sizes = logical) match ID3v2.3.0 §3.1/§5 and the test `unsynchronisation_is_reversed` (lines 514-540) asserts it.
— Baseline `cargo test --workspace` (audit/runs/baseline/INDEX.md run #3: 1681 passed / 1 failed — the one failure is `apps/desktop/tests/zero_alloc_transport.rs`, unrelated) covers this module's tests in the release profile.

## 3. Findings

### F-001 [S1] Slice-index panic in `parse_picture` on truncated APIC/PIC frames
Where: crates/codecs/src/audio/mp3/id3.rs:377-416 (panic sites 387 and 410)
Code:
```rust
    let mut i = 0usize;
    let _enc = payload.first().copied().unwrap_or(0);
    i += 1;
    let mime = if id == "PIC" {
        i += 3;
```
and (second site, lines 409-415):
```rust
    } else {
        let end = payload[i..]
            .iter()
            .position(|&b| b == 0)
```
What is wrong: `parse_picture` advances `i` past the encoding byte (and, for PIC, the 3-byte format field) and then evaluates `payload[i..]` with direct slice indexing. For an APIC frame with declared size 0 (payload empty), `i` is 1 and `payload[1..]` panics ("range start index 1 out of range for slice of length 0"). For payloads of 1–4 bytes the later description scan `payload[i..]` at line 410 panics with start > len (verified by index arithmetic: a 3-byte payload `[0x00,'A',0x00]` reaches line 410 with `i == 4 > len 3`). The frame loop calls `parse_picture` for ANY `id == "APIC" || id == "PIC"` (line 357) with no minimum-size gate, and `end = start.saturating_add(size)` only bounds the frame against the tag body, never against the picture skeleton.
Why it matters: crates/codecs/src/lib.rs:49-52 promises "No `unsafe`, no panics on any input: every decode path is bounds-checked … the fuzz targets … hammer exactly this guarantee", and mp3/decoder.rs:184-186 claims "Damage is never a panic". One corrupted or writer-truncated cover-art frame turns the whole `decode_stream` call (and the registered fuzz target `fuzz_codec_mp3`, crates/codecs/src/audio/mp3/fuzz.rs:94-96, which calls `parse_id3v2(data)` on arbitrary bytes) into a process panic. The in-tree tests only feed well-formed picture payloads (test at lines 566-585), so the guarantee is untested where it fails.
What correct looks like: use `payload.get(i..)` (the module's own idiom at lines 392, 420) or bail to an `Other` frame / typed `mp3.id3.picture_truncated` error when `payload.len() < 4`; add a fuzz-corpus seed with a zero-size APIC frame.
Confidence: HIGH — Verification: read; index math re-derived (empty payload → `payload[1..]` panics; 3-byte payload → `payload[4..]` panics). Not executed (cargo ban); the shipped CLI packet path provably never calls this parser, which is the only reason this is not an S0/S1 on the user-visible path.

### F-002 [S3] v2.3 frame-flag bits taken from the v2.4 layout; grouping/data-length-indicator ignored
Where: crates/codecs/src/audio/mp3/id3.rs:343-347
Code:
```rust
        // v2.3/2.4 compression and encryption are refused, not guessed.
        let compressed_23 = version.0 == 3 && frame_flags & 0x0080 != 0;
        let compressed_24 = version.0 >= 4 && frame_flags & 0x08 != 0;
        let encrypted = version.0 >= 3 && frame_flags & 0x04 != 0;
```
What is wrong: `encrypted` tests bit 0x04 for `version.0 >= 3`, but 0x04 is the **v2.4** encryption bit ('m' in v2.4's `%0h00kmnp`). In v2.3 the format-flag byte is `%ijk00000`: encryption is bit 0x40 ('j'), grouping is 0x20 ('i'). So a v2.3 encrypted frame (0x40) slips past the refusal and its payload is decoded as ordinary text/picture garbage; likewise v2.3 grouping (0x20) and v2.4 grouping (0x40) each prepend one byte to the payload that this parser does not strip, and the v2.4 data-length indicator (bit 0x01, 4 syncsafe bytes prepended) is never removed — every text value from such frames is wrong by the shifted prefix.
Why it matters: the module's stated policy (line 343, "refused, not guessed") is only half-enforced; tags written by encoders that set these flags get silently wrong metadata instead of the typed `mp3.id3.frame_transform` refusal. Blast radius is limited to metadata fidelity (rare flags, no panic, no audio impact).
What correct looks like: `let encrypted = version.0 == 3 && frame_flags & 0x40 != 0 || version.0 >= 4 && frame_flags & 0x04 != 0;` plus stripping the group byte (both versions) and the 4-byte data-length indicator when set.
Confidence: HIGH — Verification: read (v2.3.0 §3.3.1 / v2.4.0 §4. flag layouts from spec).

### F-003 [S3] TXXX/TXX user-defined text frames keep the embedded description in the value
Where: crates/codecs/src/audio/mp3/id3.rs:354-356
Code:
```rust
        if id.starts_with('T') {
            let value = decode_text(payload)?;
            frames.push(Id3Frame::Text { id, value });
```
What is wrong: every frame whose id starts with 'T' — including TXXX (v2.3/2.4) and TXX (v2.2), whose payload is `encoding, description<0x00>, value` — is fed wholesale to `decode_text`. For an enc-0 TXXX the decoded `value` is `"description\0value"` (only *trailing* NULs are trimmed, line 133), so callers get the description glued to the value with an embedded NUL.
Why it matters: user-defined text frames (TXXX "replaygain_track_gain" etc.) are common in real tags; `Id3Tag::text("TXXX")` returns contaminated strings. Metadata-only defect; no panic.
What correct looks like: special-case TXXX/TXX — split at the first terminator (encoding-aware, same logic as `parse_picture`'s description skip) and keep only the tail.
Confidence: HIGH — Verification: read; payload grammar from ID3v2.3.0 §4.2.2.

### F-004 [S4] `pos + skipped` can overflow on 32-bit targets (wasm) with a hostile v2.3 extended-header size
Where: crates/codecs/src/audio/mp3/id3.rs:279-297
Code:
```rust
            _ => {
                // v2.4: syncsafe size INCLUDING itself.
                syncsafe32(&body[pos..]).unwrap_or(0) as usize
            }
        };
        pos = (pos + skipped).min(body.len());
```
What is wrong: for v2.3 the skipped value is `4 + sz` where `sz` is an unvalidated plain-BE u32 (up to 0xFFFF_FFFF). On 64-bit that is fine; on the 32-bit wasm target (`apps/web`, a shipped surface) `pos + skipped` overflows: panic in debug, wrap in release (silently re-parsing from a wrong offset). Also the `_` arm applies the v2.4 syncsafe path to **v2.2** tags whose flag bit 0x40 is undefined there.
Why it matters: the crate is compiled for wasm32 (per docs/web/size-report.md lane); a crafted extended-header size is file-controlled. Hygiene-level because the realistic outcome is a wrong skip, not memory unsafety.
What correct looks like: `pos = pos.saturating_add(skipped).min(body.len());` and gate the `_` arm on `version.0 == 4` (treat v2.2 as no extended header).
Confidence: MEDIUM — Verification: read; overflow behavior from Rust integer semantics. Not executed on wasm.

### F-005 [S4] Dead error branch + false comment claiming damage accounting that does not exist
Where: crates/codecs/src/audio/mp3/id3.rs:92-94 and 215-225
Code:
```rust
    // A set high bit in any byte is a spec violation; tolerate it by
    // masking (never a panic, damage noted by the caller when sizes
    // disagree — the honest behavior for damaged tags).
```
What is wrong: (a) no caller "notes damage" — `decode_stream` (mp3/decoder.rs:188-197) consumes `total_len` blind, so a masked (under-read) size silently mis-frames the tag; the comment describes behavior that does not exist. (b) The `syncsafe32(&head[6..10])` `None` branch at 215-225 is unreachable: `head` is exactly 10 bytes, so the slice is always ≥4 — defensive dead code with an error code (`mp3.id3.size_field`) that can never fire.
Why it matters: comments asserting non-existent damage accounting are exactly the "trust nothing" class this audit exists for; dead typed-refusal paths inflate the error-taxonomy claim ("a truncated syncsafe size field" in the module doc, line 22, can never occur).
What correct looks like: delete or gate the comment to reality; either drop the unreachable branch or feed `syncsafe32` a genuinely variable-length slice.
Confidence: HIGH — Verification: read (`head` bound at 193-206 `data.get(..10)`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum Id3Frame` (3 variants) | 30-58 | REAL | — | Plain data type. |
| `struct Id3Tag` + `total_len` invariant | 60-74 | REAL | — | `total_len` verified against tests (449-464, 500-511). |
| `Id3Tag::text` | 76-86 | REAL | — | First-match lookup, correct. |
| `syncsafe32` | 88-101 | REAL | S4 (F-005b) | Masking tolerance OK; comment overclaims (F-005a). |
| `deunsynchronise` | 103-117 | REAL | — | `FF 00`→`FF`, in-bounds via `get`; matches v2.3/v2.4 scheme. |
| `decode_text` | 119-172 | REAL | — | enc 0/1/2/3 all lossy-safe, no panics; BOM-less UTF-16 defaults LE (tolerance, sane). |
| `body_utf16be` | 174-181 | REAL | — | Chunked, terminator-aware. |
| `id3v2_len` | 183-244 | REAL | — | Oversize→typed `past_end`; version gate 2..=4; footer only ≥2.4 (spec-correct). |
| `parse_id3v2` | 246-374 | REAL | S3 (F-002, F-003), S4 (F-004) | Extended-header skip v2.3/v2.4 shapes match spec; frame loop bounds-safe; flag layout bug F-002. |
| `parse_picture` | 376-422 | REAL | S1 (F-001) | Two `payload[i..]` panic sites; never returns errors (module doc's typed-error promise does not extend here). |
| `tag_v23` (test helper) | 428-447 | REAL | — | Builds a well-formed v2.3 tag. |
| 8 tests | 449-620 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `skips_and_parses_a_v23_tag` (449) | YES | Asserts len == tag.len(), version, text value, total_len. |
| `a_tag_claiming_more_than_the_file_is_typed_truncation` (467) | YES | Mutates size field → asserts `mp3.id3.past_end`. |
| `v24_footer_and_syncsafe_frame_sizes` (482) | YES | Footer counted in len; syncsafe frame size parsed; asserts text("TALB"). |
| `unsynchronisation_is_reversed` (514) | YES | FF 00 → FF with logical v2.3 sizes; asserts "\u{FF}A". |
| `v22_three_char_ids_and_picture_frames` (543) | PARTIAL | TT2 walk asserted honestly; the picture half feeds only a well-formed 27-byte APIC — the F-001 panic paths (0–4 byte payloads) are untested, which is exactly why the bug survives. |
| `bad_text_encoding_is_typed` (589) | YES | enc=9 → typed error. |
| `no_magic_is_none_not_error` (598) | YES | None vs typed truncated-header split. |
| `a_frame_running_past_the_tag_is_typed` (610) | YES | Corrupts frame size → `frame_past_end`. |
Mutation check (reasoned, not run): deleting the `end > body.len()` guard (325) would flip `a_frame_running_past_the_tag_is_typed` to fail; deleting the footer term in `id3v2_len` (227) would flip `v24_footer_and_syncsafe_frame_sizes`. Suite has real teeth — except on truncated pictures.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Error paths (each typed …): tag claiming more bytes than the buffer holds, frame running past the tag body, a bad text encoding byte, a truncated syncsafe size field" | module doc id3.rs:20-24 | SHALLOW | First three exist (lines 229, 326, 165); "truncated syncsafe size field" is dead code (F-005b); picture frames have NO typed error path and instead panic (F-001). |
| "No `unsafe`, no panics on any input" | crates/codecs/src/lib.rs:49-52 | FALSE (for this module) | F-001: `payload[i..]` panics on a 0–4 byte APIC/PIC payload. |
| "Damage is never a panic" | mp3/decoder.rs:184-186 (`decode_stream` doc) | FALSE | `parse_id3v2(data)?` at decoder.rs:193 reaches `parse_picture`. |

## 7. Dead / duplicated / unreachable code
- Whole module is UNREACHABLE from shipped entry points today: shipped MP3 decode goes packet-by-packet (`pipeline_host.rs` "mp3"/"mp1" → `Mp3Decoder::decode` → symphonia), and the demuxer's own `skip_id3` (demux/src/es/mp3.rs:352-377, separate implementation, does not parse frames) handles tags. `decode_stream` callers: crates/codecs/tests/{mp3_fuzz_survival,mp3_corpus_differential,mp3_bitexact}.rs + tests/mp3/mod.rs + fuzz target only.
- Duplication: THREE ID3 length parsers exist in-tree (this module, demux/src/es/mp3.rs:352-377, and mp2's use of this one); the demux one additionally differs in semantics — it applies the footer flag (`tag[5] & 0x10`) regardless of version, where this module correctly gates on v2.4 (id3.rs:226). Divergent copies of one spec = drift hazard (S3-class, owned by the demux wave).
- Dead: F-005b branch.

## 8. Dependencies & flags
- Feature `codec-mp3` (default on; `codecs` Cargo.toml:55,78). `apps/cli` depends on `codecs` with defaults → module compiled into the shipped binary even though unreachable at runtime (dead weight in the binary, not a defect).
- No unsafe, no external deps in this file.

## 9. Open questions for the next agent
- mp3/decoder.rs (its agent): does the packet lane ever see ID3 bytes (e.g., a container that passes the whole tag as a packet)? If yes, F-001 becomes shipped-user-visible and should be re-rated S0-adjacent.
- Demux wave: demux/src/es/mp3.rs:368 applies the v2.3-unspecified footer bit — flag to that owner.
- Fuzz wave: has `fuzz_codec_mp3` ever run with a tagged seed corpus? One mutation of the `tagged_art.mp3` corpus fixture's APIC size field to 0 should find F-001 instantly; `scripts/fuzz_registry_check.sh` only diffs the registry (baseline #18: "35 targets, diff empty") — it proves nothing about panic-freedom.
