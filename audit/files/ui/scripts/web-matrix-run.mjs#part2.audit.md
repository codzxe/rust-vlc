# AUDIT: ui/scripts/web-matrix-run.mjs#part2
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 411 (411–821 of 822) / 12 (firstExisting, wasm staging block, MIME map, parseRange, startServer, per-mode server loop, chromium-executable resolution, pwEnv, Playwright spawn+await+finally, expectationLane + 4 lane literals, gitInfo, fragment merge + report assembly/write, summary + exit) |
| Chunk boundary | part2 = 411–821 (assignments.tsv; part1 = 1–410 owned by W15-A08 — validateConfigShape, renderMatrixMarkdown [146-349], the `--restamp` block [361-396], fail/resolveRepo). The renderer my part CALLS at :796 lives in part1; one finding below is jointly owned (flagged). |
| File verdict | REAL (executed historically — the committed record is its output; internally consistent) |
| Reachable from shipped entry point? | NOT a product surface: dev/CI lane invoked by `npm run webmatrix` (package.json scripts). No shipped entry point reaches it; reachability here = the committed evidence chain (docs/web/browser-matrix-20261110.json + .md, verified below). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | python3 json inspection of `docs/web/browser-matrix-20261110.json` (schema `lumen.web.browser-matrix/1`; git block commit 3755dc3/branch w24/a/dirty:true honestly recorded; capture block matches; chromium lane playwright-run 1.64.1, 2 modes × 17 scenarios, 10/17 passed each, the SAME 7 fails in both modes — the `web.decoders_absent` cluster; 4 not-runnable lanes) — the committed record is shape-consistent with what this script writes; `cat ui/e2e/web/matrix.config.json` (ports/paths/modes; servedFixture and rangeBlindPath are flat filenames — no mkdir hazard); `rg -n 'rangeSupportingPath'` → consumed by harness.js:672 + web-matrix.spec.ts:106 (NOT dead config); `ls apps/web` + fixture path (worker.js/audio-worklet.js/fixture all exist); Read part1 lines 121-410 for the renderer/restamp context (A08's surface); static trace of every error path in my range. NOT re-run (no browser binary in sandbox; rule 8 — the committed record + config are the evidence). |

## 1. One-paragraph verdict
The driver half of the browser-matrix tool is disciplined, config-driven, provenance-honest tooling whose committed output (the 2026-11-10 record) checks out shape-for-shape: schema, capture/git blocks (including an honestly-recorded dirty tree), the real 10/17-pass chromium lanes with the decoder-absent fail cluster, and four explicitly not-run lanes that fabricate nothing. The hostile findings are error-path defects, not fakeness: (1) a failed Playwright run (zero mode fragments — e.g. missing browser) still writes a record whose chromium lane claims evidence `playwright-run` with an empty modes array, and the subsequent markdown render crashes on a non-optional `modes[0].userAgent` access (part1:234, reached from my :796) — breaking the script's own "exit code: Playwright's, propagated verbatim" contract; (2) two loud-fail guards are self-defeating — `statSync(chromiumExecutable)` at :604 THROWS ENOENT before the `.isFile()` check can produce its intended `fail()` message, and the spawn-error path at :654-664 rejects through an unguarded await, crashing with a stack trace instead. Remaining hygiene: a path-prefix guard without a separator, a leaked per-run tmpdir, and silently-dropped corrupt fragments.

## 2. What actually works (verified)
— verified by python3 inspection of docs/web/browser-matrix-20261110.json → the record this script writes is internally consistent: `git: {commit: 3755dc3…, branch: w24/a, dirty: true}` (the dirty flag the R11 P3-2 fix records), capture block matching generated, artifact sha256 present, chromium lane `playwright-run` with playwrightVersion 1.64.1 (== package.json devDependency), 2 modes (isolation-off/isolation-coop-coep, crossOriginIsolated false/true — both isolation snapshots exercised as claimed), 10/17 passed per mode with the identical 7-name fail set in both modes (the `web.decoders_absent` downstream cluster the renderer documents).
— verified by `rg -n 'rangeSupportingPath'` → the validated config key is consumed by the in-page harness (harness.js:672) and the spec (web-matrix.spec.ts:106), not dead config; `ls apps/web` + fixture → every staged file (worker.js, audio-worklet.js, avi_mjpeg_pcm.avi) exists in the tree.
— verified by reading the spawn block (:642-664): the async-spawn-over-spawnSync fix is real and correctly reasoned (spawnSync would starve the in-process HTTP servers), and the `.finally()` closes all servers on every exit path (the W24-R8 P3-3 leak fix).

## 3. Findings
### F-001 [S3] (jointly owned with part1/A08 — triggered from my range)
Where: ui/scripts/web-matrix-run.mjs:741-757, 794-796 (crash site part1:234)
Code:
```js
const fragments = config.modes
  .map((mode) => {
    const fragmentPath = path.join(outDir, `chromium-${mode.id}.json`);
    try {
      return JSON.parse(readFileSync(fragmentPath, "utf8"));
    } catch {
      return null;
    }
  })
  .filter((fragment) => fragment !== null);

const chromiumLane = {
  browser: "chromium",
  evidence: "playwright-run",
  playwrightVersion: pkg.devDependencies["@playwright/test"] ?? null,
  modes: fragments,
};
```
What is wrong: When Playwright produces ZERO fragments (browser binary missing at launch, catastrophic runner failure), `fragments = []`, yet the chromium lane is still stamped `evidence: "playwright-run"` and the record is written at :794. The markdown render at :796 then calls `renderMatrixMarkdown`, whose line 234 (part1) dereferences `chromiumLane.modes[0].userAgent` non-optionally → TypeError → unhandled crash. Net effects: (a) a committed-able JSON artifact claiming REAL run evidence with zero executed modes (only a console WARNING at :814-818 distinguishes it); (b) the script dies on the render exception with exit code 1, violating the header contract "Exit code: Playwright's, propagated verbatim" (:27) — Playwright's own status is computed at :654 but never reached.
Why it matters: This is the evidence-production tool for the web platform column; a failed run must fail loudly and leave NO record that reads as a run. The current shape mints a "playwright-run" record precisely when nothing ran.
What correct looks like: `if (fragments.length === 0) fail("no mode fragments produced — Playwright run failed; nothing recorded");` before writing anything (and part1's :234 access made optional regardless).
Confidence: HIGH — Verification: static trace of both call paths (fragment merge → report write → render call; renderMatrixMarkdown read in full); not executed (no browser in sandbox).
### F-002 [S3]
Where: ui/scripts/web-matrix-run.mjs:604-606 and 649-664
Code:
```js
if (chromiumExecutable && !statSync(chromiumExecutable).isFile()) {
  fail(`configured chromium executable is not a file: ${chromiumExecutable}`);
}
```
(and)
```js
const playwrightStatus = await new Promise((resolve, reject) => {
  child.once("error", reject);
  child.once("close", (code) => resolve(code ?? 2));
}).finally(() => {
```
What is wrong: Two loud-fail guards that crash instead: (a) `statSync` THROWS on a nonexistent path — when `WEB_MATRIX_CHROMIUM` points at a missing binary (the exact scenario the guard exists for), the ENOENT exception propagates as an unhandled stack trace, and the intended `fail(...)` message ("configured chromium executable is not a file") is unreachable; the process exits 1 with a stack instead of 2 with the message. (b) If the Playwright CLI itself is missing (`node_modules` not installed), `spawn` emits `error` → the promise REJECTS → the `await` at :654 has no enclosing try/catch → unhandled rejection crash (the `.finally` correctly closes servers, but the rejection still escapes).
Why it matters: The file's own header sells loud, configured failure ("a malformed one must fail LOUDLY here" — the config validator's rule, :44-46); both guards defeat that contract in the two most likely misconfiguration states (bad browser path, missing deps).
What correct looks like: (a) `try { if (!statSync(p).isFile()) fail(...) } catch { fail(...) }` (or `existsSync` first); (b) `try { await … } catch (error) { fail(\`playwright failed to launch: ${error}\`) }` after the finally-shaped server shutdown.
Confidence: HIGH — Verification: read both sites; Node fs/child_process semantics are static certainties.
### F-003 [S4]
Where: ui/scripts/web-matrix-run.mjs:523-528
Code:
```js
    const filePath = path.normalize(path.join(siteDist, route));
    if (!filePath.startsWith(siteDist)) {
      response.writeHead(403);
      response.end("forbidden");
      return;
    }
```
What is wrong: The traversal guard uses a raw string prefix: a sibling directory named e.g. `sitedist-evil` (or any path sharing the `siteDist` prefix) passes `startsWith(siteDist)` because the check lacks the `path.sep` boundary. Only `siteDist`'s actual siblings named with the same prefix are exposed, and none exist today (404), so this is latent, not exploitable in the committed tree.
Why it matters: A test server with a subtly wrong escape guard is exactly the kind of thing copied into the next harness; the correct form is one line.
What correct looks like: `filePath === siteDist || filePath.startsWith(siteDist + path.sep)`.
Confidence: HIGH — Verification: read + path semantics; sibling-existence checked (none).
### F-004 [S4]
Where: ui/scripts/web-matrix-run.mjs:587-588 (and 456)
Code:
```js
const outDir = path.join(os.tmpdir(), `lumen-web-matrix-${process.pid}`);
mkdirSync(outDir, { recursive: true });
```
What is wrong: The per-run output directory (and `e2e/web/sitedist`, recreated at :456) is never cleaned up — every invocation leaks a `lumen-web-matrix-<pid>` tmpdir and re-stages sitedist beside the previous run's copies. In a long-lived dev machine or CI runner this accumulates unboundedly.
Why it matters: Harness hygiene; the script otherwise carefully closes servers and propagates exit codes.
What correct looks like: `fs.rmSync(outDir, { recursive: true, force: true })` on exit (the finally shape already exists at :657-664 to hang it on).
Confidence: HIGH — Verification: read; no rm/unlink anywhere in the file (rg).
### F-005 [S4]
Where: ui/scripts/web-matrix-run.mjs:743-749
Code:
```js
    const fragmentPath = path.join(outDir, `chromium-${mode.id}.json`);
    try {
      return JSON.parse(readFileSync(fragmentPath, "utf8"));
    } catch {
      return null;
    }
```
What is wrong: A corrupt/truncated fragment (killed browser mid-write) is silently converted to `null` and filtered out; the only signal is the aggregate count WARNING at :814-818 — neither WHICH mode fragment was dropped nor WHY (missing vs unparseable) is logged. The `catch {}` swallows the parse error entirely.
Why it matters: A mode that RAN but failed to serialize disappears from the record with no diagnostic, weakening the record's "records only what ran" honesty (it records less than ran, indistinguishable from never-ran).
What correct looks like: `catch (error) { console.error(\`[web-matrix] fragment ${fragmentPath} unreadable: ${error}\`); return null; }`.
Confidence: HIGH — Verification: read.

## 4. Per-function table (my range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `firstExisting` | 414-424 | REAL | — | statSync-probe; CARGO_TARGET_DIR precedence + config candidates. |
| wasm staging block | 430-468 | REAL | — | Loud fail with build hint; sha256 recorded; range-blind double-staging (the documented 404 fix — same bytes, two routes); flat config filenames verified (no mkdir hazard). wasmProfile heuristic (`includes("/release/")`) — a path containing a literal `release/` dir anywhere misclassifies; cosmetic. |
| `MIME` map | 472-477 | REAL | — | .wasm/.avi correct; unknown → octet-stream. |
| `parseRange` | 480-514 | REAL | — | RFC 9110 open/closed ranges correct, end clamped, start≥total → null (valid 200 ignore); multi-range → logged 200 (the W24-R8 P3-4 fix); suffix ranges (`bytes=-n`) unsupported → 200 full body (valid MAY-ignore; media fetches don't issue them). |
| `startServer` | 516-567 | REAL | S4 | Per-mode COOP/COEP pair; range-blind route 200-always; 206 + content-range correct; F-003 prefix guard. |
| per-mode server loop | 571-583 | REAL | — | Port = base+index; listen error → loud fail. |
| chromium-executable resolution | 596-606 | REAL | S3 | F-002(a): statSync guard throws before failing. |
| `pwEnv` + spawn | 610-664 | REAL | S3 | Env override + PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 (no-download rule); async spawn with documented starvation rationale; F-002(b) unguarded rejection; `.finally` server shutdown correct. |
| `expectationLane` + 4 lanes | 673-722 | REAL | — | Honest not-runnable lanes: BCD-cited expectations (2026-10-19 — UNVERIFIABLE offline, but labeled expectations, never results) + concrete next steps. No fabrication. |
| `gitInfo` | 724-738 | REAL | — | commit/branch/dirty via git; nulls on git failure (honest). |
| fragment merge + report assembly | 740-796 | REAL | S3/S4 | F-001 empty-fragments record + render crash; F-005 silent drop; capture block filled from the run's own facts (787-789) ✓. |
| summary + exit | 800-821 | REAL | — | Per-mode pass counts + per-fail lines to stderr; missing-fragment count warning; `process.exit(playwrightStatus)` — the verbatim contract (defeated only by F-001's crash path). |

## 5. Test honesty
No tests exist for this script (it IS the test driver). The committed record is the execution evidence: verified internally consistent (see §2). Mutation check (reasoned, not run): if the fragment merge dropped the fail statuses, the record's 7-name fail sets and the markdown's fail-shape section could not exist; if the server ignored COOP/COEP, the two modes' crossOriginIsolated values (false/true) would agree — they don't. The evidence is self-discriminating.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Firefox/WebKit/Safari lanes are NEVER fabricated … BCD-cited expectations + 'not runnable in this sandbox'" | web-matrix-run.mjs:14-16 header | REAL | Lanes at 683-722 carry evidence "not-runnable-in-sandbox", expectations, next steps; committed record matches. |
| "Exit code: Playwright's, propagated verbatim" | web-matrix-run.mjs:27 | REAL in the happy path, FALSE in the F-001/F-002 failure paths | Render crash exits 1; statSync/spawn guards crash instead of exit 2. |
| "the JSON records only what ran" | web-matrix-run.mjs:816 | PARTLY | It can also record LESS than what ran (F-005) and claim a run that didn't happen (F-001). |
| The committed markdown's "adds NO facts" renderer claim | web-matrix-run.mjs:122-126 (part1) | Cross-noted for A08 | The title/Method dates ("2026-11-10") are hardcoded literals, not read from the record (record's capture.recorded = 2026-11-11T03:13Z) — a future --restamp re-run drifts the dates. Part1's surface; flagged here for A08. |

## 7. Dead / duplicated / unreachable code
Dead: none in my range (`rangeSupportingPath` is consumed by the harness/spec, verified). Duplicated: the 4 expectation lanes share the expectationLane factory (fine). Unreachable: none.

## 8. Dependencies & flags
node:* only (no new npm deps, as the header promises — verified imports at 29-37); spawns @playwright/test's cli from node_modules; reads matrix.config.json as the single knob source (validated in part1). Not in the shipped bundle; `npm run webmatrix` entry. The wasm artifact target (`target/wasm32-unknown-unknown/{release,debug}/lumen_web.wasm`) is absent from this sandbox (not built — rule 8; the committed record's sha256 is the only artifact evidence).

## 9. Open questions for the next agent
- A08 (part1): the renderMatrixMarkdown non-optional `modes[0]` accesses (234, and the perMode loop) and the hardcoded report dates (163, 335) — F-001's crash site and the no-literals violation live in your range.
- Who owns a browser-equipped re-run? The record's own fail-shape section says the range-blind fix "awaits a browser re-run on a machine that has one" — the harness fix (468) is in but the committed record still carries the 404 fail. REQUEST-ORCH-RUN not filed (rule 8: needs wasm build + browser, out of sandbox scope; the orchestrator's web lane decision).
