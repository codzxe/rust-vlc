# AUDIT: crates/demux/tests/mkv_fixtures.rs

| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-22 |
| Lines / functions | 469 / 17 (5 helpers, 2 oracle fns, 1 fixture builder, 8 `#[test]`s, 1 tiny open fn) |
| File verdict | REAL — unconditional half is strong and byte-exact; the two headline differentials are oracle-conditional and vacuous without ffmpeg |
| Reachable from shipped entry point? | TEST-ONLY by design: compiled as its own test target via cargo auto-discovery (`crates/demux/tests/*.rs`; Cargo.toml has no `autotests = false` / `[[test]]` overrides — verified). Runs under `cargo test -p demux`. Not shipped in binaries. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | ls of `tests/fixtures/generated/valid` + `malformed` (existence + byte sizes of all 22 referenced fixtures); rg `oracle_available\|skip_no_oracle` across crates/demux/tests; rg fixture-name usage across tests; read of testkit/src/lib.rs:124-155 (oracle skip semantics), tests/common/mod.rs (md5/open_via_input); read of mkv_corpus.rs:1-60 (corpus oracle strategy); sed quote verification. No cargo/ffmpeg run (adaptation) → per-test pass status and differential outcomes UNVERIFIED; suite-level status cites W00 baseline. |

## 1. One-paragraph verdict

This suite has two honest halves and one environment hole. Unconditionally, it is excellent: a hand-built byte-exact lacing fixture (Xiph/EBML/fixed, all three codings, payloads asserted byte-for-byte), typed error-path pins (`unknown-doctype`, `not-matroska`, mid-block `truncated` with a "must surface, not vanish" assertion), an 18-file malformed-derivative no-panic sweep with an event-storm bound, and a `walk_all` helper that — unlike the MP4 unit suite's `drain()` — panics on any `Err` (`.must()` per event), so Err≠EOS is enforced. The hole: the two headline differentials (ffprobe packet counts, ffmpeg md5 bit-exactness over the 4 committed valid fixtures) `return` early when ffmpeg/ffprobe are absent — a skip that cargo counts as a pass — and those 4 valid fixtures appear in no other test file, so on an oracle-less runner the committed valid MKV bytes get zero asserted demuxes anywhere in the repo.

## 2. What actually works (verified)

- Verified statically: `laced_fixture()` is an independent hand-rolled EBML writer (correct vint sizes, correct lacing flag bits 0x02/0x06/0x04, minimal uint encodings) — no demuxer round-trip, no golden file; `laced_frame_payloads_are_byte_exact` asserts all 9 payloads byte-for-byte and the relative-timecode stamps.
- Verified statically: `walk_all` (mkv_fixtures.rs:94-113) uses `demuxer.next_event().must()` — any mid-walk `Err` panics the test. This is the strict harness the MP4 unit suite lacks.
- Verified by ls: all 4 valid fixtures (mkv_h264_aac.mkv 26,105 B; mkv_h264_flac.mkv 33,443 B; mkv_multitrack.mkv 52,208 B; webm_vp9_opus.webm 27,259 B) and all 18 malformed derivatives exist with the exact names the test asserts (`assert!(path.exists(), "{name} missing")` would fail loudly otherwise).
- Verified statically: `open_via_input` (tests/common/mod.rs:546-550) opens fixtures through the real `input::FileAccess` + `SourceUri` stack, not just `mem_stream` — the differential exercises the shipped access seam.
- W00 baseline: workspace tests 1751 pass / 2 fail (UI golden) / 25 ignored; these tests are not `#[ignore]`d (rg: 0), so the unconditional tests presumably ran green; whether W00's environment had ffmpeg (making the differentials real vs vacuous) is UNVERIFIED.

## 3. Findings

