# CLAIM AUDIT: docs/packaging/evidence/w26m-t5-launch-verdict.md
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| Verdict counts | TRUE: 8 (all re-executed or byte-checked) TRUE-AT-TIME/RESOLVED: 2 STALE: 1 UNVERIFIED: 1 |
| Internal contradictions found | 0 (1 staleness vs ADR-004 register) |

Note: W17-A10's earlier copy sits at
audit/docs/packaging/evidence/w26m-t5-launch-verdict.md.audit.md (missing the
`docs/` path segment); this is the canonical recovery copy, re-verified
by W17-R1 (yaml re-parse, bash -n 13/13 re-run with blocks extracted to
audit/scratch/W17-R1/, pins byte-compared, receipts re-grepped).

## 1. Blunt verdict
Every machine-checkable claim reproduces: the workflow exists exactly as
described (3 jobs / 29 steps / workflow_dispatch-only / 3 always-uploads with
if-no-files-found:error / permissions contents:read / one pwsh step), my own
bash -n over all 13 bash run-blocks passed 13/13, the five action pins are
byte-identical to ci.yml's, the tauri-transport [VERIFY] marker is annotated
in place (code untouched), and both stderr receipt strings are live code.
The doc's honesty devices (PROVES vs DOES-NOT-PROVE split, silent-web-crash
blind spot, emit-vs-page-load gray zone) are real and not hidden. The one
defect is staleness AFTER SUCCESS: the header still says all verdict rows are
"PENDING-CI; first run pending" while ADR-004's table (updated the same day,
2026-11-13) already records all three launch jobs GREEN on run 43457753166,
Linux "CI half proven" — exactly the outcome this doc's §Next-actions
prescribed, but its own header table was never updated to match.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "yaml.safe_load: PASS — 1 workflow, 3 jobs (linux-launch / macos-launch / windows-launch), 29 steps" | :140-142 | TRUE (re-run) | my python3 yaml parse -> jobs ['linux-launch','macos-launch','windows-launch'], steps total 29 |
| 2 | "triggers = `workflow_dispatch` ONLY" | :142-143 | TRUE (re-run) | same parse -> triggers: ['workflow_dispatch'] |
| 3 | "Action pins ... 5/5 byte-identical SHAs" (vs ci.yml) | :150-152 | TRUE (re-compared) | checkout@3d3c42e5..., dtolnay@d124f86f..., rust-cache@6323deb..., upload-artifact@cf430e030..., setup-node@2028fbc5... all byte-equal in tauri-launch.yml (:104/:106/:137/:150/:293...) and ci.yml (:139/:141/:154/:171/:712) |
| 4 | "3/3 jobs upload with `if: always()` + `if-no-files-found: error`; `permissions: contents: read`; one `shell: pwsh`" | :153-156 | TRUE (re-run) | my parse -> always-uploads: 3, pwsh steps: 1, permissions {'contents': 'read'} |
| 5 | "bash -n over every bash run: block ... 13/13 PASS" | :145-146 | TRUE (re-run) | my extraction of the 13 bash blocks (with ${{ }} neutralized) to audit/scratch/W17-R1/ + `bash -n` -> ok=13 fail=0 |
| 6 | "the marker was ANNOTATED, not removed — one sentence added" | :128-130 | TRUE | ui/src/client/tauri-transport.ts:93-98 `// [VERIFY at the ADR-004 admission build]` + the added `// (CI half: tauri-launch.yml — clears when its no-emit-failure +` sentence; `target: { kind: "Any" }` code intact |
| 7 | "event emit failed on the shell channel: … (app.rs) / lumen-tauri: admission shell failed: … + exit 1 (main.rs)" | :68-72 | TRUE | apps/desktop-tauri/src/app.rs:166 `eprintln!("event emit failed on the shell channel: {error}");`; main.rs:59 `eprintln!("lumen-tauri: admission shell failed: {error:#}");` + comment "exit 1 (never a fake window, 2.C3's rule)" |
| 8 | "ci.yml tauri-check (run 43400610890) RED at CLIPPY: 5 expect_used denials (e.g. app.rs:168 .expect)" | :160-163 | TRUE-AT-TIME / RESOLVED | current apps/desktop-tauri/src has zero live `.expect(`/`.unwrap(` (rg -> only app.rs:241, a comment about the old face becoming a typed error) |
| 9 | "the verdict rows (ADR-004 §Launch verdicts) are all PENDING-CI" / "Linux PENDING-CI (first run pending)" | :5-6, :15 | STALE | docs/adr/ADR-004-os-webview.md:152-154 now: Linux **CI half proven** (green on run 43457753166), macOS/Windows BEST-EFFORT-PENDING (green on run 43457753166) — the first run happened 2026-11-13; this doc's header was not updated after it |
| 10 | "No `scripts/**` file was added or touched" | :59-61 | UNVERIFIED | needs git history (prohibited) |
| 11 | "Local execution: none ... discipline forbade local cargo builds/tests" | :16 | TRUE-consistent | no cargo in audit sandbox; nothing in-tree contradicts; the lane's cargo/xvfb steps are CI-side by design |
| 12 | "cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml (the subcrate's own lockfile)" | :31-32 | TRUE-structurally | apps/desktop-tauri has its own Cargo.lock (workspace-excluded subcrate; INFRA-BLOCKED.md exists at apps/desktop-tauri/INFRA-BLOCKED.md, §B.2 referenced by this doc) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "the verdict rows ... are all PENDING-CI" (:5-6, :15) | ADR-004:152-154: Linux "CI half proven (green on run 43457753166)", macOS/Windows BEST-EFFORT-PENDING (green run) | ADR-004 (same-day update) is the truth — this doc is the pre-run design snapshot and was never struck/updated after its own §Next-actions #2 executed. Run ID 43457753166 itself UNVERIFIABLE directly (no git/network), but it is the ADR register's committed record |

## 4. Fabrication / inflation indicators
NONE. The doc claims LESS than it proves (PROVES/DOES-NOT-PROVE split at
:80-93, blind spot at :95-101, gray zone at :103-109, pwsh honestly
not-checked at :147-149). The single stale row (#9) is a success-stale
(outcome advanced past the doc), not an inflation.

## 5. Matrix rows
N/A — evidence doc, not a matrix chunk.

## 6. Cross-links
- INFRA-BLOCKED/tauri context: this lane is the CI-half launch proof for the
  ADR-004 admission (§B.2); desktop-tauri build itself remains INFRA-BLOCKED
  in this sandbox (spec §13 environment limit, baseline cargo-build-desktop-tauri).
- ADR-004:9 "all three launch jobs green on run 43457753166" corroborates the
  workflow CAN execute and go green — consistent with the known-context
  release-workflow-executed evidence.
