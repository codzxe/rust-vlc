# AUDIT: crates/codecs/src/audio/mp3/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 121 / 1 (`is_mp3_stream`) + 2 tests (module root: 7 mods, 8 re-export groups) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:261-264 `"mp3" \| "mp1" => { let mut decoder = Mp3Decoder::new(); ... }` → `codecs::audio::mp3::Mp3Decoder` re-exported here (mod.rs:67); apps/web/src/host.rs likewise |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "is_mp3_stream" apps crates` (exit 0 — zero non-test callers); `rg -n "fn accepts" -A 3 mp3/decoder.rs` (exit 0); `rg -n "mp3::tests::" audit/runs/baseline/cargo-test-workspace.txt` (exit 0, 2/2 ok lines 537,547); `rg -n "Running tests/mp3" -A 6` on baseline (bitexact 2 passed in 1.01s; corpus differential 2 passed in 0.06s — SKIP path); `test -d /tmp/my-project/corpus-v4` -> ABSENT; sed quote verification |

## 1. One-paragraph verdict
A clean module root: every re-export resolves to a real submodule (decoder/header/id3/seek/xing/fuzz all on disk), the engine identity is stated honestly (`symphonia-bundle-mp3` 0.6.1, purity-pinned per Cargo.toml — MP3/MP2 are genuinely symphonia-backed, unlike Opus's ruopus), the probe function is correct and negative-tested (fLaC, RIFF, garbage refused), and the synthetic end-to-end test pins exact geometry (1 frame, stereo, 44.1 kHz, 1152×2 samples) — both tests pass in the baseline. Three nicks: the module advertises "MP3 / MP2" but the decoder also accepts and wires MP1 (Layer I) — doc drift; the headline evidence files include a corpus differential that silently SKIPs in this environment (its baseline "ok" at 0.06s with the corpus directory absent asserts nothing); and the probe is a future-facing surface with zero non-test callers.

## 2. What actually works (verified)
— verified by `rg -n "mp3::tests::" audit/runs/baseline/cargo-test-workspace.txt` -> `probe_accepts_id3_and_frame_headers_only ... ok`, `a_minimal_synthetic_stream_decodes_end_to_end ... ok` (lines 537, 547). The synthetic frame is byte-consistent with its header: 0xFF 0xFB 0x90 0x00 = MPEG-1 Layer III 128 kbps 44.1 kHz stereo, frame length ⌊144×128000/44100⌋ = 417 = 4 + 413 payload (hand-checked).
— verified by `rg -n "Running tests/mp3" -A 6 audit/runs/baseline/cargo-test-workspace.txt` -> `mp3_bitexact 2 passed ... 1.01s` — real fixture decode work (the in-tree ffmpeg-authored fixtures), not a skip.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/mp3/mod.rs:1-2 vs crates/codecs/src/audio/mp3/decoder.rs:437-439 and apps/cli/src/pipeline_host.rs:261
Code (mod.rs:1-2, verbatim):
```
//! MP3 / MP2 audio decoder (card 5.A3; doc 04 §3.1 rows "MP3 (MPEG-1/
//! 2/2.5 Layer III)" and "MP2", §2.2 row "MP3 raw stream") — the
```
vs (decoder.rs:437-439, verbatim):
```
    fn accepts(descriptor: &str) -> bool {
        matches!(descriptor, "mp1" | "mp2" | "mp3")
    }
```
What is wrong: the module's own format slice claims MP3/MP2, but the shipped decoder accepts `mp1` (MPEG Layer I) — and the CLI wires it (`"mp3" | "mp1" =>` at pipeline_host.rs:261) with its own engine mapping (`"mp1" => CODEC_ID_MP1`, decoder.rs:443). Layer I support is real, shipped, and undocumented at the module root (the MP2 sibling module handles Layer II via its own path; mp1 rides here). A reader auditing "what does this module decode" gets the wrong inventory. What correct looks like: "MP3 / MP2 / MP1 audio decoder" plus one line on Layer I in the format slice. Confidence: HIGH — Verification: read + rg.
F-002 [S3]
Where: crates/codecs/src/audio/mp3/mod.rs:52-53 (the evidence citation), crates/codecs/tests/mp3_corpus_differential.rs:78-86
Code (mod.rs:44-53 excerpt, verbatim):
```
//! LAME delay/padding trimmed (verified empirically: ffmpeg decodes
//! exactly `frames × spf − delay − padding` samples of an
//! ffmpeg-authored fixture) — and records the PCM MD5 plus the max
//! sample delta per file (`tests/mp3_bitexact.rs`,
//! `tests/mp3_corpus_differential.rs`).
```
and (mp3_corpus_differential.rs:83-85, verbatim):
```
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP mp3 corpus differential: corpus root absent");
        return;
```
What is wrong: the module root cites both test files as the recorded evidence for the gapless-window/MD5 posture, but the corpus differential half silently SKIPs (eprintln + `return` = test passes) when the corpus root or ffmpeg is absent — and the corpus IS absent here (`test -d /tmp/my-project/corpus-v4` -> ABSENT; the baseline shows `corpus_and_fixture_differential_vs_ffmpeg ... ok` in a 0.06s bundle — the fixture half may run, the corpus rows cannot). The baseline green therefore proves the gapless/MD5 claims only for the in-tree fixtures, not the corpus rows the doc implies. Same defect class as the Opus differential (my decoder.rs audit F-003) — repo-wide evidence hazard. What correct looks like: counted/asserted skips so a corpus-absent run cannot record a vacuous pass. Confidence: HIGH — Verification: read + baseline log + corpus-absence check. (mp3_bitexact.rs genuinely ran: 1.01s, 2 passed — that half is live evidence.)
F-003 [S4]
Where: crates/codecs/src/audio/mp3/mod.rs:76-85
Code:
```
pub fn is_mp3_stream(data: &[u8]) -> bool {
    if data.get(..3) == Some(b"ID3") {
        return true;
    }
    parse_frame_header(data).is_ok()
}
```
What is wrong: zero non-test callers (rg census) — the "future MP3 demuxer / registry row" it exists for has not landed. Honest (the doc says so), but it is shipped public API wired to nothing today. What correct looks like: land the raw-MP3 demuxer row or mark the probe `#[allow(dead_code)]`-equivalent with a tracking note. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `is_mp3_stream` | 79-85 | REAL | S4 (F-003) | correct logic (ID3 prefix OR valid header at 0); negative-tested |
| module decls/re-exports | 55-74 | REAL | — | all resolve; `fuzz` is `pub` by documented precedent |
| test `probe_accepts...` | 91-100 | REAL | — | 6 assertions incl. fLaC/RIFF/empty refusals |
| test `a_minimal_synthetic_stream...` | 102-120 | REAL | — | exact geometry pinned; frame length hand-verified 417 bytes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_accepts_id3_and_frame_headers_only | YES | positive and negative cases with distinct prefixes |
| a_minimal_synthetic_stream_decodes_end_to_end | YES | exact frames/channels/rate/sample-count asserts; an engine refusal fails it (Err arm is unreachable!, i.e. fail) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The entropy decode of one framed packet is the ... `symphonia-bundle-mp3` 0.6.1 engine's job" | mod.rs:16-17 | REAL | Cargo.toml `symphonia-bundle-mp3 = { version = "=0.6.1", optional = true }`, `codec-mp3` default-on; engine mapping decoder.rs:441-443 |
| "no panic on any input — every decode/parse/seek path is bounds-checked" | mod.rs:37-42 | REAL-UNPROVEN here | pinned by fuzz harness (fuzz.rs) + fuzz-survival test (baseline); full-corpus fuzz execution UNVERIFIED (cargo-build-budget) |
| "records the PCM MD5 plus the max sample delta per file" | mod.rs:51-53 | SHALLOW in this environment | corpus half SKIPs silently (F-002); fixture half real (bitexact 1.01s) |
| gapless window math (`frames × spf − delay − padding`) | mod.rs:49-51 | REAL | mp3_bitexact fixtures_decode_the_recorded_window_exactly ok in baseline (line 1261) |

## 7. Dead / duplicated / unreachable code
`is_mp3_stream` — public, tested, zero production callers (F-003, admitted future-facing). No duplication; the module correctly delegates to siblings instead of copying.

## 8. Dependencies & flags
Engine: `symphonia-bundle-mp3` + `symphonia-core` behind `codec-mp3` (default-on, shipped); `codec-mp2 = ["codec-mp3"]` rides the same engine. No unsafe on own paths (doc claim; module has no unsafe — rg). testkit dev-only.

## 9. Open questions for the next agent
- header.rs/decoder.rs owners: Layer I ("mp1") lacks any fixture or corpus lane I could find — is CODEC_ID_MP1 decode covered by ANY test? (UNVERIFIED: no mp1-named test in baseline grep; if none, the shipped mp1 path is engine-trust-only.)
- The raw-MP3 demuxer row that would give `is_mp3_stream` a caller — still pending? Track or delete.
