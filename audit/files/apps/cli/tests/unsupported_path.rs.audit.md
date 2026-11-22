AUDIT: apps/cli/tests/unsupported_path.rs
=========================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 308 / 20 (18 tests + 2 helpers) |
| File verdict | REAL (all error-path pins drive the real binary; one test name overpromises, two hygiene warts) |
| Reachable from shipped entry point? | NO — test-only; spawns the real `lumen` binary (line 18). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (single pass)
- `ls tests/fixtures/generated/malformed/avi_mjpeg_pcm__flip16.avi tests/fixtures/generated/valid/mkv_h264_aac.mkv tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4 crates/demux/fixtures/ps/ps-mp2-video-audio.mpg` (fixture existence — see below)
- Cross-read: parse.rs:215-225 (the bare-source fallback exercised by test 65-76), pipeline_host.rs:346 (the h264 arm the matroska test leans on).

## 1. One-paragraph verdict

The CLI's error-path evidence file, and it is real: 18 tests spawn the shipped binary and pin exact typed refusals — probe garbage (65 + `[probe.unsupported]`/`probe.no-demuxer`), inaccessible file/directory (66), wired-PS stub refusal by the demuxer (`demux.ps.bad-pack`), blocked rtsp scheme (69, scheme named), machine-mode unserved command (`unsupported-for-host` naming `card 3.A1`), grammar errors (64), and path redaction (`…/missing.avi`, full path asserted absent). The wired-family opens (PS/MKV/MP4/IVF) and the http loopback open are genuine end-to-end lanes. The one substantive criticism: `matroska_container_opens_and_decodes_h264` claims DECODE in name and comment but only proves the open accepted and no refusal hit stderr — a per-packet decode failure (surfacing as a stdout PlaybackError event) would still pass it. Two smaller warts: a pointless destructuring dance, and a double-run where one would do.

## 2. What actually works (verified)

— verified by reading + fixture existence: garbage/PS/missing/directory/rtsp/grammar/redaction pins assert exact codes, exact error-code substrings, and absence of success lines; `http_url_opens_over_a_real_loopback_socket` (229-254) serves a real WAV body over a real loopback socket with the production ureq transport and asserts the request log shows ranged requests — an honest open-lane proof (the "mock" is the server, not the client).
— fixtures on disk (verified): `avi_mjpeg_pcm__flip16.avi`, `mkv_h264_aac.mkv`, `mp4_mjpeg_pcm.mp4`, `ps-mp2-video-audio.mpg`, `wav_pcm_s16le_44k1_st.wav` all exist.
— UNVERIFIED at runtime (no cargo); assertions are read-verified.

## 3. Findings

