# Docs completeness sweep — card 8.G1 (issue #58)

| Field | Value |
|---|---|
| Card | 8.G1 — "Docs completeness sweep: format records complete, ADR register current, user docs + troubleshooting, README claims audited (no 'all formats' claims anywhere). AC: audit checklist all green" (doc 13 §11 wave 8.G; issue #58 task list) |
| Date / base | 2026-11-10, wave 24 lane E, branch `w24/e`, base `c339ee8` (origin/main) + this lane's commits `492db70`…`bc97962` |
| Method | Every claim below was checked against in-tree evidence with re-runnable commands (§7 lists them with recorded outputs). Nothing was verified from memory. Statuses are never changed by an audit — all flips are the parity tool's business (MP §15); this document records docs-state only. |
| Verdict | **ALL GREEN for the 8.G1 docs scope.** Two defects found on `main` during the sweep (F-1/F-2) are **code defects owned outside lane E** — recorded here, cross-referenced from README/rc-smoke/troubleshooting, fixed on lane A's branch (unmerged at sweep time). They do not gate this checklist's rows (docs scope) and are tracked to closure at wave-24 merge. |

## 1. Format records vs `docs/feature-matrix.toml` — COMPLETE

**Criterion** (from the Phase-5 records audit `docs/gates/phase5-records-audit.md`
§1, the recorded precedent): every matrix row in the format categories
(container, video-codec, audio-codec, subtitle-format) is either covered by
a `docs/formats/<record>.md` carrying the row's evidence, or is a
`not_started`/deferred row that owes no record, or is a blocked row whose
decision trail (feasibility μADR / ADR / sibling record) is the record.
Playlist formats follow their own recorded convention (§1.5 below).

**Counts (recomputed 2026-11-10 from the matrix itself):** 73 format rows =
21 container + 16 video-codec + 19 audio-codec + 17 subtitle-format;
**37 claimed on desktop** (implemented or partial: 14 / 6 / 9 / 8), **0
claimed on web**. `docs/formats/` holds **53 records** (52 at sweep start +
`streaming-dash.md` authored to close F-5, commit `bc97962`).

### 1.1 Claimed rows (implemented/partial) — all have records

Every one of the 37 claimed format rows resolves to a record. The mapping
one-to-many where records cover several rows (`audio-mp3-mp2.md` covers
codec-audio-mp3 + -mp2 + container-mp3-raw; `video-vp8-vp9.md` covers both
VP8 and VP9 rows' decision state; `container-es.md` covers
container-mpeg-es-video's landed siblings h264/hevc-annexb; `sub-srt.md`
is the declared record for `subtitle-encoding-detection` by its own
front-matter; `sub-cc.md` consolidates `subtitle-cc-608-708`). Records
authored by this lane to close the sweep's own findings are marked ★:

| Category | Claimed rows | Records |
|---|---|---|
| container (14) | matroska-webm, mp4-mov-3gp, ogg, mpegts, mpegps, wav, aiff, adts-aac, asf-wmv, avi ★, mp3-raw, h264-annexb, hevc-annexb, ivf | container-matroska, container-mp4, container-ogg, container-mpegts, container-mpegps, container-wav, container-aiff, container-adts, container-asf, container-avi ★ (`d915188`), container-es (×3 rows), container-ivf |
| video-codec (6) | mjpeg, av1, h264, hevc, prores-dnxhd, h264-software ★ | video-mjpeg, video-av1, video-h264, video-hevc, video-prores-dnxhd, video-h264-software ★ (`3a7fd06`) |
| audio-codec (9) | pcm ★, mp3, mp2, aac-lc, flac, vorbis, opus, alac, adpcm | audio-pcm ★ (`d915188`), audio-mp3-mp2, audio-aac, audio-flac, audio-vorbis, audio-opus, audio-alac, audio-adpcm |
| subtitle-format (8) | srt, ass-ssa, vtt, dvb-subs, vobsub, pgs ★, cc-608-708 ★, encoding-detection ★ | sub-srt (declared dual-row record, encoding cross-link added `3a7fd06`), sub-ass-ssa, sub-vtt, sub-dvb, sub-vobsub, sub-pgs ★ (`d915188`), sub-cc ★ (`3a7fd06`) |

Non-format lanes that follow the record convention: network-hls →
`streaming-hls.md`, network-http-client → `network-http.md`, and as of
F-5's closure network-dash (partial) → `streaming-dash.md` ★ (`bc97962`).

### 1.2 Rows with no record owed — the register (each re-checked 2026-11-10)

| Row(s) | Status today | Why no record is owed |
|---|---|---|
| container-au, -realmedia, -archive-zip, -census-long-tail | not_started (all four columns) | Nothing landed; matrix `notes` carry the deferral ("5.A2 sibling; trivial", "Legacy; no v1/v-next card", …). Records start when work starts. |
| container-flac-native | not_started | Native-carriage path undecided (G-6, phase5 audit §3); the codec is fully recorded in `audio-flac.md`. |
| container-mpeg-es-video | not_started | `container-es.md` states the m1v/m2v demux is not implemented (its Annex-B siblings are). |
| codec-video-raw-fourcc | not_started | `later` per 5.P2b-I (phase5 audit §1.2); no code, no record. |
| codec-audio-census-tail, subtitle-census-tail, plfmt-census-tail | not_started | Census-tail group rows — long-tail inventory families with no work started. |
| subtitle-microdvd, -subviewer, -sami, -ttml, -forced, -autoload | not_started | Phase 5–8 tails (phase5 audit §1.4), still untouched. |
| codec-video-hevc-software | blocked | Covered inside `video-hevc.md` ("no software HEVC decode, and none is scheduled…" — verified lines 11–14). |
| codec-video-legacy-blocked | blocked | `video-screen-codecs.md` is its umbrella record (Cinepak/Hap/Dirac/MS Video 1-class, verified §"legacy"). |
| subtitle-teletext | blocked | zvbi is C (forbidden) — matrix `blocked_reason`; the demux-side split is recorded in `container-mpegts.md` line 109; no decode will land to record. |
| subtitle-kate | blocked | `subtitle-kate.md` is the decision record (5.P2b-I part B). |
| platform-disc-dvd family | blocked | `disc-dvd-video.md` is the decision record (CSS/legal, μADR, ADR-008-not-open per the ADR register). |
| network-auth, -reconnect-buffering, -remote-playlists | not_started | No landed surface; matrix `evidence` empty (verified this sweep). |

### 1.3 Playlist-format convention (unchanged, still honest)

`plfmt-pls` / `-xspf` / `-cue` (implemented) and `plfmt-m3u-m3u8` (partial)
carry their evidence in the matrix rows + wave-4/5 worklog per the recorded
convention ("no docs/formats record owed (doc 13 5.O* Owns name none) —
matrix + worklog are the surface", phase5 audit §1.5). Re-verified: the four
rows' evidence fields resolve to crates + tests, not to missing files.
`plfmt-asx` / `-b4s` remain not_started.

**§1 result: GREEN.** 37/37 claimed rows covered; the no-record-owed
register accounts for every remaining format-category row; the one gap
found (F-5, network-dash) was closed inside this sweep by authoring
`streaming-dash.md` from the recorded matrix evidence — statuses untouched.

## 2. ADR register — CURRENT

`docs/adr/README.md` (authored this lane, commit `492db70`) indexes all
**12 files** in `docs/adr/` (11 ADRs + the ADR-002 evidence pack), quoting
each `Status:` line verbatim, with dates and implementation cross-links.
Re-verified this sweep: `ls docs/adr/*.md` = ADR-001…006, 010, 011, 012,
013 + README + ADR-002-evidence — matches the register exactly. The
register records (and this sweep confirms still true on `main`):

- **Numbering gaps:** ADR-007/008/009 never opened; ADR-008 is cited as a
  future owner (optical-disc scope, WebCodecs) and ADR-009 once
  (`docs/09-web-wasm-target.md`) — citations of a planned ADR, not files.
  Opening them is a Lead decision; the register does not invent them.
- **Drift flags (honest):** ADR-010 (patent policy) still `proposed` —
  it is a blocking sign-off row in `docs/release-checklist.md`; ADR-013
  (VA-API) still `proposed` while wave-23 landed compile-verified VA-API
  lanes — flagged in the register with the Lead next-step (ratify or
  record the deviation). ADR-004's per-OS webview verdicts remain PENDING
  (`apps/desktop/tauri/INFRA-BLOCKED.md`).

**§2 result: GREEN** — register current, drift flagged not hidden.

## 3. User guides — REAL (each verified against its source of truth)

Guides authored this lane (commit `559b897`, fixes `9a0a380`/`9cd090f`),
then re-verified for this audit:

| Guide | Source of truth | Verification (command / evidence) |
|---|---|---|
| `docs/guides/getting-started.md` | the CLI surface + fixture | `lumen --help` command list (rendered from the settings schema + command catalog, `apps/cli/src/help.rs`); the documented `decode-bench` output shape matches `scripts/rc-smoke.sh` check 4's `BENCH` parser (same `v_frames/v_decoded/...` fields); §3's 37/73/0 numbers recomputed today from the matrix (§5.2); the version paragraph matches the real two-track mechanism (`--version` = crate version, `apps/cli/Cargo.toml`; product version `0.1.0-dev` lives in `product.toml` and drives packaging — verified in `help.rs` + `scripts/release/product-env.py`) |
| `docs/guides/settings.md` | `schemas/settings.toml` (32 `[[setting]]` entries, counted this sweep) + `schemas/generated/` | Every documented key spot-checked against the schema; the extension-host crash-isolation paragraph lists the **five** `extensions.crash-*` keys that exist (`crash-backoff-initial-ms`, `-multiplier`, `-max-ms`, `crash-restart-limit`, `crash-disabled-reenable-ms` — `rg 'key = "extensions\.crash'` = 5 hits); fixed in-sweep from a wrong "trio" (`9cd090f`, F-4) |
| `docs/guides/shortcuts.md` | `keymaps/default.toml` | 150 bindings counted (`rg -c '= \{ key = '` = 150 — the guide's number); the `/keyboard-shortcuts` route exists (`ui/src/App.tsx`); the generated data module `ui/src/screens/settings/areas/hotkeys/generated/bindings.ts` exists and is drift-gated (`npm run hotkeys:check` in `ui/package.json`) |
| `docs/guides/troubleshooting.md` | real failure modes | Rows for F-1 (`ui/src/App.tsx` committed merge markers — reproduction: `npm run build` in `ui/` fails) and F-2 (stale generated settings screen — fix: `npm run settings:generate`) reference this audit; the Playwright chromium-drift row documents the real sandbox pin mismatch and the `RC_SMOKE_CHROMIUM` override actually implemented by `scripts/rc-smoke.sh` |
| `docs/guides/README.md` | index | Lists exactly the four guides + their sources of truth. |

Environment-misrecord fixes applied during the sweep (`9a0a380`, F-3):
getting-started/troubleshooting previously recorded the sandbox as
aarch64 — `uname -smr` says `x86_64`; corrected, and the stale bench
figure refreshed from the guide's own current `BENCH` example.

**§3 result: GREEN.**

## 4. README claims audit — overclaims grepped, fixed, listed

### 4.1 Overclaim greps (re-run 2026-11-10, post-fix)

Patterns grepped across `README.md` + `docs/` (excluding gates/logs,
baselines raw, corpus, perf/raw, and the archived worklog — all are
records of past states or quoted baseline material, not live claims):
`all formats`, `every format`, `all codecs`, `supports everything`,
`full parity`, `100% compatible`, `complete compatibility`,
`production-ready`, `blazingly`.

Result: **zero user-facing overclaims.** Every remaining hit is one of:
the rule itself being quoted (KNOWN-LIMITS §8 "All formats supported"
claims of any kind are prohibited, R-26; the 8.G1 card text), VLC's own
option prose inside the pinned inventory JSON (`options-linux.json` —
baseline material), plan/process language in doc 13/doc 04 about what a
card must do, or archived worklog history.

### 4.2 Overclaims fixed by the truth pass (commit `559b897`)

1. **"Prebuilt binaries are published on the project's GitHub Releases
   page"** — FALSE at the time (no tag ever pushed; CI billing blocked,
   worklog 7.CLOSEOUT). Replaced by the honest Install section: "No
   release has been published yet", pipeline-validated-by-construction
   wording, and build-from-source as the only supported path.
2. **Stale Status block** ("Phase 6 — substantially complete", a
   2026-11-09 issue-count narrative) — replaced with the current
   phase-by-phase state including Phase-7 wave-20–23 merge evidence, the
   Phase-6 exit verdict PENDING LEAD DECISION (`docs/gates/phase6-exit.md`
   §5 — verified), and the "what plays today" paragraph.
3. **Unbounded capability implication** — the old status prose implied
   broad playback with no per-format accounting. The README now states
   the exact parity numbers: **37 of 73 format rows claimed on desktop,
   zero on web**, audio output is the ADR-005 null device, the desktop
   shell is the ADR-004 skeleton — each with its doc link.
4. The false-claims guard sentence ("Nothing is claimed implemented or
   verified until `docs/feature-matrix.toml` records automated
   evidence") was kept and now sits next to numbers that obey it.

### 4.3 README numbers recomputed today (the method, re-runnable)

Parse `docs/feature-matrix.toml` rows; count rows whose `category` ∈
{container, video-codec, audio-codec, subtitle-format} (= 73); of those,
count rows whose `linux` status ∈ {implemented, partial} (= 37); count
rows whose `web` status ∈ {implemented, partial, verified} (= 0). The
per-category split (14/6/9/8) and the named-format lists in
getting-started §3 were cross-checked against the same parse — including
naming the 37th row (subtitle encoding detection) so the enumeration is
exact (guide tweak in `bc97962`).

**§4 result: GREEN.**

## 5. Findings register (this sweep)

| ID | Finding | Disposition |
|---|---|---|
| F-1 | Committed merge-conflict markers in `ui/src/App.tsx` (merge `45c5c6f`, wave 23) — `npm run build` in `ui/` fails; blocks the web half of RC smoke | **Code defect, owned outside lane E.** Fix verified on lane A's local branch `w24/a` (commit `70ed69c`, "resolve committed merge markers in App.tsx"); `origin/w24/a` not pushed at sweep time → recorded honestly here, re-run `scripts/rc-smoke.sh` at wave-24 merge. Cross-refs: README Install/Status, `docs/guides/troubleshooting.md`, `scripts/rc-smoke.sh` check-7 note. |
| F-2 | Stale generated settings screen in `ui/` (settings-schema pins drifted after the seed grew 27→32) — `npm run settings:check` fails | Same as F-1: fix on `w24/a` (`771259c`, regenerate + `settings:check` green per its commit). |
| F-3 | Guides environment misrecord (aarch64 vs x86_64) + stale bench line | **Fixed in-sweep** (`9a0a380`) — `uname` evidence. |
| F-4 | Settings guide said "trio" of crash-isolation keys; the schema has five | **Fixed in-sweep** (`9cd090f`). |
| F-5 | `network-dash` (partial) had no `docs/formats` record while sibling network lanes did | **Closed in-sweep** (`bc97962`): `streaming-dash.md` authored from the matrix row's recorded evidence; no statuses changed. |

## 6. Checklist against the card's AC

- [x] Format records complete (§1 — 37/37 claimed rows covered; no-record-owed register complete)
- [x] ADR register current (§2 — `docs/adr/README.md`, 12/12 files indexed, drift flagged)
- [x] User docs + troubleshooting (§3 — four guides, each verified against product.toml/schema/keymaps reality)
- [x] README claims audited — no "all formats" claims anywhere (§4 — greps re-run post-fix, zero hits in live docs)
- [x] Audit checklist all green (§1–§4 results; F-1/F-2 are non-docs defects, owned + tracked to wave merge)

## 7. Commands re-run this sweep (recorded outputs)

- `rg -c '= \{ key = ' keymaps/default.toml` → **150**
- `rg 'key = "extensions\.crash' schemas/settings.toml` → **5** crash-isolation keys
- `rg -c '\[\[setting\]\]' schemas/settings.toml` → **32**
- `rg -c '#\[test\]' crates/streaming/src/dash/*.rs` → 12+4+4+7 = **27** (matrix's "incl. 27 dash")
- `bash -n scripts/rc-smoke.sh` + `node --check scripts/rc-smoke-web.mjs` → **OK**
- `bash -n scripts/release/*.sh` (8 scripts) + `python3 -m py_compile scripts/release/{product-env,gen-icon}.py` → **OK**
- `python3 scripts/release/product-env.py --print --cli-package lumen-cli --desktop-package desktop` → `PRODUCT_VERSION=0.1.0-dev`, `PRODUCT_VERSION_CODE=1`, `MSI_VERSION=0.1.0`, `BIN_CLI=lumen`, `BIN_DESKTOP=lumen-desktop`, exit 0
- `PRODUCT_NAME=Lumen PRODUCT_VERSION=0.1.0-dev sh scripts/release/release-notes.sh /tmp/rc-notes-preview.md` → wrote the dispatch-mode dry-run notes (20-commit range, exit 0)
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` → parses; jobs = windows, macos-arm64, macos-x64, linux-x64, linux-arm64, release
- xtask gates (via the shared cached `target/debug/xtask` binary; disk-full sandbox made `cargo run` unavailable — same binary, recorded here for honesty): `lint-literals` → **PASS, 0 violations, 452 allowlisted**; `check-identity` → **PASS, 0 violations, 453 allowlisted**; `sbom --check` → **no drift** (157 components = 118 registry + 39 workspace/path, subject `Lumen 0.1.0-dev`); `config-validator` → **FAILURE, 4 violations — all the missing `assets/brand/*` files** (icon.ico, icon.icns, icon-512.png, logo.svg), the documented RELEASE.md known-limit #5; carried as a release-checklist row, not a docs defect.
- Matrix recompute (§4.3 method) → 73 / 37 / 0 (+ per-category 14/6/9/8)
- Merge-marker probes: `git show origin/main:ui/src/App.tsx | grep -c '<<<<<<<'` → **2** (F-1 confirmed on main); `git show w24/a:ui/src/App.tsx | grep -c '<<<<<<<'` → **0** (fix confirmed on lane A's local branch; `origin/w24/a` absent at sweep time)

The rc-smoke harness itself (`scripts/rc-smoke.sh`, commit `ff001e5`)
was NOT re-run during this final sweep: its recorded result (engine +
wasm checks green; web checks 7–8 red on main's F-1/F-2) stands, and a
re-run before the wave-24 merge would reproduce exactly the same red —
the harness's check-7 failure note already names F-1/F-2 and the fix
owners. The release checklist makes the post-merge re-run a blocking
row for any RC.
