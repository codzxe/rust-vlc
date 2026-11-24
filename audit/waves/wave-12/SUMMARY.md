# WAVE 12 SUMMARY — plugin architecture, wasmi host, WASM/web target

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 69, 100% covered |
| Findings | S0:1 S1:0 S2:16 S3:~60 S4:~100 |
| Tree purity | clean |

## THE PLUGIN ANSWER (§8.12)
NO shipped build loads ANY plugin. plugin-host-wasm has ZERO consumers
(no app depends on it); product.toml:166 wasm-extensions=false; the 2
committed plugins/extensions/*.wasm load only in tests and dev bins.
The sandbox itself is real and good: wasmi with fuel+memory limits,
fsjail (symlink/traversal escapes denied, tested), real trap isolation,
sha256 at install and load, CRC verified vs python zlib, FIPS-correct
SHA-256. But signatures are never verified (honest Unverified variant)
and the whole lane is parked (Phase 7).

## THE WEB ANSWER (§8.12b)
The shipped wasm32 module is a decoder-less shell: host.rs:477-486 refuses
EVERY decoder ("web.decoders_absent") — NO file can play in the browser on
the shipped artifact (1.23 MiB release / 4.24 MB debug). Containers: AVI
only in test builds. The 2,823-LOC io/ fetch pillar is landed but UNWIRED
(http open still refuses "card 7.H4"). loadBlob = JSON-number-array FFI
(1 call/4 bytes; ~7.5 MiB practical ceiling). No canvas render path in the
web lane (canvas.rs present() is a no-op stub; presentFrame dead-ends at
worker.js postMessage — the repo's ONLY canvas draw is in the DESKTOP
Tauri shell: shell.rs:160-162 + MediaSurface.tsx:198-228, so the master
prompt's "GUI mounts no canvas" pin is BACKWARDS — Wave 14/15 must
adjudicate). Web audio: fully-written AudioWorklet apparatus (node probe
28/28) with zero production constructors — engine audio ends in NullSink
even in the browser.

## Worst findings
1. [S0] web io/mod.rs evidence label cites a "fetch-lane playback proof
   over the REAL first-vertical fixture" that does not exist (W12-A06).
2. [S2] capabilities.rs advertises 10 of 12 descriptors unopenable
   (dnxhd flag Wave 6 confirmed; http-streaming NotStarted while io/ sits
   landed-but-unwired) (W12-A10).
3. [S2] HandleTable charges the 8-handle cap against LIFETIME opens —
   9th open/close cycle hits E_LIMIT (W12-A07).
4. [S2] abi.rs "measured cheap enough for v1" cites a measurement that
   does not exist; ~8.4M mutex-locking FFI calls per <=8MB loadBlob
   (W12-A07/A08).
5. [S2] staged_source read-ahead never fills seek-induced cache gaps ->
   pump-gate stall (W12-A03); plugin-api playlist.rs stores SECONDS as
   Nanoseconds (10^9 units bug) (W12-A04).
6. [S2] manifest.rs validate() empty-body for-loop leaves sample_rates/
   bit_depths membership unchecked (W12-A02); install.rs comments claim
   load re-checks kind — it does not (W12-A09).
7. [KEY] supervisor next_restart_ms dead always-None — restart timing
   fictional (W12-A10); autoplay gate works but the product UI never
   creates an AudioContext (W12-A03/A10).
