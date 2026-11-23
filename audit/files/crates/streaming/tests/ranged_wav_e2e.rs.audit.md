# AUDIT: crates/streaming/tests/ranged_wav_e2e.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 315 / 4 helpers (drain, wav_data_chunk, md5_hex + constants) + 3 tests |
| File verdict | REAL (strong three-way differential e2e over real sockets; ffmpeg MD5 truth independently reproduced by me) |
| Reachable from shipped entry point? | Test file; the code under test (ranged::RangedResource) IS reachable: apps/cli demux_host.rs open_http_source → RangedResource::open / SchemeRegistry::open_http |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 RIFF parse + hashlib md5 of the fixture (independent truth check); baseline citation |

## 1. One-paragraph verdict
This e2e is the honest kind: a real ffmpeg-generated WAV (35 358 bytes on disk, verified) is served by the mockhttp server over a REAL loopback TCP socket, opened through `RangedResource` as a `plugin_api::SourceStream`, and demuxed by the real `WavDemuxer`; the concatenated payload is then proven three independent ways — byte-equality with the on-disk `data` chunk parsed test-side, MD5 equality with a recorded ffmpeg digest, and byte-equality with the same fixture demuxed through the `input::FileAccess` local path. I independently reproduced the pinned MD5 with python3/hashlib: data chunk 35 280 bytes → `dcaccb18f7fc858b87684ad3b25bc7c5`, exactly the `FFMPEG_MD5` constant — the "recorded ffmpeg truth" is real, not a digest of the code's own output. The test-side MD5 is itself pinned against all seven RFC 1321 A.5 vectors. Two defects: the "no re-fetch storm" claim is asserted with a LOWER bound only (≥30 requests, no upper bound — a window-cache regression that re-fetches 10× passes), and the transport is MockClient, not the production UreqClient (cross-ref mockhttp audit F-001).

## 2. What actually works (verified)
— Baseline audit/runs/baseline/cargo-test-workspace.txt (streaming package, ranged_wav_e2e binary): `running 3 tests` → `wav_over_http_demuxes_to_the_on_disk_truth ... ok`, `wav_over_no_range_http_demuxes_to_the_on_disk_truth ... ok`, `md5_matches_rfc1321_vectors ... ok` → `test result: ok. 3 passed; 0 failed` (0.07s).
— My independent truth check: `python3` parsed tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav RIFF structure → data chunk 35 280 bytes, `hashlib.md5 = dcaccb18f7fc858b87684ad3b25bc7c5` == FFMPEG_MD5 (L43). File on disk: 35 358 bytes.

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/ranged_wav_e2e.rs:131-139
Code:
```
    // The request log proves the ranged path really did the work:
    // many range requests, all ranged, no re-fetch storm.
    let requests = server.requests();
    assert!(
        requests.len() >= 30,
        "dozens of range requests over a {}-byte file (got {})",
        bytes.len(),
        requests.len()
    );
```
What is wrong: the comment claims "no re-fetch storm" but the assertion is a lower bound only. A regression in `RangedResource`'s window cache that re-fetches every window on every seek (10× or 100× the necessary requests) passes this test untouched. Why it matters: the request-count shape is this e2e's only defense against pathological refetch behavior — the exact perf defect a ranged reader can silently develop; the defense has no teeth. Same shape in the fallback test (L200-205, `requests.len() >= 2`, no upper bound). What correct looks like: `assert!(requests.len() < 2 * bytes.len() / 1024 + 8)` or a pinned exact count with slack. Confidence: HIGH — Verification: read (no upper-bound assertion exists in either test).

F-002 [S4]
Where: crates/streaming/tests/ranged_wav_e2e.rs:23 and 93
Code:
```
//! A small chunk size (1 KiB over a ~46 KiB file) forces dozens of
//! range requests, so the demuxer's seek-heavy header walk genuinely
```
What is wrong: the fixture is 35 358 bytes ≈ 34.5 KiB, not "~46 KiB" (both occurrences). The 8 820-frame figure (L38) is correct (35 280 / 4 bytes-per-stereo-s16-frame). Cosmetic, but the stated arithmetic anchor is wrong. Confidence: HIGH — Verification: `ls -la` = 35 358 bytes.

