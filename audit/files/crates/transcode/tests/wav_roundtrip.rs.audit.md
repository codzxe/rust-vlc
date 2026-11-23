# AUDIT: crates/transcode/tests/wav_roundtrip.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 588 / 12 helpers + 5 tests |
| File verdict | REAL (honest, strong round-trip suite) — crate is library-only, no shipped consumer |
| Reachable from shipped entry point? | NO — `transcode` crate has zero dependents (rg over all Cargo.tomls); matrix `ui-dialog-convert` honestly says not_started |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p transcode --test wav_roundtrip` → 5 passed/0 failed (exit 0); rg dependents; sed quote checks |

## 1. One-paragraph verdict
This is a genuinely strong acceptance suite: 4 real committed ffmpeg-generated WAV
fixtures (u8/s16le/s24le/f32le) are demuxed by the real `WavDemuxer` over the real
`input::FileAccess` stack, decoded by the real `PcmDecoder`, encoded by the real
WAV writer inside a real submitted job, then the OUTPUT is re-demuxed and
re-decoded by the same stack and compared byte-for-byte (payload identity +
per-frame sample identity + RIFF header field checks + accounting stats). The two
copy paths (WavCopyPcm rewrap, RawEsCopy dump) assert verbatim payload identity,
and two negative tests pin typed refusal codes. I ran it: 5/5 pass. The only
caveat is product-level: nothing depends on the `transcode` crate, so this suite
proves a library nobody ships — the feature matrix is honest about that
(`ui-dialog-convert` = not_started, empty evidence).

## 2. What actually works (verified)
— verified by `cargo test -p transcode --test wav_roundtrip` → 5 passed; 0 failed
(exit 0, my run). Fixtures exist on disk under tests/fixtures/generated/valid/
(verified by find). Baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt:6430-6438.

## 3. Findings
F-001 [S3]
Where: crates/transcode/tests/wav_roundtrip.rs:353 (whole suite)
Code:
```
#[test]
fn fixtures_roundtrip_decode_encode_redemux_redecode_bitexactly() {
```
What is wrong: The suite is real, but the `transcode` crate is a leaf: no crate or
app depends on it (rg '"transcode"|transcode =' over all Cargo.tomls → only its
own). The proven capability (PCM encode, lossless remux, ES dump) is not
reachable from `lumen` CLI or desktop. Why it matters: the evidence value of
these tests is library-truth only; any matrix row citing them as user-visible
playback would be inflated (none currently does — verified ui-dialog-convert row).
Confidence: HIGH — Verification: read (rg dependency graph).

F-002 [S4]
Where: crates/transcode/tests/wav_roundtrip.rs:96-108 and 139-151
Code:
```
            SeekFrom::End(n) => {
                let len = self.twin.lock().unwrap_or_else(|p| p.into_inner()).len() as i64;
                self.pos = usize::try_from(len + n).unwrap_or(self.pos);
            }
```
What is wrong: `SharedCursor::seek` (Write+Seek side) has no bounds check for
`SeekFrom::End(n>0)` or `Start(n>len)` — pos can exceed the buffer; the paired
`MemStream::read` then computes `end = min(pos+len, data.len()) < pos` and
`self.data[self.pos..end]` panics (start > end), and `read_into`'s
`out[..end - self.pos]` underflows. Never triggered by the current sinks (the
WAV writer only seeks within the written file), so latent test-infra landmine,
not a product bug. What correct looks like: clamp or refuse like
`MemStream::seek`'s `target > len` check at L160-162 does. Confidence: HIGH —
Verification: read; not triggered (tests pass).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedCursor::write/flush/seek | 80-109 | REAL | S4 (F-002) | resize-on-write cursor; End-seek unbounded |
| sink_pair | 69-78 | REAL | — | Arc<Mutex<Vec<u8>>> twin seam |
| MemStream::over + SourceStream impl | 119-169 | REAL | — | size-known seekable in-memory source; SourceStream::seek IS bounds-checked (L160) |
| mem_seek_error | 172-180 | REAL | — | typed refusal |
| open_fixture | 188-193 | REAL | — | real FileAccess + WavDemuxer over committed fixture |
| demux_all | 203-223 | REAL | — | collects fmt bytes + concatenated payload |
| CollectSink::accept | 232-244 | REAL | — | keeps (format, samples) per frame |
| decode_all | 248-280 | REAL | — | real PcmDecoder configured from source fmt bytes |
| flattened | 283-289 | REAL | — | byte-compare side |
| FixturePcm::open | 300-305 | REAL | — | DecoderFactory wiring seam (channels/rate seeded) |
| fixtures_roundtrip… | 352-453 | REAL | — | 4 codecs × (stats + header + payload + per-frame sample identity) |
| wav_from_pcm_copy_rewraps_without_decoding | 459-506 | REAL | — | decode:None, verbatim payload, re-demux, sample equality |
| raw_es_copy_dumps_the_payload_verbatim | 508-538 | REAL | — | output == payload exactly, no header |
| copy_facts_disagreeing_with_the_track_fail_typed | 540-568 | REAL | — | pins "transcode.copy.facts-mismatch" |
| submit_refuses_a_wav_pcm_graph_without_decode | 570-588 | REAL | — | pins "transcode.graph.decode-required" |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_roundtrip…bitexactly | YES | any decode/encode/header/accounting drift fails exact asserts; would catch endianness, truncation, size lies |
| wav_from_pcm_copy_rewraps_without_decoding | YES | `assert_eq!(&output[44..], demuxed.payload.as_slice())` is verbatim payload |
| raw_es_copy_dumps_the_payload_verbatim | YES | whole-output equality |
| copy_facts_disagreeing… | YES | exact error code pinned |
| submit_refuses_a_wav_pcm_graph_without_decode | YES | exact error code pinned |
Caveat: both "identity" sides use OUR demuxer/decoder — a symmetric bug (e.g.
both demux and writer wrong in the same way) would pass. Mitigated by header
field checks (RIFF/data sizes) and ffmpeg-provenance fixtures, but no external
oracle decodes the OUTPUT. Noted, not a defect.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded by OUR decoder, encoded by OUR WAV writer… identical" | wav_roundtrip.rs:1-6 | REAL | ran the suite, 5/5 pass |
| "Nothing is written to disk by these tests" | wav_roundtrip.rs:12-13 | REAL | sink is in-memory SharedCursor (read) |
| ffmpeg fixtures "committed… under tests/fixtures/generated/valid/" | wav_roundtrip.rs:10-12 | REAL | files exist (find); CASES frames values are ffprobe truths per demux suite (UNVERIFIED against ffprobe here — no ffmpeg run, cross-checked only with demux-suite records) |

## 7. Dead / duplicated / unreachable code
- Whole `transcode` crate library-only (F-001). Test file itself has no dead code.

## 8. Dependencies & flags
Test uses: codec_api, codecs (pcm), demux (wav), media_types, plugin_api,
testkit, transcode, input (via dev-dep). No feature gates.

## 9. Open questions for the next agent
- Who is intended to call `transcode::submit` (UI dialog 7.E3? CLI command)?
- Verify CASES `frames` (8820/9600) against ffprobe duration_ts once ffmpeg is
  available in a lane (claimed recorded by demux suite).
