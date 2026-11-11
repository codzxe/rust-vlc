# Browser matrix — the 2026-11-10 record (card 7.I1, issue #49)

Machine record: `docs/web/browser-matrix-20261110.json` (schema `lumen.web.browser-matrix/1`).
This report is RENDERED from that record by `ui/scripts/web-matrix-run.mjs`
(the `report.markdownName` writer); it adds no facts of its own.

## Honesty note — read this first

- **The chromium lane is a real Playwright run** (evidence tier `playwright-run`):
  captured 2026-11-10T22:14:59.907Z against commit `9a13c3974591749e75903f257c1d4b90c3dffcbe` on branch `w24/a`,
  driving 157.0.8103.50 with Playwright 1.64.1.
- **This report is capture-based, not live**: the sandbox's Chromium binary was deleted
  after the capture (only ffmpeg remains under the Playwright cache; re-download is
  forbidden), so the matrix is NOT re-runnable in this sandbox. Everything below reads
  the committed record; nothing here was re-executed to produce it.
- **Firefox / WebKit / Safari / Safari-iOS never executed** — no binaries exist in this
  sandbox and downloading is forbidden. Their lanes carry BCD-cited expectations and the
  next step (real machines / CI runners), never a fabricated result (AGENTS §3.4).
- **Verdicts are mixed and reported as mixed**: the engine core boots, negotiates WebGL2,
  answers typed refusals and drives the transports in a real browser; media playback is
  refused by design (see the fail-shape notes under the chromium lane).

## Provenance

