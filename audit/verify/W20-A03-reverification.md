# W20-A03 Re-verification Report (batch 3, independent)

| Field | Value |
|---|---|
| Auditor | W20-A03 (re-verification wave 20, batch 3) |
| Date | 2026-11-23 |
| Method | Re-derived from scratch: source reads + rg inventories + 5 executed proofs (2 rustc standalone, 3 targeted cargo tests). Original audit files under audit/ NOT read for the findings; the only unavoidable contact was incidental rg output lines, none used as evidence below. |
| Repo root | /home/z/my-project/audit-workspace |

## F-1 — AVI probe walk_hdrl/walk_strl wasm32-only slice panic
**Verdict: CONFIRMED** (static + executed arithmetic emulation; true wasm32 compile blocked — see blocker).

Derivation: `walk_hdrl`'s loop guard does a non-saturating `pos + 8` while the
advance chain saturates; on a 32-bit `usize` (wasm32) a lying chunk size pins
`pos` at `usize::MAX`, the guard's `pos + 8` wraps (release) or overflows
(debug), and `&prefix[pos..pos + 4]` panics.

Evidence — crates/media-probe/src/signature/avi.rs:104-107 (verbatim):
```
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let fourcc = &prefix[pos..pos + 4];
        let size = u32_le(prefix, pos + 4) as usize;
```
avi.rs:124-127 (verbatim, the saturating advance):
```
        pos = pos
            .saturating_add(size)
            .saturating_add(8)
            .saturating_add(size & 1);
```
Same pattern in `walk_strl` (avi.rs:140 guard, :156-159 advance). Attack
input: RIFF/`AVI `/LIST `hdrl` + any fourcc at 24 with size `0xFFFFFFFF`.
Reachable on wasm32: apps/web/Cargo.toml:63 depends on media-probe; built with
`--target wasm32-unknown-unknown` (apps/web/README.md:34); open path calls
`probe_stream(&with_builtins())` (apps/web/src/open.rs:230-232); AviSignature
registered in `with_builtins` (crates/media-probe/src/signature/mod.rs:132);
registry runs every signature over the attacker-controlled prefix (mod.rs:164-169).

Execution (audit/scratch/w20a03_avi_wasm32_proof.rs, `rustc -O`, native run):
- native 64-bit verbatim loop on the hostile prefix → `returned pos=4096 — NO panic` ("native cannot fire" holds).
- u32-emulated (wasm32 pointer width) same loop → `PANIC: range start index 4294967295 out of range for slice of length 4096`.

Blocker: only `x86_64-unknown-linux-gnu` rust-std is installed
(`rustup target list --installed`), so a real `--target wasm32-unknown-unknown`
compile could not be executed; the wasm32 half is proven by exact arithmetic
emulation of the saturating/wrapping semantics. Confidence: HIGH.

## F-2 — PropVariant pinned 16 bytes vs x64 PROPVARIANT 24
**Verdict: CONFIRMED substance; the task's PATH is WRONG; crate UNREACHABLE.**

Derivation: the union's two members ([u64;2] and the 2+6+8 header) are both 16
bytes on x64, so the Rust type is 16 bytes; the real Win32 PROPVARIANT is 8
(vt/reserved) + 16 (payload union incl. CAUB/DECIMAL) = 24 bytes. The 16-byte
stack value is handed by pointer to `IPropertyStore::GetValue` and
`PropVariantClear`, so up to 8 bytes of every OS write land outside the Rust
object (stack corruption domain).

Evidence — real file is crates/platform-windows/src/windows/com.rs (NOT
apps/desktop-tauri/src/windows/com.rs, which does not exist). com.rs:237-244
(verbatim):
```
#[repr(C)]
#[derive(Clone, Copy)]
pub union PropVariant {
    /// Header: `VT_LPWSTR` = 31 plus three reserved words.
    pub header: PropVariantHeader,
    /// Padding pinning the struct at its ABI size.
    pub raw: [u64; 2],
}
```
Hand-off: `empty_prop_variant()` com.rs:262-263 (`PropVariant { raw: [0, 0] }`),
stack slot in `property_store_string` com.rs:369-390 → `get_value(store, key,
&mut variant)` :381 and `PropVariantClear(variant)` :297.

