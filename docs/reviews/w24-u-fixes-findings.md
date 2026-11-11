# W24-U fix audit — build-msi.ps1 escaping + parse repair + -SelfTest (W24-R13, 2026-11-11)

Auditor: W24-R13 (read-only review scout, lane U audit). Subject:
`git diff origin/w24/q...origin/w24/u` — 2 commits (`f5524e5` fix,
`1244e82` worklog), 2 files (`scripts/packaging/build-msi.ps1`,
`worklog.md`). Lane U's claim under audit: T's F-1
(unescaped `Source="$base"` + missing `&apos;`) closed, and — found
while fixing — the pre-fix script did not PARSE AT ALL (4 ParserErrors
from bash-style `\"`), repaired with the PowerShell-legal `` `" ``
idiom, generator extracted to `New-MsiWxs`, `-SelfTest` added with
hostile fixtures + raw-text escape pins.

**Verification tooling disclosure (auditor):** no pwsh on PATH in this
sandbox; U's downloaded PowerShell 7.7.0 was still present at
`/tmp/pwsh-test/pwsh` and was reused for every parse/run below — the
same verification-only, nothing-committed status U disclosed.

## Verdict

**PASS — merge-ready.** Every audited claim reproduced exactly as U's
worklog states it. Zero blocking findings; 3 confirmed handed-over
follow-ups (all LOW, all outside U's file ownership, all real — one
proven empirically); 3 informational notes. Per-item verdicts below,
each with the evidence I executed (not asserted).

## Per-item verdicts

### 1. The parse fix — PASS (real parse, pwsh 7.7.0)

`[System.Management.Automation.Language.Parser]::ParseFile` over both
revisions:

- pre-fix (`git show origin/w24/q:scripts/packaging/build-msi.ps1`):
  **4 parse errors**, all at lines 161/164 — `Unexpected token
  '$id\" Source=\"$base\" KeyPath=\"yes\" />'` etc. Exactly the count,
  lines, and shape U reported: the two File-row strings used bash-style
  `\"`, which PowerShell treats as a literal string-close, so the
  script was a hard parse error for every consumer (test-msi-upgrade.ps1
  would have died at parse time on its first live run — invisible to
  all prior waves because no wave had pwsh, and ps1-sanity's structural
  model treats `\"` as a close).
- post-fix (w24/u HEAD): **0 parse errors**. The literals now use the
  PowerShell backtick-quote idiom `` `" `` (lines 120/123), the same
  idiom test-msi-upgrade.ps1 line 114 already used.
- consumer check (untouched by this lane): test-msi-upgrade.ps1 also
  parses clean (0 errors) — the invocation pair is now jointly legal.

### 2. XmlEscape completeness + application point — PASS

```powershell
$Text.Replace('&', '&amp;').Replace('<', '&lt;').
    Replace('>', '&gt;').Replace('"', '&quot;').Replace("'", '&apos;")
```

- **Full set**: `& < > "` and `'` as `&apos;` — identical contract to
  gen-wxs.sh's `xml_escape` (sed `-e` chain in the same order), so the
  twins' change-both-together rule is honored on the set.
- **Escape order correct**: `&` is replaced FIRST in the chain (`.Replace`
  applies left-to-right), so the `&` introduced by later `&lt;`/`&gt;`/
  `&quot;`/`&apos;` replacements is never re-escaped. Escaping `&` last
  would double-escape (`&lt;` → `&amp;lt;`); that mutant would be caught
  by both the raw-text pin and the round-trip assertion (see item 3's
  red/green).
- **Application point**: `$escBase = XmlEscape $base` where
  `$base = [IO.Path]::GetFileName($doc)` — the doc Source value IS the
  pure basename in both twins (staging dir is the wix working directory;
  there is no path prefix to preserve, and none is escaped). The exe
  rows (`Source="$CliExe"` / `$DesktopExe`) stay unescaped — exactly
  symmetric with gen-wxs.sh's `${BIN_CLI}.exe` (schema-validated
  package names). No over- or under-escaping relative to the twin.

