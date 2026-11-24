# W20-A02 — Independent Re-Verification (§8.1) of 10 S0 Findings

| Field | Value |
|---|---|
| Auditor | W20-A02 (independent re-derivation, no original-audit files read) |
| Repo root | /home/z/my-project/audit-workspace (read-only; writes only under audit/) |
| Binary used | target/release/lumen (release lane, exit 0 opens) |
| Method | source trace + execution (binary runs, targeted cargo tests, crafted fixtures) |
| Verdicts | CONFIRMED: 9 · REFUTED: 1 (finding 9) |

---

## Finding 1 — MP4 in-session seek always fails (engine calls reanchor; Mp4Demuxer never implements it; MediaInfo says seekable:true)

**Derivation.** The engine's seek flow (crates/playback-engine/src/seek.rs:369-412 `pipeline_seek`) performs step 3 (flush) then step 4: `pipeline.demuxer_reanchor(target_span(target_ns))` → crates/playback-engine/src/pipeline.rs:1498-1499 `self.demuxer.reanchor(target)`. The in-session seek path NEVER calls `Demuxer::seek` (the pure query) — verified by rg: the only `.seek` use in playback-engine is `seekability()` at pipeline.rs:1190. The `Demuxer` trait (crates/plugin-api/src/demux.rs:113-120) provides a default `reanchor` that always returns the typed refusal:

```rust
    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        let _ = target;
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("plugin.demux.reanchor.unsupported"),
        ))
    }
```

`impl Demuxer for Mp4Demuxer` (crates/demux/src/mp4/mod.rs:2008-2084) implements exactly `start`, `next_event`, `seekability`, `seek` — NO `reanchor`. `seekability()` (mp4/mod.rs:2039-2048) returns `SeekSupport::Fast { indexed: true }` for progressive MP4, which `Session::emit_open_facts` (crates/playback-engine/src/session.rs:2781-2784) translates into `seekable: true` in MediaInfoReady — so the demuxer *claims* indexed seekability, the engine *reports* seekable:true, and every seek then dies at the unimplemented acting half. Other demuxers do implement `fn reanchor` (avi mod.rs:971, wav mod.rs:1095, ogg mod.rs:1361, mkv mod.rs:1026, adts mod.rs:1235, aiff mod.rs:699, ivf mod.rs:677, ps mod.rs:1143) — MP4 alone does not. Mp4Demuxer::seek's ~300-line pure-query implementation (mp4/mod.rs:2050-2152+) is dead code on the engine path.

**Execution** (`printf 'seek 1s\n' | timeout 60 ./target/release/lumen open /home/z/my-project/samples/sample.mp4`):

```
event: MediaInfoReady { info: MediaInfo { … duration: Known { time: Timestamp(3000000) }, seekable: true, … codec: … "h264" … "aac" } }
event: state-changed Buffering → Playing
event: TimingSeekFailed { error: CommandError { stage: Demux, code: Unsupported, message: "[demux] unsupported (plugin.demux.reanchor.unsupported)", details: {"position_us": "0", "reason": "demux"} } }
```

Same on sample-faststart.mp4 (seekable: true + TimingSeekFailed identical). Control contrast, same binary, same command on sample.wav: `event: TimingSeekCompleted { position: Timestamp(1000000), seek_kind: Command }` — WavDemuxer implements reanchor, so the failure is MP4-specific, not a host/env artifact.

**Verdict: CONFIRMED (S0).** Evidence: executed outputs above; quotes plugin-api/src/demux.rs:113-120, mp4/mod.rs:2008-2084 (impl block without reanchor), seek.rs:388-410, pipeline.rs:1498-1499, session.rs:2779-2792.

---

## Finding 2 — apps/cli/src/host.rs media_info() reports seekable:true regardless of engine

**Derivation.** `pub(crate) fn media_info()` (apps/cli/src/host.rs:1283-1312) synthesizes the `media.get-info` snapshot from `facts` (duration, container, tracks) and hardcodes two engine-truth fields:

```rust
                seekable: true,
                is_live: false,
```
(host.rs:1307-1308)

Nothing in the function reads the engine's `MediaInfoReady` payload (which carries the engine-computed `seekable`); rg shows MediaInfoReady is consumed nowhere in apps/cli (only host.rs:1283 `media_info` + dispatch.rs:133 `Command::MediaGetInfo => … host.media_info()?`).

