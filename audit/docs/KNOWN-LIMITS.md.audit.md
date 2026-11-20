# CLAIM AUDIT: docs/KNOWN-LIMITS.md

| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Date | 2026-11-16 |
| Claims extracted | 26 | REAL 13 | REAL-UNPROVEN 5 | SHALLOW 2 | STUB 0 | PAPER-ONLY 0 | FALSE 4 | UNVERIFIED 2 |
| Internal contradictions found | 1 |

## 1. Blunt verdict

The register's discipline is real: most codec/format/platform rows carry verdicts that reconcile with the code (null audio, H.264 landed, HLS refusal, cpal not admitted, hardware rows honest). But the w30.10 "docs-truth" refresh (2026-11-15) is already lying again in three places at HEAD: the Opus row ("no pure-Rust decoder") was false the day the file was compiled (the matrix row flipped implemented on 2026-11-09, one day before), the GUI-video row ("no `<canvas>` exists in ui/src") was killed by wave w31.4 one day after the refresh, and §7 still asserts "CI billing blocked" in the same file whose §1 says billing was unblocked and the workflow executed. A known-limits register that is wrong about what is known is worse than no register: it manufactures false confidence in both directions.

## 2. Claim ledger

| # | Claim (verbatim ≤25 words) | Location (path:line) | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Refreshed 2026-11-15 (wave `w30.10`, the docs-truth lane): stale rows struck with closing evidence" | docs/KNOWN-LIMITS.md:8 | REAL | worklog.md:2135-2150 (Task ID w30.10.docs-truth) describes exactly this strike-and-admit pass. |
| 2 | "Status truth lives in `docs/feature-matrix.toml` (252 rows; audited — see `docs/PARITY-REPORT.md`)" | docs/KNOWN-LIMITS.md:9 | SHALLOW | 252 rows REAL (my parse); "audited" points at PARITY-REPORT, whose PASS result is now FAILURE with 2 R4 violations at HEAD. |
| 3 | "Verified parity is 0/252, pending the evidence flip" | docs/KNOWN-LIMITS.md:17 | REAL | Matrix verified=0 on all four platforms at HEAD; executed parity-report agrees. |
| 4 | "Billing was unblocked 2026-11-12 and the release workflow has since executed (v0.1.0–v0.1.2 shipped…)" | docs/KNOWN-LIMITS.md:17 | UNVERIFIED | GitHub Actions/releases invisible from the tree. In-repo support: release-checklist.md:238-248 (run 43466814633), worklog w28.2/w28.3 CI-run references, local git tags v0.1.0–v0.1.2 at matching dates. Contradicting in-repo evidence: release.yml:26 "THIS WORKFLOW HAS NEVER EXECUTED (CI billing blocked)" still present at HEAD. |
| 5 | "64/252 rows `implemented` per desktop platform at the 2026-11-10 recompute (w27's h264-sw flip makes it 65…)" | docs/KNOWN-LIMITS.md:17 | FALSE | At HEAD: windows 66, macos 66, linux 61 — the uniform "65" never described this tree; the w31.x flips changed all three columns. |
| 6 | "No wall-clock timing vs VLC — all recorded timing is vs ffmpeg 7.1.5 on a shared 2-core sandbox" | docs/KNOWN-LIMITS.md:18 | REAL | docs/perf/breadth.md header: "ffmpeg 7.1.5-0+deb13u1 — VLC 3.0.23 is absent from this sandbox … ffmpeg is the speed oracle everywhere"; PERF-VS-VLC.md:9. |
| 7 | "Single-maintainer Lead — the only committer/reviewer/dependency-approver" | docs/KNOWN-LIMITS.md:19 | REAL-UNPROVEN | git log author pattern ("Z User"/Lead) is consistent; full history attribution not exhaustively checked. |
| 8 | "MD5/comparison digests implemented in-tree from public specs (RFC 1321)" | docs/KNOWN-LIMITS.md:20 | REAL | tools/bench-vs-vlc/src/md5.rs exists; tools/bench-vs-vlc/README.md:20 "MD5 from RFC 1321 (public spec; zero new dependencies)". |
| 9 | "w27 admitted `rust_h264` 0.6.1 … MP4/MKV/TS/ES H.264 decode + play end to end, bit-exact 17/17 vs the ffmpeg 7.1.5 oracle" | docs/KNOWN-LIMITS.md:26 | REAL | docs/dependencies/records/rust_h264-0.6.1.md §D "Classification: bit-exact"; crates/codecs/tests/h264_sw_differential.rs exists with pinned MD5s; worklog.md:2031/2036 "17/17 green"; executed fact: MP4 H.264+AAC plays end to end. |
| 10 | "HEVC software decode: none … ADR-013 (VA-API lane) is a draft — decision pending" | docs/KNOWN-LIMITS.md:27 | REAL | Matrix row codec-video-hevc-software = blocked on all platforms; ADR-013 Status: proposed (draft), decision pending. |
| 11 | "Hardware decode lanes: not landed … no OS-decoder adapter exists" | docs/KNOWN-LIMITS.md:28 | SHALLOW | Runtime activation genuinely not landed; but the clause "no OS-decoder adapter exists" is wrong — VT (macOS) and VA-API (Linux) adapter lanes landed compile-verified (merge 9cff3c47 "hw decode adapters — VT/MF/VA-API lanes, compile-verified"; matrix h264-hw note: "hw lanes landed under crates/codecs/src/video/h264/hw/"); Media Foundation does not exist (matrix note records the OF-5 correction). |
| 12 | "ADR-010 is proposed, pending counsel sign-off" | docs/KNOWN-LIMITS.md:29 | REAL | docs/adr/ADR-010-codec-patent-shipping.md:3 "Status: proposed (drafted 2026-11-09…". |
| 13 | "Opus: no pure-Rust decoder … WebM/YouTube-class audio unplayable" | docs/KNOWN-LIMITS.md:30 | FALSE | Matrix row codec-audio-opus at HEAD: `implemented` on windows/linux/macos (flipped 2026-11-09, issue #7, via ruopus 0.1.2 admission, opus_corpus_differential 2/2 — evidence string names ruopus-0.1.2.md). The row was stale the day KNOWN-LIMITS was compiled (2026-11-10). |
| 14 | "DTS/WMA: no decoder; TrueHD young-crate; AC-3 self-build" | docs/KNOWN-LIMITS.md:31 | REAL | Matrix rows codec-audio-dts, codec-audio-wma, codec-audio-truehd-mlp, codec-audio-ac3-eac3 all `blocked`. |
| 15 | "rav1d runs asm-OFF … costs ~0.29× vs libdav1d full-asm" | docs/KNOWN-LIMITS.md:32 | REAL | docs/perf/breadth.md:102 "honest-gap ratio 0.292× (… asm-off purity cost)"; rav1d-1.1.0.md record exists. |
| 16 | "MJPEG 0.47–0.69× ffmpeg wall; lossy audio 2.3–4.6× ffmpeg CPU" | docs/KNOWN-LIMITS.md:33 | REAL | breadth.md §3 ratios 0.477/0.539/0.472/0.685; §2 lossy-vs-ffmpeg 2.3–4.6× range. |
| 17 | "Open path: local files, every form … the v0.1.2 Windows open blocker (no local file opened by any path) is gone" | docs/KNOWN-LIMITS.md:36 | REAL-UNPROVEN | Code + worklog: apps/cli/src/demux_host.rs routes local + `file:` URL opens; worklog w30.1/w30.2 records the fix; executed Linux open runs green end-to-end. Windows-specific behavior unverifiable from this sandbox (no Windows host). |
| 18 | "HLS/DASH/RTSP URL sources: typed-refused …; http(s) progressive/ranged OPENS since w31.3 … `product.toml` exposes `streaming-http` = true" | docs/KNOWN-LIMITS.md:37 | REAL | product.toml:157-160 exact flags verified; demux_host.rs:427-433 `"http" | "https" => open_http_source` + typed refusal for other schemes ("the {scheme} streaming lane is not implemented"). |
| 19 | "OS-webview shell: real and shipped … w30.7 the ~30 ms pacer (`ShellRuntime::pump_once`) … deep verdicts remain manual" | docs/KNOWN-LIMITS.md:50 | REAL-UNPROVEN | pump_once real (apps/desktop-tauri/src/app.rs:202); crash-face/pacer decisions in worklog w30.6/w30.7. "Shipped" artifacts are GitHub-side (unverifiable); the row honestly keeps the deep verdicts open. |
| 20 | "Audio output: null backend only … cpal is NOT admitted … no sound on any OS yet" | docs/KNOWN-LIMITS.md:51 | REAL | crates/audio-output/src/cpal.rs:43 `AUDIO_ADR005_ADMITTED: bool = false`; lib.rs builtin_backends = [NullBackend, UnsupportedBackend]; worklog.md:2294-2296 "audio output remains the null device engine-wide". |
| 21 | "GUI video display: not wired — no `<canvas>` exists in `ui/src`" | docs/KNOWN-LIMITS.md:52 | FALSE | ui/src/components/player/MediaSurface.tsx renders a real `<canvas>` (element at ~:174) with a 2D putImageData presenter (presentLuf1Frame, ~:225-227); PlayerScreen.tsx:97/1062/1098 mounts MediaSurface; commit e983c683 (w31.4): "a 2D putImageData presenter in the media surface … so the GUI displays the engine's presented frames end to end". The row died one day after the w30.10 refresh. Runtime pixels remain UNVERIFIED (no display in sandbox) — but "no canvas exists in ui/src" is flatly false. |
| 22 | "TLS: seam active, real handshake unproven in-tree … out-of-band measurement recorded" | docs/KNOWN-LIMITS.md:53 | REAL-UNPROVEN | ureq-3.5.0.md record exists; no network egress in this sandbox to re-prove a handshake. |
| 23 | "CI billing blocked — Full matrix, RC builds, 3-OS runners all blocked (~670 h/month budget line); every lane re-verified locally instead" | docs/KNOWN-LIMITS.md:86 | FALSE | Contradicted by this same file's §1 (line 17): "Billing was unblocked 2026-11-12 and the release workflow has since executed". The w30.10 refresh struck §1/§2/§4 rows and left §7's billing row stale — in-file contradiction. |
| 24 | "Soak 4 h measurement pending … the 5 MB/h ceiling is [PROPOSAL until first real measurement]" | docs/KNOWN-LIMITS.md:87 | REAL-UNPROVEN | docs/perf/soak-nightly-20261109.md exists; measurement not re-run here. |
| 25 | "19/35 differential inputs re-based … the 2026-10-23 originals are unrecoverable" | docs/KNOWN-LIMITS.md:92 | REAL | docs/corpus/vlc-oracle-run-20261109.json `inputs` block records the re-basing + double-run sha256 randomness proof. |
| 26 | "MSI does not auto-launch after install (Start-Menu + Desktop shortcuts are the entry points; the w30.9 post-install decision)" | docs/KNOWN-LIMITS.md:108-109 | UNVERIFIED | Windows installer behavior; the w30.9 decision is recorded in worklog, but no install was executed in this sandbox. |

## 3. Contradictions

| This doc says | Other source says (path:line) | Which is true (proof) |
|---|---|---|
| §7: "CI billing blocked … every lane re-verified locally instead" (KNOWN-LIMITS.md:86) | §1 of the same file: "Billing was unblocked 2026-11-12 and the release workflow has since executed" (KNOWN-LIMITS.md:17) | §1 is the refreshed (w30.10) row; §7 is the stale leftover. One file asserts both — the register contradicts itself. |
| §2: "Opus: no pure-Rust decoder … unplayable" (KNOWN-LIMITS.md:30) | feature-matrix.toml row codec-audio-opus: implemented windows/linux/macos, evidence "ruopus 0.1.2 … opus_corpus_differential 2/2 pass"; README.md:42 counts 9 audio codecs claimed (opus among them) | The matrix — opus decode is implemented on desktop. KNOWN-LIMITS's row is stale-at-birth (flip 2026-11-09 vs compile 2026-11-10). |
| §4: "no `<canvas>` exists in `ui/src`" (KNOWN-LIMITS.md:52) | ui/src/components/player/MediaSurface.tsx: `<canvas>` element + putImageData presenter; PlayerScreen.tsx mounts it; worklog w31.4 (commit e983c683) | The code — the canvas exists and is statically wired; runtime pixel delivery remains unverified, but the row's stated fact is false. |
| §1: "64/252 … (w27's h264-sw flip makes it 65)" (KNOWN-LIMITS.md:17) | Executed parity-report at HEAD: windows=66/252 linux=61/252 macos=66/252 (audit/runs/baseline/xtask-parity-report.txt) | The current run — the uniform-65 arithmetic never matched this tree. |

## 4. Fabrication/inflation indicators

1. **Stale-at-birth rows**: the Opus row was false when the register was compiled (2026-11-10) — the compile pass did not reconcile against the matrix it calls "status truth".
2. **One-day decay**: the w30.10 refresh (2026-11-15) added the "GUI video display: not wired" row as a *current* limit; wave w31.4 (same repo, next day) wired the canvas. The register's cadence cannot keep up with the code, and the w30.6/w30.7/w30.8 strike-throughs show the file oscillates between under-claiming and over-claiming.
3. **Selective refresh**: w30.10 struck rows in §1/§2/§4 but left §7's "CI billing blocked" standing — a partial truth pass that manufactures internal contradiction instead of eliminating it.
4. **The "65" arithmetic**: presenting a single uniform per-platform number ("w27's h264-sw flip makes it 65") when the matrix's three desktop columns diverged (66/61/66) hides the linux-column regression behind a symmetric fiction.
