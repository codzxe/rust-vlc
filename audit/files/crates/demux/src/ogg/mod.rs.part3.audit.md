# AUDIT: crates/demux/src/ogg/mod.rs#part3 (lines 985-1476)
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Chunk boundaries | part3 = lines 985-1476 of 1477 (part1 = 1-492 → W04-A05? no: part1 A04? — assignments.tsv: ogg/mod.rs#part2 → W04-A05, #part3 → W04-A03; part1 belongs to another wave-4 agent). The part2/part3 boundary cuts through `complete_packet` (937-1068); I read the whole function for coherence, findings scoped to my range. Part3 = complete_packet's data half, mint_track, probe_tail, open, scan_landing, reference_stream, helpers, the full `Demuxer` impl, and the inline `clock_unit` test module. |
| Lines / functions | 492 / 15 production + 5 inline tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`apps/cli/src/containers/mod.rs:68` `"ogg" => ogg::open(stream, opts)` → `containers/ogg.rs:82` `demuxer.start()`; baseline differential: `corpus_oggs_match_the_ffmpeg_truth ... ok`, `every_ogg_fixture_matches_the_ffmpeg_truth ... ok`) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | full read of 930-1477 + part1 context (1-330, 581-745); `sed -n` on every quoted range; `rg -n "keyframe"` (no engine consumer of `Packet::keyframe` outside an ivf test); `rg -n "packet.track" crates/playback-engine/src/pipeline.rs` (silent drop of unwired tracks, lines 2194-2199); `rg -n "chain" docs/feature-matrix.toml` (container-ogg row 155-165); `rg -n "ogg" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (19 ogg tests + corpus + fixtures all ok, lines 1802-1828, 2003-2007, 2166-2169) |

## 1. One-paragraph verdict
Real and differentially verified Ogg demuxer tail: page-completion stamping (last-packet-owns-
granule), track minting with tail-probe durations, the seek core, and the trait impl are all
implemented and pinned by 19 passing unit tests plus ffmpeg-differential corpus and fixture
suites in the committed baseline. Two major defects: (1) the seek scan byte-steps with two
heap-allocating reads per byte when a page header fails to parse, and the
`MAX_SEEK_SCAN_PAGES` honesty bound counts only *parsed* pages — a corrupt region turns a
seek into an unbounded file-length scan, contradicting the module's own bounded-recovery
policy that the walk path (`recover_sync`, ≤ 4 MiB) does enforce; (2) chained links mint
fresh track ids mid-stream, but nothing carries them to the host — the CLI calls `start()`
once, `DemuxEvent` has no new-tracks variant, and the playback engine silently drops every
packet whose track has no wired limb, so all content past link 1 vanishes without an error
while the feature-matrix row claims "chained streams … implemented". Plus: `reanchor`
ignores the sticky error (succeeds, then `next_event` still returns the old fatal error —
inconsistent with the AIFF/WAV twins), and `open`'s page budget exhaustion silently
returns a partial track list as final.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt`: lines 1802-1828 — all 19 `ogg::` tests ok (incl. `chained_links_boundary_discontinuity_and_fresh_tracks`, `seek_and_reanchor_land_on_the_last_page_before_the_target`, `only_the_last_packet_on_a_page_owns_the_granule`); lines 2166-2173 — the **committed-fixture** suite green: `every_ogg_fixture_matches_the_ffmpeg_truth`, `ogg_chain_two_links_is_two_links_with_a_fresh_track`, `ogg_derivatives_stay_inside_the_typed_vocabulary`, `ogg_fixtures_and_derivatives_survive_the_fuzz_harness` (fixtures exist in-repo at `tests/fixtures/generated/valid/ogg_*.ogg`; no env gating — real differential evidence).
— CORRECTION (evidence honesty): my earlier citation of `corpus_oggs_match_the_ffmpeg_truth ... ok` (log lines 2003-2007) is **vacuous** — `corpus_ogg.rs` skips-as-passes when `/tmp/my-project/corpus-v4/media` is absent (`corpus_dir()` → `None` → eprintln + return, lines 104-124), and the baseline block finished in 0.00s with 0 ignored — the corpus was not present in the sandbox. The same applies to `corpus_adts_*`, `corpus_wav`, `corpus_mp4` baseline lines. The committed-fixture suites (ogg_fixtures, wav_aiff_fixtures, adts fixtures) are the real evidence; env-gated corpus lines prove nothing in this baseline.
— verified by read: `probe_tail`'s CRC re-validation is byte-exact (bytes 0..22 + 4 zeroed CRC bytes + bytes 26.. — the Ogg page layout), and `scan_landing`'s landing REPORT uses the previous reference page's granule (the documented fix for the measured 1.0 s over-report defect).
— verified by read: `Clock::Theora` shift is bounded `< 63` at identify (mod.rs:277), so `1u64 << shift` in `complete_packet` (line 1039) cannot shift-overflow.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ogg/mod.rs:1217-1230
Code:
```rust
        while at + page::HEADER_BYTES as u64 <= len {
            let head27 = self.reader.read_prefix_at(page::HEADER_BYTES, at)?;
            if head27.len() < page::HEADER_BYTES {
                break;
            }
            let head = self.read_full_head(at, head27[26])?;
            let Ok(hdr) = page::parse_header(&head, at, len) else {
                at += 1;
                continue;
            };
            pages += 1;
            if pages > MAX_SEEK_SCAN_PAGES {
                return Err(error::seek_out_of_range());
            }
```
What is wrong: On a header that fails to parse (no `OggS` capture, bad version, bad
lengths), the scan advances **one byte** and retries — and each iteration performs
`read_prefix_at(27)` (a fresh 27-byte `Vec` allocation + seek + read) plus
`read_full_head` (another allocation). `pages` — the only bound, `MAX_SEEK_SCAN_PAGES` =
200,000 — is incremented **only for successfully parsed pages**. A corrupt or garbage
region therefore degrades the seek into a byte-at-a-time scan of the entire remaining
file with ~2 allocations per byte; a 100 MB damaged file means ~10⁸ iterations before
the scan even reaches the landing region (or `len`). The walk path handles the identical
situation with `recover_sync` (≤ `MAX_RECOVERY_SCAN_BYTES` = 4 MiB, damage-noted,
bounded) — the module doc's "bounded sync-recovery scan" promise — but the seek path
silently lacks it.
Why it matters: User-triggerable stall/DoS (seek into or past corruption in any damaged
ogg file — the exact scenario the damage policy exists for); the declared
`SlowLinear`/200k-page honesty bound is evadable by input the attacker controls.
What correct looks like: On parse failure, hunt the next capture pattern in a chunked
window (`find_capture` over a bounded block, like `recover_sync`), and bound the loop by
*iterations* (or bytes scanned), not by parsed-page count.
Confidence: HIGH — Verification: read (loop structure; `read_prefix_at` allocation at riff.rs:307-313; contrast `read_page`'s `recover_sync` at mod.rs:598-600 and its 4 MiB budget at 717)

### F-002 [S2]
Where: crates/demux/src/ogg/mod.rs:1073-1082 (mint_track) with the drop at crates/playback-engine/src/pipeline.rs:2194-2199
Code:
```rust
    fn mint_track(&mut self, serial: u32) -> Result<(), EngineError> {
        let Some(stream) = self.streams.get(&serial) else {
            return Ok(());
        };
        if stream.codec == Some(OggCodec::Skeleton) {
            return Ok(());
        }
        if self.tracks.iter().any(|t| t.id == stream.track) {
            return Ok(());
        }
```
What is wrong: `mint_track` runs when a chained link's streams are identified — i.e.
**during `next_event`**, long after the host took the track list. But (a)
`apps/cli/src/containers/ogg.rs:82` calls `demuxer.start()` exactly once; (b)
`DemuxEvent` (plugin-api demux.rs:149-167) has no new-tracks variant; (c) the playback
engine's `push_packet` drops every packet whose track id has no wired limb:
`let Some(limb) = self.limbs.iter().find(|limb| limb.track.id == packet.track) else { …
dropped by policy … }`. Net effect: for a chained Ogg file (common — concatenated
album/opus rips), link 1 plays, then **every packet of link 2+ is silently discarded**
— no error, no event — while the module doc advertises "mints fresh track ids for the
new link" and the feature-matrix row `container-ogg`
(docs/feature-matrix.toml:155-165) claims `status = implemented` on windows/linux/macos
for "Demux Ogg: … chained streams …". The row's evidence string ("crates/demux/src/ogg/**")
cannot see the seam it is supposed to prove.
Why it matters: Whole links of real user files disappear silently; the matrix's
"implemented" cell for chained streams is not deliverable end-to-end (borderline S0 by
the "matrix evidence strings that prove nothing" rule — kept S2 because the demuxer
itself implements the behavior and single-link Ogg is genuinely delivered).
What correct looks like: A `DemuxEvent::TracksChanged` (or per-link re-query contract)
that the host consumes to wire new limbs before the packets flow; until then the matrix
row should say partial with the seam named.
Confidence: HIGH — Verification: read (call chain verified: mint_track ←
complete_packet ← step ← next_event; containers/ogg.rs:82; plugin-api demux.rs:149-167;
pipeline.rs:2194-2199)

### F-003 [S3]
Where: crates/demux/src/ogg/mod.rs:1361-1380
Code:
```rust
    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Compute (the scan + validation)…
        let landing = self.scan_landing(target)?;
        // …then commit. D4: every failure above left us untouched.
        self.walk = landing.1;
        self.pending.clear();
```
What is wrong: `reanchor` (and the `seek` query) never consult `self.sticky`. After a
mid-stream fatal error (e.g. the typed sticky truncation of a page running past the
end), `reanchor(target)` **succeeds** — it scans, commits `walk`, clears pending,
pushes a `Discontinuity` — and then the very next `next_event` (line 1315-1317) returns
the old sticky error forever. The engine is told the seek landed while no data can ever
arrive. The AIFF/WAV twins take the opposite path: their `locate` refuses with the
sticky error up front (aiff/mod.rs:570-572). Neither behavior is good — aiff cannot
recover, ogg pretends to recover — and they are mutually inconsistent.
Why it matters: A seek-back after an end-truncation — the standard "let me rewatch what
played" move — either refuses (aiff/wav) or lies (ogg); the demuxer family has no
coherent recovery contract.
What correct looks like: `reanchor` clears `sticky` when committing a landing strictly
before the damage position (the demuxer has `walk`/`length` to decide), or refuses
up-front like the twins. Pick one contract for the family and pin it in plugin-api.
Confidence: HIGH — Verification: read (sticky check at 1315-1317 precedes every
next_event; no sticky read anywhere in 1344-1380)

### F-004 [S3]
Where: crates/demux/src/ogg/mod.rs:1162-1174
Code:
```rust
        let mut budget = MAX_OPEN_PAGES;
        while self.walk < len && budget > 0 {
            self.step()?;
            budget -= 1;
            let ready = !self.streams.is_empty()
                && self
                    .streams
                    .values()
                    .all(|s| (s.codec.is_some() && s.packets_out > 0) || s.ended);
            if ready {
                break;
            }
        }
```
What is wrong: When the 10,000-page budget runs out before every link-0 stream is
identified-and-started, the loop simply exits and `open` returns
`Ok(self.tracks.clone())` with `started = true` — a **partial** track list reported as
final, with no damage note and no typed signal (contrast: budget exhaustion in the walk
path mints `error::recovery_budget_exhausted`). Streams identified later do get tracks
pushed (mint_track during next_event), but the host already consumed the list (same
seam as F-002).
Why it matters: A crafted file (or an extreme MUX) silently loses tracks at open with
zero observability — the recovery/damage channel that every other bounded path uses is
bypassed.
What correct looks like: On budget exhaustion either return the typed budget error, or
surface a `Recovered { reason: "open page budget exhausted" }` and document the partial
enumeration.
Confidence: HIGH — Verification: read (fall-through to `Ok(self.tracks.clone())` at 1175-1181; no damage/error path on `budget == 0`)

### F-005 [S4]
Where: crates/demux/src/ogg/mod.rs:1192-1198
Code:
```rust
        let target_nanos = target
            .start
            .timestamp()
            .ok()
            .and_then(|t| t.as_nanos().ok())
            .map(|n| n.0)
            .unwrap_or(0);
        let target_granule = clock.granule_of_nanos(target_nanos);
```
What is wrong: A target span whose start stamp is `Missing` (or unconvertible) silently
maps to nanos 0 → a seek to the beginning of the file. The AIFF twin refuses the same
input with the typed `seek_out_of_range` (aiff/mod.rs:580-583). One engine, two
behaviors for the same contract violation.
Why it matters: A caller bug (missing stamp) is masked as "seek to 0" — plays from the
start instead of failing loudly; cross-demuxer behavioral drift makes the engine's seek
flow unmappable.
What correct looks like: `Stamp::Missing`/unconvertible → the typed refusal, matching
the twins.
Confidence: HIGH — Verification: read

### F-006 [S4]
Where: crates/demux/src/ogg/mod.rs:1084-1088
Code:
```rust
        let kind = if codec == OggCodec::Theora {
            MediaKind::Video
        } else {
            MediaKind::Audio
        };
```
What is wrong: Every `OggCodec::Unknown` stream (magic not in the vocabulary — e.g. a
future/unknown *video* codec like Daala/UVS wrapped in Ogg) is minted as
`MediaKind::Audio`. The codec descriptor honestly says "ogg-opaque", but the track
*kind* is a fabricated guess that drives the engine's audio/video wiring and the
CLI's `--no-video` selection.
Why it matters: An unknown video stream is wired as audio; decode selection and track
listing lie about the content's kind.
What correct looks like: An unknown-kind track (or kind inferred from nothing = a
documented default with a damage note), not a silent Audio guess.
Confidence: MEDIUM — Verification: read (no kind evidence exists for Unknown streams; Theora is the only video mapping today)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `complete_packet` (data half 987-1068; whole fn 937-1068 spans part2/part3) | 937-1068 | REAL | — | last-packet-owns-granule stamping; Theora keyframe via KFGSHIFT mask (shift < 63 guaranteed); insert-only-map arms are documented unreachable residue, skipped not panicked; `discontinuity` flag consumed on first data packet only |
| `mint_track` | 1073-1121 | REAL | F-002/F-006 | once-per-track guard; tail-granule duration; late mints invisible to host (F-002); Unknown→Audio guess (F-006) |
| `probe_tail` | 1125-1154 | REAL | — | 128 KiB window; CRC re-validated with correct zeroed-CRC reconstruction; last valid granule per serial wins; bounded (window-sized) resync |
| `open` | 1159-1182 | REAL | F-004 | budget exhaustion silently partial (F-004); no-streams typed error; ready-check requires every stream past its first data packet |
| `scan_landing` | 1188-1254 | REAL | F-001/F-005 | header-only scan from byte 0; landing = previous reference granule (the documented over-report fix); byte-stepping on parse failure unbounded (F-001); Missing target → 0 (F-005) |
| `reference_stream` | 1257-1263 | REAL | — | first clock-bearing serial in arrival order; None → typed seek refusal |
| `PageHeaderAndBody` (struct) | 1267-1270 | REAL | — | data-only |
| `stash_header` | 1273-1282 | REAL | — | 256 KiB budget; over-budget header dropped with a 0-byte damage note (the stream keeps demuxing) |
| `find_capture` | 1285-1287 | REAL | — | `windows(4).position` |
| `nanos_to_stamp` | 1291-1298 | REAL | — | u128 math, i64::MAX clamp, no panic |
| `Demuxer::start` | 1301-1312 | REAL | — | caches on non-empty tracks (a later call returns the grown list — the only way late tracks could surface, nothing calls it); sticky on failure |
| `Demuxer::next_event` | 1314-1342 | REAL | — | sticky first; damage flushed before terminal EOS; idempotent EOS |
| `Demuxer::seek` | 1344-1351 | REAL | — | position save/restore honors the non-destructive contract; restore error would mask the scan result (`?` on restore) — cosmetic |
| `Demuxer::seekability` | 1353-1359 | REAL | — | `SlowLinear` — honest (no index), but see F-001 for the bound being evadable |
| `Demuxer::reanchor` | 1361-1380 | REAL | F-003 | commits walk, drops cross-page partials, Discontinuity event + packet mark; no sticky consult (F-003) |
| `clock_unit::clock_conversions_are_exact` | 1387-1404 | REAL | — | exact nanos both directions for all three clock shapes |
| `clock_unit::identify_parses_the_magic_set` | 1406-1434 | REAL | — | Vorbis/Opus/FLAC/Skeleton/Unknown magic + clocks |
| `clock_unit::theora_header_offsets_pin_the_corpus_layout` | 1436-1458 | REAL | — | pinned against the real ffmpeg/libtheora corpus header (KFGSHIFT 6, 1/2 fps) |
| `clock_unit::flac_streaminfo_rate_bits` | 1460-1468 | REAL | — | 20-bit rate at offset 14 |
| `clock_unit::find_capture_ignores_short_tails` | 1470-1475 | REAL | — | boundary cases |

## 5. Test honesty
Inline `clock_unit` tests (my range) are exact-value and mutation-sensitive. The
demuxer-level suite (ogg/tests.rs, another agent's file) covers chains, MUX, recovery,
granule holes — all green in baseline (1802-1828). Gaps that matter to part3:
- No test seeks in a file with a corrupt region — F-001's unbounded byte-scan is
  untested (`seek_and_reanchor_land_on_the_last_page_before_the_target` uses a clean
  file).
- No test exercises reanchor-after-sticky-error (F-003) or budget-exhausted open
  (F-004).
- The chained-links test validates the demuxer's own accessors, not the host seam —
  F-002 is invisible to it by construction.
Mutation reasoning (no edits): flipping `landing = Some((prev_granule, at))` to use
`hdr.granule` would fail the seek test (baseline:1826) — the landing-report contract is
pinned; deleting the CRC zeroing in probe_tail would fail
`single_stream_start_packets_and_duration` (duration comes from tail granules).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "chained link boundary: … mints fresh track ids for the new link" | module doc lines 12-16 | REAL at the demuxer / NOT DELIVERED end-to-end | demuxer test baseline:1815 passes; host drops link 2+ packets (pipeline.rs:2194-2199, F-002) |
| "a bounded sync-recovery scan (≤ MAX_RECOVERY_SCAN_BYTES) hunts the next CRC-valid capture" | module doc lines 42-44 | REAL for the walk / FALSE for the seek path | walk: recover_sync + 4 MiB budget (mod.rs:717); seek: unbounded byte-step (F-001) |
| "stamps only the last-completing packet of each granule-bearing page; every other packet carries Stamp::Missing" | module doc lines 20-27 | REAL | `owns_granule` gate at 1027-1034; test baseline:1824 |
| "Ogg has no index: landing needs a page-header scan" (SlowLinear) | seekability doc 1357-1358 | REAL | scan_landing structure; but the 200k-page bound is evadable (F-001) |
| container-ogg matrix row: "chained streams" status implemented (3 platforms) | docs/feature-matrix.toml:155-165 | SHALLOW | demuxer-only; engine seam drops the content (F-002) |

## 7. Dead / duplicated / unreachable code
- `scan_landing`'s `if head27.len() < page::HEADER_BYTES { break; }` (1219-1221): dead —
  the loop condition `at + 27 <= len` guarantees a full 27-byte read. Harmless defense.
- `complete_packet`'s three "unreachable state residue" arms (950-959, 1002-1007,
  1046-1048): documented dead-state tolerance, not dead code — kept deliberately
  panic-free.
- `PageHeaderAndBody` — fine.

## 8. Dependencies & flags
Reuses `avi::riff::{PayloadSlots, SourceReader}` and `avi::header::DamageLog`
(crate-internal, consistent with the aiff/wav pattern). No features, no unsafe, no
unwrap/expect in production paths. Constants are PROPOSAL-tagged with rationales
(256 KiB packet/header budgets, 4 MiB recovery, 128 KiB tail, 200k seek pages, 10k open
pages) — the seek-page bound is the one not actually enforced (F-001).

## 9. Open questions for the next agent
- W04-A05 (ogg part2): `identify`'s Vorbis rate is read as LE24 (bytes 12-14) with
  `len >= 15` — the spec's u32 rate byte 15 is dropped; crafted rates ≥ 2²⁴ truncate
  silently (S4-grade, in your range — please carry it).
- Does any engine path re-call `Demuxer::start` after events begin (the only channel
  late-minted tracks could use)? Owner: playback-engine wave.
- Should `SeekSupport::SlowLinear` carry a byte-bound so hosts can refuse pathological
  seeks up front? Owner: plugin-api wave.