### F-001 [S3] The two headline differentials pass without verifying anything when ffmpeg is absent; the 4 committed valid fixtures then have zero asserted coverage repo-wide
Where: crates/demux/tests/mkv_fixtures.rs:116-123 (and 149-152)
Code:
```rust
fn valid_fixtures_match_ffprobe_packet_counts() {
    // The live-ffprobe oracle: skip loudly when absent (the W25 CI
    // failure — a fresh runner has no ffmpeg; LUMEN_REQUIRE_ORACLE=1
    // makes absence a hard failure for local strictness).
    if !oracle_available() {
        skip_no_oracle("mkv fixture packet-count differential");
        return;
    }
```
What is wrong: `skip_no_oracle` only `eprintln!`s and returns (testkit/src/lib.rs:145-151) — the test is not `#[ignore]`d, so cargo records it as **passed**. Same for `audio_payload_is_bit_exact_vs_ffmpeg` (:149-152). rg across `crates/demux/tests/*.rs`: the fixture names (`mkv_h264_aac.mkv`, `mkv_h264_flac.mkv`, `mkv_multitrack.mkv`, `webm_vp9_opus.webm`) appear ONLY in this file; the other real-bytes suite (mkv_corpus.rs) requires both live ffprobe AND an external corpus dir (`/tmp/my-project/corpus-v4` or `LUMEN_CORPUS`, mkv_corpus.rs:30-37) and skips loudly otherwise. Net: on a runner without ffmpeg, the committed valid MKV/WebM fixtures are demuxed to EOS with assertions nowhere; "1751 tests pass" can include these two as zero-verification passes.
Why it matters: The file's doc advertises "the committed ffmpeg-authored fixtures" as the card's evidence; that evidence evaporates on exactly the machines the doc itself predicts ("a fresh runner has no ffmpeg"). The mitigation that exists is honest (loud eprintln, `LUMEN_REQUIRE_ORACLE=1` hard-fail knob, testkit/src/lib.rs:126-133) — the defect is that pass/vacuous-pass is indistinguishable in the cargo summary and the fixtures have no oracle-free floor (e.g., a committed expected-counts sidecar like the AVI corpus's `expectation/1`).
What correct looks like: Commit the ffprobe counts/md5s once (the "fixtures are committed so the truth is stable" argument at :36-37 already concedes they are stable) and assert against the sidecar unconditionally, using live ffmpeg only to refresh the sidecar.
Confidence: HIGH — Verification: read + rg + ls (whether W00's baseline ran them for real: UNVERIFIED).

### F-002 [S4] Doc claim "all < 32 KiB committed" is false for two of the four fixtures
Where: crates/demux/tests/mkv_fixtures.rs:8-10
Code:
```rust
//! ffmpeg-authored fixtures (lavfi sources, ffmpeg 7.1.5, all < 32
//! KiB committed):
```
What is wrong: `ls` gives mkv_h264_flac.mkv = 33,443 bytes (32.66 KiB) and mkv_multitrack.mkv = 52,208 bytes (50.98 KiB) — both over 32 KiB (32,768).
Why it matters: A factual claim about committed artifacts, checkable in one command, that is wrong — small, but exactly the class of unverified self-description this audit exists to count.
What correct looks like: "< 52 KiB" or per-file sizes.
Confidence: HIGH — Verification: ls (sizes pasted in §2).

### F-003 [S4] `malformed_derivatives...` asserts typedness only at open; walk-phase outcomes are printed, never asserted
Where: crates/demux/tests/mkv_fixtures.rs:383-396
Code:
```rust
            Ok(tracks) => {
                let mut events = 0u64;
                let outcome = loop {
                    match demuxer.next_event() {
                        Ok(DemuxEvent::EndOfStream) => break "eos".to_string(),
                        Ok(_) => {}
                        Err(e) => {
                            let code = e.context.code.clone();
                            break code; // owned: outlives the loop arm
```
What is wrong: The name promises "typed or recover"; the body asserts the `demux.` prefix only for `start()` refusals (:378-380). For files that open, any walk `Err` code is captured into `outcome` and only `println!`ed (:397-401) — a demuxer that returned a garbage/unstable code, or `Recovered`-looped forever under the 1M budget, still passes. The only behavioral assertions are `path.exists()`, the event-storm bound, and `checked == 18`.
Why it matters: Name-vs-body honesty gap; the printed table is not a test oracle.
What correct looks like: Assert the captured code starts with `demux.` too (one line), or rename to `..._never_panics`.
Confidence: HIGH — Verification: read.

### F-004 [S4] `probe_packets` hand-rolls JSON parsing with silent `unwrap_or(0)`/`unwrap_or_default()` defaults
Where: crates/demux/tests/mkv_fixtures.rs:64-69
Code:
```rust
        streams.push((
            field("codec_name").unwrap_or_default(),
            field("nb_read_packets")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        ));
```
What is wrong: A split-on-`"index":` pseudo-JSON parser; a field-shape change in ffprobe output silently degrades `codec_name` to `""` (never asserted here, harmless) or `nb_read_packets` to 0 (would fail the count assert loudly — fail-loud, not silent-pass, for the fields that matter). The `field` closure's numeric branch `rest[..rest.find([',', '}'])?]` mis-parses nested objects (none appear in the fields read).
Why it matters: Test-infra fragility pinned to ffprobe's exact JSON layout; acceptable for a differential harness, worth recording.
What correct looks like: serde_json (the repo already depends on serde ecosystem elsewhere) or assert-parse-failure loudly.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function / test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fixture` (helper) | 30-34 | REAL | — | Path from CARGO_MANIFEST_DIR to repo-root fixtures; existence asserted by callers. |
| `probe_packets` (helper) | 38-72 | REAL | S4 | F-004 hand-rolled JSON; fail-loud on the fields that matter. |
| `ffmpeg_stream_md5` (helper) | 75-92 | REAL | — | ffmpeg `-map 0:k -c copy -f md5`; non-zero exit is `unreachable!` (fixture-contract failure, allowed form). Oracle-dependent. |
| `walk_all` (helper) | 94-113 | REAL | — | Strict `.must()` per event — Err panics (contrast mp4/tests.rs `drain`). Keyframe tallies collected; only asserted in the lacing test, unused in the packet-count test (harmless dead compute). |
| `valid_fixtures_match_ffprobe_packet_counts` | 115-145 | REAL-if-oracle / VACUOUS-if-not | S3 | F-001. When it runs: stream count, track count, total + per-stream packet counts vs live ffprobe — strong differential. |
| `audio_payload_is_bit_exact_vs_ffmpeg` | 147-195 | REAL-if-oracle / VACUOUS-if-not | S3 | F-001. Per-audio-track concatenated payload md5 vs ffmpeg raw dump; per-stream mapping (not interleave) — methodologically correct. |
| `el` (helper) | 201-218 | REAL | — | Minimal EBML size vint; 3-byte path caps at 0x1FFFFF with no guard for larger payloads (fixtures are tiny — test-only nit). |
| `uint_payload` (helper) | 220-224 | REAL | — | Minimal big-endian uint; 0 encodes as single 0x00 (`unwrap_or(7)`) — correct. |
| `laced_fixture` (helper) | 228-284 | REAL | — | Independent spec-built EBML writer; correct lacing bits; no circularity. |
| `hand_built_laced_fixture_decodes_all_three_codings` | 286-298 | REAL | — | 9 frames, all keyframes, `damage_bytes() == 0`. Strong; oracle-free. |
| `laced_frame_payloads_are_byte_exact` | 300-340 | REAL | — | All 9 payloads byte-exact + stamp set {0,8,16}; strict `.must()` loop. Strongest test in the file. |
| `malformed_derivatives_are_typed_or_recover_never_panics` | 344-407 | REAL (as no-panic) | S4 | F-003; 18 files asserted to exist; open refusals asserted typed; walk outcomes printed only; 1M event-storm bound. |
| `unknown_doctype_is_the_typed_refusal` | 411-422 | REAL | — | Same-length DocType patch; pins `demux.mkv.unknown-doctype` + empty `doctype()`. |
| `zero_head_is_the_typed_bad_vint` | 424-432 | REAL | — | Name says "bad_vint" but the pinned code is `demux.mkv.not-matroska` (the zeroed first byte breaks the EBML magic before any vint parse) — name mislabel, body honest. |
| `truncated_cluster_is_the_typed_truncation` | 434-458 | REAL | — | Cuts mid-Xiph payload; asserts the typed `demux.mkv.truncated` surfaces (`saw_typed` guard) — not just no-panic. |
| `probe_matches_the_ebml_magic` | 460-465 | REAL | — | Magic accept/reject + 3-byte short reject. |
| `open` (helper) | 467-469 | REAL | — | `open_via_input` — the real access stack (tests/common/mod.rs:546-550). |

## 5. Test honesty (§3.4 judgment)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `hand_built_laced_fixture_decodes_all_three_codings` / `laced_frame_payloads_are_byte_exact` | YES | Exact counts, byte-exact payloads, stamps, damage==0; strict Err handling. A lacing-decode regression (any of 3 codings) fails them. |
| `unknown_doctype...`, `zero_head...`, `truncated_cluster...`, `probe_matches...` | YES | Exact typed codes / magic pinned. |
| `malformed_derivatives...` | PARTIALLY | Only open-phase typedness + no-panic + storm bound (F-003). |
| `valid_fixtures_match_ffprobe_packet_counts` / `audio_payload_is_bit_exact_vs_ffmpeg` | YES with ffmpeg; NO without | F-001: early `return` = vacuous pass on oracle-less runners. |

Circular goldens: none (live-oracle differentials + independent hand-built bytes). Mocks-of-the-subject: none. Ignored tests: none (rg: 0 `#[ignore]`).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all corpus + fixture SimpleBlocks carry lacing bits `00`" (justifying the hand-built laced fixture) | mkv_fixtures.rs:3-6 | UNVERIFIED | Would require parsing every corpus/fixture block; not executed here. The mitigation (hand-built laced fixture) is real regardless. |
| "ffmpeg 7.1.5, all < 32 KiB committed" | mkv_fixtures.rs:9-10 | FALSE (size half) | F-002: ls shows 33,443 B and 52,208 B fixtures. ffmpeg version claim UNVERIFIED (no run). |
| "the fixtures are committed so the truth is stable" | mkv_fixtures.rs:36-37 | REAL | Files verified present; truth stability conceded — which makes the absence of a committed sidecar (F-001) the stranger. |
| "the W25 CI failure — a fresh runner has no ffmpeg" | mkv_fixtures.rs:118-119 | UNVERIFIED | Historical CI claim; no CI logs inspected by this agent. |
| Laced frames "share the block stamp (no DefaultDuration declared)" | mkv_fixtures.rs:334-337 | REAL | Pinned by the `stamps.iter().all(...)` assertion at :336-339. |

## 7. Dead / duplicated / unreachable code

- `walk_all`'s keyframe tallies unused by the packet-count test (noted in table).
- No duplicated logic; the ffprobe-parsing helpers here overlap in spirit with mkv_corpus.rs's `ffprobe_streams` (different file, different record type) — cross-file duplication of the "shell out to ffprobe" pattern, minor.

## 8. Dependencies & flags

- No feature flags; compiled by default test auto-discovery.
- Runtime test deps: ffmpeg/ffprobe on PATH for the two differentials (graceful, loud skip; `LUMEN_REQUIRE_ORACLE=1` hard-fail — testkit/src/lib.rs:124-134); `testkit` (Must, oracle fns); `input` crate (real FileAccess); committed fixtures under repo-root `tests/fixtures/generated/{valid,malformed}` (all 22 verified present).

## 9. Open questions for the next agent

- W00/verify agents: did the baseline environment have ffmpeg on PATH? This decides whether "mkv audio bit-exact vs ffmpeg" was ever actually executed in the recorded 1751-pass run (UNVERIFIED here).
- Claim-ledger agents (W18/W19): if any matrix row cites "MKV demux bit-exact vs ffmpeg" as evidence, it is oracle-conditional per F-001 — check the row's hardware_class honesty.
- mkv_corpus.rs owner: its skip path (corpus dir absent) has the same vacuous-pass shape as F-001; audit it under the same lens.
