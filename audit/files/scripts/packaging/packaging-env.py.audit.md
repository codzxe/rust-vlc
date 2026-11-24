# AUDIT: scripts/packaging/packaging-env.py
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 361 / 6 (die, load_toml, check_type, validate, product_identity_env, main) |
| File verdict | REAL (executed: --print / --shell / --github-env all exit 0; failure path exercised too) |
| Reachable from shipped entry point? | YES (release.yml:187,468 invoke it; 17 packaging scripts consume its vars: build-rpm.sh, build-msi.sh, gen-wxs.sh, gen-desktop-entry.sh, build-appimage.sh, build-deb.sh, build-dmg.sh, build-zip.sh, build-targz.sh, stage-linux-payload.sh, gen-sboms.sh, sign-*.sh, gate.sh, verify-linux-artifacts.sh, build-msi.ps1, test-msi-upgrade.ps1) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | python3 packaging-env.py --print -> exit 0 (49 vars); --shell -> exit 0, sourced OK; --github-env -> exit 0, 49 lines appended; tauri-pair mode -> exit 1 with clean error (cargo not on sandbox PATH — environment, not a defect) |

## 1. One-paragraph verdict
Real, working config-injection bridge. I executed all three output modes
(exit 0 each; 49 variables; `--shell` output re-sourced under bash with values
intact) and the failure path (missing cargo → `packaging-env: error:
product-env.py failed: cargo not found on PATH`, exit 1 — fail-closed with
actionable message). The schema-subset validator genuinely enforces the
committed packaging.schema.json (type/enum/const/pattern/min/max/required/
additionalProperties/items), the semantic pass really checks that
shipped.docs + smoke_fixture files exist, secret env names are checked
pairwise-distinct, and the shell quoting idiom is correct POSIX
(`val.replace("'", "'\\''")` = the `'\''` idiom — verified by sourcing).
Defects found are latent-hygiene only: a misleading guard error on a
missing PRODUCT_NAME, an unhandled KeyError path in check_type if the schema
ever gains "number"/"null", str()-based uniqueItems, and a docstring that
misdescribes --github-env as taking FILE directly.

## 2. What actually works (verified)
- `python3 scripts/packaging/packaging-env.py --print` -> exit 0; output begins `PRODUCT_NAME=Lumen` … 49 KEY=VALUE lines; `PACKAGING_LAUNCH_SMOKE_FIXTURE=tests/fixtures/generated/valid/wav_pcm_s16le_44k1_st.wav` (file exists — semantic pass genuinely ran).
- `--shell` -> exit 0; `bash -c 'source env.sh; echo $PRODUCT_NAME'` -> `Lumen` (quoting round-trips).
- `--github-env --github-env-file <audit/scratch file>` -> `packaging-env: appended 49 variables`, exit 0.
- Identity guard live-fire: `python3 -c "import tomllib,json; print('lumen' in json.dumps(tomllib.load(open('packaging/packaging.toml','rb'))).lower())"` -> False — no product-name duplication in packaging.toml, so the guard passes on real data.
- release.yml:187-192 invokes the exact documented command (with the tauri pair) in the release lane.

## 3. Findings
F-001 [S4]
Where: scripts/packaging/packaging-env.py:273-277
Code:
```
    if env.get("PRODUCT_NAME", "").lower() in json.dumps(packaging).lower():
        # Guard the single-source rule: identity prose must not be
        # duplicated into the packaging file.
        die("packaging.toml appears to contain the product name — "
```
What is wrong: if PRODUCT_NAME were ever absent, `"" in <any string>` is True in
Python, so the guard fires and blames packaging.toml for a duplication that
does not exist — a wrong diagnosis on a fail-closed path. Unreachable today
(product-env.py:182 always emits PRODUCT_NAME) but a booby trap for the next
editor of product-env.py. Why it matters: misleading error text sends the fix
to the wrong file. What correct looks like: `name = env.get("PRODUCT_NAME") or
die("product-env.py did not emit PRODUCT_NAME")`. Confidence: HIGH —
Verification: read.

F-002 [S4]
Where: scripts/packaging/packaging-env.py:99-105
Code:
```
    ok = {
        "object": lambda v: isinstance(v, dict),
        ...
        "boolean": lambda v: isinstance(v, bool),
    }[expected]
```
What is wrong: a dict-keyed lookup with no `.get` + default — schema types
"number"/"null" (both legal JSON Schema) raise KeyError, i.e. a raw traceback
instead of the die() error protocol the file promises ("a hard error with an
actionable message"). Current schema uses only {array,object,string,integer,
boolean} (verified by walking the schema), so no live impact — the exact
"extend the validator in the same change" contract (line 68-69 comment) is only
half-implemented: extension dies, but ugly. Confidence: HIGH — Verification:
read + schema walk.

