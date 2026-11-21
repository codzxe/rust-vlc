# AUDIT: crates/codecs/src/video/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 42 / 0 functions — 7 module declarations (3 cfg-gated, 4 unconditional) + module docs |
| File verdict | REAL (accurate module root, linked and executed in the shipped build; docs carry drift findings) |
| Reachable from shipped entry point? | YES — `codecs` is a default dependency of the shipped CLI (apps/cli/Cargo.toml:51, no feature subset) → `pub mod video` (lib.rs:60) is compiled into `lumen`; five of its seven codec modules are dispatched by apps/cli/src/pipeline_host.rs (`mjpeg`:242, `av1`:304, `vp9`:313, `dnxhd`:326, `h264`:346) and three of those executed end-to-end in the baseline (mjpeg AVI first-vertical plays + bench 60/60; h264 4:2:0 MP4 plays — INDEX CORRECTION v2; av1 differential 6/6 + CLI ivf lanes — prior W06-A08 session). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read tool full file; `rg -n "pub mod\|pub use"` h264/mod.rs (sw gating), hevc/mod.rs; `rg -c "unsafe" crates/codecs/src/` (lib.rs charter cross-check → lib.rs audit W06-A10 already owns it); `rg -n "mjpeg\|vp9\|dnx\|av1" apps/cli/src/pipeline_host.rs` (dispatch census); `rg -n "register\|PluginRegistry" crates/codecs/src/` → zero (doc 01 §2.5(b) surface absent); `rg -ln "mpeg12"` repo census; `rg -n "vp9\|dnx"` baseline test logs (:535-546 dnx 12 tests ok, :776-780 vp9 5 tests ok, :1292 vp9_differential ok); Read docs/web/size-report.md:37-64 + apps/web/Cargo.toml:78-114 (wasm lane); Read docs/01-architecture.md §2.5:722-746 + docs/04-codec-format-roadmap.md §4.5:326-333; `sed -n` quote verification mod.rs:5-26,40-42 |

## 1. One-paragraph verdict
A 42-line module root whose seven declarations all resolve to real trees, whose three cfg gates exactly match the crate's `codec-*` features, and whose feature taxonomy paragraph is verifiable against Cargo.toml, the wasm build matrix, and the size report. Three of its codecs are executed end-to-end in the baseline evidence, two more are dispatched with test-executed engines, and two — `hevc` and `mpeg12` — are compiled into every native shipped build with no entry point that can ever reach them, a dead-weight fact the file openly defers onto a registration card (doc 01 §2.5(a)) that has not landed anywhere in the tree. The doc prose drifts on its own history: the wave-6 sentence names only dnx while the file itself documents vp9 as the second 6.M card, and the gating dichotomy ignores the in-house `codec-h264-sw` gate on the only h264 decode path that ships.

## 2. What actually works (verified)
— All 7 declared modules exist (ls src/video: av1 dnx h264 hevc mjpeg mpeg12 vp9 — exactly the declared set, no undeclared module, no dangling declaration).
— cfg gates match features: `codec-av1`/`codec-dnx`/`codec-vp9` all exist in codecs/Cargo.toml [features] (:66, :67, :81) and all three are in `default` (:48-59) — "default enables the full set" is TRUE; the pre-7.K1 graph is unchanged for default consumers.
— Executed lanes through this root (baseline): mjpeg — avi-firstvertical-open-session.txt plays, bench 60/60 frames; h264 — sample-420.mp4 Buffering→Playing→Stopped→Idle (INDEX CORRECTION v2); av1 — av1_differential 6/6 bit-exact vs ffmpeg + CLI ivf lanes (prior W06-A08 session, baseline :524-534, orchrun :2797,2812); vp9 — 5 unit tests + `differential_lanes_vs_ffmpeg_libvpx_vp9` ok (test-workspace.log.txt:776-780, :1292); dnx — 12 unit tests ok (:535-546).
— wasm claim verified: docs/web/size-report.md:39-41 records "rav1d 1.1.0 does not compile for wasm32-unknown-unknown (39 errors, measured 2026-11-10)"; apps/web/Cargo.toml:89-90 makes codecs on wasm32 `optional, default-features = false` and codec-av1 is absent from every wasm feature lane (:113-114) — "wasm subsets never carry this module" holds by construction.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/mod.rs:13-16 (policy), consumer census for the four unconditional modules
Code:
```
//! set, so the unconditional pre-7.K1 graph is unchanged); the
//! in-house modules ([`h264`], [`hevc`], [`mjpeg`], [`mpeg12`]) stay
//! unconditional until the doc 01 §2.5(a) registration card owns
//! their gates.
```
What is wrong: the gating card this file waits for does not exist: `rg "pub fn register|PluginRegistry"` over crates/codecs/src/ returns zero — doc 01 §2.5(b)'s `register(registry, features)` surface (§2.5:734-735) was never built, so nothing can "own their gates". Consequence today: `hevc` (parser tree + hw seam, no dispatch arm anywhere; apps/cli/src/containers/ts.rs:968 asserts `unreachable!("no hevc decoder exists in this build")`) and `mpeg12` (module-level census: consumers are its own tests + a comment in apps/web/Cargo.toml:94) compile into every native shipped CLI build with no reachable entry point — dead weight on the exact scale the wasm lane works to avoid (apps/web/Cargo.toml:92-96 explicitly leans on tree-shaking to drop them from wasm builds).
Why it matters: the deferral sentence describes a future card as if it were a landed plan; in its absence the file is the shipping decision, and it ships two dead codec trees in every native binary. Disclosed drift is still drift.
What correct looks like: either land the per-codec gates for the in-house modules (they are pure-Rust, zero-dep — gating is mechanical), or the doc states the current fact plainly: "the in-house modules are ungated by decision; hevc/mpeg12 currently have no shipped consumer."
Confidence: HIGH — Verification: rg census + read (ts.rs:968, apps/web/Cargo.toml, doc 01 §2.5). mpeg12's full per-file census is my next audit (bitreader.rs) — module-level conclusion only here.

