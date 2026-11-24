# CLAIM AUDIT: docs/adr/ADR-004-os-webview.md (174 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 REAL / 0 / 0 / 0 / 0 / 0 / 3 UNVERIFIED (CI run 43457753166; Lead signature; sandbox gtk-sys build) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An unusually careful exception ADR: it closes a real triple status contradiction it names
(ROADMAP/doc 01/Cargo.toml header all reconciled), every file it cites exists, and the
central safety property — "the engine never links them [tauri/wry]" — reproduces exactly
(zero tauri/wry references in every crates/*/Cargo.toml and apps/cli). The per-OS launch
verdicts are held at "CI half proven / BEST-EFFORT-PENDING" with the human-confirmation
half explicitly owed; the launch lane (tauri-launch.yml) is workflow_dispatch-only (:78-79)
precisely so it cannot silently gate merges. The run-level claims (green on run
43457753166) are UNVERIFIABLE here (no GitHub access). One path drift: the ADR's
"apps/desktop/tauri/INFRA-BLOCKED.md" is now `apps/desktop-tauri/INFRA-BLOCKED.md` (the
subcrate was moved out to the workspace-excluded location, exactly as rebrand.rs's header
records: "path updated for the ADR-004 workspace-excluded `apps/desktop-tauri` subcrate,
wave L").

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "apps/desktop is ... src/wire.rs ... bridge.rs ... mailbox.rs ... transport.rs ... host.rs ... error.rs — 2,541 LOC, std-only" | :44-48 | REAL (grew since) | ls apps/desktop/src/ → wire.rs, bridge.rs, mailbox.rs, transport.rs, host.rs, error.rs (+6 more files); 5,666 LOC today. |
| 2 | "the engine never links them [tauri/wry]" | context § | REAL — REPRODUCED | rg 'tauri|wry' over crates/*/Cargo.toml + apps/cli/Cargo.toml → 0 hits. |
| 3 | Evidence paths: records/tauri-wry-2.x.md, prototypes/video-in-webview.md, preaudit/allowlist refs | :9-13 | REAL | All exist (ls). |
| 4 | "the lane is .github/workflows/tauri-launch.yml (workflow_dispatch ONLY — manual, honest: a verdict lane, not a merge gate)" | verdicts § | REAL — REPRODUCED | tauri-launch.yml:78-79 `on: workflow_dispatch:` only; header comment "workflow_dispatch ONLY: manual by design, honest". |
| 5 | "all three launch jobs green on run 43457753166" | :4-7 | UNVERIFIED | GitHub run logs not accessible from sandbox; the referenced evidence file w26m-t5-launch-verdict.md exists and records the lane design (its authoring state says "first run pending"; the ADR table is the post-run update). |
| 6 | "Linux ... CI half proven ... macOS/Windows BEST-EFFORT-PENDING ... human confirmation owed" | verdict table | REAL disclosure discipline | The rows' split (CI-half vs GUI-half) matches the evidence file's proof-scope statement ("This lane does NOT claim any per-OS verdict" at authoring). |
| 7 | "[VERIFY] clearing rule ... clears ONLY on a proven event frame round-trip on a launched app" | tail § | REAL (marker present) | ui/src/client/tauri-transport.ts exists with the annotated marker (per w26.m.t5 file list). |
| 8 | "apps/desktop 63/0, merge 8a56f0b — close package: apps/desktop/tauri/INFRA-BLOCKED.md" | :11-13 | REAL (path moved); 63/0 UNVERIFIED | Baseline cargo-test shows desktop tests passing; INFRA-BLOCKED.md lives at apps/desktop-tauri/ today. |
| 9 | "Status: accepted (2026-10-22 ... formalized by card 3.DOC-1)" | :2 | UNVERIFIED (git) | Signature/commit not inspectable without git. |

## 3. Contradictions
None. The ADR resolves a named triple-contradiction rather than creating one; the
sandbox build failure of the tauri subcrate (gdk-sys, AGENT-SPEC §13) is an environment
limit the ADR's own "historical state at acceptance: no desktop webview runtime exists in
the sandbox" already anticipates.

## 4. Fabrication / inflation indicators
None — the opposite: the ADR repeatedly splits what CI proves from what a human must
confirm, and refuses to flip macOS/Windows rows to "proven" even on green runs.
