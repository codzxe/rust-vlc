# Troubleshooting

Causes are stated with their in-tree evidence. Every error below was
reproduced in the reference sandbox (x86_64 Linux, Intel Xeon,
`uname -m` = `x86_64`; 2026-11-10) unless marked otherwise.

## Playback

**"error: [probe.unsupported]: unsupported … (probe.no-demuxer)"**

You opened a file whose format is not in the claimed set. The CLI
sniffs the first bytes, finds no demuxer and fails with a typed error
naming the code, the sniffed bytes and the probe hints (extension,
size) — for example a `.txt`:

```
error: [probe.unsupported]: unsupported (sniffed 6e 6f 74 ...) (probe.no-demuxer): probe hints: extension=txt, size=10
  code: probe.no-demuxer
```

Check whether the format has a claimed row in
`docs/feature-matrix.toml` (37 of 73 rows claimed on desktop as of
2026-11-10) and a record in `docs/formats/` for its known gaps.
Unsupported is a decision, not an accident: blocked rows carry their
reason and next step there.

**The file plays but I hear nothing / see no window**

Expected in the current phase: on a default build the audio path goes
to the **null backend** (instrumented with sample/frame counters —
Devil-Fix wave FX3, 2026-11-25) — Devil-Fix wave FX4 (2026-11-26)
wired the shipped **CLI host** to the backend factory (the cpal
adapter when built with `backend-cpal`, the instrumented null backend
otherwise; `--audio-backend`/`--no-audio` selection — FX4-A01/A06)
and the volume/mute/balance/EQ verbs (FX4-A02), but the
desktop/tauri GUI audio lanes are **pending verification** (FX4-A03/A04)
and audibility on real hardware is **[VERIFY] pending the Windows CI
run of the `audio-cpal-check` lane + a Windows release smoke** — and
a `lumen` CLI session opens **no video window** by design (the CLI
lane is headless; `decode-bench` is the frame-count oracle). The
GUI's video lane is real in code and instrumented since Devil-Fix
wave FX6 (2026-11-26, FIX-PLAN §3 F1.2): the canvas exists
(`ui/src/components/player/MediaSurface.tsx:342`, `putImageData` —
wave-15 finding 1), the mailbox is bounded with a documented drop
policy, timestamp passthrough and produced/dropped counters (FX6-A01),
the 10 ms ticker ships policy-aware with the counters surfaced
(FX6-A02), the tauri frame channel is bounded with
shipped/dropped-by-overflow counters (FX6-A03), and the UI surfaces the frame-lane counters the
webview observes (presented/skipped; FX6-A05) — but frames
actually watched by a user
on Windows hardware are **[VERIFY] pending the Windows GUI run**
(smoothness and 4K starvation included; `docs/KNOWN-LIMITS.md` §4).
The engine really demuxes/decodes/synchronizes — watch the
event stream, `time` and `info` in the CLI, and in the GUI the
surfaced lane counters (presented/skipped; FX6-A05). This is the
honest state,
tracked in `docs/KNOWN-LIMITS.md` §4.

**A claimed format still fails**

Run `lumen decode-bench <file>` — it decodes to null and prints one
`BENCH` line with per-track frame counts. If `v_decoded`/`a_decoded`
are lower than the container's frame count or the command errors,
you have hit a real gap: every format record lists its known limits
(e.g. H.264/HEVC are partial — hardware-seam only, no software
decode; ProRes/DNxHD partial; ASS styling subset). Check
`docs/formats/<record>.md` "Known gaps" before reporting.

**Subtitles show mojibake**

Charset detection is BOM-class only (UTF-8 / UTF-16LE / UTF-16BE);
BOM-less files fall back to UTF-8-then-Windows-1252 with a typed
damage event, and BOM-less UTF-16 is deliberately not sniffed (it
degrades as damage, never misdetects). Full subsdec-class heuristics
are not implemented (`subtitle-encoding-detection` = partial).
Convert the file to UTF-8 with a BOM.

## Build

**`cargo build` fails with a lockfile error**