### 3. -SelfTest — PASS (run by the auditor, both red/green halves reproduced)

- **Real generator path**: the `-SelfTest` block calls `New-MsiWxs` —
  the SAME function the main flow invokes at line 348. Not a copy;
  cannot drift. Same discipline as gen-wxs.sh's `generate_wxs`.
- **Fixtures**: identity `A&B "C" <D>` / `P&Q` / `it's <a> & "b" > c`,
  docs `a&b<c>"d".md` + `it's.md` — identical to gen-wxs.sh
  `--self-test`'s fixtures.
- **Proof**: python `xml.etree` parse (malformed → death), 4 File rows,
  hostile basename + Name + Manufacturer round-trip, id sanitation +
  uniqueness, PLUS two raw-text pins (`Source="a&amp;b&lt;c&gt;&quot;d&quot;.md"`
  and `Source="it&apos;s.md"`) — deliberately stronger than the bash
  twin's proof (see follow-up F-U1).
- **Executed**: `pwsh scripts/packaging/build-msi.ps1 -SelfTest` →
  `PASS (4 File rows parsed; ...)` rc=0.
- **Red/green (auditor-run mutants in /tmp, copies only)**:
  - dropping `.Replace("'", '&apos;')` → rc=1 failing exactly the raw
    assertion `"raw .wxs does not escape ' as &apos;"`. The pin does
    what round-trip alone cannot (a bare `'` parses fine inside a
    double-quoted attribute).
  - reverting to unescaped `$base` in Source → rc=1 with
    `ET.ParseError: not well-formed (invalid token): line 25, column 51`
    — the exact original F-1/W24-R6 failure mode, reproduced through
    the .ps1.
- Temp-dir hygiene: `$rc=1` default, `finally { Remove-Item ... }` —
  and pwsh runs `finally` even on `exit` (auditor-verified), so the
  cleanup holds on every path out of the try.

### 4. New-MsiWxs extraction — PASS

- **Staging loop staging-only**: main flow (lines 340–345) does
  Test-Path + Copy-Item per doc, nothing else — matches build-msi.sh's
  division (build-msi.sh stages; gen-wxs.sh refuses).
- **Refusals live in the generator**: duplicate-basename (lines 113–115)
  and empty-docs-list (line 126) are inside `New-MsiWxs`, mirroring
  gen-wxs.sh's `generate_wxs` — so `-SelfTest` and the main flow share
  the refusal code too.
- **Template moved verbatim (machine-checked)**: old here-string
  (origin/w24/q) vs new, after normalizing only the two env→param
  interpolations (`$msiVersion`→`$Version`,
  `$($env:PRODUCT_MSI_UPGRADE_CODE)`→`$UpgradeCode`): **byte-identical**.

### 5. No regressions — PASS (all reproduced through the real script, fake-root e2e)

