# AUDIT: apps/cli/tests/unsupported_path.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-24 |
| Lines / functions | 308 / 3 helpers + 16 #[test] |
| File verdict | REAL (strong binary-level error-surface suite; one falsified header claim, no hostile-TS lane) |
| Reachable from shipped entry point? | N/A (test file) — drives the REAL `lumen` binary via `CARGO_BIN_EXE_lumen` (run.rs argv lane, the shipped entry point) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `./target/debug/deps/unsupported_path-7b9ed041c8c263b5` → 16 passed, 0 failed (0.28 s; binary newer than source); fixture `ls` survey (all referenced fixtures present); sed verifications; hostile 188-byte TS probe executed separately this session (see F-001) |

## 1. One-paragraph verdict
This is a real, honest binary-level error-surface suite: all 16 tests spawn the actual `lumen`
binary, assert sysexits-class exit codes (64/65/66/69) plus the typed error text on stderr, and
the lane set is broad — probe refusals (garbage + a committed flip16 derivative), input
refusals (missing file, directory), wired-family stub refusals (`demux.ps.bad-pack`), four
end-to-end wired containers (PS/MP4/MKV-with-h264-decode/IVF-AV1), blocked/file/http URL lanes
(the http one over a REAL loopback socket asserting ranged requests hit the wire), not-ready,
unserved-catalog refusal naming the owning card, grammar failures, and path redaction. All 16
pass under my own run. The defects: the module header's "nothing panics" is falsified by an
input class the suite never probes — I executed the shipped binary on a 188-byte 0x47-led file
TODAY and it panics (exit 101, phase5.rs:161), and this suite has zero TS lanes to catch it;
plus a dead destructuring block, two fixed temp filenames, and no coverage of the scheme-drift
shapes (`c://x`, `1://x`) that parse.rs's third grammar misclassifies.

## 2. What actually works (verified)
- `./target/debug/deps/unsupported_path-7b9ed041c8c263b5` → "16 passed; 0 failed" — every test
  below executed green in 0.28 s.
- Fixtures verified present: generated/malformed/avi_mjpeg_pcm__flip16.avi,
  generated/valid/{avi_mjpeg_pcm.avi, mkv_h264_aac.mkv, mp4_mjpeg_pcm.mp4,
  wav_pcm_s16le_44k1_st.wav}, generated/ivf/av1_64x48_30fps.ivf,
  crates/demux/fixtures/ps/ps-mp2-video-audio.mpg.
- The strongest lanes are genuinely strong: `http_url_opens_over_a_real_loopback_socket` serves
  a WAV body from `streaming::mockhttp::MockServer`, opens through the production ureq
  transport, AND asserts a `Range` header appears in the server's request log (the ranged
  transport really rode the socket); `matroska_container_opens_and_decodes_h264` pins the
  post-landing h264 decode (asserts the old `no decoder for `h264`` refusal is GONE);
  `error_messages_redact_user_paths` asserts both the collapsed `…/missing.avi` form AND the
  absence of `/home/someone`.
- The `run()` harness (L17-43) is carefully shaped: piped stdio, BrokenPipe on early-exit
  opens asserted as the expected shape, signal deaths mapped to -1 (no harness panic).