**Execution.** Crafted a minimal FLAC-in-WAV fixture (WAVEFORMATEXTENSIBLE + KSDATAFORMAT_SUBTYPE_FLAC GUID + 34-byte STREAMINFO — the form crates/demux/src/wav/format.rs:62-82 parses; `WavCodec::Flac` → `samples_per_unit()` = None (format.rs:233) → `WavDemuxer::seekability()` returns `SeekSupport::None` (wav/mod.rs:1081-1089)). Generator: audit/verify/make_flac_wav.py → audit/verify/fixtures/flac-in-wav.wav. Run `printf 'info\nseek 1s\n' | ./target/release/lumen open audit/verify/fixtures/flac-in-wav.wav`:

```
event: MediaInfoReady { info: MediaInfo { … seekable: false, … codec: … Some("flac") } }   ← engine truth: NOT seekable
ok: media
  container: wav
  seekable: yes                                                                            ← CLI answer: hardcoded lie
event: TimingSeekFailed { error: CommandError { stage: Control, code: Unsupported, message: "the source cannot seek", details: {"position_us": "142", "reason": "not-seekable"} } }
```

The engine says `seekable: false`; the CLI `info` command answers `seekable: yes` in the same session. Plain `info` on sample.wav also prints `seekable: yes` (there the engine happens to agree — the divergence only becomes observable on a genuinely unseekable source, which the hardcoded constant can never represent).

**Verdict: CONFIRMED (S0).** Evidence: host.rs:1283-1312 (quote above), executed FLAC-in-WAV divergence output.

---

## Finding 3 — engine-info / decode-bench understate the build's decoders (mjpeg+pcm only) while the same binary decodes h264+aac

**Derivation.** `pub fn engine_info()` (apps/cli/src/host.rs:1629-1661) hardcodes the decoder list:

```rust
        decoders: vec![
            DecoderInfo {
                name: "mjpeg".to_string(),
                …
            DecoderInfo {
                name: "pcm_s16le".to_string(),
                …
```
(host.rs:1643-1658)

Two entries, hand-written — while the same binary's real seam, `HostDecoderProvider::decoder_for` (apps/cli/src/pipeline_host.rs:235-381), dispatches TWELVE families: `"mjpeg"` (242), `"flac"`, `"mp3"|"mp1"`, `"mp2"`, `"vorbis"`, `"opus"`, `"aac"|"aac-lc"` (288-292), `"alac"`, `adpcm_*`, `"av1"|"av01"`, `"vp9"`, `"dnxhd"`, `"h264"|"avc1"` (346) — and its own refusal text admits it:

```rust
            other => Err(host_error(
                …
                    "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, \
                     mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"
```
(pipeline_host.rs:372-378)

bench.rs hardcodes the same understatement: only `fact.codec == "mjpeg"` (bench.rs:300) and `pcm_*` (bench.rs:312-313) get limbs; everything else gets the literal at bench.rs:334 `"no decoder in this build (first vertical: mjpeg + pcm)"`.

**Execution.**
- `./target/release/lumen engine-info` → `decoders: mjpeg, pcm_s16le` (2 entries; exit 0).
- `./target/release/lumen decode-bench /home/z/my-project/samples/sample.mp4` →
  `bench: video track#0 h264 — not benched (no decoder in this build (first vertical: mjpeg + pcm)); audio track#1 aac — not benched (…) (container mp4)` / `BENCH … v_frames=0 v_decoded=0 a_frames=0 a_decoded=0 …` — zero frames benched in a binary that carries the h264+aac decoders.
- `./target/release/lumen open /home/z/my-project/samples/sample-hevc.mp4` → `error: [decode.unsupported]: unsupported (cli.codec_unavailable): no decoder for 'hevc' in this build (landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)` (exit 65) — the same binary listing 12 landed decoders in its refusal while engine-info advertises 2.
- `./target/release/lumen open /home/z/my-project/samples/sample.mp4 </dev/null` → full walk `Idle→Opening→Buffering(MediaInfoReady seekable:true, h264+aac)→Playing→Stopped→Idle`, exit 0 — the open path selects those tracks; a `seek 1s` run on the same file additionally emitted `event: error [decode.corrupt] the engine recovered 26 damaged unit(s) while resynchronizing` (RecoveredDamage{stage: decode}, event.rs:573-581) — decode-stage activity on the h264 lane.
- Binary contents: `strings target/release/lumen | rg rust_h264` shows the full rust_h264-0.6.1 decoder (cabac.rs, cavlc.rs, deblock.rs, decode_cabac.rs, dpb.rs, inter_pred.rs, intra_pred.rs, sps.rs, …) embedded in the shipped release binary.