Fake-root methodology (T's): script copy + fake `target/release/*.exe`
+ fake `wix` that writes the `-o` artifact, OS_ID/ARCH_ID satisfied by
env. All runs through the REAL script under pwsh 7.7.0:

| Check | Result |
|---|---|
| e2e real-ish config (`LICENSE README.md`) | rc=0, `dist/<base>.msi` + `.wxs` written, emitted `.wxs` parses |
| `-VersionOverride 0.0.9` | rc=0, emitted `Package/@Version="0.0.9"` — upgrade-path predecessor-build contract intact |
| duplicate basenames (`README.md sub/README.md`) | rc=1, same-basename message unchanged |
| empty `PACKAGING_DOCS` | rc=1, `docs is empty` message unchanged |
| missing doc (`nope.md`) | rc=1, `not found` message unchanged |
| hostile docs e2e (`a&b<c>"d".md` + `it's.md`) | rc=0, output parses, both basenames round-trip, both raw escaped forms present |
| **cross-twin output diff** | gen-wxs.sh and New-MsiWxs run with the SAME env (incl. hostile identity values): outputs differ ONLY in the 4 provenance-comment lines (each names itself) and the .ps1's missing trailing newline after `</Wix>` (pre-existing here-string vs heredoc semantics, XML-insignificant) — byte-level `diff` + `od` confirm; exactly U's claim |

### 6. Gate + battery — PASS (auditor-run)

- `source /home/z/my-project/scripts/cargo-env.sh && bash
  scripts/packaging/gate.sh` → **PASS** (51 vars; 14 scripts bash -n +
  sh -n; python tools parse; ps1 structural; stagers; both
  .UNSIGNED.txt fallback contracts).
- `cargo run -p xtask --locked -- lint-literals` → **PASS — 0
  violations, 452 allowlisted**.
- `python3 scripts/packaging/ps1-sanity.py` on BOTH `.ps1` → PASS
  (ASCII-only, delimiters balanced, param first).

### 7. Scope — PASS

Diffstat: exactly `scripts/packaging/build-msi.ps1` + `worklog.md`.
No .rs/.sh/.py/.toml touched; no fmt/clippy owed; gate's 14-script
bash -n sweep unaffected. Worklog entry present, accurate (every
number in it that I spot-checked reproduced: 4→0 parse errors, line
25 column 51, rc values, message texts, cross-twin diff shape).

## Findings by severity

**BLOCKING / HIGH / MEDIUM: none.**

**LOW (confirmed follow-ups, all outside U's ownership — recorded for
the Lead; none block this merge):**

- **F-U1 (real, proven)** — gen-wxs.sh `--self-test` lacks the raw-text
  escape-set pin its .ps1 twin now has. Auditor proof: a /tmp mutant of
  gen-wxs.sh with the `s/'/\&apos;/g` expression deleted STILL PASSES
  `--self-test` (rc=0) — round-trip cannot see a reverted `&apos;`.
  Fix when a lane owns gen-wxs.sh: port the two raw-text asserts.
- **F-U2 (real)** — `docs/packaging/README.md` predates `-SelfTest`:
  the .msi proof row (line 64) and the local-proof paragraph (line 141)
  credit only `gen-wxs.sh --self-test` for the .wxs-generation proof;
  build-msi.ps1's `-SelfTest` is undocumented there.
- **F-U3 (real)** — gate.sh wires NEITHER self-test (grep for
  self-test/SelfTest/gen-wxs in gate.sh: no hits). Step 4 stays
  structural-only for .ps1 (`no pwsh in this sandbox` — still the
  honest standing assumption; the 7.7.0-in-/tmp parses are
  verification tooling, not a committed dependency). The runbook's
  local-proof story for both self-tests remains manual.

**INFO (no action owed by this lane):**

- **I-1** — both twins compute an escaped description
  (`$escDescription` / `esc_description`) that the .wxs template never
  emits. Pre-existing, symmetric, harmless; keep or retire both
  together someday.
- **I-2** — the .ps1's `.wxs` output ends `</Wix>` with no trailing
  newline (here-string semantics) vs the bash heredoc's `\n`.
  XML-insignificant; pre-existing; correctly left alone under
  minimal-diff.
- **I-3** — honest-boundary discipline maintained: `wix build` on the
  escaped Source values, the release.yml Windows-runner pwsh parse, and
  Windows PowerShell 5.1 remain `[VERIFY at first live run]`. This
  audit's parse/run evidence is pwsh 7.7.0 on Linux — the same class
  and version as U's own proof, so it narrows but does not close those
  boundaries.

## Bottom line

F-1 is closed on the .ps1 twin with the full escape set, the basename
escaping, one real generation path, and a self-test whose red/green
discrimination the auditor independently reproduced — plus the
out-of-scope-but-same-lines discovery that the script never parsed,
now repaired and parser-verified. U's worklog is accurate on every
spot-checked claim. Recommend the Lead accept w24/u and queue F-U1
(gen-wxs raw-text pin), F-U2 (README proof row), F-U3 (gate wiring)
as small follow-up lanes.
