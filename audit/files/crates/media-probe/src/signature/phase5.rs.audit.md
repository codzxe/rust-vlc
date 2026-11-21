# AUDIT: crates/media-probe/src/signature/phase5.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 208 / 12 items (module docs, 1 const, 1 helper fn, 8 signature structs each with `probe` + `extensions`) |
| File verdict | REAL (with an S1 panic bug on the exactly-188-byte window) |
| Reachable from shipped entry point? | YES — this is the shipped probe lane itself. `lumen open <file>` → `apps/cli/src/demux_host.rs:444-446` `probe_stream(&registry, …)` → `ProbeRegistry::with_builtins()` registers `TsSignature` among the 8 (signature/mod.rs:139) → `registry.probe(&window, …)` calls every signature's `probe` on the window (`fill_window` reads `min(max_lookahead=64 KiB, file size)` bytes, media-probe/src/lib.rs:193-216, config.rs:18). The S1 below is on this path for a 188-byte file. Executed: `tests/registry_roundtrip.rs` 3/3 (`every_landed_container_probes_to_its_format_id`, `extension_lists_cover_their_fixtures`, `near_misses_and_garbage_stay_unprobed`) + `probe_io.rs` + `fixtures_matrix.rs` + `corpus.rs` all pass (orchrun-test-workspace-nofailfast.log.txt:3395-3428); the AVI/MP4 first-tier signatures (avi.rs/mp4.rs, other files) plus every shipped open ran through this registry. |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 1 |
| Commands run | Full-file Read (208 lines) · `rg -n "188\|0x47\|TsSig" crates/media-probe/tests/*.rs crates/media-probe/src` (test coverage of the TS boundary: the near-miss case is 201 bytes — registry_roundtrip.rs:286 — and the positive case is a large fixture :134; NOTHING covers a 188-byte window) · Read `registry_roundtrip.rs:120-145, 275-300` (the executed TS cases) · Read `media-probe/src/lib.rs:193-216` `probe_stream`/`fill_window` (window length = min(lookahead, source bytes) — the 188-byte file case is reachable) · `rg -n "DEFAULT_MAX_LOOKAHEAD" config.rs` (64 KiB) · `rg -n "TsSignature" signature/mod.rs:139` (registered builtin) · baseline runs cited above. |

## 1. One-paragraph verdict
Seven of the eight phase-5 signatures are exactly what their docs claim — minimal, correct, mirror-pinned magic gates that never guess past their bytes (I hand-verified each mask: the ADTS layer-00 discrimination `prefix[1] & 0xF6 == 0xF0` correctly refuses MP3's Layer-III `01`, the EBML magic is compared big-endian, the WAV/AIFF form checks sit behind 12-byte guards, and every slice is length-guarded) — and the round-trip suite pins the mirrors against committed fixtures, executed in the baseline. The eighth, `TsSignature`, contains a textbook off-by-one that panics: the second-sync check gates on `prefix.len() >= TS_PACKET_BYTES` (188) and then indexes `prefix[TS_PACKET_BYTES]` — a window of exactly 188 bytes (any 188-byte file starting with `0x47`, including a legitimate single-packet MPEG-TS file, which the function's own comment says should be "the demuxer's own walk to judge") indexes out of bounds and crashes the probe on the shipped open path. No test covers the boundary (the near-miss test uses 201 bytes), so the executed suite is green over a live panic. The remaining defect is the documented mirror-twin duplication itself, whose drift protection stops at the committed fixtures.

## 2. What actually works (verified)
— WAV/AIFF/Ogg/MKV/ADTS/PS/IVF signatures (:57-147, :181-208): every slice preceded by a length guard; magics byte-exact (RIFF/RF64/BW64+WAVE; FORM+AIFF/AIFC; OggS; 0x1A45DFA3 BE; ADIF / 12-bit-sync+layer-00; 00 00 01 BA; DKIF). Extension lists match the demuxers' claims (pinned by `extension_lists_cover_their_fixtures`, executed).
— The ADTS/MP3 layer discrimination (:140): `prefix[1] & 0xF6 == 0xF0` checks sync-low-nibble == F and layer bits == 00 while leaving ID and protection_absent free — traced correct against ISO 13818-7's layout, and pinned from both sides by the executed es-signature tests (`adts_layer00_is_not_mpeg_audio`, `layer3_header_probes_mp3`).
— Honest scope docs (:23-33): ID3-prefixed ADTS/MP3 non-anchoring, extension-routed ES, family-level mkv/mp4 ids — all recorded as gaps, not smoothed.
— Round-trip discipline: `tests/registry_roundtrip.rs` proves mirror-vs-fixture agreement for every landed container (3/3 executed, orchrun :3422-3428); `near_misses_and_garbage_stay_unprobed` pins 6 negative shapes (executed).
— Shipped execution: every `lumen open` runs this registry (demux_host.rs:444-446); the baseline's AVI/MP4 smoke opens and the whole 4534-test suite ran with these signatures live.

## 3. Findings

