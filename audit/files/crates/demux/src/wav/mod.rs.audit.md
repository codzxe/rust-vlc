# AUDIT: crates/demux/src/wav/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 1109 / 28 (incl. trait impl) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli/src/host.rs:904 `open_source` → demux_host.rs:378 `open_source_with` → probe (`media_probe::probe_stream`) → containers/mod.rs:73 `"wav" => wav::open` → containers/wav.rs:50 `WavDemuxer::new` → `start`/`next_event` via `SelectDemuxer`. Desktop (apps/desktop) embeds `lumen-cli` as a lib (apps/desktop/Cargo.toml `lumen-cli = { path = "../cli" }`), so the same chain serves both shipped entry points. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 5 |
| Commands run | `target/debug/deps/demux-036dd705535b2fb3 wav::` → 43 passed, 0 failed (exit 0); `~/.cargo/bin/cargo test -p demux --doc wav` → 1 passed (exit 0) |

## 1. One-paragraph verdict
This is a genuinely implemented, bounds-disciplined WAV/RF64/BW64 demuxer. The `fmt`/extensible
parse (in the sibling `format.rs`, verified here because mod.rs's correctness rests on it) enforces
channels 1..=64, rate ≥ 1, the extensible 40-byte floor, GUID resolution, and
`nBlockAlign == channels × width`; every walk position saturates; every declared extent is clamped
to parent ∩ stream; the 0xFFFFFFFF `data` sentinel resolves to "rest of stream" in plain RIFF
(damage-noted) and to `ds64.data_size` in RF64/BW64; an RF64 head without `ds64` is refused at
`start`. Executed: 43 lib tests + the rustdoc example pass. Remaining defects are corner-path
accounting/doc lies, no S0–S2 found. Cross-cutting repo fact established by this file's
reachability work: **`demux::asf` has no probe signature and no registry arm — see finding F-101
(adjacent) and the ASF audits.**

## 2. What actually works (verified)
- `target/debug/deps/demux-036dd705535b2fb3 wav::` → `test result: ok. 43 passed; 0 failed` —
  includes the sentinel tests (`plain_riff_data_sentinel_means_to_stream_end`,
  `rf64_sentinel_without_ds64_is_a_declared_lie`), truncation, ADPCM, FLAC-in-WAV, seek.
- `~/.cargo/bin/cargo test -p demux --doc wav` → `test crates/demux/src/wav/mod.rs - wav::WavDemuxer (line 216) ... ok` (1 passed, exit 0).
- Arithmetic safety re-derived by reading: `MAX_PACKET_BYTES/unit ≥ 1` for every accepted
  format (max unit = ADPCM `block_align` ≤ 65535 < 65536, PCM ≤ 64×8 = 512, FLAC = 1), so the
  packetizer can never emit a zero-length packet / no-progress loop (mod.rs:867, format.rs:216-235).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/wav/mod.rs:882-895
Code:
```rust
        let delivered = payload.len() as u64;
        if delivered < want_bytes as u64 {
            let whole = (delivered / unit) * unit;
            self.damage
                .note(want_bytes as u64 - whole, "payload-read-short");
            self.surface_damage();
            if whole > 0 {
                self.walk += whole;
                self.frames_emitted += spu.map_or(0, |s| (whole / unit) * u64::from(s));
            } else {
                self.walk = region.effective_end;
            }
            return Ok(None);
```
What is wrong: on a short read the `whole` whole units that WERE delivered are dropped — no packet
emitted — yet the damage note counts only `want_bytes - whole`. The delivered-and-discarded `whole`
bytes are neither emitted nor damage-counted; the comment above it (lines 877-881) justifies
dropping "a partial unit", not whole units. Why it matters: silent data loss beyond the documented
"counted, never silent" contract (module doc lines 33-43), and damage accounting under-counts by
`whole`. Trigger requires a source that shrinks below the already-clamped extent mid-walk
(pathological but the code path exists precisely for it). What correct looks like: emit a packet of
`whole` bytes when `whole > 0`, or at minimum note `whole` as lost.
Confidence: HIGH — Verification: read (test `payload-read-short` does not exist in tests.rs; the
43-test run does not cover a shrinking source).

F-002 [S3]
Where: crates/demux/src/wav/mod.rs:445-457 (+ 496-509)
Code:
```rust
            if size == u64::from(SIZE_SENTINEL) && id == FourCC::tag(b"data") {
                if let Some(ds64) = self.ds64 {
                    size = ds64.data_size;
                } else {
                    // The sentinel resolved to no ds64: in a plain
                    // RIFF this is the streaming-writer "to the end
                    // of the stream" — never a 4 GiB interpretation;
                    // in an RF64/BW64 the mandatory ds64 never
                    // arrived (the post-walk check refuses that one
                    // structurally).
                    self.damage.note(0, "data-sentinel-without-ds64");
                    size = self.reader.length().saturating_sub(payload_start);
```
What is wrong: if a `data` chunk with the 0xFFFFFFFF sentinel appears BEFORE the `ds64` chunk in
an RF64/BW64 (spec requires ds64 first; malformed but parseable), `self.ds64` is still `None` at
that point, so `size` resolves to "rest of stream" and `DataRegion.declared_end` is never revisited
when `ds64` arrives later (lines 475-495 only recompute the walk extent, not the data region). The
demuxed audio then includes the `ds64` chunk's bytes. Damage-noted, malformed-input-only, no memory
issue. What correct looks like: on ds64 arrival, re-resolve a sentinel-resolved data region.
Confidence: MEDIUM — Verification: read (no test covers data-before-ds64 ordering).

F-003 [S3] (adjacent file, established by this audit)
Where: crates/demux/src/wav/format.rs:430-441
Code:
```rust
        let samples_per_block = match (declared, derived) {
            (Some(declared), Some(derived)) => {
                // A declaration that disagrees with the block
                // geometry is a lie about arithmetic, not a variant:
                // trust the geometry, count nothing silently — the
                // typed refusal (the ADPCM decoder would decode the
                // declared count against the same bytes).
                if declared == 0 || declared > derived {
                    return Err(error::bad_format_params(offset));
                }
                declared
```
What is wrong: the comment says a declared/derived disagreement is "the typed refusal" and "trust
the geometry", but the code ACCEPTS every `0 < declared < derived` and uses the declared value.
Doc-code contradiction; consequence is PTS-ladder drift vs the real block geometry for such files.
What correct looks like: either refuse on `declared != derived` (per the comment) or rewrite the
comment. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/wav/mod.rs:479-490
Code:
```rust
                    // The container-level size truth: recompute the
                    // walk extent from the ds64 riff size.
                    if sentinel {
                        let real_declared = 8u64.saturating_add(ds64.riff_size);
                        let real_end = real_declared.min(self.reader.length());
```
What is wrong: "recompute the walk extent" is a widen-only operation (`if real_end > extent_end`,
line 490); with a sentinel head the extent already equals the full stream length (line 425), so a
`ds64.riff_size` smaller than the stream never tightens the walk. Trailing junk past the true RIFF
end is walked (positionally skipped). Harmless; the comment overstates. Confidence: HIGH —
Verification: read.

F-005 [S4]
Where: crates/demux/src/wav/fuzz.rs:12-18 and fuzz/fuzz_targets/fuzz_demux_wav.rs:7-10
Code:
```rust
//! # Wiring status (honest)
//!
//! The cargo-fuzz target file (`fuzz/fuzz_targets/fuzz_demux_wav.rs`)
//! is a Lead-owned dependency decision (`libfuzzer-sys`), exactly as
//! the AVI card recorded; until it lands the harness logic is proven
//! by the deterministic in-crate sweeps here and by the integration
//! corpus run over the mutated fixture family.
```
What is wrong: both files claim the cargo-fuzz target has not landed. It HAS:
fuzz/fuzz_targets/fuzz_demux_wav.rs exists and calls `demux::wav::fuzz::fuzz_demux_wav(data);`
(line 19), and `fuzz_demux_wav` appears in fuzz/Cargo.toml. The "honest" wiring-status docs are
false/stale. Confidence: HIGH — Verification: read + `ls fuzz/fuzz_targets/` + rg in fuzz/.

F-006 [S4]
Where: crates/demux/src/wav/mod.rs:466-474
Code:
```rust
                if size < 4 {
                    self.damage.note(size, "fact-too-small");
                } else if let Some(bytes) = self.reader.read_exact_or_end_at(4, payload_start)? {
                    let raw = [bytes[0], bytes[1], bytes[2], bytes[3]];
                    let declared = u64::from(u32le(&raw, 0));
                    if declared > 0 {
                        self.fact_frames = Some(declared);
                    }
```
What is wrong: a `fact` chunk whose 4 payload bytes physically cannot be read (stream ends inside
it) is silently ignored — no damage note, no truncation. Every other skip path in the walk notes
damage. Confidence: MEDIUM — Verification: read.

F-007 [S4]
Where: crates/demux/src/wav/mod.rs:803-820 (`parse_info_list`)
What is wrong: a zero-length INFO value (`value_size == 0`) matches neither the accept branch
(`value_size > 0`) nor the over-ceiling branch — silently skipped, no damage note. Same
silent-skip class as F-006. Confidence: HIGH — Verification: read.

F-008 [S4]
Where: crates/demux/src/wav/mod.rs:297 + 722 (with format.rs:33-34)
What is wrong: `fmt_chunk_bytes` doc claims "The verbatim `fmt ` chunk payload"; the stored bytes
are clamped to `MAX_FMT_BYTES` (4096) and to the RIFF extent (mod.rs:711-714), so for a fmt chunk
> 4 KiB the "verbatim" claim is false (a prefix rides `extra_data`). No accepted format needs > 74
bytes, so blast radius is pathological files only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 139-147 | REAL | — | 12-byte head check, executed in tests |
| StickyShape::of/mint | 190-202 | REAL | — | shape replay; sticky error contract |
| WavDemuxer::new | 278-299 | REAL | — | rejects unknown-length/unseekable/empty via SourceReader |
| format | 301-306 | REAL | — | accessor |
| fmt_chunk_bytes | 308-314 | REAL | S4 | "verbatim" overclaim (F-008) |
| info / is_rf64_like / fact_sample_length / damage_total / pool_bytes | 316-349 | REAL | — | observability accessors |
| frames | 351-362 | REAL | — | None for FLAC (honest) |
| fail | 364-368 | REAL | — | sticky-set + return |
| surface_damage / emit | 370-386 | REAL | — | damage-record-first order preserved |
| parse_container | 389-694 | REAL | S3 | sentinel resolution (F-002), walk/clamp logic otherwise correct |
| parse_fmt_chunk | 698-725 | REAL | — | bounded by MAX_FMT_BYTES/extent; first-wins dup policy |
| parse_ds64 | 731-766 | REAL | S4 | `sample_count` parsed but never read anywhere (dead field) |
| parse_info_list | 773-827 | REAL | S4 | zero-size value silent skip (F-007); anti-spin guard present |
| step | 831-937 | REAL | S3 | short-read drops delivered whole units (F-001); ladder/duration math verified |
| locate | 944-1002 | REAL | — | PCM exact-frame, ADPCM block-floor, out-of-range typed refusal |
| landing_span | 1006-1019 | REAL | — | — |
| Demuxer::start | 1023-1042 | REAL | — | idempotent, sticky on failure |
| Demuxer::next_event | 1044-1073 | REAL | — | damage-before-EOS ordering correct |
| Demuxer::seek | 1075-1079 | REAL | — | pure query (compute-then-commit honored) |
| Demuxer::seekability | 1081-1093 | REAL | — | redundant second fmt check, correct result |
| Demuxer::reanchor | 1095-1108 | REAL | — | commits walk/frames, purges pending, revives eos |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| plain_riff_data_sentinel_means_to_stream_end | YES | asserts exact payload bytes AND exact Recovered event sequence (riff-size-sentinel, data-sentinel-without-ds64, EndOfStream) — reading 0xFFFFFFFF as 4 GiB or refusing would change the outcome |
| rf64_sentinel_without_ds64_is_a_declared_lie | YES | asserts the exact refusal code `demux.wav.bad-structure` |
| declared_size_past_actual_is_typed_truncation_after_packets | YES | asserts packets-then-typed-truncated order |
| trailing_partial_frame_is_damaged_and_dropped | YES | asserts damage reason + no packet |
| drains_plain_pcm16_whole_frames / pts_ladder_counts_sample_frames | YES | byte-exact payload + tick math |
| adpcm_seek_lands_on_block_boundaries | YES | block-floor assertions |
| oversized_info_value_is_damage_not_fatal | YES | damage_total > 0 + empty info |
| All 26 tests in wav/tests.rs | YES (spot-checked 8) | byte-built inputs, exact event/code assertions; run executed (43 passed incl. format/error module tests) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`data` size sentinel (`0xFFFFFFFF`) outside RF64/BW64 means 'to the end of the stream' … never read as 4 GiB" | mod.rs:37-41 | REAL | code at 445-457 (Q1) + test at tests.rs:807 executed |
| "An RF64/BW64 head without its mandatory `ds64` is the one structural lie that refuses at `start`" | mod.rs:41-43 | REAL | code at 561-563 + test at tests.rs:776 executed |
| "a `data` declaration that runs past the file … fails *typed* (`error::truncated`) after the packets — never a silent EOS" | mod.rs:35-38 | REAL | code at 844-849 (Q3) + test at tests.rs:489 executed |
| "the verbatim `fmt ` payload rides the engine's `DecoderConfig::extra_data`" | mod.rs:46-48 | FALSE-in-corner | clamped to 4096/extent (F-008); PAPER corner |
| "fuzz target 'until it lands'" | wav/fuzz.rs:14-18 | FALSE | target exists and is wired (F-005) |
| "the block-aligned ADPCM packetization … declaration disagreement = typed refusal" | format.rs:432-438 | FALSE-in-corner | code accepts declared < derived (F-003) |

## 7. Dead / duplicated / unreachable code
- `Ds64.sample_count` (mod.rs:764) — parsed, stored, never read. Dead field.
- `wav/error.rs::payload_too_large` — self-declared "vocabulary without a minting site today"
  (error.rs:147-152); dead constructor kept intentionally (documented).
- Nothing else dead; `fuzz.rs` IS wired (fuzz/fuzz_targets/fuzz_demux_wav.rs:19) despite its stale
  comment.

## 8. Dependencies & flags
- Depends on `crate::avi::riff` (SourceReader, PayloadSlots, FourCC, pad_for, u32le) and
  `crate::avi::header::DamageLog` — cross-format reuse, no external crates (C5 std-only honored).
- No feature gates anywhere in the wav module (all paths compile in the release lane).
- No `unsafe`, no `unwrap`/`expect`/`panic!` in non-test code (grep-verified during read; house
  rule at lib.rs:37-41 upheld in this file).

## 9. Open questions for the next agent
- The engine-side consumer of `fmt_chunk_bytes` (`DecoderConfig::extra_data` →
  `codecs::audio::pcm::decoder::parse_waveformatex`): does the PCM decoder accept exactly the same
  blob set (the "one accepted-set" claim, mod.rs:44-49)? Audit on the codecs side.
- `emit()`'s ordering contract when packets are produced while damage is still queued mid-stream
  (only reachable via F-001's path) — confirm no packet can be reordered before its damage record.
- Environment note for all future agents: the Bash tool's displayed stdout deletes the literal
  two-char sequence `[m` (e.g. real `#[must_use]` displays as `#ust_use]`); the Read tool shows
  true bytes. Verify `[m`-containing quotes via Read/od, not sed.