Execution (audit/scratch/w20a03_propvariant_size.rs, `rustc -O`):
`size_of::<PropVariant>() = 16`, align 8; CAUB-style member = 16 on x64 →
documented ABI 24 → "crate size 16 vs x64 ABI 24 => 8-byte under-write domain".
Blast radius: `rg 'platform-windows'` in all Cargo.tomls — no app depends on
the crate (only its own tests). Latent ABI defect, not reachable from any
shipped entry point today.

## F-3 — coreaudio ELEMENT_MAIN=1 vs kAudioObjectPropertyElementMaster=0
**Verdict: CONFIRMED (static); crate UNREACHABLE.**

Derivation: Apple's AudioObjectBase.h defines
`kAudioObjectPropertyElementMain = 0` (previously `kAudioObjectPropertyElementMaster`),
wildcard = 0xFFFFFFFF. The crate pins 1 — i.e. element/channel 1 — and builds
every `AudioObjectPropertyAddress` with it, so every HAL property query and
listener registration addresses the wrong element and would fail at runtime.

Evidence — crates/platform-macos/src/macos/coreaudio.rs:133-137 (verbatim):
```
/// `kAudioObjectPropertyElementMaster` (renamed
/// `kAudioObjectPropertyElementMain`) — the whole-object element.
/// `[VERIFY]` the numeric value on real hardware (the declaration is
/// compile-only evidence in this sandbox).
const ELEMENT_MAIN: u32 = 1;
```
Used at coreaudio.rs:373, 402, 432, 447, 471, 500, 638, 648, 709, 729, 745.
Static proof only (no macOS SDK in sandbox; file's own header marks it
`[VERIFY]`). Blast radius: platform_macos consumed only by its own tests
(crates/platform-macos/tests/seam_behaviour.rs:17); no app depends on the crate.

## F-4 — audio-engine volume +inf
**SKIPPED per instructions (covered by A01).**

## F-5 — first-vertical transcripts stale at HEAD
**Verdict: CONFIRMED (baseline citation; no heavy re-run).**

Evidence — audit/runs/baseline/cargo-test-workspace.txt:
- :2764 `test committed_transcripts_match_fresh_runs ... FAILED`
- :2767 `test final_frame_is_current ... FAILED`
- :2773-2774 `thread 'committed_transcripts_match_fresh_runs' ... panicked at tests/integration/first-vertical/tests/evidence.rs:47:9:` / `assertion `left == right` failed: the committed transcript for `open` drifted from a fresh run — regenerate with: cargo test -p first-vertical --test evidence -- --ignored regen`
- :2781-2782 `thread 'final_frame_is_current' ... panicked at tests/integration/first-vertical/tests/evidence.rs:162:5:` / `assertion `left == right` failed: the committed final-frame.png drifted from a fresh run`
- :2791 (verbatim) `test result: FAILED. 1 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.54s`
- :2794 `### PACKAGE first-vertical exit_code=101`

Cross-check: tests/integration/first-vertical/tests/evidence.rs:47-50 and
:162-165 hold exactly those `assert_eq!` calls with those messages. The
committed evidence artifacts (transcript + final-frame.png) do not match fresh
runs at HEAD.

## F-6 — OrderingGuard/LatestCoalescer/BoundedEventQueue zero production consumers
**Verdict: CONFIRMED (rg-complete).**

Derivation: tree-wide search for the three names + `debug_assert_wire_ordered`
finds exactly two consumer files: the defining file itself (all usages inside
its `#[cfg(test)] mod tests`, events.rs:1544-1545+) and the test file
crates/playback-engine/tests/events_contract.rs. No production module imports
them.