F-003 [S4]
Where: crates/streaming/tests/ranged_wav_e2e.rs:91
Code:
```
    let fetch: Arc<dyn HttpFetch> = Arc::new(server.client());
```
What is wrong: both e2e lanes ride MockClient, the test-owned transport, not the production UreqClient — so this proof covers RangedResource + demuxer + a real socket + a real HTTP/1.1 wire, but NOT the production HTTP stack (redirect/policy/body-read code paths of client.rs). The header's "real network media" claim is true at the socket/wire level; the transport-substitution caveat is not stated in this file. Cross-ref: mockhttp audit F-001 (MockClient ignores RedirectPolicy::cross_host; UreqClient's unit tests are the only real-transport coverage). Confidence: HIGH — Verification: read + rg (0 UreqClient uses in tests/).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FIXTURE / FFMPEG_MD5 constants | 37-43 | REAL | — | MD5 independently reproduced (python3) |
| drain | 45-57 | REAL | — | concatenates packet payloads; unreachable! on error (test-side OK) |
| wav_data_chunk | 59-82 | REAL | — | test-side RIFF walk with word-align (`size & 1`), bounds-checked |
| wav_over_http_demuxes_to_the_on_disk_truth | 86-149 | REAL | S3/S4 | three-way truth; ≥30 all-ranged requests; no upper bound (F-001); MockClient (F-003) |
| wav_over_no_range_http_demuxes_to_the_on_disk_truth | 154-218 | REAL | — | RequireRange vs FallbackToScan lanes; probe bytes=0-1023 pinned; scan fetches rangeless |
| md5_hex | 227-288 | REAL | — | test-side RFC 1321; K computed from sin (f64) but whole function pinned by vectors |
| md5_matches_rfc1321_vectors | 292-315 | REAL | — | all 7 A.5 vectors |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| wav_over_http_demuxes_to_the_on_disk_truth | YES (correctness) / NO (refetch perf) | byte-exact payload + MD5 + file-path equality catch any byte or seek bug; request-shape only lower-bounded (F-001) |
| wav_over_no_range_http_demuxes_to_the_on_disk_truth | YES | same three-way truth; probe/scan header shapes pinned exactly |
| md5_matches_rfc1321_vectors | YES | 7 published vectors — a wrong test-side MD5 cannot survive |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "byte-exact against the file on disk — ... strictly stronger than a digest comparison" | ranged_wav_e2e.rs:12-14 | REAL | assert_eq!(net_payload, wav_data_chunk(&bytes)) L112-116 |
| "MD5 against the recorded ffmpeg truth" | ranged_wav_e2e.rs:15-18 | REAL | python3 hashlib reproduces `dcaccb18…c7c5` from the committed fixture |
| "byte-exact against the same fixture demuxed through the input crate's FileAccess" | ranged_wav_e2e.rs:19-21 | REAL | L121-129 |
| "1 KiB over a ~46 KiB file" | ranged_wav_e2e.rs:23 | FALSE (size) | file is 35 358 bytes (F-002) |
| "the request log proves it" / "no re-fetch storm" | ranged_wav_e2e.rs:26, 132 | PARTIAL | only the ≥30/ranged-shape half is proven (F-001) |

## 7. Dead / duplicated / unreachable code
None. The MD5 implementation duplicates the demux fixture suite's test-side MD5 ("the same shape the demux fixture suite uses", L221-223) — deliberate duplication to keep test binaries independent; acceptable.

## 8. Dependencies & flags
demux, plugin_api, streaming (fetch/mockhttp/ranged), input, media_types, testkit — all workspace-internal. No features. No unsafe.

## 9. Open questions for the next agent
- ranged.rs's window cache is the perf-critical half this e2e under-bounds (F-001); its unit tests may pin exact request counts — the ranged.rs auditor should confirm.
- Would the same three-way check pass over UreqClient against the mock server (plain HTTP)? A one-line variant of this e2e with `streaming::client::UreqClient` would close the transport-substitution gap (F-003).
