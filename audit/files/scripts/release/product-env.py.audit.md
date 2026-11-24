# AUDIT: scripts/release/product-env.py
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 262 / 5 (die, load_toml, cargo_bin_name, msi_version, main + nested need) |
| File verdict | REAL (executed: --print exit 0 with 13 vars; --github-env append verified; two error paths exit 1; msi_version unit-tested on 6 inputs) |
| Reachable from shipped entry point? | YES: release lane via packaging-env.py (product_identity_env, packaging-env.py:193-211 runs `product-env.py --print`) -> release.yml:187 GITHUB_ENV; consumers make-app.sh, make-dmg.sh, package-windows.sh, package-deb.sh, gen-wxs.sh, release-notes.sh read its vars |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | python3 product-env.py --print -> 13 lines exit 0; --github-env --github-env-file -> appended 13 exit 0; --tauri-package alone -> pair error exit 1; --github-env-file ALONE -> prints to stdout, NO file written (usage-drift proof); msi_version unit tests (6 cases) |

## 1. One-paragraph verdict
Real and correct at its job: identity from product.toml, description prose from
the locale catalog via identity.description_key, bin names from cargo metadata
(root workspace for cli/desktop; a second manifest-scoped metadata run for the
workspace-EXCLUDED tauri crate — the exclusion claim verified against root
Cargo.toml), MSI numeric version derived and unit-tested on six inputs
("-dev"/"+build"/padding/truncation/invalid all behave as documented). Fail
paths are loud (pair misuse dies; cargo absent dies; every identity field
validated non-empty string). One demonstrated defect: the docstring's own Usage
line documents `--github-env-file "$GITHUB_ENV"` as sufficient, but the code
requires the mode flag `--github-env` too — running the documented form
prints KEY=VALUE to stdout and silently never writes the file (proven:
pe-drift.txt does not exist after the run). The github-env mode has no live
invoker (packaging-env.py consumes --print), so CI is unaffected — the trap
bites humans following the file's own usage text.

## 2. What actually works (verified)
- `python3 scripts/release/product-env.py --print` -> exit 0, 13 variables (PRODUCT_NAME=Lumen ... MSI_VERSION=0.1.3) — exactly the wave-22 identity contract.
- `--github-env --github-env-file audit/scratch/W17-A08/pe-gh.txt` -> `appended 13 variables`, exit 0, file written (PRODUCT_NAME=Lumen first line).
- `--tauri-package lumen-desktop-tauri` (no manifest) -> exit 1, `product-env: error: --tauri-package and --tauri-manifest are a pair` — the XOR pair check (line 221) works.
- msi_version unit tests (module import): `0.1.3`->`0.1.3`; `0.2.0-dev`->`0.2.0`; `0.2`->`0.2.0`; `0.1.3.4`->`0.1.3`; `1.0+build`->`1.0.0`; `x.y` -> die (exit 1, error message).
- Root-workspace exclusion of the tauri crate (the reason for the second metadata run): verified via root Cargo.toml exclude list (apps/desktop-tauri excluded — see tauri-launch/release.yml comments; cross-checked in build-rpm lane docs).