| Field | Value |
|---|---|
| Record schema | `lumen.web.browser-matrix/1` |
| Capture | 2026-11-10T22:14:59.907Z @ `9a13c3974591749e75903f257c1d4b90c3dffcbe` (branch `w24/a`) |
| Record stamp | regenerated 2026-11-10T23:18:51.068Z @ `389bfccfb97e797b09152477ffa4779566e51f71` |
| Regen reason | stamp-only regeneration (W24-R8 P2-1): align the record's git block with the current committed state. NO browser re-run, NO lane data touched — the capture facts live in .capture. |
| Record git block | `389bfccfb97e797b09152477ffa4779566e51f71` on `w24/a` (the tree this record was last maintained against; the record's own containing commit is its child — see git log) |
| wasm artifact | `../../../../../tmp/my-project/target/wasm32-unknown-unknown/release/lumen_web.wasm` — release profile, 1294311 bytes |
| artifact sha256 | `aa70ec5920d80149450dc040f82ef0063ed403ac8655b51b56d738b5134a19e2` |
| Fixture | `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (425516 bytes) |
| Node | v24.22.2 |
| Harness | ui/e2e/web (card 7.I1) |
| Chromium executable | `/home/z/.agent-browser/browsers/chrome-157.0.8103.50/chrome` |
| Chromium launch args | --no-sandbox --disable-dev-shm-usage |

## Lane summary

| Browser | Evidence | Result |
|---|---|---|
| chromium | playwright-run (REAL) | 9/17 (isolation-off), 9/17 (isolation-coop-coep) scenarios passed |
| firefox | not-runnable-in-sandbox | NOT RUN — BCD-cited expectations only |
| webkit | not-runnable-in-sandbox | NOT RUN — BCD-cited expectations only |
| safari | not-runnable-in-sandbox | NOT RUN — BCD-cited expectations only |
| safari-ios | not-runnable-in-sandbox | NOT RUN — BCD-cited expectations only |

## Chromium — the real run

Playwright 1.64.1; user agent `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/157.0.0.0 Safari/537.36`.
Every mode below ran the full 17-scenario conformance suite against the real
engine-in-Worker module (real wasm instantiation, real fetch, real postMessage ordering).

### Mode `isolation-off` (COOP/COEP OFF, crossOriginIsolated=false)

Browser 157.0.8103.50; wasm lane: 14 fetches — status 200, content-type application/wasm, content-length 1294311.

**9/17 scenarios passed.**

| Scenario | Category | Status | ms | Evidence / error (clipped; full text in the JSON) |
|---|---|---|---|---|
| boot-handshake | capability | ✅ pass | 83 | {"engineVersion":"0.1.0","catalogVersion":"1","features":["first-vertical:avi-mjpeg-pcm","audio.null-device","video.headless-counter-presenter","scheduler.cooperative-pump"],"threading":"single","renderer":"webgl2","audi… |
| renderer-ladder-consistency | capability | ✅ pass | 37 | {"probed":{"webgpuApi":true,"webgpuAdapter":false,"webgl2Context":true},"activeBackend":"webgl2"} |
| open-play-pause-stop | happy-path | ❌ fail | 171 | open must be class-A accepted, got {"error":{"stage":"decode","code":"unsupported","message":"unsupported (web.decoders_absent): the wasm32 module ships the engine core without the builtin decoders (the codecs crate's ex… |
| play-to-end | happy-path | ❌ fail | 30162 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| media-info-and-tracks | happy-path | ❌ fail | 30147 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| open-rtsp-refused | error-path | ✅ pass | 38 | {"stage":"input","code":"unsupported-for-host","capability":"rtsp-rtp-udp-access"} |
| open-file-refused | error-path | ✅ pass | 29 | {"stage":"input","code":"unsupported-for-host","capability":"local-filesystem"} |
| open-http-pending-lane | error-path | ✅ pass | 36 | {"stage":"input","code":"unsupported-for-host","capability":"http-streaming"} |
| open-garbage-blob-refused | error-path | ✅ pass | 32 | {"stage":"probe","code":"unsupported"} |
| seek-not-served | error-path | ❌ fail | 30196 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| unserved-config-command | error-path | ✅ pass | 41 | {"stage":"control","code":"unsupported-for-host"} |
| event-envelope-invariants | invariants | ❌ fail | 30146 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| idempotent-pause-echo | invariants | ❌ fail | 30159 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| autoplay-gate-parks-and-resumes | autoplay | ❌ fail | 30168 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| fetch-range-supported-server | transport | ✅ pass | 7 | {"status":206,"contentRange":"bytes 0-1023/425516","acceptRanges":"bytes"} |
| fetch-range-blind-server | transport | ❌ fail | 4 | a range-blind server ignores the window (200): expected 200, got 404 |
| audio-context-and-worklet | transport | ✅ pass | 48 | {"contextConstructible":true,"workletModuleLoaded":true,"contextState":"running","sampleRate":44100} |

### Mode `isolation-coop-coep` (COOP/COEP ON, crossOriginIsolated=true)

Browser 157.0.8103.50; wasm lane: 14 fetches — status 200, content-type application/wasm, content-length 1294311.

**9/17 scenarios passed.**

| Scenario | Category | Status | ms | Evidence / error (clipped; full text in the JSON) |
|---|---|---|---|---|
| boot-handshake | capability | ✅ pass | 43 | {"engineVersion":"0.1.0","catalogVersion":"1","features":["first-vertical:avi-mjpeg-pcm","audio.null-device","video.headless-counter-presenter","scheduler.cooperative-pump"],"threading":"single","renderer":"webgl2","audi… |
| renderer-ladder-consistency | capability | ✅ pass | 52 | {"probed":{"webgpuApi":true,"webgpuAdapter":false,"webgl2Context":true},"activeBackend":"webgl2"} |
| open-play-pause-stop | happy-path | ❌ fail | 139 | open must be class-A accepted, got {"error":{"stage":"decode","code":"unsupported","message":"unsupported (web.decoders_absent): the wasm32 module ships the engine core without the builtin decoders (the codecs crate's ex… |
| play-to-end | happy-path | ❌ fail | 30140 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| media-info-and-tracks | happy-path | ❌ fail | 30154 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| open-rtsp-refused | error-path | ✅ pass | 38 | {"stage":"input","code":"unsupported-for-host","capability":"rtsp-rtp-udp-access"} |
| open-file-refused | error-path | ✅ pass | 35 | {"stage":"input","code":"unsupported-for-host","capability":"local-filesystem"} |
| open-http-pending-lane | error-path | ✅ pass | 28 | {"stage":"input","code":"unsupported-for-host","capability":"http-streaming"} |
| open-garbage-blob-refused | error-path | ✅ pass | 33 | {"stage":"probe","code":"unsupported"} |
| seek-not-served | error-path | ❌ fail | 30175 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| unserved-config-command | error-path | ✅ pass | 42 | {"stage":"control","code":"unsupported-for-host"} |
| event-envelope-invariants | invariants | ❌ fail | 30155 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| idempotent-pause-echo | invariants | ❌ fail | 30159 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| autoplay-gate-parks-and-resumes | autoplay | ❌ fail | 30157 | pump loop exceeded 30000ms; last report: {"type":"pumped","report":{"state":"idle","positionUs":0,"busy":false,"eventsFlushed":0}} |
| fetch-range-supported-server | transport | ✅ pass | 6 | {"status":206,"contentRange":"bytes 0-1023/425516","acceptRanges":"bytes"} |
| fetch-range-blind-server | transport | ❌ fail | 3 | a range-blind server ignores the window (200): expected 200, got 404 |
| audio-context-and-worklet | transport | ✅ pass | 49 | {"contextConstructible":true,"workletModuleLoaded":true,"contextState":"running","sampleRate":44100} |

### Fail shapes in this capture (interpretation)

- `open-play-pause-stop` fails with the **typed refusal `web.decoders_absent`**: the wasm32
  module ships the engine core without the builtin decoders (the codecs crate's external
  engines do not build for wasm32-unknown-unknown — the Lead-approved typed refusal until
  the crate splits; see the capability snapshot's `builtin-software-decoders` row). This is
  the designed v1 posture, reported honestly — not a crash.
- 12 scenario entries fail with `pump loop exceeded 30000ms`: every one of them
  requires an OPEN session first, and open is refused by `web.decoders_absent` above — the
  suite waits out its pump budget on an engine that correctly stays idle. The failure
  cluster is downstream of the decoder-less posture, not an independent defect.
- `fetch-range-blind-server` fails with `expected 200, got 404` — a **harness staging
  defect, not an engine defect**: the range-blind route was never staged as a file, so the
  server 404s before the range-blind behavior could be exercised. Diagnosed from this
  capture, fixed on w24/a after it (the fixture is now staged under the range-blind name
  too); the fix awaits a browser re-run on a machine that has one.

## Lanes not runnable in this sandbox

### firefox — not-runnable-in-sandbox

Current Firefox desktop (BCD 2026-10-19): WASM MVP+SIMD128 yes; Wasm GC 120+/EH-final 131+; SAB 79+ isolation-gated; Atomics.waitAsync 145+; WebGPU 141+ Windows-only; WebGL2 51+; AudioWorklet 76+; WebCodecs 130+ desktop (Android no).

BCD-cited expectations:
- doc 09 §8.1 row set (Chrome/Edge | Firefox | Safari columns)
- v1 suite needs none of SAB/WebGPU: engine core + WebGL2 floor + fetch Range — expected to pass on current Firefox desktop
- renderer ladder expectation: WebGL2 active (WebGPU Windows-only per BCD; Firefox Android permanently WebGL2, doc 09 §10)

**Next step:** Run the same Playwright suite with a real Firefox binary on a dev machine or a CI runner with browsers installed (doc 09 §8.2 scheduled tier [PROPOSAL]); record per-scenario rows into this JSON.

### webkit — not-runnable-in-sandbox

Playwright's WebKit ≈ Safari macOS (BCD 2026-10-19): WASM yes; GC 18.2+/EH-final 18.4+; SAB 15.2+ isolation-gated; WebGPU 26+; WebGL2 15+; AudioWorklet 14.1+; OffscreenCanvas 16.4+; rVFC 15.4+.

BCD-cited expectations:
- doc 09 §8.1 row set
- v1 suite expected to pass on Safari 18+ desktop via the WebGL2 lane; deltas vs real Safari must be recorded (Playwright WebKit is a build, not the shipping browser)

**Next step:** Add the weekly Playwright-WebKit smoke tier (doc 09 §8.2 [PROPOSAL]) once a machine with the WebKit build exists; real-Safari validation stays manual per release.

### safari — not-runnable-in-sandbox

Safari macOS (BCD 2026-10-19): WASM MVP+SIMD128 yes; Wasm GC 18.2+/EH-final 18.4+; threads+SAB 15.2+ (COOP/COEP mandatory); WebGPU 26+; WebGL2 15+; AudioWorklet 14.1+; WebCodecs 16.4+.

BCD-cited expectations:
- doc 09 §8.1 row set
- autoplay policy is gesture-gated (doc 09 §5.3): expect gestureRequired on the suspended-context path — the suite's parked/resume scenario exercises exactly that

**Next step:** Per-release manual protocol on a real Mac (doc 09 §8.2/§11.4): fresh profile → capability probe → fixture corpus → results recorded as feature-matrix evidence.

### safari-ios — not-runnable-in-sandbox

Safari iOS (BCD 2026-10-19): aligns with Safari 18.2+/26+ rows; no SAB without COOP/COEP on iOS; WebGPU iOS 26+; lower memory ceilings (doc 09 §2.4 budget must respect them); fullscreen + autoplay gesture-gated.

BCD-cited expectations:
- doc 09 §8.1 iOS constraints note
- physical-device evidence is MANDATORY for any web = verified claim touching SAB, WebGPU or autoplay (doc 09 §11.4)

**Next step:** Physical iOS device run per release (doc 09 §8.2 manual protocol); no simulator substitute for the memory-pressure + background-throttling rows.

## Method

- Harness: `ui/e2e/web/` (card 7.I1) — the in-page conformance suite (`site/harness.js`,
  735 lines) drives the REAL engine Worker; the driver `ui/scripts/web-matrix-run.mjs`
  stages the site (page + wasm artifact + worker/audio-worklet glue + first-vertical
  fixture), serves one static server per isolation mode (Range-aware + range-blind +
  per-mode COOP/COEP) and runs `playwright.web.config.ts`.
- Every knob is config-driven from `ui/e2e/web/matrix.config.json` (ports, artifact paths,
  isolation modes, pump budgets, launch args, report names); nothing is a code literal
  (doc 07 §9).
- Playwright 1.64.1; Chromium 157.0.8103.50
  (`/home/z/.agent-browser/browsers/chrome-157.0.8103.50/chrome`), launch args --no-sandbox --disable-dev-shm-usage.
- The wasm artifact under test: `../../../../../tmp/my-project/target/wasm32-unknown-unknown/release/lumen_web.wasm`, 1294311 bytes,
  sha256 `aa70ec5920d80149450dc040f82ef0063ed403ac8655b51b56d738b5134a19e2` (release profile).
- Capture date: 2026-11-10. The record you are reading is that capture (see the honesty
  note); it was NOT re-run to produce this report.

## Shipped COOP/COEP posture (decision record)

**Off by default** (`serving.coopCoepDefault = false` in
`ui/e2e/web/matrix.config.json`; doc 09 §9.5, deciding §12 Q8): the v1 wasm module is
single-threaded, so cross-origin isolation buys nothing today and its embeddability cost
is real. The harness above still exercises BOTH isolation modes on purpose (doc 09 §2.5
item 6) — testing coverage is not the deployment posture. Flipping the default is a Lead
decision (lands with threading, doc 09 §3.3); the deploy-check script that would assert
headers on the shipped site remains [PROPOSAL] (doc 09 §2.3).

