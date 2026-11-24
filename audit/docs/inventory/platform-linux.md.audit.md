# CLAIM AUDIT: docs/inventory/platform-linux.md

| Field | Value |
|---|---|
| Auditor | W18-A10 |
| Date | 2026-11-24 |
| Claims extracted | 24 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 2 / 0 / 0 / 0 / 1 / 12 |
| Internal contradictions found | 0 |
| Doc length | 297 lines; evidence date 2026-10-19 (Task 0.C5-a) |

## 1. Blunt verdict

This is an honest, well-hedged external-evidence inventory, and it is the
best-cited doc in the inventory family: method limitations are disclosed up
front (no WebSearch/WebFetch, curl-only, GitHub quota exhaustion, GitLab
anti-bot), 9 `[VERIFY]` and 1 `[PROPOSAL]` markers name exactly what was
NOT confirmed, and every load-bearing local fact I could re-execute checks
out (apt versions, OS, the VLC module dump it cites). Its one systemic
weakness: ALL raw evidence lives in git-ignored `tmp/linuxaudit/` which is
NOT in the tree — every web/crates.io/upstream-issue claim is
non-reproducible from the repo and rests on a one-day curl session's
word. One claim has aged into falsehood against the current distro mirror
(WebKitGTK security micro-release), which is honest drift, not deception.
The downstream decisions it fed (ADR-001/003/004/005, and ultimately
ADR-013's VA-API lane) all exist and are consistent with its evidence.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "this sandbox (Debian 13 trixie, x86_64, headless) has no WebSearch/WebFetch tools; all web evidence gathered via `curl`" | :5 | REAL | Same sandbox class confirmed: `/etc/os-release` "PRETTY_NAME=Debian GNU/Linux 13 (trixie)"; `tmp/linuxaudit/` absent from tree (as stated: git-ignored). |
| 2 | "On Linux … the only production webview is WebKitGTK" | :7/:13 | UNVERIFIED (external) | No in-tree raw evidence; consistent with the repo's ADR-004 deferral (apps/desktop/Cargo.toml:11-16 "NO tauri/wry import (the honest deferral)"). |
| 3 | "Candidate B (frame streaming to canvas) is structurally safer" | :7 | REAL (as adopted) | apps/desktop/Cargo.toml:7-9 "v0 is the ADR-003 variant-B evidence core: the frame-streaming wire protocol (src/wire.rs)…" — the repo took the doc's safer path. |
| 4 | "VA-API is the only real hw-decode API (libva 2.22 …)" | :7/:137 | REAL (local half) | `apt-cache policy libva2` → "Installed: 2.22.0-3 / Candidate: 2.22.0-3" — byte-matches "libva2 2.22.0-3". |
| 5 | "every Rust binding (cros-libva lineage) wraps the same C ABI" + binding table (§3.3) | :150-160 | UNVERIFIED (external) | crates.io table not reproducible in-tree; consistent with repo's later choice of hand-rolled FFI: crates/codecs/src/video/h264/hw/vaapi/{dl.rs,ffi.rs,library.rs}. |
| 6 | "No pure-Rust VA-API exists or is possible (the API *is* a C ABI)" | :162 | REAL (analytic) | The repo's ADR-013 lane is exactly a C-ABI FFI over dlopen'd libva — the doc's framing predicted the implementation shape. |
| 7 | "Debian trixie: pipewire 1.4.2, wireplumber 0.5.8" | :138-143/:185 | REAL | `apt-cache policy pipewire` → Candidate 1.4.2-1; wireplumber → 0.5.8-2 (micro-bump). |
| 8 | "Debian trixie ships `gstreamer1.0-vaapi 1.26.2-1`" | :110 | REAL | `apt-cache policy gstreamer1.0-vaapi` → "Candidate: 1.26.2-1". |
| 9 | "Debian 13 trixie | 2.52.6 (main) / 2.54.0 (security)" | :44 | FALSE (aged) | Current mirror: main 2.52.6-1~deb13u1 unchanged, security now 2.54.0-**1~deb13u3** (apt-cache policy libwebkit2gtk-4.1-0). The security column drifted two micro-releases since 2026-10-19 — expected mirror drift, not deception; the doc's floor analysis (≥2.44–2.46) is unaffected. |
| 10 | "wry 0.57.0 … webkit2gtk =2.0.2 … GTK3 / WebKitGTK 4.1 API, not 6.0" | :56-57 | UNVERIFIED (external) | No wry/tauri/webkit2gtk anywhere in the repo's lockfiles (grep 0 hits in Cargo.lock; apps/desktop imports none) — nothing in-tree corroborates or contradicts. |
| 11 | "Child webviews (`build_as_child`) are Linux-X11-only in wry 0.57" | :59 | UNVERIFIED (external) | Same as #10; this fed the Candidate-A-weak-on-Wayland verdict (§6.1). |
| 12 | "WebCodecs … **verified in-binary** … 79 'WebCodecs' strings" | :109 | UNVERIFIED (in-repo) | Raw evidence `tmp/linuxaudit/wk-strings.txt` absent (git-ignored per :285-294); libwebkit2gtk not installed in this sandbox (Installed: none) so the strings probe cannot be re-run locally. Method (download → dpkg-deb -x → strings) is sound and fully described. |
| 13 | "WebCodecs `prefer-hardware` honored since 2.48" (2.48 release notes) | :110 | UNVERIFIED (external) | Release-note citation; not reproducible in-tree. |
| 14 | "VLC 3.0.23 on Debian trixie ships exactly this architecture — `05-vlc-list.txt` (0.A2 dump) lists modules `vaapi`, `vaapi_drm`, `vaapi_filters`, `glconv_vaapi_x11`, `glconv_vaapi_wl`, `glconv_vaapi_drm`" | :176 | REAL | `grep` on docs/baselines/raw/linux/05-vlc-list.txt: `:81 vaapi`, `:82 vaapi_drm`, `:149-151 vaapi_filters`, `:293 glconv_vaapi_x11`, `:305 glconv_vaapi_wl`, `:313 glconv_vaapi_drm` — every named module present, plus vdpau family at :142-148. Fully verified in-tree. |
| 15 | "VDPAU … bridge `vdpau-va-driver` is **gone from trixie**" | :170 | REAL | `apt-cache policy vdpau-va-driver` → "Installed: (none) / Candidate: (none)" (exit 0) — the package is indeed absent from trixie. |
| 16 | "wgpu 30.0.1 … pure-Rust … Vulkan first-class on Linux" | :207 | UNVERIFIED (external) + MOOT | No wgpu in Cargo.lock at all; renderer-wgpu ships std-only with the software compositor and the wgpu import deferred (crates/renderer-wgpu/Cargo.toml:5-16 "the wgpu import is a recorded Lead decision (… the deferral record with the evidence)"). The doc's GPU facts fed an import that has not happened. |
| 17 | "cpal 0.18.2 has native ALSA/JACK/PipeWire/PulseAudio backends" | :188 | UNVERIFIED (external) | cpal not in any lockfile; repo never adopted it. |
| 18 | "only the `pulseaudio` crate is pure Rust" (audio client) | :7/:187 | UNVERIFIED (external) | Repo's actual ADR-005-adjacent choice: crates/platform-linux/src/linux/pulse_ffi.rs "hand-rolled `extern \"C\"` + runtime dlopen (no libpulse link-time" (Cargo.toml:15) — the doc's dlopen-vs-crate framing, not its pure-Rust candidate. |
| 19 | "no ADR decisions made here (evidence only)" | :6 | REAL | docs/adr/ contains ADR-001/003/004/005 (all post-date the doc's feed role) + ADR-013-va-api-hw-decoder.md — decisions recorded elsewhere, as claimed. |
| 20 | 9 `[VERIFY]` markers / 1 `[PROPOSAL]` | :5-258 | REAL (counted) | `grep -c '\[VERIFY\]'` → 9; `grep -c '\[PROPOSAL\]'` → 1 — the doc's uncertainty budget is exactly as large as it prints. |
| 21 | "Raw evidence … All under git-ignored `tmp/linuxaudit/` (not committed)" | :285-294 | REAL (and the doc's key weakness) | Directory absent — by design. Consequence: zero external claims in this doc are reproducible from the repo; the only in-tree evidence is the 0.A2 VLC dump (#14) and the local apt facts. |
| 22 | Distro availability matrix (Ubuntu/Fedora rows, §1.1.1) | :42-50 | UNVERIFIED (external) | Launchpad/API-sourced; Fedora/Arch honestly marked "[VERIFY … not queried this run]". |
| 23 | Issue-evidence table (tauri#14924, wry#1731, … §1.3) | :67-77 | UNVERIFIED (external) | GitHub issue numbers/states not re-checkable (no network); table is internally consistent with the §6.5 risk rows that cite it. |
| 24 | "libva2 + drm/x11 variants **installed in this sandbox**" | :137 | REAL | `apt-cache policy libva2` shows Installed 2.22.0-3 (present); consistent. |

## 3. Contradictions

None found. The doc's recommendations align with what the repo later did
(variant-B desktop core, dlopen'd VA-API FFI lane, no tauri/wry import
pending ADR-004). The single false item (#9) is mirror drift after the
doc's evidence date, self-disclosed as a dated snapshot ("all URLs accessed
2026-10-19").

## 4. Fabrication / inflation indicators

None material. The doc understates rather than inflates: 9 explicit
[VERIFY] admissions, method honesty up front, quota/anti-bot limits
recorded. The one structural critique: the evidence-inventory genre means
~60% of its claims (rows 2, 5, 10-13, 16-18, 22-23) are non-reproducible
from the repository — a reader must trust a dead tmp/ directory. If
devil-audit standards were applied to evidence docs the way they are to
code, this doc would be REAL-UNPROVEN at the file level, not REAL.

## 5. Matrix rows

Not a matrix chunk — no rows. Downstream couplings verified instead:
ADR-001 (hw decode) + ADR-003 (variant B, adopted in apps/desktop) +
ADR-004 (os-webview deferral, consistent with zero tauri/wry in lockfiles)
+ ADR-005 (audio; the repo's pulse dlopen crate is UNREACHABLE per W8's
prior audits: audit/files/crates/platform-linux/src/linux/*.audit.md —
"platform-linux has zero dependents").

## Commands run

`apt-cache policy libva2 / libwebkit2gtk-4.1-0 / pipewire / wireplumber /
gstreamer1.0-vaapi / vdpau-va-driver` (all exit 0, outputs quoted above);
`cat /etc/os-release`; `grep` over docs/baselines/raw/linux/05-vlc-list.txt
(vaapi/glconv/vdpau modules); `grep -c '[VERIFY]'` → 9; Cargo.lock greps
(wry/tauri/webkit2gtk/wgpu/cpal: 0 hits); `ls tmp/linuxaudit/` → absent.

## Open questions for the next agent

- Row #12 (WebCodecs in-binary proof): re-run the download+strings probe
  on a networked machine and commit a digest — it is the load-bearing fact
  for ADR-003's B-path and currently exists nowhere reproducible.
- The security-column drift (#9) suggests the distro-matrix tables carry
  no refresh cadence; consider a dated re-probe before any ADR-004
  admission decision cites them.