## 3. Findings
F-001 [S3]
Where: scripts/release/product-env.py:43-48 (docstring Usage) vs 125-133 (argparse) and 250-258 (emit)
Code:
```
Usage (from the repo root):
  python3 scripts/release/product-env.py \
      --cli-package lumen-cli --desktop-package desktop \
      [--tauri-package lumen-desktop-tauri \
       --tauri-manifest apps/desktop-tauri/Cargo.toml] \
      [--print | --github-env-file "$GITHUB_ENV"]
```
Code (emit):
```
    lines = [f"{k}={v}" for k, v in env.items()]
    if mode == "print":
        for line in lines:
            print(line)
```
What is wrong: `--github-env-file` alone leaves `mode == "print"` (default), so
the documented invocation PRINTS to stdout and never opens the file —
empirically demonstrated: `product-env.py --github-env-file <path>` printed 13
lines and the file was NOT created (`ls` -> No such file). A caller following
the file's own usage gets the wrong operation with zero error output. The
`--github-env` mode flag exists (:126) but is absent from the Usage line.
Why it matters: silent wrong-mode operation is the exact "step cannot fail"
disease; here the step cannot fail because it does something else entirely.
What correct looks like: either default mode to github-env when
--github-env-file is given, or fix the Usage line to show both flags (and die
when --github-env-file is passed without --github-env). Confidence: HIGH —
Verification: ran.
F-002 [S4]
Where: scripts/release/product-env.py:158-160
Code:
```
    version_code = ident.get("version_code")
    if not isinstance(version_code, int):
        die("product.toml identity.version_code is missing or not an integer")
```
What is wrong: `isinstance(True, int)` is True in Python, so a TOML
`version_code = true` would pass and export PRODUCT_VERSION_CODE="True".
packaging-env.py's check_type (:103) explicitly excludes bools for exactly
this class; this validator does not. Confidence: HIGH — Verification: read
(Python semantics).
F-003 [S4]
Where: scripts/release/product-env.py:32-34 (docstring) — same class as packaging-env.py F-004
Code:
```
  --github-env FILE    append KEY=VALUE lines to FILE (GitHub Actions
```
What is wrong: shows one flag taking FILE; the CLI needs `--github-env` +
`--github-env-file FILE` (die at :132-133 if mode without file, but nothing
dies for file without mode — that is F-001's silent half). Confidence: HIGH —
Verification: read.
F-004 [S4]
Where: scripts/release/product-env.py:107
Code:
```
    fields = fields[:3]
```
What is wrong: a 4+ numeric-field version silently truncates (0.1.3.4 ->
0.1.3) instead of erroring — the caller cannot know his version was mangled
(two distinct versions collide in MSI space). The comment documents "max
three" but not the collision. What correct looks like: die on >3 fields and
require an explicit product.toml decision. Confidence: HIGH — Verification:
ran (unit test).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 69-71 | REAL | — | stderr + exit 1 |
| load_toml | 74-82 | REAL | — | FileNotFoundError + TOMLDecodeError handled |
| cargo_bin_name | 85-93 | REAL | — | exactly-one-bin enforcement; not-found dies |
| msi_version | 96-110 | REAL | S4 (F-004 truncation) | unit-tested 6 inputs; strip -suffix/+build; pad; die on non-numeric |
| main | 113-258 | REAL | S3 (F-001 usage trap), S4 (F-002 bool) | identity validation complete (12 fields); locale-catalog description; pair XOR check; newline guard; both modes executed |

## 5. Test honesty
No in-file tests; I substituted direct execution (4 CLI invocations + 6 unit
inputs). Mutation reasoning: an empty product.toml field dies via need();
a renamed bin would change BIN_* (cargo metadata is the real source — no
literal to drift); a bogus display_version dies in msi_version. The
honesty posture ("never a silent fallback") holds everywhere EXCEPT the
--github-env-file-without-mode trap (F-001), which is silent.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "packaging scripts read ... from HERE — never from hard-coded literals" | :4-7 | REAL | consumers' `:?` guards (gen-wxs.sh:122 etc.) |
| "product.toml is the single source of truth" chain incl. locale catalog | :9-14 | REAL | locales/<locale>.toml read at :172-179 executed (description came from catalog) |
| "workspace-EXCLUDED ... second metadata run scoped to the subcrate" | :18-23 | REAL-UNPROVEN | exclusion consistent with tauri docs; second run cannot execute here (no cargo on sandbox PATH) — packaging-env tauri test failed exactly at this call with the clean die message |
| "Anything missing is a hard error" | :25-26 | REAL | pair-misuse + missing-field dies executed |
| Usage line `[--print | --github-env-file "$GITHUB_ENV"]` | :43-48 | FALSE (as sufficient form) | F-001 proof: file not written |

## 7. Dead / duplicated / unreachable code
`--github-env` mode: no invoker in tree (rg: zero call sites; packaging-env.py
uses --print). Not dead by design intent (wave-22 direct GITHUB_ENV path) but
currently uncalled — and the mode is the one with the usage trap.

## 8. Dependencies & flags
stdlib only. Uses cargo metadata (read-only). Consistent with
packaging-env.py's bridge contract (passthrough args verified end-to-end by
release.yml:187-192 usage).

## 9. Open questions for the next agent
- release-notes.sh:5 says content is "config-driven via product-env.py" —
  when auditing release-notes.sh (in my list) verify HOW it reads the env
  (literal fallbacks?).
- If anyone starts invoking product-env.py --github-env directly, fix the
  usage doc first (F-001).