## 3. Findings
F-001 [S3]
Where: apps/cli/tests/unsupported_path.rs:1-3 (claim); missing lane; counter-evidence
crates/media-probe/src/signature/phase5.rs:160-161 (executed this session)
Code:
```
//! The card 2.D2 error paths, through the REAL binary: every failure
//! is a stage-tagged typed error on stderr plus a sysexits-class exit
//! code (the honest-printing AC); nothing panics, nothing is silent.
```
What is wrong: the header claims the CLI's failure surface never panics; the suite is the
designated proof of that claim for the open path, yet it contains ZERO MPEG-TS lanes — and the
shipped binary PANICS on a 188-byte 0x47-led file: executed TODAY, `./target/debug/lumen open
<188-byte 0x47 file>` → "thread 'main' panicked at crates/media-probe/src/signature/
phase5.rs:161:16: index out of bounds: the len is 188 but the index is 188", exit 101 (controls
187 B → 65, 189 B → 65). The suite's garbage lane probes 384 bytes of 0x37-led noise — one
more lane with a 188-byte 0x47 file would have failed this suite for months (the Wave-4/5
finding is still unfixed). The panic itself is S1 and filed against the probe/ts entry chain
(ts.rs audit F-002); what THIS file owns is the falsified "nothing panics" claim and the
missing hostile lane. What correct looks like: add the 187/188/189-byte 0x47 triple as a lane
(expected: 65/65-typed-refusal-or-panic-fixed/65), and scope the header claim to "the tested
failure lanes" until the probe fix lands. Confidence: HIGH — Verification: ran (panic + two
controls executed this session; suite executed 16/16).
F-002 [S4]
Where: apps/cli/tests/unsupported_path.rs:65-76
Code:
```
    let (_out, err, code) = {
        let (_o, e, c) = (out, err, code);
        (String::new(), e, c)
    };
    assert_eq!(code, 65, "stderr:\n{_out}{err}");
```
What is wrong: the block is a no-op reshuffle — it moves `out/err/code` into `_o/e/c` and
returns `(String::new(), e, c)`, discarding the captured stdout for nothing; `_out` is always
the empty string it just created. Dead obfuscation (a copy-paste artifact), and the test's own
doc line "the same typed probe refusal, naming the sniffed bytes" is only HALF asserted: unlike
`garbage_file...` (which asserts `probe.no-demuxer`), this test checks only
`error: [probe.unsupported]` — the "sniffed bytes named" half is unpinned. Confidence: HIGH —
Verification: read.
F-003 [S4]
Where: apps/cli/tests/unsupported_path.rs:49-52, :112-115
Code:
```
    let garbage = std::env::temp_dir().join("lumen-cli-garbage.bin");
    std::fs::write(&garbage, [0x37u8, 0x37, 0x13, ...].repeat(64)).must();
```
What is wrong: both scratch files use FIXED names in the shared temp dir with no pid suffix
(the ts.rs `TempDir` convention embeds `std::process().id()`); concurrent suite instances race
on the same path, and a panic between write and `remove_file` leaks a file the next run
silently reuses. Test-only blast radius. Confidence: HIGH — Verification: read + cross-file
comparison.
F-004 [S4]
Where: apps/cli/tests/unsupported_path.rs:184-254 (the URL lanes tested) vs parse.rs:504-509
(the third scheme grammar)
What is wrong: the URL lane coverage is rtsp (blocked → 69), file (opens), http (opens) — the
exact three schemes whose classification is uncontroversial. The drift shapes my parse.rs audit
executed — `c://x` → unsupported-for-host "scheme: c" (the streaming registry's drive-letter
rule would say local path) and `1://x` → invalid-argument `uri.scheme.invalid` — have NO lane
here. The scheme-classification seam is therefore unguarded at every level: no unit test in
parse.rs, no integration lane here. Confidence: HIGH — Verification: ran (both drift shapes
executed against the binary this session) + read.
F-005 [S4]
Where: apps/cli/tests/unsupported_path.rs:267-280
Code:
```
    assert_eq!(code, 0, "machine mode answers, then EOF:\n{err}");
    // (the response line itself carries the refusal)
    let (out, _err, _code) = run(
```
What is wrong: the test spawns the binary TWICE to check one response (first run asserts exit
0, second run asserts the stdout line) — each `--control-stdio` run is a fresh process, so the
assertions are consistent, but the doubling is pure waste and the comment hints the author
found it awkward; a single run's `(out, _err, code)` would carry both facts. Hygiene only.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file < 100 production LOC; test inventory)
| Test | Lines | Verdict | Would it fail if code were broken? |
|---|---|---|---|
| run (harness) | 17-43 | REAL | n/a (harness; BrokenPipe + signal shapes handled) |
| garbage_file_is_a_typed_probe_refusal | 47-60 | REAL | YES — exit 65 + `[probe.unsupported]` + `probe.no-demuxer` + no ok line |
| magic_broken_derivative_is_a_typed_probe_refusal | 64-76 | REAL | PARTIAL — exit+stage only; sniffed-bytes half unpinned (F-002) |
| missing_file_is_inaccessible | 79-84 | REAL | YES — 66 + `[input.inaccessible]` |
| directory_source_is_inaccessible | 87-92 | REAL | YES |
| mpegps_stub_is_refused_typed_by_the_wired_demuxer | 109-119 | REAL | YES — `demux.ps.bad-pack` by name |
| mpegps_container_opens_and_accepts_the_session | 125-134 | REAL | YES — exit 0 + accepted session on the PS fixture |
| matroska_container_opens_and_decodes_h264 | 141-160 | REAL | YES — two runs; asserts the old h264 refusal text is GONE |
| mp4_container_opens_and_accepts_the_session | 166-171 | REAL | YES |
| ivf_container_opens_and_accepts_the_session | 177-182 | REAL | YES |
| blocked_scheme_url_is_unsupported_for_host | 188-198 | REAL | YES — 69 + scheme named + "not implemented" |
| file_url_opens_while_blocked_schemes_refuse | 207-221 | REAL | YES — canonicalize→URL opens, rtsp contrast |
| http_url_opens_over_a_real_loopback_socket | 229-254 | REAL | YES — socket + Range-header request-log assertion |
| play_without_session_is_not_ready | 257-262 | REAL | YES — 69 + `[session.not-ready]` |
| unserved_command_is_typed_with_the_owner | 266-280 | REAL | YES — `unsupported-for-host` + `card 3.A1` (double spawn, F-005) |
| grammar_failures_are_usage_errors | 284-298 | REAL | YES — 64 ×4 with token-named messages |
| error_messages_redact_user_paths | 302-308 | REAL | YES — collapsed form present, full path absent |