### F-002 [S4]
Where: crates/codecs/src/video/mod.rs:8-10 vs :18 and :40-41
Code:
```
//! story. Wave 6's M-cards add the admitted engines behind the
//! contract: [`dnx`] (filmcraft-dnx, DNxHD/DNxHR VC-3). Per-codec
```
```
/// The rusty_vp9 VP9 decoder (card 6.M-vp9).
```
What is wrong: the history paragraph names only dnx as the wave-6 M-card addition, while the file's own line 41 documents a second one (`card 6.M-vp9`, rusty_vp9, admitted the same wave); and av1 is dated "wave 7" here but carries card id 5.E1 at line 18 — two numbering systems (waves vs cards) interleaved in one paragraph without reconciliation.
Why it matters: the intro is the file a new reader uses to map the codec roster's provenance; as written it undercounts the current roster (vp9 invisible in the narrative) and mixes coordinate systems.
What correct looks like: "Wave 6's M-cards add dnx and vp9; av1 (card 5.E1) arrives in wave 7."
Confidence: HIGH — Verification: read (all four line ranges).

### F-003 [S4]
Where: crates/codecs/src/video/mod.rs:10-14 vs h264/mod.rs:55-56
Code:
```
//! Cargo feature history: the EXTERNAL-engine modules ([`av1`],
//! [`dnx`], [`vp9`]) ride their `codec-*` Cargo features since card
```
```
#[cfg(feature = "codec-h264-sw")]
pub mod sw;
```
What is wrong: the stated dichotomy — external engines are feature-gated, in-house modules are unconditional — omits the one gate that guards the only h264 decode path that actually ships: `codec-h264-sw` gates the in-house `sw` submodule (h264/mod.rs:55-56), an in-house-code-behind-a-feature hybrid that fits neither bucket of the paragraph.
Why it matters: a reader predicting build contents from this paragraph would conclude an in-house h264 software decoder ships ungated; in a `--no-default-features` build the h264 module compiles without any decoder at all (parser + dead hw tree only).
What correct looks like: name the three gating axes: external engines (av1/dnx/vp9), the h264 sw engine (`codec-h264-sw`, default-on), and the ungated in-house parsers/decoders.
Confidence: HIGH — Verification: read (both files; codecs/Cargo.toml:53,73).