The workspace builds `--locked`; after any dependency change the
committer must commit a fresh `Cargo.lock` (release row in
`docs/release-checklist.md`). Pull and rebuild.

**`npm run build` in `ui/` fails with syntax errors in `src/App.tsx`**

Known defect on `main` as of 2026-11-10: merge commit `45c5c6f`
committed unresolved conflict markers into `ui/src/App.tsx` — two
regions, the screen imports (~line 15) and the route table (~line 100)
(`docs/audit/docs-sweep-20261110.md` finding F-1). It breaks the
production web build (`tsc` + `vite build`), the dev server (the
module transform fails with "Encountered diff marker" — the browser
shows the Vite error overlay) and three vitest files. Fix: resolve
both regions keeping BOTH sides — the `/discovery` AND `/convert`
imports and routes (the merge commit's own stated intent
"i18n/App keep discovery+convert") — then re-run `npm run build`.
Until it lands upstream, the UI is unusable in every mode; use the
CLI and watch the audit report for the fix.

**`vitest` reports a settings-drift failure ("schema.ts is stale")**

Known defect on `main` as of 2026-11-10: the generated settings
screen data is stale vs `schemas/settings.toml`
(`docs/audit/docs-sweep-20261110.md` finding F-2). Fix: `npm run
settings:generate` in `ui/` and commit the result (settings-lane
owned).

**Playwright says the browser executable doesn't exist**

The pinned `@playwright/test` 1.64.1 expects `chromium_headless_shell-1248`;
the reference sandbox ships `chromium-1200` /
`chromium_headless_shell-1200` and browsers are never downloaded
there. The RC smoke harness works around this by launching the
pinned sandbox chromium via an explicit `executablePath`
(`scripts/rc-smoke.sh` web row). On a fresh machine use
`npx playwright install chromium` once (the
`ui/playwright.config.ts` instruction).

## Devices

**No audio device appears / device switching does nothing**

Device *discovery* landed in wave 23 (hotplug events, `/discovery`
UI, issue #42). *Output* on a default build is still the null backend
— enumeration without audible sink — **updated 2026-11-26 (Devil-Fix
wave FX4, agent FX4-A07)**: the shipped **CLI host** now routes
output through the backend factory — the cpal adapter when built with
`backend-cpal`, the instrumented null backend otherwise (FX4-A01;
`--audio-backend`/`--no-audio` selection flags, FX4-A06) — and the
volume/mute/balance/EQ verbs are wired in the CLI lane (FX4-A02,
closing wave-07's S2 "the entire user-facing volume/EQ/mute/balance
surface is dead" finding there). The desktop/tauri GUI lanes are
**pending verification** (FX4-A03/A04), and audibility on real
hardware — including what `audio.set-device` would switch you to — is
**[VERIFY] pending the Windows CI run of the `audio-cpal-check`
lane + a Windows release smoke**. Known and tracked; see
`docs/KNOWN-LIMITS.md` §4 and the device-discovery merge record
(`d5611d1`).

**Hardware decode seems inactive**

It is: `features.hw-decode = false` in `product.toml` (rule F2 — the
adapter lanes are compile-verified, the per-OS integration gate has
not run; ADR-001). `playback.hw-decode-policy` therefore stays inert
at `software-only`. `engine-info`'s decoder list derives from the
CLI's single decoder dispatch table (14 codec families — the one
registry): `engine-info`, `decode-bench` and the `landed:` refusal
string all report from it. Before that single-sourcing the list was a
hardcoded two-entry understatement (mjpeg/pcm_s16le) while the same
binary decoded H.264 + AAC end to end — the audit's four-registry
story (engine-info 2 · refusal string 12 · bench "none" · dispatch
16 arms/14 families; audit/final/CONTRADICTIONS.md C-002; W20-A02
F-3, wave-13).

## Reporting

Bugs carry reproduction steps + expected/actual; the repo's rules are
in `AGENTS.md` (every defect gets a regression test). The full
honest-limits register is `docs/KNOWN-LIMITS.md`.