## 5. Test honesty
Covered in the per-function table above; headline: 15 of 16 lanes would fail on the specific
regression they name; the flip16 lane is half-pinned (F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every failure is a stage-tagged typed error ... nothing panics, nothing is silent" | unsupported_path.rs:1-3 | FALSE for an untested input class | 188-byte 0x47 file → panic exit 101, executed this session (F-001); no TS lane exists |
| "the pack start code probes mpegps, the registry dispatches ... the DEMUXER refuses the truncated pack header typed" | :100-103 | REAL | mpegps_stub test executed |
| "the mp2 limb decodes (the streaming AU-split: 39 frames from 5 PES)" | :122-124 | REAL-UNPROVEN | the test asserts only exit 0 + accepted session; the 39-frame count is asserted nowhere in this file (PS host's own suite's pin) |
| "since the 2026-11-15 H.264 software-track landing the h264 limb DECODES" | :136-140 | REAL | executed; asserts the old refusal is gone |
| "the production ureq transport (rustls TLS, bounded redirects) opens it" | :225-227 | REAL (loopback, plain http) | executed; TLS/bounded-redirect halves UNVERIFIED by this lane |

## 7. Dead / duplicated / unreachable code
- The no-op destructuring block (:70-73, F-002).
- The rtsp refusal is asserted twice (L190-198 and again inside file_url test L218-220) —
  harmless duplication.

## 8. Dependencies & flags
- Test-only: std::process, testkit::Must, streaming::mockhttp (dev-dep), media_types (URL
  construction). Uses `CARGO_BIN_EXE_lumen` (the real binary). No features/unsafe.

## 9. Open questions for the next agent
- Add the 187/188/189-byte 0x47 lane (F-001) — it converts the live S1 probe panic from
  "invisible to the suite" to "suite-failing" the day it lands.
- command_surface.rs owner: does that suite (my next file) carry the TS/hostile-input lanes
  this one lacks? If not, the shipped binary's panic surface has NO binary-level guard at all.
- parse.rs cross-ref: this suite DOES exercise the bare-source fallback (`run(&[<path>])` at
  :67, :81, :89, :114) — the in-file absence noted in parse.rs F-005 is covered here at the
  binary level; the is_url drift shapes remain uncovered everywhere (F-004).