Evidence — definitions (verbatim first lines):
- events.rs:949 `pub struct OrderingGuard {`
- events.rs:1356 `pub struct LatestCoalescer {`
- events.rs:1484 `pub struct BoundedEventQueue<T> {`

Production wiring proof: crates/playback-engine/src/session.rs:74 imports
`use crate::events::{DamageCoalescer, DrainStaleness, EventPolicy, ProgressWindow, TimeTickPolicy};`
— the three claimed types absent; lib.rs:118-121 `pub use events::{...}`
re-exports omit them. `debug_assert_wire_ordered` (events.rs:1312) has ZERO
callers anywhere despite its doc (:941-944) claiming "engine-internal
self-checks". LatestCoalescer's own doc (:1348-1351) admits it: "recorded as a
handoff, not wired here". events_contract.rs:70-73 imports BoundedEventQueue +
OrderingGuard — a test file, not production; LatestCoalescer is not even
imported there (its only users are events.rs's own tests, e.g. :2488).

## F-7 — audio.set-volume / set-eq refused by every shipped host; EQ math wired to nothing
**Verdict: CONFIRMED.**

Derivation: the catalog defines audio.set-volume but NO audio.set-eq; no
shipped host serves any audio command; the only responder is the browser
dev-shell mock; Eq3Band's real biquad math has zero production consumers.

Evidence:
- Catalog: crates/commands/src/catalog/ids.rs:117-123 (verbatim):
```
        // audio (§2.6)
        AudioSetVolume => "audio.set-volume",
        AudioGetVolume => "audio.get-volume",
        AudioSetMute => "audio.set-mute",
        AudioListDevices => "audio.list-devices",
        AudioSetDevice => "audio.set-device",
        AudioSetDelay => "audio.set-delay",
```
  `rg 'set-eq'` in crates/commands = 0 hits — the command does not exist.
- CLI: apps/cli/src/dispatch.rs SERVED :38-54 (no AudioSetVolume) + fallback
  (verbatim :154-155):
```
        // ---- everything else: the honest typed refusal ----
        other => Err(unserved(other)),
```
  → UnsupportedForHost (:164-171, `format!("`{id}` is not served by this
  build's command-line host ({owner})")`; owner text :193-194: "audio device
  control joins with the real backends (ADR-005)").
- Desktop: apps/desktop/src/dispatch.rs:67 `host: lumen_cli::host::CliHost`
  (same refusal). Control-server: apps/control-server/src/dispatch.rs:66-70 —
  every non-introspection command → `NotReady`. Web: apps/web/src/host.rs
  SERVED :762-775 (no audio ids at all). `rg 'AudioSetVolume|set_volume'`
  across apps/**/*.rs = 0 hits.
- Only responder: ui/src/client/demo-transport.ts:482-484 (dev-shell mock:
  `this.volume = Math.max(0, Math.min(1, request.args.volume))`).
- EQ: crates/audio-engine/src/effects/eq.rs:194 `pub struct Eq3Band` (real
  RBJ biquad shelf/peak math), :235 `impl AudioEffect for Eq3Band`; consumers
  = eq.rs's own tests only; pipeline.rs/mix.rs/governor.rs/volume.rs never
  reference EffectChain/Eq3Band (rg in audio-engine/src).

## F-8 — web capability snapshot advertises unopenable descriptors
**Verdict: substance CONFIRMED; the task's counts corrected (12 of 14, opt-in feature builds only).**

Derivation: the snapshot's descriptor list (feature-gated) names decoders the
web open path can never reach: the open path refuses every non-`avi`
container, and the AVI wiring can only ever produce `mjpeg` + `pcm_*` track
codecs (the demuxer's StreamFormat enum has exactly two variants and skips
unrecognized audio tags like ADPCM).

Evidence:
- apps/web/src/capabilities.rs:156-161 (verbatim):
```
pub fn decoder_descriptors(profile: TargetProfile) -> Vec<String> {
    match profile {
        TargetProfile::Native => vec!["mjpeg".to_string(), "pcm_s16le".to_string()],
        TargetProfile::Wasm32 => wasm32_builtin_descriptors(),
    }
}
```
- wasm32_builtin_descriptors :172-225 — with all codec features: mjpeg,
  pcm_s16le, adpcm_*, flac, mp3, mp1, mp2, vorbis, aac, aac-lc, alac, opus,
  vp9, dnxhd = **14 advertised**; openable = mjpeg + 6 pcm names →
  **12 of 14 unopenable** (incl. dnxhd :219-220). The task's "10 of 12" was
  not reproduced under any feature combination I checked.
- Static chain: apps/web/src/open.rs:238-249 (non-`avi` → typed refusal);
  open.rs:266-296 harvest (MotionJpeg → "mjpeg", Pcm → pcm_*, else "unknown");
  crates/demux/src/avi/header.rs:92-117 (StreamFormat = MotionJpeg | Pcm only);
  header.rs:812-824 (unrecognized tags incl. ADPCM → per-stream skip).
- Mitigation (why this is a narrow finding): the SHIPPED default wasm module
  advertises zero decoders (capabilities.rs:223-224) and the blocked row
  :312-319 discloses the exact limitation; only opt-in `builtin-codecs*`
  builds over-claim, and host.rs:561-572 documents that blob-consuming
  decoders refuse typed on fact-less lanes.

## F-9 — the four decoder registries disagree
**Verdict: CONFIRMED (all cited line numbers verified).**

Inventory (CLI host):
1. engine-info: apps/cli/src/host.rs:1643-1657 — decoders = mjpeg, pcm_s16le
   (2). Baseline audit/runs/baseline/mp4-smoke.md:99 `decoders: mjpeg, pcm_s16le`.
2. pipeline_host dispatch: apps/cli/src/pipeline_host.rs:241-371 — 16 arms:
   mjpeg :242, flac :253, `mp3|mp1` :261, mp2 :266, vorbis :271, opus :276,
   `aac|aac-lc` :288, alac :293, `adpcm_` :298, `av1|av01` :304, vp9 :313,
   dnxhd :326, `h264|avc1` :346, `pcm_` :347.
3. bench: apps/cli/src/bench.rs:300 (mjpeg only), :313 (pcm_ guard), :334
   (verbatim) `"no decoder in this build (first vertical: mjpeg + pcm)"`.
4. "landed" string: apps/cli/src/pipeline_host.rs:376-377 (verbatim):
```
                    "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, \
                     mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"
```
   — 12 names; omits the dispatch's vp9 (:313) and dnxhd (:326); shown at
   runtime in baseline mp4-smoke.md:57 (HEVC refusal).

Four different answers (2 / 16 arms / 2-with-first-vertical-message / 12) —
the registries disagree with each other; the "landed" string in the error the
user sees lies about the dispatch table 30 lines above it.

## F-10 — renderer-wgpu contains zero wgpu calls
**Verdict: CONFIRMED.**

Evidence — crates/renderer-wgpu/Cargo.toml:27-29 (verbatim):
```
[dependencies]
media-types = { path = "../media-types" }
video-engine = { path = "../video-engine" }
```
No wgpu dependency. `rg 'wgpu::|use wgpu|extern crate wgpu'` across the crate
(src + tests): zero code hits — every "wgpu" occurrence is a doc comment or a
string-returning shape helper (surface.rs:60 `wgpu_present_mode()`,
color.rs:45 `wgpu_texture_format()` returning `&'static str` variant names).
lib.rs:18-19 (verbatim):
```
//! The crate is *named* renderer-wgpu because doc 01 names it so; **v0
//! does not import wgpu**, for four verified reasons:
```
Reachable from shipped hosts (apps/cli/Cargo.toml:60, apps/desktop/Cargo.toml:42)
— the actual present lane is the CPU software compositor. Name lies; the
deferral is honestly documented in the crate itself.

## F-11 — 66 #[ignore] + corpus-v4 absence make differential lanes vacuous-green
**Verdict: CONFIRMED and UPGRADED in scope.**

Derivation + evidence:
- #[ignore] count (Grep over **/*.rs outside audit/): crates 53 + apps 2 +
  tests 8 + tools 3 = **66** — exactly matches baseline
  audit/runs/baseline/loc-patterns.txt:34 (`#[ignore]                  66`).
- corpus-v4 is absent (`ls /tmp/my-project/corpus-v4` → "No such file or
  directory"), LUMEN_CORPUS unset. Corpus-gated test functions
  (`= corpus_dir() else` pattern): **62 occurrences across 32 files**
  (apps/cli containers mp4/es/ts/wav/mkv/ogg/adts; demux tests ts_pes_corpus,
  corpus_mp4, mkv_corpus, es_corpus, adts_corpus, corpus_wav, corpus_ogg,
  mkv_editions_tags, mkv_attachments_chapters; 16 codecs corpus/differential/
  bench files).
- The skip pattern (crates/demux/tests/ts_pes_corpus.rs:242-249, verbatim):
```
fn corpus_ts_lanes_match_the_ffprobe_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(ts-pes): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
```
  → the test fn returns Ok: green with zero assertions executed.
- EXECUTED 3 samples (targeted `cargo test -p <crate> --test <file> <filter>
  -- --nocapture`, each exit 0):
  1. `-p demux --test ts_pes_corpus corpus_ts_lanes_match_the_ffprobe_truth`
     → prints the SKIP line, then `test corpus_ts_lanes_match_the_ffprobe_truth ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored`.
  2. `-p codecs --test flac_bitexact corpus_flacs_decode_byte_exact_or_reject_typed`
     → `corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP` → `ok. 1 passed`.
  3. `-p codecs --test adpcm_corpus_differential real_corpus_decodes_bit_exact`
     → `corpus(adpcm): ... — SKIP` → `ok. 1 passed`.
- Differential-vs-ffmpeg/ffprobe lanes by name: adpcm, vorbis (bitexact),
  flac (bitexact), mp3, mp2, aac, opus, mjpeg-field, ts-pes, mp4, mpeg12,
  h264-parser, hevc-parser = **≥ 13 lanes**, plus container corpus tests
  (mkv/wav/ogg/adts/es). The task's "~10 differential lanes" is an
  UNDERCOUNT; the vacuous-green surface is 62 corpus-gated tests, all green
  in this environment while asserting nothing.

## Blockers / UNVERIFIED items
- F-1: true wasm32 compile not run (only x86_64 rust-std installed in this
  sandbox; `rustup target list --installed` = x86_64-unknown-linux-gnu).
  wasm32 half proven by executed u32-emulation of the exact saturating/wrap
  semantics; native half executed verbatim.
- F-2/F-3: platform-windows / platform-macos runtime behavior not executed
  (no Windows SDK / no macOS SDK in sandbox) — both are static ABI proofs,
  and both crates are currently unreachable from shipped entry points.
- No git run, no workspace-wide cargo commands run (per spec); the only cargo
  invocations were the 3 targeted single-test runs above on prebuilt
  binaries (0.36s compile, all exit 0).

## Commands run (all from repo root, PATH=$HOME/.cargo/bin:$PATH)
- rustc -O + execute: audit/scratch/w20a03_avi_wasm32_proof.rs (F-1), audit/scratch/w20a03_propvariant_size.rs (F-2) — outputs pasted above/in log.
- cargo test -p demux --test ts_pes_corpus corpus_ts_lanes_match_the_ffprobe_truth -- --nocapture (exit 0)
- cargo test -p codecs --test flac_bitexact corpus_flacs_decode_byte_exact_or_reject_typed -- --nocapture (exit 0)
- cargo test -p codecs --test adpcm_corpus_differential real_corpus_decodes_bit_exact -- --nocapture (exit 0)
- rg/Grep inventories + sed verbatim-quote verification for every citation above.
