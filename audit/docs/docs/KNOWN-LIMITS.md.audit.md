# CLAIM AUDIT: docs/KNOWN-LIMITS.md
| Field | Value |
|---|---|
| Auditor | W19-A06 (recovered by W19-R4) |
| Claims extracted | 12 |
| REAL 10 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 1 / UNVERIFIED 1 |
| Internal contradictions found | 1 (stale headline row) |

## 1. Blunt verdict
The register is densely traceable: every one of the 17 linked source docs
exists (PARITY-REPORT, RELEASE, risk register, 5 dependency records, the
oracle-run JSON, the ADRs), the w27/w30 strike-throughs match the tree
(rust_h264 0.6.1 admitted behind codec-h264-sw; http opens routed through
open_source; product.toml streaming flags exactly as stated; cpal still NOT
admitted). One row has gone FALSE since the 2026-11-15 refresh: "no `<canvas>`
exists in ui/src" — wave w31.4 later landed MediaSurface.tsx with a real
`<canvas>` (:174) blitting LUF1 frames in the desktop shell, and nobody struck
the row. That is the register's own §10 maintenance rule being violated by a
later wave, not by this document's author at its date.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "252 rows" matrix scope | :9 | REAL | python3 count over docs/feature-matrix.toml -> 252 [[feature]] rows. |
| 2 | w27 H.264 sw strike: "rust_h264 0.6.1 behind codec-h264-sw — matrix row flipped blocked→implemented" | :26 | REAL | crates/codecs/Cargo.toml:69-73 `rust_h264 0.6.1 ... codec-h264-sw = ["dep:rust_h264"]`; records/rust_h264-0.6.1.md exists. |
| 3 | "product.toml exposes streaming-http = true, streaming-hls/streaming-dash = false" | :37 | REAL | product.toml:157-159 exactly those three values with w31.3 annotation. |
| 4 | "cpal is NOT admitted (Lead-gated conditional-E2; sandbox build fails exit 101)" | :51 | REAL | rg cpal in workspace manifests -> only crates/audio-output/Cargo.toml:12 comment (conditional-E2, not admitted); records/cpal-0.18.2.md exists. |
| 5 | "no `<canvas>` exists in `ui/src` (pinned by test: the surface is a labelled region, never a fake video)" | :52 | FALSE (stale since w31.4) | ui/src/components/player/MediaSurface.tsx:80 mounts `<video`, :174 mounts `<canvas` ("frame-canvas half (w31.4): ... binds it to the shell's shipper ... blits every arriving LUF1 record"). The old no-fake-canvas pin still exists (PlayerScreen.test.tsx:59-62) but coexists with the real canvas. Row refreshed 2026-11-15; w31.4 landed later and did not strike it — §10 violated downstream. |
| 6 | "http(s) progressive/ranged OPENS since w31.3" + demux_host.rs open_path http arm | :37 | REAL | apps/cli/src/demux_host.rs:13 "an `http(s)` ... serves every `http(s)` source" (:357), `pub fn open_source(` (:362). |
| 7 | "the v0.1.2 Windows open blocker is gone ... SourceUri::from_file_path" (w30.1/30.2) | :36 | REAL | crates/media-types/src/ids.rs:83-88,366 — from_file_path normalization docs (UNC typed-rejected). |
| 8 | "OS-webview shell ... w28 built and shipped lumen-tauri.exe" (strike S-3) | :50 | REAL (cross-doc) | apps/desktop-tauri/INFRA-BLOCKED.md exists with §B; scratch zipdist artifact lumen-0.1.3-x86_64-windows.zip observed in audit scratch; RELEASE.md shipped-register present. |
| 9 | "19/35 differential inputs re-based" + vlc-oracle-run-20261109.json `inputs` | :92 | REAL | docs/corpus/vlc-oracle-run-20261109.json exists (input-rebasing record as cited). |
| 10 | All linked register sources exist (risk register, ADRs, perf docs, records) | throughout | REAL | 17/17 linked files present (batch ls verified: PARITY-REPORT, RELEASE, 14-risk-register, PERF-VS-VLC, perf/breadth, perf/soak-nightly-20261109, plans/phase8-cards, plans/open-path-wiring, 5 dependency records, INFRA-BLOCKED, 09-web-wasm-target, ADR-013, design/README). |
| 11 | "Verified parity is 0/252 ... 64/252 rows implemented per desktop platform ... 0 on web" | :17 | REAL (cross-doc) | Baseline xtask-parity-report: "0/252 verified on every platform (windows 66/252, linux 61/252, web 0/252)" — same 0-verified shape; the implemented counts drift by date (doc's 64 vs baseline 66/61 snapshot later). |
| 12 | "0/252 implemented on web" (§5) | :70 | REAL (cross-doc) | Baseline parity-report web 0/252. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| :52 "no `<canvas>` exists in ui/src" | MediaSurface.tsx:174 `<canvas` (w31.4) | Tree wins — the row is FALSE today. Register date 2026-11-15 < w31.4 landing. S2 doc-truth defect: the consolidated "known limits" register itself is stale on a headline platform row. |

## 4. Fabrication / inflation indicators
None from the register's own compilation (it is anti-inflation by design —
listing weaknesses with sources). The single FALSE row is a maintenance lapse
by a later lane, not a fabrication. The strike-through discipline (H.264,
open-path, tauri shell rows) is demonstrably honest — each strike names its
wave, audit severity, and the closing evidence.