### F-001 [S3] — "opens_and_decodes_h264" proves open+configure, not decode; a decode-broken h264 passes
Where: apps/cli/tests/unsupported_path.rs:136-160
Code:
```rust
/// The Matroska parent fixture (h264+aac): since the 2026-11-15
/// H.264 software-track landing the h264 limb DECODES (the mkv
/// host's `config_blob` passes the avcC `CodecPrivate` verbatim —
...
    assert_eq!(code, 0, "stdout:\n{out}\nstderr:\n{err}");
    assert!(out.contains("ok: accepted session=1"), "{out}");
    assert!(
        !err.contains("no decoder for `h264`"),
```
What is wrong: the assertions prove (a) exit 0, (b) the session was accepted, (c) no decoder-refusal on stderr. They prove the decoder was constructed and configured at wire time (a configure refusal fails the open with a nonzero exit). They do NOT prove any frame decoded: a `H264SwDecoder` that configured cleanly but failed on the first packet would emit a `PlaybackError` event on stdout and the session would degrade — none of that is asserted absent, and no `advance` drives media to `Ended` (contrast session_script's `scripted_session_runs_to_end`, which does).
Why it matters: the test's name and comment claim the decode behavior the file's own title ("error paths") does not need; if a regression broke h264 packet decode while leaving configure intact, this test stays green and the comment keeps asserting the lie.
What correct looks like: add `advance 512` + assert `state-changed Playing → Ended` (the session_script pattern), or assert zero `event: error` lines in the transcript.
Confidence: HIGH — Verification: read (control flow: decode errors are stdout events, not stderr, per run.rs:729-734 `EventKind::PlaybackError` printing).

### F-002 [S4] — dead-weight destructuring dance
Where: apps/cli/tests/unsupported_path.rs:70-73
Code:
```rust
    let (_out, err, code) = {
        let (_o, e, c) = (out, err, code);
        (String::new(), e, c)
    };
```
What is wrong: a three-line no-op that moves a tuple into itself to discard `out`; `let (_out, err, code) = (String::new(), err, code);` (or just `_out` naming) does the same.
Why it matters: obfuscation in an evidence file; a reader must prove the dance is a no-op.
Confidence: HIGH — Verification: read.

### F-003 [S4] — double binary run where one suffices
Where: apps/cli/tests/unsupported_path.rs:267-280
Code:
```rust
    let (_out, err, code) = run(
        &["--control-stdio"],
        "{\"requestId\": 1, \"cmd\": \"playback.set-rate\", \"args\": {\"rate\": 2.0}}\n",
    );
    assert_eq!(code, 0, "machine mode answers, then EOF:\n{err}");
    // (the response line itself carries the refusal)
    let (out, _err, _code) = run(
```
What is wrong: the same child process is spawned twice to check exit code and stdout separately; one run captures all three.
Why it matters: doubled runtime per assertion set; hygiene.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fixture` | 10-15 | REAL | — | manifest-dir relative. |
| `run` | 17-43 | REAL | — | BrokenPipe on fast-exit children handled and ASSERTED (29-35) — correct harness shape. |
| `garbage_file_is_a_typed_probe_refusal` | 47-60 | REAL | — | 65 + two codes + no `ok:` line. Temp file cleaned (52). |
| `magic_broken_derivative_is_a_typed_probe_refusal` | 64-76 | REAL | S4 | exercises the bare-source argv fallback (no `open` verb); F-002 dance. |
| `missing_file_is_inaccessible` | 79-84 | REAL | — | 66 + code. |
| `directory_source_is_inaccessible` | 87-92 | REAL | — | directory → 66. |
| `mpegps_stub_is_refused_typed_by_the_wired_demuxer` | 109-119 | REAL | — | re-targeted post-PS-wiring pin; documents why the registry default arm is now unreachable with real bytes (94-108) — honest history. |
| `mpegps_container_opens_and_accepts_the_session` | 125-134 | REAL | — | PS end-to-end open (`--no-video`, mp2 limb). |
| `matroska_container_opens_and_decodes_h264` | 141-160 | REAL | S3 | F-001 name/comment overpromise. |
| `mp4_container_opens_and_accepts_the_session` | 166-171 | REAL | — | mjpeg+PCM MP4 — NOTE: NOT the h264+aac litmus combo (see open questions). |
| `ivf_container_opens_and_accepts_the_session` | 177-182 | REAL | — | av1 IVF open. |
| `blocked_scheme_url_is_unsupported_for_host` | 188-198 | REAL | — | 69 + scheme named + "not implemented". |
| `file_url_opens_while_blocked_schemes_refuse` | 207-221 | REAL | — | `file:` URL canonical open + rtsp contrast; `SourceUri::from_file_path` used (real URL construction). |
| `http_url_opens_over_a_real_loopback_socket` | 229-254 | REAL | — | loopback + ranged-request log assert. |
| `play_without_session_is_not_ready` | 257-262 | REAL | — | 69. |
| `unserved_command_is_typed_with_the_owner` | 266-280 | REAL | S4 | F-003 double run; pins `card 3.A1` ownership naming. |
| `grammar_failures_are_usage_errors` | 284-298 | REAL | — | four 64-pins with exact messages. |
| `error_messages_redact_user_paths` | 300-308 | REAL | — | `…/missing.avi` present, `/home/someone` absent — V8 redaction pinned. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all except matroska decode claim | YES | exact exit codes + exact typed-error substrings + absence asserts against a real binary. |
| matroska_container_opens_and_decodes_h264 | PARTIAL | fails on open/configure regressions; PASSES on a first-packet decode regression (F-001). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the h264 limb DECODES … the session accepts on both limbs" | unsupported_path.rs:136-140 | SHALLOW | open+configure proven (exit 0, accepted, no refusal); frame decode not asserted (F-001). Cross-evidence: W00 baseline `advance`→Ended/100% on h264+aac MP4 (task-supplied fact) supports decode working, but THIS test does not prove it. |
| "the pre-landing exit-65 `cli.codec_unavailable` shape is closed" | unsupported_path.rs:139-140 | REAL-UNPROVEN | the `!err.contains` assert at 146-149 pins the absence; not run here. |
| "the production ureq transport (rustls TLS, bounded redirects) opens it" | unsupported_path.rs:225-228 | REAL-UNPROVEN (loopback, plaintext http) | the loopback test proves the http lane; the rustls/TLS half is NOT exercised by this test (no https server) — claim partially ahead of evidence. |

## 7. Dead / duplicated / unreachable code

- The F-002 dance (70-73) — dead weight, not dead code.
- No ignored tests; no duplication beyond the F-003 double-run.

## 8. Dependencies & flags

- Integration test; needs built binary + committed fixtures (all verified present) + `streaming::mockhttp` (real socket server) + `media_types::SourceUri`. No features. No corpus dependency — runs everywhere.

## 9. Open questions for the next agent

1. There is NO committed binary-level test opening an **h264+aac MP4** (the project's litmus combo): mp4 test here is mjpeg+pcm; h264 test here is MKV; the h264+aac MP4 evidence is only W00's manual smoke. Wave 20 A05 (MP4 trace) should close that gap or flag it.
2. The https/TLS lane: does ANY test exercise ureq+rustls over TLS, or is "rustls TLS, bounded redirects" (225-227) claim-ahead-of-evidence? (streaming crate's tests are another wave's files.)
3. `probe.no-demuxer` vs the 15-signature wired registry: the comment at 94-108 says every registered signature is wired — the containers/mod.rs registry is another agent's file to confirm.
