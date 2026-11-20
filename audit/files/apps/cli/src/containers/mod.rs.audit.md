# apps/cli/src/containers/mod.rs — W13-A01 audit

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A01 |
| Date | 2026-11-16 |
| Lines | 142 (2 functions + 1 test) |
| File verdict | **REAL** — a complete, total dispatch with an honest typed default arm; the file itself is clean, but the family module it dispatches mp4 to carries a FALSE capability doc (F-001, cross-file) |
| Reachable from shipped entry point | YES — `open` → run.rs:132 → dispatch.rs:81 → host.rs:725 → demux_host.rs:458 `crate::containers::open(&container, stream, &opts)` |
| Findings | F-001 [S2, cross-file], F-002 [S4] |

## 1 One-paragraph verdict

The registry is exactly what it claims: one match over the probed format id, fifteen wired arms, one honest typed default arm that names the format and never reads the stream. The dispatch is total over the probe's registered signature set (the in-file test's own comment counts all 15 ids and re-targets the default-arm pin to a hypothetical id — a deliberately honest test shape). The executed MP4 run rode arm `"mp4" => mp4::open(stream, opts)` (67) to a full decode. The rot is one file over: the mp4 family module's header still asserts that no software h264 decoder is dispatched in this build — the exact opposite of the executed behavior — which makes the registry's most-exercised family advertise itself as a refusal lane.

## 2 What actually works (verified)

- Arm 67 dispatched my live and the orchestrator's executed opens: `printf 'advance 512\ntime\nquit\n' | lumen open …/sample-h264-aac.mp4` → `tracks 2 ready` → `Playing → Ended` → `ok: time 3.041814s (100.0%)` → `EXIT=0` (audit/runs/baseline/mp4-smoke-open-advance.txt). The probe named `mp4` (demux_host.rs:448), this file's arm constructed the demuxer, and the harvest fed the decoder (see demux_host.rs audit §2 for the full chain).
- The default arm is reachable and typed: the in-file test (115-141) drives `open("mpegps-dv-extended", …)` → `[probe.unsupported]` with the format named in message AND details, exit class 65. The test's comment is explicit that the arm guards future registered ids — the honest framing.
- Fifteen real arms: adts, aiff, avi, ivf, matroska, flac, hevc (annexb ES), h264 (annexb ES), mp2, mp3, mp4, ogg, mpegps, mpegts, wav — matching the test comment's count "all 15 registered signature ids (`adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav`)" (105-107).
- Every family lane is exercised through the REAL binary by apps/cli/tests/container_conformance.rs (in the 1001-pass suite), including `mp4_h264_family_lane` (:426) on this dispatch.

## 3 Findings