### F-001 [S1]
Where: crates/media-probe/src/signature/phase5.rs:156-170
Code:
```
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        if prefix.is_empty() || prefix[0] != 0x47 {
            return None;
        }
        if prefix.len() >= TS_PACKET_BYTES {
            if prefix[TS_PACKET_BYTES] != 0x47 {
                return None;
            }
            return Some(anchored(FormatId::mpegts(), TS_PACKET_BYTES + 1));
        }
        // Shorter than one packet: a lone sync byte is all the window
        // can see — the demuxer's own probe accepts it, so the claim
        // stays anchored (score 70, one byte consumed).
        Some(anchored(FormatId::mpegts(), 1))
    }
```
What is wrong: Off-by-one panic. The second-sync check requires `prefix.len() >= TS_PACKET_BYTES` (188) but then reads `prefix[TS_PACKET_BYTES]` — index 188 — which is in bounds only when `len >= 189`. A window of **exactly 188 bytes** (the boundary the gate itself admits) panics with "index out of bounds: the len is 188 but the index is 188". The window length is `min(max_lookahead, source bytes)` (`fill_window`, lib.rs:206-216; default lookahead 64 KiB, config.rs:18), so every **188-byte file whose first byte is `0x47`** produces exactly this window. That set includes (a) a legitimate single-packet MPEG-TS file — the case the function's own doc comment says is "the demuxer's own walk to judge" (:150-152) — and (b) any arbitrary 188-byte file starting with ASCII 'G' (0x47): the registry calls every signature's `probe` on every open, so the file need not be TS at all.
Why it matters: The probe is the first stage of the shipped `lumen open` path (demux_host.rs:444-446) and its entire job is to classify arbitrary untrusted bytes without crashing — "Panics are contract violations" is the layer's stated law (plugin-api/demux.rs:51, the same discipline the probe inherits). A 188-byte input crashes the CLI with a Rust panic before any format validation runs. The executed test suite is green over the live panic because no test uses a 188-byte window: the near-miss case is `[0x47] + [0u8; 200]` (201 bytes — registry_roundtrip.rs:286) and the positive case is a large committed fixture (:133-137). This is the exact off-by-one shape the checklist's "panics on malformed input (… index/slice …)" row exists for, on valid input (a one-packet TS file), reachable from the shipped entry point.
What correct looks like: `if prefix.len() > TS_PACKET_BYTES { … }` — a 188-byte window then falls to the lone-sync anchor branch exactly as the comment intends (or gate on `>= TS_PACKET_BYTES + 1`). Add the 188-byte window to `near_misses_and_garbage_stay_unprobed` and a one-packet-TS positive case.
Confidence: HIGH — Verification: read (the index/length arithmetic is unambiguous); execution UNVERIFIED (no cargo permitted in sandbox) — REQUEST-ORCH-RUN filed in my log: generate a 188-byte 0x47-leading file and `lumen open` it; expected panic at phase5.rs:161.

