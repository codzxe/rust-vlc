# AUDIT: crates/codecs/src/audio/vorbis/headers.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 400 / 8 prod items (+7 consts) + 2 test helpers + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open *.ogg` → `apps/cli/src/containers/ogg.rs:202-208` joins the three header packets → `pipeline_host.rs:271` seeded `VorbisDecoder` → `vorbis/decoder.rs:714` `VorbisHeaders::from_extra_data` → headers.rs:220; also `apps/cli/src/demux_host.rs:210` `parse_identification` in the open-path geometry seam; MKV lane via `containers/mkv.rs` `vorbis_chain` → same decoder path). Executed in baseline: `containers/ogg.rs:364` `vorbis_fixture_decodes_the_full_stream` (66,150 samples) passed in run #3. |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n 'is_vorbis_ident\|from_packets\|from_extra_data\|engine_extra_data\|parse_identification'` (caller map); `sed -n` quote verification; hand-decode of the spec §4.2.2 field layout; cross-read `vorbis/comments.rs:111-140` (header_len bounds guarantee). |

## 1. One-paragraph verdict
A spec-faithful, well-tested header parser whose identification-header validation I verified field-by-field against Vorbis I §4.2.2 (layout, LE endianness, version-0 gate, exponent range 6..=13, bs0≤bs1, framing bit) — and it is genuinely wired into both shipped Vorbis lanes (Ogg, MKV) with a passing end-to-end baseline test. But the chain split has a slice-index panic on a truncated setup header: only `is_empty()` is guarded before `&setup[1..7]`, so a chain whose post-comment remainder is 1..6 bytes starting with `0x05` crashes the process — in the shipped configure path, from hostile file bytes, contradicting the module's own "a damaged chain dies as OUR typed error" claim. The remaining findings are dead-API hygiene.

## 2. What actually works (verified)
— verified by hand-decode against spec §4.2.2: `parse_identification` (headers.rs:90-166) — type/magic at 0..7, LE version at 7..11 (must be 0), channels at 11 (≠0), LE rate at 12..16 (≠0), three LE i32 bitrate hints at 16..28, nibble exponents at 28 with range+order validation, framing bit at 29 — all correct; `1 << bs0` / `1 << bs1` samples correct for exponents ≤13.
— verified by reading `comments.rs:111-140`: `header_len` bounds-walks every length prefix, so `comment_end ≤ data.len()` and the `&data[IDENT_LEN..comment_end]` slice at :224 cannot go out of range.
— verified by baseline run #3 (`test-workspace.log`, 1681 passed): the ogg vorbis end-to-end decode (66,150 sample frames, negotiated 2ch) and this file's 5 unit tests ran green.
— `engine_extra_data` (ident ‖ setup, comment withheld) matches the engine's documented non-laced extradata branch (decoder.rs:245).

## 3. Findings
### F-001 [S1]
Where: crates/codecs/src/audio/vorbis/headers.rs:225-237
Code:
```
        let setup = data.get(comment_end..).unwrap_or(&[]);
        if setup.is_empty() {
            return Err(corrupt(
                "vorbis.setup.truncated",
```
…
```
        if setup[0] != TYPE_SETUP || &setup[1..7] != MAGIC {
```
What is wrong: The empty case is typed, but a NON-empty remainder shorter than 7 bytes is not. When `setup.len()` ∈ 1..=6 and `setup[0] == 0x05` (TYPE_SETUP), the `||` short-circuit takes the second operand and `&setup[1..7]` indexes past the end → slice-index panic ("range end index 7 out of range for slice of length N").
Why it matters: Hostile-input crash in the shipped decode lane. Reachability: `VorbisHeaders::from_extra_data` is called by `VorbisDecoder::configure` (decoder.rs:714) on `extra_data` seeded from the Ogg harvest (containers/ogg.rs:202-208 joins whatever header packets the demuxer stashed — `is_header_packet` matches any packet starting 0x01/0x03/0x05, including a 1..6-byte truncated setup packet of a cut-off or crafted file) and from the MKV `vorbis_chain` join. A corrupted `.ogg` whose setup header is truncated to <7 bytes starting with `0x05` panics the whole `lumen` process at open. The module doc (:25-28) claims "a damaged chain dies as OUR typed error with a stable code" — this damage dies as a panic. The in-file test at :384-392 only exercises a WRONG type byte (7 full bytes), never the short-with-correct-type window.
What correct looks like: `if setup.len() < 7 || setup[0] != TYPE_SETUP || &setup[1..7] != MAGIC` (fold into the same `vorbis.setup.magic`/`truncated` typed error, e.g. `Truncated { expected: 7, found: setup.len() }`), plus a regression test with a 1..6-byte `0x05`-prefixed remainder.
Confidence: HIGH — Verification: read (index arithmetic; short-circuit evaluation of `||`; panic NOT executed — no rustc in sandbox; REQUEST-ORCH-RUN in log).