### F-001 [S2 — cross-file, filed here because the registry is the dispatching surface] The mp4 family module's header claims the h264 blob is never used — the opposite of the shipped behavior
Where: apps/cli/src/containers/mp4.rs:27-32 (the module this file's arm 67 dispatches to)
```rust
 * **h264/hevc `extra_data` = the `avcC`/`hvcC` child payload**
 *   (the config record WITHOUT the box header — ffmpeg's extradata
 *   shape). No software h264/hevc decoder is dispatched in this
 *   build: the blob is the forward-honest config, and
 *   `decoder_for` refuses the codec typed (`cli.codec_unavailable`)
 *   — the hw-path cards own the seam.
```
Wrong: three sentences, two false. (1) "No software h264 … decoder is dispatched in this build" — pipeline_host.rs:328 dispatches exactly that: `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)`; the shipped default features compile it (crates/codecs/Cargo.toml:47-53 `codec-h264-sw` in `default`). (2) "`decoder_for` refuses the codec typed (`cli.codec_unavailable`)" — it does not, for h264; the executed binary decoded the h264 limb of this exact harvest to `Ended (100.0%)`, and conformance test container_conformance.rs:426 explicitly asserts the ABSENCE of `cli.codec_unavailable` on this lane. (The hevc half of the sentence is still true — no hevc decoder landed, the `hevc` blob IS forward-honest.) (3) "the hw-path cards own the seam" — the sw path is the shipped default per the feature-matrix row's notes ("software is the shipped-default path", docs/feature-matrix.toml codec-video-h264-software notes).
Correct: rewrite the bullet to the 2026-11-15 truth — the avcC blob configures `H264SwDecoder` end-to-end; only hevc remains a forward-honest refusal.
Confidence: high — the doc quote, the dispatch arm, the executed decode, and the conformance assertion all in evidence.
Severity S2: this is the lane the 2026-11-15 landing rests on, and its own family module documents it as absent. (The registry file itself is not at fault; the lie sits in the module the registry owns.)

### F-002 [S4] The wiring-order line in the module doc
Where: containers/mod.rs:14-15 — "Entries keep the §5 unit order: wav → aiff → adts → ogg → ivf → mp4 → mkv → ts → ps → es (`avi` is wired here as the moved first vertical)" — the match arms themselves are alphabetical, not in §5 unit order. Cosmetic doc drift only (the collision rule — one module + one arm — is honored; verified: 15 `mod` declarations at 22-32, one arm each plus the ES family's shared arms). S4.

## 4 Per-function table

| Function (line) | Role | Verdict |
|---|---|---|
| `open` (45-76) | THE format dispatch: 15 wired arms + the typed default refusal | REAL — executed (arm 67, MP4 smoke) |
| `unwired_refusal` (81-92) | default arm's typed `[probe.unsupported]` with the format named | REAL — pinned by the in-file test |

## 5 Test honesty

- In-file `unwired_format_is_refused_with_the_format_named` (115-141): honest and deliberately self-aware — it re-targeted from the real `mpegps` id to the hypothetical `mpegps-dv-extended` when the last real family landed, with a comment saying so. The test asserts the refusal shape (stage, code, message content, details row).
- Every wired arm has a REAL-binary lane in apps/cli/tests/container_conformance.rs (the 1001-pass suite): mp4 at :426 (`mp4_h264_family_lane`: 100% time, `Playing → Ended`, no `cli.codec_unavailable`, `--no-video`/`--no-audio`/`--start-time` variants), the other families per that file's header (avi, wav, aiff, adts, ogg×2, mkv/webm, mpegts, ivf, raw es).
- The ES family's five probe ids ride two module functions (`es::open_flac`, `open_annexb`, `open_mpeg_audio`) — covered by the es lanes in the same suite.

## 6 Doc claims

- docs/feature-matrix.toml:152 (container-mp4 notes): "2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane — mp4_h264_aac.mp4 plays to Ended (exit 0, apps/cli mp4_h264_family_lane)" — CONSISTENT with the executed run and this file's dispatch.
- docs/feature-matrix.toml:428: "engine e2e apps/cli mp4_h264_family_lane" — CONSISTENT.
- The module's own doc (1-20) — accurate about the shape (one module + one arm per wiring unit; default refusal kept forever). The mp4.rs bullet quoted in F-001 is the contradicting text, and it contradicts the feature-matrix row for the same capability — the tree's two documentation layers disagree, with feature-matrix.toml being the correct one.

## 7 Dead/duplicated/unreachable

None in this file. Every arm is reachable; the default arm guards future registered ids by design (and is tested against a hypothetical one).

## 8 Dependencies & flags

Depends only on `commands` (typed refusal), `plugin_api` (SourceStream), and `crate::demux_host` (OpenOptions/OpenedMedia). The family modules own the demux/codec posture; for the mp4 family the shipped default features (crates/codecs/Cargo.toml:47-53: `codec-h264-sw` in `default`) make the h264 arm of pipeline_host.rs live in this exact build — the fact F-001's doc text denies.

## 9 Open questions

- The `hevc` annexb arm opens a demux lane with NO decoder anywhere (hevc absent from pipeline_host.rs's arms) — open accepts, then the engine's refusal fires at wire time (`cli.codec_unavailable`). Is that the intended "probe-wired, decode-refused" shape (versus refusing at open)? The registry's design says yes (demux and decode are separate truths); recorded as intent, not defect. UNVERIFIED by execution (no hevc fixture in my runs).
