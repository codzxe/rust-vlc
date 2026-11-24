# CLAIM AUDIT: docs/inventory/raw/options/manifest.json

| Field | Value |
|---|---|
| Auditor | W18-A09 |
| Date | 2026-11-24 |
| Lines | 77 (JSON) + sibling script docs/inventory/raw/options/parse-options.py (484 lines) |
| Claims extracted | 21 (15 mechanically re-executable, 2 path-resolution, 4 external/historical) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 15 REAL (re-executed) / 0 / 0 / 0 / 4 PAPER-ONLY-UNVERIFIED / 2 FALSE (relative paths) / 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict

This manifest is the strongest evidence artifact audited so far: its core derivation claims are machine-re-executable and I re-executed them — every count reproduces exactly (615/124, 1476/187, 1476/187, 3364/5509/1539 lines, 191 sections, 1439/1433/6 vlcrc, 3/2/1 leftovers, 252/252 default agreement, nesting assertion true, options-linux.json = 1476 options). The one thing that is broken is embarrassingly basic: all five `file`/`outputs` paths carry one `../` too many (`../../../baselines/...` and `../../../inventory/...`), resolving to non-existent repo-root paths; the dumps actually live at docs/baselines/raw/linux/ and the output at docs/inventory/options-linux.json. The sibling script itself is path-correct (chdir to repo root via parents[4]) — the manifest's relative-path fields were never re-resolved after the raw tree moved under docs/. The web_evidence URLs (HTTP 200s) and the "byte-identical across two runs (cmp)" claim are historical, unverifiable from this sandbox.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "file: ../../../baselines/raw/linux/12-options-longhelp.txt … 615 option records, 124 module sections; 1539 lines" | manifest source_dumps[0] | counts REAL; path FALSE | Path: `ls baselines/` at repo root → no such dir; actual file at docs/baselines/raw/linux/12-options-longhelp.txt (Glob verified). Counts re-executed: `wc -l` → 1539 ✓; python parse_dump() → options=615, sections=124 ✓. |
| 2 | "13-options-advanced-longhelp.txt … 1476 option records, 187 module sections; 3364 lines" | source_dumps[1] | counts REAL; path FALSE | `wc -l` → 3364 ✓; parse_dump() → 1476/187 ✓. Same broken `../../../` prefix. |
| 3 | "14-options-full-help.txt … same 1476 options … 5509 lines; canonical source for options-linux.json" | source_dumps[2] | counts REAL; path FALSE | `wc -l` → 5509 ✓; parse_dump() → 1476/187 ✓; docs/inventory/options-linux.json load → `options: 1476` ✓ (derivation chain verified end-to-end). |
| 4 | "15-options-help-and-probes.txt … 407-line --help dump + honest probe matrix" | source_dumps[3] | REAL (structure-verified); path FALSE | File is 435 lines; PROBE 1 block spans lines 10-416 = 407 lines between the `# $ …--help` command (:9) and `# exit=0` (:418) — matches. Path prefix broken as above. |
| 5 | "16-options-vlcrc.txt … 191 sections, 1439 name=value lines (all commented = defaults), 4707 verbatim lines" | source_dumps[4] | REAL; path FALSE | `rg -c '^\['` → 191 ✓; `awk '/^#[a-zA-Z0-9_+-]+=/{n++} END{print n}'` → 1439 ✓; uncommented name=value count → 0 (all defaults, as claimed) ✓; 4726 total lines − 18-line preamble − 1-line footer = 4707 ✓ (preamble verified at :1-18, footer `# ---- verbatim vlcrc ends here ----`). |
| 6 | "captured_by: … byte-identical across two runs (cmp)" | source_dumps[4].captured_by | REAL-UNPROVEN | Historical reproducibility run of the pinned VLC binary; no VLC in sandbox → UNVERIFIED. |
| 7 | "derivation.script: parse-options.py; run: python3 parse-options.py (paths are repo-relative; run from repo root)" | derivation | REAL (script executes) | python3 import of the module + parse_dump/parse_vlcrc calls all ran clean (exit 0); script does `os.chdir(repo)` (parse-options.py:268-269) with `repo = parents[4]` — path-correct. NOT re-run as main() because main() writes docs/inventory/options-linux.json (read-only rule). |
| 8 | "outputs: ../../../inventory/options-linux.json" | derivation.outputs | FALSE (path) | Resolves to repo-root/inventory/options-linux.json — does not exist; actual: docs/inventory/options-linux.json. Off by one `../`, same defect class as the five dump paths. |
| 9 | parser_grammar: "module header: '^ <Name> (<module_id>)$'; option line 6-space '--flag' or 2-space '-x, --flag'" | derivation.parser_grammar | REAL | parse-options.py:20 `MODULE_RE = re.compile(r"^ (\S.*?) \(([a-zA-Z0-9_.-]+)\)\s*$")` and :93-96 `if (line.startswith("      --") or re.match(r"^  -[a-zA-Z], --", line)):` — grammar matches the description exactly. |
| 10 | "parsed option records == raw option-line counts: 615/615, 1476/1476, 1476/1476" | checks_enforced[1] | REAL (re-executed) | Raw option-line counts via the script's exact rules (startswith("      --") or `^  -[a-zA-Z], --`): 615 / 1476 / 1476 ✓ == parsed counts 615/1476/1476 ✓. |
| 11 | "leftovers … are footer notes only: 3/2/1 across dumps 12/13/14" | checks_enforced[2] | REAL (re-executed) | len(l12)=3, len(l13)=2, len(l14)=1 (parse_dump return values) ✓. |
| 12 | "vlcrc: 1439 name=value lines, 1433 unique names, 6 duplicated names == the 6 name collisions in the help dumps" | checks_enforced[3] | REAL (re-executed) | parse_vlcrc → (1433 names, 1439 total, 6 dups) ✓; cross-extraction agreement with help-dump collisions: 6 == 6 ✓ (script's own collisions computation). |
| 13 | "default agreement: 252/252 … 0 disagreements" (after enabled/disabled→1/0 normalization) | checks_enforced[4] | REAL (re-executed) | My replicated loop over d14 × vlcrc with the same normalization → `agree=252 disagree=0` (command output in run log). |
| 14 | "option sets nested: names(--longhelp) <= names(--advanced --longhelp) <= names(--full-help)" | checks_enforced[0] | REAL (re-executed) | `n12 <= n13 <= n14` → True ✓. |
| 15 | "oracle_identity: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | manifest top | REAL (cross-doc) | Identical string in docs/feature-matrix.toml:37 `baseline = "VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)"` — the two inventories pin the same oracle. |
| 16 | web_evidence: two videolan.org wiki URLs, "http_status: 200 (accessed 2026-10-19)", "local_copy: not retained" | web_evidence[0-1] | PAPER-ONLY / UNVERIFIED | No network in this sandbox; no local copy retained to check. Claimed facts are quoted-only. |
| 17 | "clean_room: Dumps are observed runtime behavior of the pinned binary. No VLC source consulted. No binaries committed." | clean_room | PARTIALLY VERIFIED | "No binaries committed": docs/baselines/raw/linux/ = 17 .txt dumps + probe-work dir, no binary blobs (ls). "Observed runtime behavior": UNVERIFIED — the pinned VLC binary is not in this sandbox; the dumps carry `# exit=` markers consistent with captured runs. |
| 18 | "platform: linux (Debian 13 trixie, x86_64, headless container, uid 1001)" | manifest top | UNVERIFIED | Environment of the original capture; not re-checkable. Dump headers (:2-3) repeat the same environment string — internally consistent. |
| 19-21 | Remaining claims: task/agent identity, "615 option records … 124 module sections" phrasings repeated per dump, probe-matrix honesty notes | various | REAL (as #1-#5) / UNVERIFIED (identity) | Covered above. |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| All dump paths use prefix `../../../baselines/raw/linux/` and output `../../../inventory/options-linux.json` | Actual tree: docs/baselines/raw/linux/12-16*.txt and docs/inventory/options-linux.json; the sibling script hardcodes `RAW = "docs/baselines/raw/linux"` (parse-options.py:18) | The script is right; the manifest's relative paths are wrong (one `../` extra — as if the manifest were written for a location one directory deeper, e.g. docs/inventory/raw/options/raw/). The R4-style evidence rule "resolves to an existing artifact" fails for all six path fields as literally written. |

## 4. Fabrication / inflation indicators

- NONE in the data layer: every count I could re-execute reproduces bit-for-bit (615/1476/1476 records; 3/2/1 leftovers; 191/1439/1433/6 vlcrc; 252/252 defaults; nesting true; 1476 options in the derived JSON). This manifest's numbers are the real output of the real parser over the real dumps.
- The only rot is mechanical: six stale relative-path fields (S4) — evidence pointers that no longer resolve from their own file location.

## 5. Verdict block

PAPER-ONLY descriptors around a REAL, re-executed derivation: 15 of 21 claims re-verified by running the sibling parser and direct file inspection; 2 path-resolution claims FALSE (fixable by deleting one `../`); 4 claims (cmp-reproducibility, two wiki URLs, capture environment) UNVERIFIABLE from this sandbox. No inflated counts, no fake evidence strings, no internal numeric drift — the one defect class is stale paths.