F-003 [S4]
Where: scripts/packaging/packaging-env.py:153
Code:
```
        if schema.get("uniqueItems") and len(value) != len(set(map(str, value))):
```
What is wrong: `map(str, value)` conflates cross-type equality (1 vs "1", True
vs "True"-style collisions) that JSON Schema treats as distinct, so a mixed
array can false-positive "items are not unique". Current schema's only
uniqueItems is on the 2-string `targets` array — no impact today. Confidence:
HIGH — Verification: read.

F-004 [S4]
Where: scripts/packaging/packaging-env.py:28-29 (docstring) vs 229-234 (argparse)
Code:
```
  --github-env FILE      append KEY=VALUE lines to FILE (GitHub Actions
                         environment-file format)
```
What is wrong: the docstring shows one flag taking FILE; the CLI actually
requires two flags (`--github-env` + `--github-env-file FILE`) and dies
otherwise (line 233-234). A copy-paste of the documented form fails. Why it
matters: the file's own header is the usage contract. What correct looks like:
document both flags. Confidence: HIGH — Verification: read + argparse read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 78-91 | REAL | — | prints to stderr, exit 1; used consistently |
| load_toml | 83-91 | REAL | — | FileNotFoundError/TOMLDecodeError handled; unreachable assert harmless |
| check_type | 98-108 | REAL | S4 | F-002 KeyError path on future "number"/"null" |
| validate | 111-171 | REAL | S4 | keyword allowlist + type/const/enum/pattern/min/max/required/addProps/items recursion all genuine; F-003 uniqueItems str-conflation; pattern uses Python re (ECMA-262 divergence possible — fail direction mostly closed: invalid regex dies) |
| product_identity_env | 178-211 | REAL | — | runs product-env.py --print; returncode checked; unparseable lines fatal; passthrough args correct |
| main | 214-357 | REAL | S4 | F-001 guard; brand icon/URL export with type checks; newline guard before emit; modes print/shell/github-env all executed; put() flattening matches packaging.toml keys 1:1 |

## 5. Test honesty
No tests in-file. Mutation reasoning: validate() would die if packaging.toml
drifted from schema (e.g. add a bogus key — additionalProperties:false path,
line 164-168); the semantic pass would die if a shipped.doc were deleted; the
secret-distinct check would die on a duplicated env name. These are real
asserts — the script would FAIL loudly on each mutation class. I did not mutate
files (read-only repo); the negative path I could exercise without edits
(missing cargo) failed correctly.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A missing or invalid value is a hard error ... never a silent fallback" | :16-19 | REAL | missing cargo → exit 1 with message (run); missing docs file → die at :266-268 |
| "packaging scripts carry ZERO identity/packaging literals; they read the environment this script emits" | :16-17 | REAL-UNPROVEN (per-script; spot-verified in gen-wxs.sh:132 `PACKAGING_DOCS:?` guard; full check in each script's own audit) | rg consumers: 17 scripts |
| "--shell ... safe to eval/source even with spaces in values" | :23-27 | REAL | sourced under bash; `'\''` idiom correct (:350-351) |
| "--github-env FILE append KEY=VALUE lines to FILE" | :28-29 | SHALLOW (doc) | works, but needs two flags (F-004) |
| "this script re-asserts the two files do not overlap on identity keys" | :34-36 | REAL | guard :273 executed on real data (no 'lumen' in packaging.toml json) |
| "Omit the pair and BIN_TAURI is simply not emitted" | :48-49 | REAL | --print without pair emitted no BIN_* lines |

## 7. Dead / duplicated / unreachable code
`raise AssertionError("unreachable")` (line 91) is dead by construction (die
raises SystemExit) — intentional exhaustiveness marker, harmless. No
duplication found; product-env.py reuse instead of copy is real.

## 8. Dependencies & flags
stdlib only (argparse/json/re/subprocess/sys/tomllib/pathlib); python>=3.11
(tomllib) — matches CI's python3. Calls scripts/release/product-env.py (wave-22
tool) via `sys.executable` — correct interpreter propagation.

## 9. Open questions for the next agent
- packaging.schema.json "pattern" semantics are ECMA-262; the validator uses
  Python `re.fullmatch` — if a future schema pattern uses a JS-only construct
  the die message will say "not a valid regex" (fail-closed) — acceptable, but
  the schema author should know.
- release.yml:283 asserts the bridge exports every [signing.*] *_env key —
  cross-check `put()` calls (lines 322-332) against packaging.toml [signing]
  when auditing release.yml (currently 1:1: 8 of 8).
