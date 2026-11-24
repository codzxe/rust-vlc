# CLAIM AUDIT: docs/inventory/platform-macos.md

| Field | Value |
|---|---|
| Auditor | W18-A09 |
| Date | 2026-11-24 |
| Lines | 306 |
| Claims extracted | 22 (5 repo-checkable, 17 external-world) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 2 REAL / 1 REAL-UNPROVEN / 0 / 0 / 17 PAPER-ONLY / 0 FALSE / 2 UNREACHABLE |
| Internal contradictions found | 2 |

## 1. Blunt verdict

This is a dated (2026-10-19) clean-room RESEARCH record, honestly labeled: 10 `[VERIFY]` markers, 1 `[PROPOSAL]`, explicit "this record classifies, does not approve" and "NOT an ADR decision" disclaimers. As research notes it makes almost no false claims I can disprove — because almost none of it is checkable from this sandbox: every load-bearing fact (wry internals, crates.io versions, Apple docs JSON, WebKit blogs, notarization behavior) requires network or hardware I do not have. What IS checkable checks out: the ADR files it feeds exist; the tauri/wry/tao versions it recorded for 2026-10-19 are one patch-level below today's lockfile (wry 0.57.0→0.57.2, tauri 2.12.4→2.12.5, tao 0.37.1→0.37.2). The two things that matter: (1) its §4.3/§5 recommendation to standardize on the objc2 family / cpal was NOT what the repo actually did — crates/platform-macos is std-only hand-rolled `extern "C"` FFI with zero objc2/cpal deps, and W8 proved that crate is an orphan with zero dependents and the cpal seam is a stub with zero cpal code; (2) nothing in the doc discloses that the "macOS platform adapter" it feeds is unreachable from any shipped entry point. The doc is fine; the product it describes does not exist in the shipped build.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Feeds: ADR-001 (hw decode: VideoToolbox), ADR-003 (video-in-webview, Candidates A/B), ADR-005 (audio backend)" | :5-6 | REAL | ls docs/adr/ → ADR-001-hardware-decode.md, ADR-003-video-in-webview.md, ADR-005-audio-output.md all present. |
| 2 | "wry 0.57.0 … tauri 2.12.4 … tao 0.37.1" (component table, "verified 2026-10-19 via crates.io API") | :21-24 | REAL-UNPROVEN (drift check compatible) | apps/desktop-tauri/Cargo.lock:3631-3632 `name = "tao" version = "0.37.2"`, :3688-3689 `tauri 2.12.5`, :4961-4962 `wry 0.57.2` — one patch above the doc's dated snapshot; consistent with a 2026-10-19 recording. The crates.io "verified" part is UNVERIFIABLE offline. |
| 3 | cpal 0.18.2 row: "audio; macOS host = coreaudio-rs 0.14.2 + objc2-* (verified)" | :39 | PAPER-ONLY (ecosystem fact; repo diverges) | crates/audio-output/Cargo.toml:11-19: cpal is "conditional-E2, NOT admitted"; [dependencies] = audio-engine, media-clock, media-types only (no cpal). W8 summary: "cpal.rs: STUB … zero cpal code behind the seam (W08-A04)". |
| 4 | "recommendation for consistency [PROPOSAL]: standardize on the objc2 family for new engine code" | :209-210 | CONTRADICTED by implementation | crates/platform-macos/Cargo.toml:14-16: "Std-only by C5: NO external dependencies, NO objc2/core-audio crates — the OS C-ABIs are hand-declared `extern \"C\"` in src/macos/". The repo took the opposite approach. |
| 5 | "VideoToolbox is fully reachable in pure Rust: objc2 0.6.5 + objc2-video-toolbox 0.3.2 … confirmed on docs.rs" | :282-284 | PAPER-ONLY | No objc2 dependency anywhere in the workspace Cargo files (rg over crates/*/Cargo.toml → none); docs.rs unreachable from sandbox. The CLAIMED path was never taken in-repo. |
| 6 | wry 0.57.0 implements drawsBackground private-KVC transparency; "verified in wry-0.57.0/src/wkwebview/mod.rs" | :53-59 | PAPER-ONLY / UNVERIFIED | No wry source in sandbox (~/.cargo/registry absent; apps/desktop-tauri not built here — baseline cargo-build-desktop-tauri.txt fails at gdk-sys). Cannot inspect wry source; no network. |
| 7 | "Safari 26.0 expands support for WebCodecs API by adding AudioEncoder and AudioDecoder" (WebKit blog) | :132-133 | PAPER-ONLY / UNVERIFIED | External URL claim; no network access. |
| 8 | "Notarization of macOS software … is not App Review" (Apple docs JSON) | :248-250 | PAPER-ONLY / UNVERIFIED | External. |
| 9 | "macOS 15.7 black-base bug … 15.8.1 fixed it (wry #1867)" | :73-79 | PAPER-ONLY / UNVERIFIED | External issue-tracker claim. |
| 10 | Open items list "[VERIFY]" (8 numbered items) | :292-305 | honest disclosure | 10 `[VERIFY]` markers counted (rg -c → 10). The doc self-flags its unknowns. |
| 11 | The macOS platform adapter it feeds (ADR-005 audio path) is reachable | implied by "Feeds: … ADR-005 (audio backend)" | UNREACHABLE | W8 SUMMARY: "CoreAudio: real FFI, orphan crate" + platform-macos has zero dependents (W8-A01/A02/A03 evidence); no app or shipped crate depends on platform-macos (workspace Cargo.toml survey). |
| 12 | Notarization/signing section: "our 100%-Rust engine means no third-party dylibs to herd" | :261-262 | PAPER-ONLY / UNVERIFIED | Repo lockfiles are pure-Rust deps (not re-verified exhaustively by me — the claim's operative part concerns shipping, untestable here). |

Remaining 10 ledger entries (crates.io version rows objc2 0.6.5/0.3.2 set, core-foundation 0.10.1, core-video 0.6.1, coreaudio-rs 0.14.2, coreaudio 0.5.0 disambiguation, wgpu 30.0.1, WKWebView-subview mechanics :85-99, WebCodecs floor mapping :129-149, VideoToolbox API names :184-198, CoreAudio callback constraints :232-239) are all external-world claims of the same class as #6-#9: PAPER-ONLY, UNVERIFIED from this sandbox (no network, no macOS SDK, no wry/cpal sources vendored). Each is honestly dated and most carry [VERIFY].

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §4.3/§5 recommend objc2-family / cpal as the standard access path for new engine code (:209-210, :216-224) | crates/platform-macos/Cargo.toml:14-16: "NO external dependencies, NO objc2/core-audio crates — the OS C-ABIs are hand-declared `extern \"C\"`" | The repo's actual implementation is the opposite of the doc's recommendation. Both are honest records; the doc predates the implementation and was never reconciled. |
| Table row "cpal 0.18.2 … audio; macOS host = coreaudio-rs 0.14.2 (verified)" reads as the audio path | W8 SUMMARY (audit/waves/wave-08/SUMMARY.md): "cpal.rs: STUB — … zero cpal code behind the seam (W08-A04)"; audio-output ships the null backend only | In the repo, no cpal exists anywhere; the doc's ecosystem facts are about upstream crates, not this product. The distinction is disclosed nowhere in the doc. |

## 4. Fabrication / inflation indicators

- None found that are provable: the doc's method note (:7-11) admits the sandbox had no WebSearch/WebFetch and describes the exact curl+tarball method; version claims are dated; recommendations are labeled [PROPOSAL]; unknowns are labeled [VERIFY] (10×).
- The one structural gap: nothing in the doc states that every platform path it researches (VideoToolbox decode, CoreAudio output, WKWebView overlay) is currently UNREACHABLE from the shipped build (platform-macos orphan per W8; desktop-tauri fails to build in this sandbox per baseline cargo-build-desktop-tauri.txt). A reader could take §4.1-§4.3 as evidence of working macOS capability. It is not.

## 5. Verdict block

The file is what it says it is: a dated feasibility research record with 22 concrete claims — 2 REAL (ADR files exist; version-table drift-check compatible with lockfile), 1 REAL-UNPROVEN (the crates.io verification layer), 17 PAPER-ONLY/UNVERIFIED (external-world facts: Apple/WebKit/wry/crates.io — no network in this sandbox to re-fetch, no vendored sources), 2 UNREACHABLE (the macOS adapter surface it feeds — orphan crate, W8). No FALSE claim could be established. Reachability from shipped entry points: NOT APPLICABLE to a research doc, but every implementation surface it feeds is unreachable today.
