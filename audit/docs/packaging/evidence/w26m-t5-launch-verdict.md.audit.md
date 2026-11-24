# CLAIM AUDIT: docs/packaging/evidence/w26m-t5-launch-verdict.md
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Claims extracted | 10 |
| Verdict counts | TRUE: 7 TRUE-AT-TIME/RESOLVED: 1 UNVERIFIED: 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This evidence doc's verifiable claims ALL check out under execution: the
tauri-launch.yml workflow exists exactly as described (3 jobs, 29 steps,
workflow_dispatch-only, 3/3 always-uploads with if-no-files-found:error,
permissions contents:read, all 5 action pins byte-identical to ci.yml's pins —
I re-parsed the YAML), the tauri-transport.ts [VERIFY] marker was ANNOTATED not
removed (verbatim found), the two stderr receipt strings are real code
(main.rs:59 / app.rs:166-168), and the claimed local validation (13/13 bash -n
blocks + one pwsh block) reproduces exactly when I re-run it. The one
time-conditional claim — ci.yml tauri-check RED with 5 expect_used denials
(run 43400610890) — is TRUE-at-time and RESOLVED on the current tree: zero
live `.expect(`/`.unwrap()` sites remain in apps/desktop-tauri/src, and
app.rs:238-246's comment documents the conversion to typed error returns. The
doc is scrupulous about what it does NOT prove (no JS observer, no screenshot,
no user-close verdict, the silent web-crash blind spot) — an honest evidence
file. PENDING-CI verdicts are correctly recorded as pending, not as results.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "`yaml.safe_load`: PASS — 1 workflow, 3 jobs (linux-launch / macos-launch / windows-launch), 29 steps" | :140-142 | TRUE | python3 yaml.safe_load -> jobs ['linux-launch','macos-launch','windows-launch'], total steps 29 |
| 2 | "triggers = `workflow_dispatch` ONLY" | :142-143 | TRUE | parsed trigger list: ['workflow_dispatch'] |
| 3 | "Action pins ... 5/5 byte-identical SHAs" vs ci.yml | :150-152 | TRUE | pins: checkout@3d3c42e5…, dtolnay@d124f86f…, rust-cache@6323deb…, upload-artifact@cf430e03…, setup-node@2028fbc5… — all match ci.yml lines 139/141/154/171/712 byte-for-byte |
| 4 | "3/3 jobs upload with `if: always()` + `if-no-files-found: error`; `permissions: contents: read`; one `shell: pwsh`" | :153-156 | TRUE | parsed: 3 uploads always(), 3 if-no-files-found error, perms contents:read, windows-launch step shell=pwsh |
| 5 | "`bash -n` over every bash `run:` block ... 13/13 PASS" | :145-146 | TRUE (re-run) | my python yaml+tempfile re-run: `bash -n blocks: ok=13 fail=0` + 1 non-bash (pwsh) block |
| 6 | "the marker was ANNOTATED, not removed — one sentence added" | :128-130 | TRUE | ui/src/client/tauri-transport.ts:93-98: `// [VERIFY at the ADR-004 admission build]` + `// (CI half: tauri-launch.yml — clears when its no-emit-failure +` — marker present, `target: { kind: "Any" }` code intact |
| 7 | "event emit failed on the shell channel: … — app.rs forward_pending's stderr line" / "lumen-tauri: admission shell failed: … + exit 1 — main.rs" | :68-72 | TRUE | apps/desktop-tauri/src/app.rs:167-168 `if let Err(error) = app.emit(...)` / `eprintln!("event emit failed on the shell channel: {error}");`; main.rs:59 `eprintln!("lumen-tauri: admission shell failed: {error:#}")` |
| 8 | "ci.yml tauri-check (run 43400610890) is RED at the CLIPPY step: 5 `expect_used`-class denials (e.g. src/app.rs:168's `.expect(...)`)" | :160-163 | TRUE-AT-TIME / RESOLVED-NOW | current tree: rg `.expect(|.unwrap()` in apps/desktop-tauri/src -> only app.rs:241, a COMMENT: "the old `.expect(\"…\")` face becomes a returned `tauri::Error`" — the denials were fixed after 2026-11-13; run ID itself UNVERIFIABLE (no git) |
| 9 | "No `scripts/**` file was added or touched" | :59-61 | UNVERIFIED | needs git history |
| 10 | "Local execution: none ... discipline for this task forbade local cargo builds/tests" | :16 | TRUE-consistent | no cargo in sandbox; nothing in tree contradicts |

## 3. Contradictions
None. Note the doc's own honesty devices check out: the "PROVES vs DOES NOT
PROVE" split (:80-93), the silent-web-crash blind spot (:95-101), and the
emit-vs-page-load gray zone (:103-109) are all documented limitations, not
hidden.

## 4. Fabrication / inflation indicators
None. The doc claims LESS than it proves (verdicts stay PENDING-CI). The one
stale item (#8) is a dated observation of CI reality that was later fixed —
the doc explicitly said "the fix is not t5's file" (:167).

## 5. Matrix rows
N/A — evidence doc, not a matrix chunk.

## 6. Cross-link to the hostile-focus question
Claim #8 matters for the CI-red answer: at 2026-11-13, ci.yml's tauri-check
job was RED (clippy denials in the workspace-excluded subcrate — the ONLY
compile lane for it). On the current tree the denials are gone (typed-error
conversion landed), so tauri-check's clippy step is expected GREEN now —
UNVERIFIED in sandbox (gdk-sys build impossible; baseline desktop-tauri build
failure is an environment limit per spec §13).
