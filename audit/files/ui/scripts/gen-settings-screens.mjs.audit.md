# AUDIT: ui/scripts/gen-settings-screens.mjs
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 663 / 16 items |
| File verdict | REAL |
| Reachable from shipped entry point? | NO shipped binary — dev toolchain: `npm run settings:generate` / `settings:check` (ui/package.json:24-25), EXECUTED in baseline (#24 settings:check EXIT=0); parseToml also imported by ui/scripts/gen-hotkeys-data.mjs:24 (baseline hotkeys:check EXIT=0) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | `rg -n -F '\\' schemas/settings.toml product.toml keymaps/default.toml` (only keymaps:216 `Ctrl+\\`, which IS decoded) · `rg -n 'list<' schemas/settings.toml` (only list<string>, :603/:618) · `rg -n 'feature_gate = "(?!")' -P schemas/settings.toml` (4 gated entries: hw-decode, recording×2, wasm-extensions) · `rg -n "featureExposed" ui/src/screens/settings/` (runtime filter exists: SettingsSections.tsx:191, SettingField.tsx:185, shared.tsx:83) · `rg -n "settings.area.(recording|extensions)" ui/src/i18n/messages.ts` (:773-774, :2251-2252 — registered) · `sed -n` verification of every quoted range · baseline: audit/runs/baseline/ui-settings_check.log.txt (EXIT=0, "10 ui areas, 102 ui entries, 121 total"), ui-hotkeys_check.log.txt (EXIT=0, 141 bindings), ui-vitest.log.txt (1705 passed incl. generated.test.ts drift tests) |

## 1. One-paragraph verdict
This generator is one of the few genuinely working pieces of the settings pipeline: it parses schemas/settings.toml + product.toml with a hand-rolled TOML subset parser, enforces the documented D1–D5 rules loudly (duplicate keys, unresolved default_ref, unknown screens, enum msgid collisions after sanitization), emits deterministic TypeScript, and its `--check` drift guard is not only wired to npm but actually TESTED (generated.test.ts spawns the script pristine→0 and mutated→1). Baseline run #24 proves it executes green against the committed tree. The defects that remain are all latent-parser/emitter mismatches that today's inputs do not trigger: TOML escapes other than `\"`/`\\` are silently passed through rather than decoded or rejected; `list<T>` types other than `list<string>` validate but would emit TypeScript that cannot compile or silently stringify numeric defaults; a duplicate key inside one `[[setting]]` block is silently last-wins at the parser layer even though the tool's whole job is duplicate detection; and the "temporary" `msgid()` cast seam in the emitted Screens.tsx outlived the key registration it was waiting for. It wires the UI to schema data, but remember the bigger picture: settings VALUES still never persist (W11) — this tool generates the FORM, not the function.

## 2. What actually works (verified)
— Baseline `audit/runs/baseline/ui-settings_check.log.txt`: `gen-settings-screens --check: OK (10 ui areas, 102 ui entries, 121 total)` EXIT=0 — parseToml + buildModel + both emitters byte-match the committed generated files.
— Drift detection is test-proven: ui/src/screens/settings/generated.test.ts:104-121 runs the script via execFileSync on a pristine schema (expects exit 0) and three mutated temp copies (expects exit 1); included in baseline ui-vitest.log.txt (1705 passed, 1 skipped).
— Gated entries are correctly flagged but deliberately not filtered at generation time, and the runtime filter exists: SettingsSections.tsx:191 `const visible = rows.filter((row) => isSettingEntry(row) && row.featureExposed);` and SettingField.tsx:185 `if (!entry.featureExposed) return null;` — the 4 entries gated on hw-decode/recording/wasm-extensions (all `false` in product.toml:164-168) never render.
— The "never writes outside generated/" claim (script header :18-19) holds: only outputs are `resolve(outDir, "schema.ts"|"Screens.tsx")` (:620-623) written after `mkdir(outDir)` (:649-650); outDir defaults to ui/src/screens/settings/generated (:30).
— The w26.k14 enum msgid collision guard is real and reachable: :303-311 throws on sanitized collisions (e.g. "5.1" vs "5:1" → both "5-1").

## 3. Findings

### F-001 [S3]
Where: ui/scripts/gen-settings-screens.mjs:61-72
Code:
```
    while (i < n && text[i] !== '"') {
      if (text[i] === "\\") {
        const e = text[i + 1];
        if (e === '"' || e === "\\") {
          out += e;
          i += 2;
          continue;
        }
      }
      out += text[i];
      i++;
    }
```
What is wrong: only `\"` and `\\` are decoded. Every other TOML escape — `\n`, `\t`, `\r`, `\b`, `\f`, `\uXXXX` — falls through to `out += text[i]` and is emitted as a literal backslash-plus-letter, silently. TOML spec requires decoding them; a correct subset parser must at minimum reject what it cannot decode (the file's own header, :6-8, advertises "a TOML-subset parser", not a silently lossy one).
Why it matters: this parser is shared dev infrastructure — gen-hotkeys-data.mjs:24 imports it for keymaps/default.toml, and keymaps/default.toml:216 already uses an escape (`key = "Ctrl+\\"`). The day a label or description carries `\n`, the generated msgid silently contains a literal `\n` two-char sequence; the failure surfaces far away (l10n grammar rule or a visible backslash in the UI), not at the parser.
What correct looks like: decode the full spec escape set or `throw new TomlError(\`unsupported escape \\\\${e}\`)` for anything else.
Confidence: HIGH — Verification: read; escape-free inputs verified by `rg -n -F '\'` over schemas/settings.toml, product.toml (zero hits) and keymaps/default.toml (only :216 `Ctrl+\\`, which this code handles correctly).

### F-002 [S3]
Where: ui/scripts/gen-settings-screens.mjs:216 (validator) vs :419 (emitted union) vs :399 (emitter)
Code:
```
const LIST_TYPE_RE = /^list<(.+)>$/;
```
```
  lines.push('  | "list<string>"');
```
```
  if (Array.isArray(v)) return `[${v.map((x) => JSON.stringify(String(x))).join(", ")}]`;
```
What is wrong: the validator accepts ANY `list<T>` (`LIST_TYPE_RE` captures any inner type), but the emitted `SettingValueType` union contains only `"list<string>"`, and `tsValue` stringifies every array element via `String(x)`. A future `type = "list<int>"` with `default = [1, 2]` passes buildModel's type check (:249) and then either (a) fails tsc with "list<int> not assignable to SettingValueType" in a GENERATED file, or (b) if the union is widened, emits `defaultValue: ["1", "2"]` — numbers silently become strings. The file's own w26.b02 comment (:421-423) documents exactly this class of bug for path/url/percent and fixed only that instance.
Why it matters: the generator's contract is "schema-valid input → compiling, type-faithful output"; this hole breaks the contract one keystroke away.
What correct looks like: either restrict LIST_TYPE_RE to `list<string>` (loud rejection at :249) or emit a matching `list<T>` union arm per allowed inner type and type-faithful array emission.
Confidence: HIGH — Verification: read; current schema uses only `list<string>` (rg: settings.toml:603,618), so latent.
### F-003 [S3]
Where: ui/scripts/gen-settings-screens.mjs:206
Code:
```
    const key = parseBareKey();
    expect("=");
    current[key] = parseValue();
```
What is wrong: a duplicate key inside one table (e.g. two `description = "…"` lines in one `[[setting]]` block) silently overwrites — last one wins, no error. TOML spec makes duplicate keys an error; this tool's entire purpose is duplicate detection (D2/D3/D4 at :252-267), yet the layer below those checks swallows duplicates the model rules don't see (same key twice within one block).
Why it matters: schemas/settings.toml is a 1627-line hand-authored file; a copy-pasted `default =` line inside a block would be silently resolved to the last value with zero diagnostics, and `--check` stays green because regeneration is idempotent on the same swallowed input.
What correct looks like: `if (Object.prototype.hasOwnProperty.call(current, key)) throw new TomlError(\`duplicate key "${key}"\`);` before assignment.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: ui/scripts/gen-settings-screens.mjs:328-329
Code:
```
    } else if (raw.range !== undefined || raw.enumeration !== undefined) {
      throw new TomlError(`${where}: type "${type}" carries neither range nor enumeration`);
    }
```
What is wrong: the message states the opposite of the detected condition. This branch fires when a non-numeric type (bool/string/path/url/list) DOES carry a range or enumeration; the text says it "carries neither".
Why it matters: a maintainer who writes `type = "bool"` with `range = {…}` gets an error that reads like the range is missing — sending them to ADD the very thing they must delete.
What correct looks like: `type "${type}" must not carry range/enumeration` (or equivalent).
Confidence: HIGH — Verification: read.

### F-005 [S4]
Where: ui/scripts/gen-settings-screens.mjs:554 (emitted into generated/Screens.tsx:115)
Code:
```
  lines.push("const msgid = (id: string): MessageKey => id as MessageKey;");
```
What is wrong: the "temporary" w26.b02 cast seam (`id as MessageKey`) defeats compile-time verification that generated msgids are catalog keys. The in-file justification (:539-553) says it exists "until the Lead registers the new screens' keys post-wave (contract §4.5)" — registration HAS happened (ui/src/i18n/messages.ts:773-774 union arms `"settings.area.extensions"|"settings.area.recording"`, :2251-2252 catalog values; section titles e.g. messages.ts:998/:2479), so the cast is a stale no-op that still suppresses the type check for FUTURE generated msgids.
Why it matters: drift between a newly generated section title and the catalog will pass `tsc --noEmit` (baseline #22) and be caught only by the vitest coverage test (generated.test.ts `covered(section.title)`) — a weaker, later gate.
What correct looks like: drop the cast and emit title msgids as string literals typed by the MessageKey union (they now type-check natively), or emit `satisfies MessageKey` checks.
Confidence: HIGH — Verification: read + rg registration evidence above.

### F-006 [S4]
Where: ui/scripts/gen-settings-screens.mjs:613-614
Code:
```
  const inputPath = inputIdx >= 0 ? resolve(args[inputIdx + 1]) : defaultInput;
  const outDir = outDirIdx >= 0 ? resolve(args[outDirIdx + 1]) : defaultOutDir;
```
What is wrong: `node gen-settings-screens.mjs --check --input` (missing operand) makes `args[inputIdx + 1]` `undefined`; `resolve(undefined)` throws a raw TypeError ("The "path" argument must be of type string") with a stack dump instead of a usage error. Same for `--out-dir`.
Why it matters: the flags are documented for the drift test (:15-16); a human typo produces an opaque crash.
What correct looks like: check `inputIdx >= 0 && inputIdx + 1 < args.length` and print usage + exit 2.
Confidence: HIGH — Verification: read (behavior of node:path resolve with undefined is deterministic); not executed — no node runs allowed in session.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TomlError (class) | 37 | REAL | — | trivial Error subclass, fine |
| parseToml | 39-209 | REAL | S3 | executed baseline #24/#33(settings/hotkeys checks); subset parser with F-001/F-003 leniency |
| — skipWs | 45-56 | REAL | — | ws + comment skip; correct |
| — parseString | 58-76 | REAL | S3 | F-001 escape passthrough; unterminated string → loud TomlError |
| — parseValue | 78-147 | REAL | — | arrays/inline tables/bare tokens; unsupported values throw loudly; `1e999`→Infinity edge (no current input) |
| — expect | 149-153 | REAL | — | trivial, throws with offset |
| — parseBareKey | 155-162 | REAL | — | bare keys only; quoted keys fail loudly (subset, documented) |
| — navigate | 164-184 | REAL | — | [[x]] vs [x] conflict → loud; duplicate [table] headers silently merge (TOML spec violation, no current input triggers) |
| lookupPath | 218-225 | REAL | — | correct dotted-path resolver, undefined on miss |
| buildModel | 227-388 | REAL | S3/S4 | D1–D5 enforced loudly (verified :252-267, :272-285, :291-317, :360-365); F-002 list<T> hole; F-004 message |
| constName | 394 | REAL | — | `a.b-c` vs `a-b.c` would collide → duplicate TS declaration (loud downstream) |
| tsValue | 396-401 | REAL | S3 | F-002 array stringification |
| emitSchemaTs | 403-526 | REAL | — | deterministic; byte-verified vs committed files by baseline #24 |
| emitScreensTs | 528-602 | REAL | S4 | emits F-005 msgid cast; per-entry imports deduped (verified in generated/Screens.tsx:12-113) |
| main | 608-655 | REAL | S4 | F-006 operand handling; --check byte-compare + exit 1 on drift (test-proven) |
| isDirectRun guard | 657-663 | REAL | — | correct direct-run detection, error → exit 1 |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| generated.test.ts "pristine --check → 0" (:104) | YES | byte-compares regeneration against committed generated files; any emitter/parser change that alters output fails |
| generated.test.ts mutated-schema → exit 1 (:109,114,121) | YES | runs the real script via execFileSync on temp copies; mutation-detector is exercised end-to-end |
| generated.test.ts coverage (:251-253, covered(...)) | YES | every description/enum-option/section-title msgid must exist in catalog or lane file; a dropped registration fails |
| gen-hotkeys-data --check (baseline ui-hotkeys_check.log.txt EXIT=0) | YES (as drift guard) | same byte-compare over keymaps/default.toml via shared parseToml |
No unit tests exist for parseToml itself (edge cases like escapes/duplicate keys are untested — matches F-001/F-003 being latent).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "zero deps — a TOML-subset parser suffices" | file header :6-8 | REAL | imports only node:fs/promises, node:path, node:url (:21-23); npm ci baseline #21 |
| "NEVER writes outside ui/src/screens/settings/generated/ (unless --out-dir)" | file header :18-19 | REAL | only writes :620-623/:649-650, outDir default :30 |
| "idempotent" regeneration | :13 | REAL | deterministic emitters, no timestamps; --check byte-compare green in baseline #24 |
| "--check exit 1 on drift" | :14 | REAL | generated.test.ts:109-121 expects exit 1 on mutated inputs; vitest 1705 passed (baseline #23) |

## 7. Dead / duplicated / unreachable code
- Duplicate TOML-subset parser family: this file's parseToml (dev-time) vs ui/src/screens/player/layout.ts:177 parseTomlSubset (runtime, player screens) — two hand-rolled parsers with different capability sets; gen-hotkeys-data.mjs:8-10 openly says layout.ts's parser "cannot" handle dotted headers, so the hotkeys generator reuses THIS one. Duplication is acknowledged but unresolved (S4, cross-file; layout.ts belongs to another agent).
- `raw.placement ?? {}` (:257) and `raw.cli ?? {}` (:264) are dead defaults: placement/cli are mandated fields checked at :245-246, so the `?? {}` branches can never trigger.
- No unreachable exports: parseToml/buildModel are exported and consumed (gen-hotkeys-data.mjs:24; generated.test.ts imports emitted artifacts, not these fns — buildModel has NO direct test consumer, exercised only via main()).

## 8. Dependencies & flags
- Node builtins only (fs/promises, path, url) — matches "zero deps" claim; no npm deps added.
- Wired scripts: package.json:24-25 (settings:generate / settings:check); referenced by eslint.config.js:90 (generated-emitter exemptions).
- No feature flags. Not shipped in lumen/lumen-tauri binaries (dev tool).

## 9. Open questions for the next agent
- ui/scripts/gen-hotkeys-data.mjs (owner: whoever has it) inherits F-001/F-003 through the parseToml import — re-flag there if it is in your list.
- Does the settings RUNTIME ever write a changed value back? This generator only builds the form; W11 established persistence is absent — settings screens are a form over a store that never reaches disk (cross-ref, not re-audited here).