### F-002 [S4]
Where: crates/codecs/src/audio/vorbis/headers.rs:189-211
Code:
```
        let collected: Vec<&[u8]> = packets.into_iter().collect();
        if collected.len() != 3 {
```
What is wrong: `from_packets` receives the demuxer's TRUE packet boundaries, then discards them: it concatenates and re-derives the split through the comment header's length prefix inside `from_extra_data`. A comment header whose declared lengths lie (corrupt file) shifts the derived setup boundary and produces a typed `vorbis.setup.magic` error even though the actual packet boundaries were known-good and could have recovered the chain. Additionally the function's only caller is a test (`crates/codecs/tests/vorbis/mod.rs:182`) — the shipped seams all use the joined-blob form, so this public API is dead in the product.
Why it matters: Dead public API whose design bakes in a robustness regression (boundaries → parse → re-derive) relative to what its signature advertises ("the ogg seam's native shape").
What correct looks like: Split by the given boundaries directly (ident = packets[0], comment = packets[1], setup = packets[2]), or delete the function until a shipped caller needs it.
Confidence: HIGH — Verification: read (caller grep; control flow).

### F-003 [S4]
Where: crates/codecs/src/audio/vorbis/headers.rs:72-78
Code:
```
#[must_use]
pub fn is_vorbis_ident(data: &[u8]) -> bool {
    data.len() >= 7 && data[0] == TYPE_IDENT && &data[1..7] == MAGIC
}
```
What is wrong: Exported probe helper with zero shipped callers: the Ogg demuxer identifies codecs with its own match (`demux/src/ogg/mod.rs` `identify`, first-byte 0x01 for Vorbis) and media-probe has its own sniff; `rg` finds `is_vorbis_ident` only at its definition and the `mod.rs` re-export.
Why it matters: Dead public API duplicating the demuxer's identification logic — a second place a future Vorbis-II-style change must be kept in sync.
What correct looks like: Either make the ogg demuxer/probe consume it (the layering comment at :10 suggests that was the intent) or drop it from the public surface.
Confidence: HIGH — Verification: read (repo-wide caller grep).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| IDENT_LEN/MAGIC/TYPE_*/VERSION/BS_* consts | 34-50 | REAL | — | Values correct per spec §4.2.1/§4.2.2. |
| Identification (struct) | 52-70 | REAL | — | Plain data; doc honest about the >8-channel configure refusal downstream. |
| is_vorbis_ident | 72-78 | UNREACHABLE (shipped) | S4 | Correct logic; no shipped caller (F-003). |
| parse_identification | 80-166 | REAL | — | Field-by-field spec-correct (hand-verified); every access guarded by the ≥30 check; typed error per violation. |
| VorbisHeaders (struct) | 168-179 | REAL | — | Holds verbatim packets for the engine feed. |
| VorbisHeaders::from_packets | 181-211 | UNREACHABLE (shipped) | S4 | Test-only caller; boundary-discard design (F-002). |
| VorbisHeaders::from_extra_data | 213-253 | REAL | S1 | Chain split correct for well-formed input; truncated-setup panic window (F-001); ident slice `data[..IDENT_LEN]` safe post-parse; comment slice safe via header_len's bounded walk (verified in comments.rs). |
| VorbisHeaders::engine_extra_data | 255-264 | REAL | — | ident ‖ setup join; matches engine's non-laced branch; shipped caller decoder.rs:245. |
| tests: ident/comment helpers | 271-286, 356-365 | helpers | — | Correct builders (hand-checked against the parser). |
| identification_roundtrips_every_field | 288-303 | REAL test | — | Every field asserted. |
| bad_magic_is_the_typed_unsupported_refusal | 305-315 | REAL test | — | Code + sniff both asserted. |
| truncation_version_geometry_and_framing_are_all_typed | 317-354 | REAL test | — | 5 typed codes; would fail on any guard removal (but not on F-001 — see §5). |
| header_chain_split_rejects_short_and_foreign_remainders | 367-399 | REAL test | — | comment.magic, setup.truncated, setup.magic, headers.count all pinned. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| identification_roundtrips_every_field | YES | Exact per-field assertions. |
| bad_magic_is_the_typed_unsupported_refusal | YES | Error code + sniff asserted. |
| truncation_version_geometry_and_framing_are_all_typed | YES | Five distinct codes asserted. |
| header_chain_split_rejects_short_and_foreign_remainders | PARTLY | Covers empty remainder and WRONG-type 7-byte remainder; the 1..6-byte CORRECT-type remainder (F-001's panic window) is untested — the suite would NOT fail for the missing length guard. |
| Untested branch (no test at all): `vorbis.ident.channels` (zero channels, :117-125). | — | Mutation: deleting that arm would not fail any test. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a damaged chain dies as OUR typed error with a stable code" | headers.rs:25-28 | FALSE (for the truncated-setup case) | F-001 panic. |
| "The chain is self-delimiting: … the setup header is the remainder" | headers.rs:11-14 | REAL | Verified: ident exactly 30, comment length-walked, setup = remainder. |
| "extra header packets beyond three are a typed rejection" | headers.rs:16-18 | REAL | `vorbis.headers.count` (:194-204), tested (:393-398). |
| "symphonia maps up to 8 positions — above that configure is a typed refusal" | headers.rs:55-56 | REAL-UNPROVEN | The refusal lives in decoder.rs (not this file); not re-verified here — decoder.rs auditor to confirm. |
| Vorbis decode implemented (matrix `codec-audio-vorbis` context) | feature-matrix vorbis row | REAL | Shipped ogg+mkv lanes; e2e test passed in baseline run #3 (66,150 samples vs pinned reference). |

## 7. Dead / duplicated / unreachable code
- `is_vorbis_ident` (shipped-dead, F-003); `from_packets` (test-only, F-002).
- Identification sniffing duplicated between this file and the ogg demuxer's `identify` (first-byte match) — the layering note admits codecs cannot be a demux dep, but the shared predicate could live in media-types.

## 8. Dependencies & flags
- `codec-vorbis` default ON (crates/codecs/Cargo.toml) — compiled in every shipped build; the parse side itself is dependency-free (media-types taxonomy only), the engine (symphonia-codec-vorbis) is fed only after this file's validation.
- No unsafe, no allows.

## 9. Open questions for the next agent
- vorbis/decoder.rs auditor: confirm the >8-channel configure refusal (claimed at :55-56) and whether `decode_block` re-validates the ident packet the engine receives.
- ogg demuxer auditor: does a truncated final header PAGE deliver a short setup packet to `header_packets` (making F-001 reachable with a merely CUT-OFF file, not a hand-crafted one)? My read of the page reassembly says yes, but the page-continuation logic is yours to confirm.