### F-002 [S4]
Where: crates/media-probe/src/signature/phase5.rs:7-13
Code:
```
//! One struct per format, mirroring its demuxer's own
//! `probe(header)` gate byte-for-byte — the magic lives in `demux`
//! (layering forbids the crate edge), so this module is a
//! mirror-pinned twin: the round-trip suite
//! (`tests/registry_roundtrip.rs`) proves the mirror against the
//! committed fixtures the demuxer cards authored, and any drift is a
//! test failure, not a silent guess.
```
What is wrong: The mirror's drift protection is only as strong as the fixture set: the round-trip suite proves signature-vs-fixture agreement, not signature-vs-demuxer agreement. A demuxer's `probe` gate that changes without a fixture change (or a fixture whose bytes don't exercise a gate's boundary — exactly how F-001 survived) drifts silently. The duplication is a documented, deliberate tradeoff (layering), but its safety claim ("any drift is a test failure") overstates what the tests can see.
Why it matters: F-001 is the live demonstration: the demuxer-side TS probe (`demux::ts::probe`, the claimed mirror) accepts a single-packet file per its own design; the mirror twin panics on it — a drift the round-trip suite cannot catch because no fixture sits on the boundary. Same class as my report.rs F-001 (vocabulary/wiring drift with no structural pin).
What correct looks like: Boundary-vector round-trips: for each signature, a table of window lengths at every gate boundary (11/12/13 for WAV/AIFF, 1/2/3/187/188/189 for TS, …) asserted against the demuxer's own probe where a dev-dependency edge is allowed (tests may cross the layering the lib cannot), or at minimum against golden verdicts.
Confidence: HIGH — Verification: read; the round-trip suite's case list read (:120-145, :275-300); F-001 as the demonstrated miss.
## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (mirror design, scoring ladder, known gaps) | 1-33 | REAL | S4 | F-002 (drift-protection overstatement). The gap list (:23-33) is honest and matches code. |
| `TS_PACKET_BYTES` = 188 | 40-43 | REAL | — | Mirrors `demux::ts::PACKET_BYTES` (comment); correct ISO 13818-1 stride. |
| `anchored` helper | 46-53 | REAL | — | Uniform hit shape: SCORE_MAGIC_ANCHOR (70), empty hints, consumed = magic size. |
| `WavSignature::probe`/`extensions` | 59-73 | REAL | — | 12-byte guard; RIFF/RF64/BW64 + WAVE form. Correct. |
| `AiffSignature::probe`/`extensions` | 79-93 | REAL | — | 12-byte guard; FORM + AIFF/AIFC. Correct. |
| `OggSignature::probe`/`extensions` | 100-108 | REAL | — | 4-byte OggS; version/CRC left to demuxer (documented). |
| `MkvSignature::probe`/`extensions` | 115-125 | REAL | — | BE u32 == 0x1A45DFA3 — correct endianness (EBML is big-endian on wire). |
| `AdtsSignature::probe`/`extensions` | 135-147 | REAL | — | ADIF magic or 12-bit sync + layer-00 (`& 0xF6 == 0xF0` — mask traced correct; ID/protection bits free). The MP3 non-steal is pinned by executed es tests. |
| `TsSignature::probe`/`extensions` | 155-175 | **REAL + S1** | S1 | F-001 (off-by-one panic on the 188-byte window). Extensions [ts, m2t, mts] fine. |
| `PsSignature::probe`/`extensions` | 183-193 | REAL | — | `starts_with(00 00 01 BA)` — no length hazard. |
| `IvfSignature::probe`/`extensions` | 200-208 | REAL | — | 4-byte DKIF; consumed 4. Matches demux::ivf::probe (verified in my ivf/mod.rs audit). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `every_landed_container_probes_to_its_format_id` (registry_roundtrip) | YES | Per-format fixture → expected id; a broken magic fails its row. Does NOT cover boundary lengths (F-001 survives it). |
| `extension_lists_cover_their_fixtures` | YES | Extension coverage per family pinned. |
| `near_misses_and_garbage_stay_unprobed` | YES (for its 6 shapes) | OggT/AIFX/DKIY/00 00 01 BB/garbage + the 201-byte TS near-miss refuse typed. Its TS case is 201 bytes — the 188-byte boundary is absent (F-001 survives). |
| es-signature tests (`adts_layer00_is_not_mpeg_audio`, `layer3_header_probes_mp3`, …) | YES | Pin the ADTS/MP3 mask from both sides. Executed (media-probe lib test block, orchrun :3309+). |
| `probe_io.rs` / `fixtures_matrix.rs` / `corpus.rs` | YES | Window/restore semantics and corpus lanes — executed (:3395-3421). |
All executed, 0 failed — green over the F-001 panic (no 188-byte case exists anywhere in the crate; verified by rg over tests/).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "mirroring its demuxer's own probe(header) gate byte-for-byte" | module docs :7-8 | REAL (7 of 8) / **FALSE (TS)** | The TS mirror diverges from its demuxer's acceptance on the single-packet file: `demux::ts::probe` accepts (per :150-152's own comment), the twin panics (F-001). |
| "any drift is a test failure, not a silent guess" | module docs :10-13 | SHALLOW | F-002 — the round-trip suite pins fixture agreement only; F-001 is drift the suite cannot see. |
| "the probe never guesses past its bytes. Track hints stay empty" | :15-21 | REAL | `anchored` always: score 70, empty hints; verified at :46-53 and in every signature. |
| "Known content-probe gaps (recorded, not smoothed)" (ID3 prefixes, extension-routed ES, family ids) | :23-33 | REAL | Verified against the es signature module's design (my report.rs audit's cross-checks) and the CLI's es::open routing (containers/mod.rs:62-66). |
| feature-matrix rows for wav/aiff/ogg/matroska/adts/mpegts/mpegps/ivf (probe-side) | docs/feature-matrix.toml (multiple rows) | REAL (probe side) | These signatures + registry_roundtrip 3/3 executed; each container's end-to-end status is the respective row's own audit's business. |

## 7. Dead / duplicated / unreachable code
- Duplication is the module's design (mirror twins, F-002); no dead code — all 8 signatures registered (`with_builtins`, signature/mod.rs:132-140) and exercised.

## 8. Dependencies & flags
- `crate::report` (FormatId/ProbeHit/SourceMeta), `super::SCORE_MAGIC_ANCHOR` (70, signature/mod.rs:91). No feature gates; no unsafe; no allocations beyond the `FormatId` String per hit (bounded, once per open).

## 9. Open questions for the next agent
- The `signature/mod.rs` assignee: the registry iterates ALL signatures on every window — confirm iteration order cannot mask the F-001 panic behind an earlier signature's hit (it cannot: every signature's probe is called before the winner is chosen — :181-202 — so the panic fires regardless of any other hit; but pin it in your audit).
- REQUEST-ORCH-RUN (mine, in the log): `python3 -c "import sys; sys.stdout.buffer.write(b'\x47'+b'\x00'*187)" > /tmp/g188.bin && ./target/release/lumen open file:///tmp/g188.bin` — expect a panic with "index out of bounds" at crates/media-probe/src/signature/phase5.rs:161. Also rerun `cargo test -p media-probe` after any fix to confirm no regression.