**Verdict: CONFIRMED (S0).** The engine-info snapshot and the bench coverage message are hardcoded lies about the build: the binary embeds and dispatches h264+aac (12 decoder families) while both surfaces claim only mjpeg+pcm / "no decoder in this build". (Note: "open … to Playing" does not print per-frame counts anywhere in the CLI — decode of the h264/aac lanes is proven by the provider dispatch + embedded rust_h264 + the decode-stage recovery event; v_frames=0 belongs to decode-bench's own refusal to construct the limbs.)

---

## Finding 4 — apps/desktop-tauri/src/app.rs:301 spawn_session_pump undefined (E0425 compile break)

**Derivation.** app.rs setup closure:

```rust
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
            Ok(())
```
(app.rs:300-303)

Repo-wide rg for `spawn_session_pump` (all `*.rs`): exactly ONE hit — the call at apps/desktop-tauri/src/app.rs:301. No definition exists in the crate (sources: app.rs, frame_channel.rs, lib.rs `pub mod app; pub mod frame_channel;`, main.rs `mod win32`), nor in crates/ (`fn spawn_session_pump`: no matches). The imports at app.rs:84-94 (std, tauri, commands, desktop) cannot supply it. The intended symbol is the sibling defined at app.rs:190:

```rust
fn spawn_event_pump(app: AppHandle) {
```
(app.rs:190 — the shell's ticker thread, per the module doc at app.rs:42)

A call to an undefined name is rustc E0425 (`cannot find function`): `lumen-desktop-tauri` cannot compile as committed.

**Execution.** UNVERIFIED by compilation — the sandbox cannot build the crate (per AGENT-SPEC §13, desktop-tauri fails at gdk-sys before the crate's own name-check phase; environment limit, not a repo defense). Static proof is decisive on its own: identifier used, never defined anywhere in the repository.

**Verdict: CONFIRMED (S0, static; E0425).**

---

## Finding 5 — crates/subtitles CEA-608: PAC detection keyed on first byte 0x40-0x5F instead of 0x10-0x1F

**Derivation.** The pair dispatcher (crates/subtitles/src/cc/eia608.rs:381-397):

```rust
    pub(crate) fn decode_pair(&mut self, hi: u8, lo: u8) -> bool {
        self.flags = PairFlags::default();
        if (0x10..=0x1F).contains(&hi) {
            return self.control_code(hi & 0x07, lo);
        }
        if (0x40..=0x5F).contains(&hi) && lo >= 0x40 {
            return self.pac(hi, lo);
        }
```
(eia608.rs:383-388; identical wrong condition in the channel latch at crates/subtitles/src/cc/mod.rs:341)

Per CEA-608, control codes (PAC included) have their FIRST byte in 0x10-0x1F and their SECOND byte in 0x40-0x7F; first bytes 0x20-0x7F are TEXT. The implementation applies the PAC's second-byte range (0x40-0x5F) to the FIRST byte. The code's own family table proves the internal inconsistency: mid-row `0x11/0x19 + 0x20..=0x2F` (eia608.rs:548), misc `0x14/0x15/0x1C/0x1D`, tab `0x17/0x1F + 0x21..=0x23` (eia608.rs:587) — all correctly keyed on first byte 0x10-0x1F; only PAC is keyed on 0x40-0x5F.

Consequences: (a) a REAL PAC (e.g. 0x11 0x44) is routed to `control_code(1, 0x44)` → `mid_row(0x44)` → falls to `_ => false` (eia608.rs:566) — silently dropped, no row/pen positioning ever happens from real PACs; (b) plain text pairs whose first char is 0x40-0x5F and second ≥ 0x40 are swallowed as fake PACs.

**Byte-level derivation, "HELLO WORLD!"** (parity-stripped bytes; CEA-608 feeds 2-byte pairs; pen/cursor initial row 15):

| pair | bytes | dispatch | emitted text |
|---|---|---|---|
| (H,E) | 0x48,0x45 | hi∈0x40-0x5F, lo≥0x40 → pac(0x48,0x45) | — (swallowed; cursor row 11) |
| (L,L) | 0x4C,0x4C | pac(0x4C,0x4C) | — (swallowed; row 14) |
| (O,␣) | 0x4F,0x20 | lo<0x40 → text | "O␣" |
| (W,O) | 0x57,0x4F | pac(0x57,0x4F) | — (swallowed; row 10) |
| (R,L) | 0x52,0x4C | pac(0x52,0x4C) | — (swallowed; row 3) |
| (D,!) | 0x44,0x21 | lo<0x40 → text | "D!" |

Rendered result: two stray fragments — "O␣" on row 15 and "D!" on row 3; 8 of 12 characters are eaten by fake PACs. (The original finding's "-> 'D!'" is the same swallow mechanism with a slightly different pair alignment/reading; my from-scratch derivation yields "O␣D!" as the full grid text.)

**Corroboration from the repo's own history** (docs/worklog/archive-2026-10.md:9953, verbatim):

> acceptance: "AB"/"HI"/"OK" text pairs were PACs (second byte ≥ 0x40 → re-encoded as "A1"/"H1"/"ok" with comments)

The author observed real text pairs being consumed as PACs and re-encoded the test vectors instead of fixing the dispatcher. Today's tests confirm the blindness: eia608 tests feed only pairs like (0x31,0x32), (0x41,0x31) "A1", (0x48,0x31) "H1" — uppercase letter followed by a <0x40 character — never uppercase-followed-by-uppercase; `pac_row_table_matches_reference_layout` calls `ch.pac(0x44,…)` DIRECTLY (eia608.rs:649-666), bypassing the broken dispatch. Executed: `./target/debug/deps/subtitles-85f02b91c21cc80b eia608::tests::` → `test result: ok. 8 passed; 0 failed` — the suite passes because it never decodes uppercase text.

**Verdict: CONFIRMED (defect real; S0 as stated, with the reachability caveat that `crates/subtitles` has no workspace dependents — same orphan status as finding 8 — so no shipped entry point reaches the decoder today).**

---

## Finding 6 — scripts/quality/23_alloc_budget.sh: python sys.exit(1) swallowed by trailing `say`s

**Derivation.** The gate's failure signal (script tail, verbatim):

```bash
    sys.exit(1)
PYEOF

say ""
say "written: $OUT/alloc/alloc-budget.txt"
```
(lines 131-135; `say` is `printf` → always 0)

The script runs `set -uo pipefail` (line 14) — NO `-e` — so the python heredoc's exit 1 does not stop the script, and the script's exit code is the last command's: `say` → 0. The gate structurally cannot fail.

**Execution** (ran the script with python3 present; to protect the repo's docs/audit/reports tree I ran a byte-identical copy under audit/verify/quality/ with only ROOT/OUT redirected):

1. Normal threshold: `bash …/23_alloc_budget.sh 8` → `**worst excess: 0.65x** over 8 files; **0 over threshold**` → `SCRIPT_EXIT=0` (honest-looking pass).
2. Forced failure `ALLOC_RATIO_MAX=0.01 bash …/23_alloc_budget.sh 8`:

```
**worst excess: 0.65x** over 8 files; **7 over threshold**.
OVER THRESHOLD:
  0.65x  tests/fixtures/generated/valid/aiff_pcm_s16be_44k1_st.aiff
  … (7 files)
written: …/alloc-budget.txt
SCRIPT_EXIT=0
```

The gate detected 7 violations, python called `sys.exit(1)`, and the script still exited **0** — the failure is swallowed by the trailing says. Any caller (CI, full_gate) sees success.

**Verdict: CONFIRMED (S0 — quality gate that cannot fail).**

---

## Finding 7 — platform-windows dxgi.rs vtables omit the 4 inherited IDXGIObject slots; get_desc1 actually lands on EnumOutputs

**Derivation.** Real dxgi.h MIDL layout: `IDXGIFactory1` = IUnknown(3) + IDXGIObject{SetPrivateData, SetPrivateDataInterface, GetPrivateData, GetParent}(4) + IDXGIFactory{EnumAdapters, MakeWindowAssociation, GetWindowAssociation, CreateSwapChain, CreateSoftwareAdapter}(5) + IDXGIFactory1{EnumAdapters1, IsCurrent}(2) = 14 slots. `IDXGIAdapter1` = IUnknown(3) + IDXGIObject(4) + IDXGIAdapter{EnumOutputs, GetDesc, CheckInterfaceSupport}(3) + IDXGIAdapter1{GetDesc1}(1) = 11 slots.

The declared vtables (crates/platform-windows/src/windows/dxgi.rs:48-77 and 91-106):

```rust
struct Factory1Vtbl {
    pub unknown: crate::windows::com::UnknownVtbl,
    pub _create_swap_chain: …
    pub _get_adapter: …
    pub _enum_adapters: …
    pub _make_window_association: …
    pub _get_window_association: …
    pub _create_software_adapter: …
    pub enum_adapters1: …
    pub _is_current: …
}
```
(3 + 8 = 11 slots; none of the 4 IDXGIObject slots present; plus an invented `GetAdapter` slot that does not exist on IDXGIFactory at all)

```rust
struct Adapter1Vtbl {
    pub unknown: crate::windows::com::UnknownVtbl,
    pub _get_parent: …
    pub _enum_outputs: …
    pub _get_desc: …
    pub _check_interface_support: …
    pub get_desc1: …
}
```
(3 + 5 = 8 slots vs the real 11)

Overlaying the declared structs on the real vtable (fields start at slot 4 = SetPrivateData):

Factory1Vtbl: `enum_adapters1` is the 10th slot = real slot 10 = **IDXGIFactory::GetWindowAssociation** — the call at dxgi.rs:198 `((*factory_vtbl).enum_adapters1)(factory, index, &mut adapter)` actually invokes GetWindowAssociation, a one-out-pointer method, with the enumeration index as its argument — writes to address 0/1/2… or answers S_OK with `adapter` untouched → empty list or crash.

Adapter1Vtbl: `get_desc1` is the 8th slot = real slot 8 = **IDXGIAdapter::EnumOutputs** — the call at dxgi.rs:238 `((*adapter_vtbl).get_desc1)(adapter, &mut desc)` actually invokes EnumOutputs(Output: UINT, ppOutput: **), passing `&mut desc` (a stack pointer truncated to a UINT as the output index) and leaving ppOutput to be read from an uninitialized register — memory corruption. Exactly the stated claim: **get_desc1 actually calls EnumOutputs**.

**Execution.** Static arithmetic only — the crate is `cfg(target_os = "windows")` and cannot compile/run in this sandbox (and per finding 8 it is an orphan no shipped binary links anyway). The slot arithmetic above is the proof.

**Verdict: CONFIRMED (S0-class COM-binding corruption; static, Windows-target).**

---

## Finding 8 — platform-windows / platform-linux / platform-macos are orphan crates (zero workspace dependents)

**Derivation.** Workspace root (Cargo.toml:41):

```toml
members = ["crates/*", "apps/*", "tools/xtask", "tests/integration/first-vertical", "tools/mem-audit", "tools/sweep"]
```

The three platform crates are members via the `crates/*` glob. Repo-wide Cargo.toml grep for `platform-(windows|linux|macos)` matches ONLY the three crates' own manifests. Cargo.lock confirms: each appears exactly once, as a package whose dependencies run OUTWARD (audio-output, media-types, testkit) with no reverse edges:

```
[[package]]
name = "platform-linux"
dependencies = [ "audio-output", "media-types", "testkit" ]
[[package]]
name = "platform-macos"
dependencies = [ "audio-output", "media-types", "testkit" ]
[[package]]
name = "platform-windows"
dependencies = [ "audio-output", "media-types", "testkit" ]
```

A Cargo.lock reverse-dependency scan (python over all `[[package]]` blocks) found ZERO packages depending on any of the three. apps/cli/Cargo.toml, apps/desktop/Cargo.toml, apps/web/Cargo.toml contain no platform dependency. Code-level: `platform_windows|platform_linux|platform_macos` appear only in the three crates' own test files. No shipped entry point (lumen CLI, desktop, web) can reach any of them.

**Verdict: CONFIRMED (orphan crates — compiled into every workspace lane, imported by nothing).**

---

## Finding 9 — apps/web/src/io/mod.rs evidence label cites a fetch-lane playback proof fixture that does not exist

**Derivation.** The label (apps/web/src/io/mod.rs:56-63):

```rust
//! # Evidence labels
//!
//! Everything here is proven headless-native against the mock double
//! (the seekability matrix, the bounded-memory proofs, and the
//! fetch-lane playback proof over the REAL first-vertical fixture).
```

Checking both halves of the claim:
- The cited proof EXISTS: `fetch_lane_serves_the_real_fixture_to_the_demuxer` (apps/web/src/io/staged_source.rs:1159-1218), whose own doc header is "THE FETCH-LANE PLAYBACK PROOF over the REAL first-vertical fixture", reads `testkit::fixture_bytes("generated/valid/avi_mjpeg_pcm.avi")` (staged_source.rs:1161), serves it through the accept-ranges mock, probes through the real `media_probe` registry, and walks `AviDemuxer` to end-of-media through staged windows.
- The cited fixture EXISTS: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi — 425,516 bytes on disk, declared in tests/fixtures/generated/MANIFEST.toml (`path = "generated/valid/avi_mjpeg_pcm.avi"`, sha256 1f56202d…, size 425_516).

**Execution.** `PATH=$HOME/.cargo/bin:$PATH cargo test -p lumen-web fetch_lane_serves_the_real_fixture_to_the_demuxer` (compiled 17.48s):

```
running 1 test
test io::staged_source::tests::fetch_lane_serves_the_real_fixture_to_the_demuxer ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out
```

The fixture resolves (testkit::fixture_bytes is existence-checked, testkit/src/lib.rs:235-248), the test runs and passes. The claim "cites a fixture that does not exist" is disproven on both the fixture and the proof.

Related genuine defects I did verify while chasing this: the repo's own gate `cargo xtask feature-audit` reports 2 missing evidence paths — `crates/codecs/src/video/h264/sw` (row 23) and `ui/src/screens/settings/areas/advanced.tsx` (row 163) — neither of which is the io/mod.rs label. The only legitimate quibble with the label is that "playback proof" overstates a demux-to-end proof (no decode/render) — but the test's own header uses that exact phrase, so the label faithfully describes the test.

**Verdict: REFUTED.** The cited fixture and the cited proof both exist; the proof was executed and passed.

---

## Finding 10 — feature-matrix rows 095/099/100/101 say not_started vs landed, tested code (http client, reconnect, auth, remote playlists)

**Derivation.** Matrix rows (docs/feature-matrix.toml; row numbers are 1-based [[feature]] ordinals — 95=network-http-client, 99=network-reconnect-buffering, 100=network-auth, 101=network-remote-playlists), verbatim status/evidence:

```toml
id = "network-http-client"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
notes = "6.A1: ureq or reqwest per ADR-011; all network responses are untrusted input (MP §12); mock-server tests (redirect loops, timeouts, 416 range)."

id = "network-reconnect-buffering"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
notes = "6.D1/6.D2 [PROPOSAL parameters]; buffering budget per product.toml limits."

id = "network-auth"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
notes = "6.E1; keychain policy + crate per OS [VERIFY]; no secrets in logs/errors (media-types policy doc 01 §2.5)."

id = "network-remote-playlists"
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
notes = "6.E2; HTML listing parse = [PROPOSAL decision]; icecast/shoutcast metadata out of v1."
```

The code, landed and tested (crates/streaming):
- **HTTP client**: client.rs (998 lines — the ureq 3.5.0 adapter, max_redirects(0) + manual bounded redirect following), schemes.rs (1137), fetch.rs (730, the HttpFetch port), ranged.rs (1886, Range/seekability probes). Tests executed: see below.
- **Reconnect**: reconnect.rs (1560 lines — RetryPolicy/BackoffSchedule/Jitter, RetryFetch generation-safe cancellation, resume-from-offset). Tests: tests/net_interrupted.rs — executed: `manifest_reset_fails_typed_with_bounded_backoff … ok`, `live_window_edge_survives_wire_delay … ok`, `mid_segment_drop_resumes_byte_exact … ok` (3 passed).
- **Auth**: auth.rs (1562 lines — HTTP Basic (RFC 7617) + Digest MD5 (RFC 7616) challenge-first, credential vocabulary, redaction). Tests executed: `--test auth_e2e` → 8 passed (`unchallenged_server_never_receives_credentials`, `digest_wrong_password_is_typed_auth_failure`, `url_userinfo_is_accepted_stripped_and_used`, …).
- **Remote playlists**: remote_playlist.rs (500 lines — M3U/XSPF fetch, format sniff, RFC 3986 §5 relative-URI resolution against the final URL, bounded redirect loops). Tests executed in the lib run: `remote_playlist::tests::m3u_resolves_locations_against_the_playlist_url … ok`, `https_playlist_redirect_to_https_target_follows … ok`, `redirect_loop_is_typed … ok`, ….

Aggregate executed: `cargo test -p streaming --lib` → **212 passed; 0 failed; 2 ignored** (0.43s). Module LOC for the four rows: 998+730+1560+1562+500 ≈ 5,350 (4,620 without fetch.rs) — the "~4,900 LOC" scale is correct. The matrix rows say not_started on all four platforms with empty evidence while this code ships in the workspace and its tests pass — the same pattern the orchestrator baseline records for the whole matrix (parity-report 0/252 verified; feature-audit FAILURE).

**Verdict: CONFIRMED (S0 — status fields contradict landed, tested, executed code).**

---

## Summary table

| # | Finding | Verdict | Key evidence (all re-derived, none copied) |
|---|---|---|---|
| 1 | MP4 in-session seek always fails | CONFIRMED (S0) | Executed: TimingSeekFailed `plugin.demux.reanchor.unsupported` + MediaInfo `seekable: true` on both MP4 samples; WAV contrast completes; Mp4Demuxer impl block (mp4/mod.rs:2008-2084) has no reanchor |
| 2 | host.rs seekable:true regardless of engine | CONFIRMED (S0) | host.rs:1307 hardcoded; executed FLAC-in-WAV: engine `seekable: false` vs CLI `seekable: yes` in one session |
| 3 | engine-info/bench understate decoders | CONFIRMED (S0) | Executed engine-info (2 decoders), decode-bench (v_frames=0, "no decoder in this build"), HEVC refusal listing 12 landed decoders, strings showing rust_h264-0.6.1 embedded; provider dispatch at pipeline_host.rs:242-346 |
| 4 | spawn_session_pump undefined (E0425) | CONFIRMED (S0, static) | Repo-wide: single hit = the call (app.rs:301); intended fn `spawn_event_pump` at app.rs:190; compile run blocked by gdk-sys sandbox limit |
| 5 | 608 PAC keyed on 0x40-0x5F not 0x10-0x1F | CONFIRMED | eia608.rs:383-388 dispatch; byte derivation "HELLO WORLD!" → "O␣D!" (8/12 chars eaten by fake PACs); worklog:9953 admits "AB"/"HI"/"OK" text pairs were PACs and tests were re-encoded around it |
| 6 | alloc-budget sys.exit(1) swallowed | CONFIRMED (S0) | Executed: ALLOC_RATIO_MAX=0.01 → "OVER THRESHOLD" + 7 violations printed, SCRIPT_EXIT=0 |
| 7 | DXGI vtables omit 4 IDXGIObject slots | CONFIRMED (static arithmetic) | Factory1Vtbl 11 slots vs real 14 (enum_adapters1→real GetWindowAssociation); Adapter1Vtbl 8 vs 11 (get_desc1→real EnumOutputs, call at dxgi.rs:238) |
| 8 | platform-* orphan crates | CONFIRMED | Cargo.toml members glob; Cargo.lock reverse-scan: zero dependents; no app manifest references them |
| 9 | io/mod.rs cites nonexistent fixture | REFUTED | Fixture avi_mjpeg_pcm.avi exists (425,516 B, MANIFEST-declared); proof test exists; EXECUTED: 1 passed |
| 10 | matrix rows 95/99/100/101 not_started vs landed code | CONFIRMED (S0) | Verbatim status/evidence (all not_started, evidence "") vs executed: 212 lib + 8 auth_e2e + 3 net_interrupted tests passing over client/reconnect/auth/remote_playlist (~5.3k LOC) |