## 4. Per-function table
No functions — per rule 9, the units are the declarations and their doc claims:
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (roster + feature history) | 1-16 | REAL (drift) | S3/S4 | taxonomy F-003, history F-002, ungated-deferral F-001; v0-single-codec and wasm claims verified |
| `pub mod av1` (cfg codec-av1) | 18-23 | REAL — reachable+executed | — | dispatched pipeline_host.rs:304-308; differential 6/6 + ivf lanes (prior session evidence); wasm-absence claim verified |
| `pub mod dnx` (cfg codec-dnx) | 24-26 | REAL — dispatched, no producer | — | arm at :326; its own comment: "No container lane mints the `dnxhd` descriptor yet" — unreachable from real files in v1; engine executed by 12 unit tests + differential (:535-546) |
| `pub mod h264` | 27-29 | REAL — reachable+executed | — | sw lane :346 (420 MP4 plays); parser subset consumed by sw bridge; hw subtree dead (W06-A01) |
| `pub mod hevc` | 30-32 | UNREACHABLE | — | no dispatch arm; ts.rs:968 unreachable! asserts the lane's absence; decoder test-constructed only |
| `pub mod mjpeg` | 33-35 | REAL — reachable+executed | — | :242; AVI first-vertical plays, bench 60/60 |
| `pub mod mpeg12` | 36-39 | UNREACHABLE | — | no dispatch arm; consumers: own tests + apps/web comment (module-level census; file detail in my bitreader.rs audit) |
| `pub mod vp9` (cfg codec-vp9) | 40-42 | REAL — reachable+executed | — | :313 (ivf/webm lanes); 5 unit tests + vp9_differential ok (:776-780, :1292) |

## 5. Test honesty
No tests in this file; the root's claims are pinned by the children's suites, all executed in the baseline (counts above). The one unpinned claim is the negative one: nothing tests that hevc/mpeg12 have no consumers — that is this audit's census, not a test.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v0 carried a single codec: mjpeg — the first vertical's video path (doc 03 §7)" | mod.rs:3-5 | REAL | baseline rc-smoke (AVI first-vertical only, by design); engine-info decoders literal (INDEX CORRECTION v2) |
| "wave 7 adds av1 … the flagship of doc 04 §4.5's honest v1 video story" | mod.rs:6-8 | REAL (pointer) | av1 shipped+executed; doc 04 §4.5:326-333 does make AV1 the software flagship |
| doc 04 §4.5: "H.264 (hw-only) … VP8/VP9 (missing)" | docs/04 §4.5:328-331 (the section this file cites) | **STALE/FALSE today** | h264 sw decoder shipped+executed (420 MP4 plays; h264/mod.rs:22-23 "the blocked era … is closed"); vp9 shipped+dispatched+differential ok — the cited section contradicts the code this file roots (doc-04 owner's defect, recorded here as a contradiction) |
| "EXTERNAL-engine modules ride codec-* features since 7.K1; default enables the full set" | mod.rs:10-13 | REAL | codecs/Cargo.toml:47-81 read-verified |
| "in-house modules stay unconditional until the doc 01 §2.5(a) registration card owns their gates" | mod.rs:13-16 | REAL-as-description / PAPER-ONLY (the card) | no `register`/`PluginRegistry` anywhere in the crate (rg zero); doc 01 §2.5:726-735 promises per-codec features + register() — unimplemented (F-001; lib.rs audit F-003 concurs on the module-map drift) |
| "rav1d does not compile for wasm32 (39 errors, measured 2026-11-10)" | mod.rs:19-21 | REAL | docs/web/size-report.md:39-41 (measurement record); apps/web/Cargo.toml:89-90,113-114 (wasm never enables it) |

## 7. Dead / duplicated / unreachable code
- `hevc` + `mpeg12` module trees: compiled, unconditional, zero shipped consumers (F-001); hevc's decoder is test-constructed only; wasm builds shed them only via DCE (apps/web/Cargo.toml:94-96 comment), native builds keep them.
- `dnx`: dispatched but descriptor-less — no container mints `dnxhd` (arm comment, pipeline_host.rs:331-334) — engine reachable only from its differential/tests.
- No duplication at this level; each codec appears exactly once.

## 8. Dependencies & flags
Module root only. The three cfg gates (`codec-av1`, `codec-dnx`, `codec-vp9`) match [features] exactly; the four in-house declarations are ungated (deliberate, F-001). The crate's charter claims ("zero external dependencies", "No unsafe") are false for the crate as shipped — owned and counted by the lib.rs audit (W06-A10 F-001/F-002), not re-counted here; the unsafe blocks live in this file's children (h264::hw vaapi/videotoolbox, av1/decoder.rs, mjpeg/idct_simd.rs — 204 unsafe mentions in codecs/src by my rg count).

## 9. Open questions for the next agent
- mpeg12 file-level census: my own next audit (bitreader.rs) answers whether ANY parser entry has a non-test caller.
- Whoever owns doc 04: §4.5's "H.264 (hw-only)" and "VP9 missing" are stale against shipped reality — the parity report (0/252, baseline #9) already tells the pessimistic story; the roadmap should match.
- The `dnxhd` descriptor gap: mkv V_MS/FOURCC + ASF futures own it (arm comment) — when they land, re-check the dnx row's reachability.
