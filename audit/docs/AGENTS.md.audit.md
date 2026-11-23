# CLAIM AUDIT: AGENTS.md
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Claims extracted | 14 |
| REAL: 7 / REAL-UNPROVEN: 2 / SHALLOW: 1 / STUB: 0 / PAPER-ONLY: 0 / FALSE: 0 / UNVERIFIED: 4 | |
| Internal contradictions found | 3 |

## 1. Blunt verdict
AGENTS.md is a working protocol that is actually followed, not aspirational wallpaper: every artifact anchor it names exists (MASTER-PROMPT.md, engineering-directives.md with the full D-0..D-11 set, the wave program, the ADR register, 22 dependency-admission records, the root worklog with 124 live `Task ID:` sections in exactly the mandated format plus a 294-entry archive), the `[VERIFY]`/`[PROPOSAL]` marker convention is practiced in 288/311 files, and the "never fabricate / no 'verified' without automated evidence" rule is backed by real machinery (the compat-matrix writer refuses `to = "verified"` in code; 0/252 matrix cells say verified). The one materially overstated claim is hard rule 3, "No unsafe code in application or core crates": it is enforced by a workspace `unsafe_code = "deny"` lint (real), but eleven files in core/application/test crates carry explicit per-file `#![allow(unsafe_code)]` re-allows (codecs MJPEG SIMD, codecs rav1d AV1 seam, codecs h264 VAAPI/VideoToolbox adapters, apps/web wasm ABI, apps/desktop-tauri windows module, testkit alloc counter) — sanctioned, documented in-file exceptions the rule text neither discloses nor provides for (its only exception clause covers "platform adapters need an approved ADR"). The code comments even contradict each other about how many sanctioned exceptions exist.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "This file is binding for every agent (human or AI) working in this repository." | AGENTS.md:3 | UNVERIFIED | process claim; adherence observable (worklog format, markers) but "binding" cannot be executed |
| 2 | "The authoritative project brief is `docs/MASTER-PROMPT.md`. If anything here conflicts … the MASTER-PROMPT wins." | AGENTS.md:4-5 | REAL | `ls docs/MASTER-PROMPT.md` → exists (369 lines) |
| 3 | "Lead agent: … all commits to `main` during Phase 1." | AGENTS.md:9-11 | UNVERIFIED | requires git history; git forbidden to agents (rule 8 is self-consistent with this audit's constraints) |
| 4 | "Clean-room: no VLC source, comments, assets, logos, shaders, branding. Public specifications, observed behavior, independent tests only." | AGENTS.md:27-28 | REAL-UNPROVEN | posture corroborated: deny.toml:394-397 bans libvlc-sys/vlc-rs/vlc; VLC appears only as external oracle (tests/compat oracle.rs, docs/corpus vlc-diff reports = observed behavior); full source-diff clean-room proof out of scope |
| 5 | "Rust-only engine … Any dependency … must be recorded … and approved by the Lead before use." | AGENTS.md:29-32 | REAL | docs/dependencies/records/ = 22 admission records (rav1d-1.1.0.md, cpal-0.18.2.md, ureq-3.5.0.md, tauri-wry-2.x.md …); deny.toml bans the C-wrapper families; ADR register docs/adr/ (13 ADRs) |
| 6 | "No unsafe code in application or core crates. Platform adapters need an approved ADR + documented invariants." | AGENTS.md:33-34 | SHALLOW | root Cargo.toml:53-54 `[workspace.lints.rust] unsafe_code = "deny"` (mechanism REAL); but `rg -l 'allow\(unsafe_code\)'` → 22 files, of which 11 are NOT platform-adapter crates: codecs ×8 (idct_simd.rs, av1/decoder.rs, h264/hw/vaapi ×4, h264/hw/videotoolbox ×2), apps/web/src/abi.rs, apps/desktop-tauri/src/main.rs, testkit/alloc_count.rs. Default-deny + documented exceptions = the rule as written overstates |
| 7 | "Never fabricate: no invented test results, no 'verified' without automated evidence … marked `[VERIFY]` … `[PROPOSAL]`." | AGENTS.md:35-38 | REAL | marker scan: `[VERIFY]` in 288 files, `[PROPOSAL]` in 311; the no-self-verification rule is enforced in code (compat_matrix.rs:444-449 rejects `to = "verified"`; matrix has 0/252 verified cells) |
| 8 | "Never weaken or delete a test to pass it. Every defect gets a regression test." | AGENTS.md:39-40 | UNVERIFIED | process claim; requires history diffing |
| 9 | "Engineering directives are binding (docs/engineering-directives.md): … (D-1..D-5) … (D-6..D-10) … (D-11)." | AGENTS.md:41-50 | REAL | file exists; `rg -o 'D-[0-9]+' docs/engineering-directives.md | sort -u` → D-0 D-1 … D-11 (complete set incl. the D-0 the doc's own checklist names) |
| 10 | "Scope discipline: touch only your assigned files." | AGENTS.md:54-56 | UNVERIFIED | process claim (this audit itself operates under the same constraint) |
| 11 | "your final report to the Lead is ≤200 words" | AGENTS.md:60-62 | UNVERIFIED | process claim |
| 12 | "Single shared log: `worklog.md` at the repository root (append-only). Prior entries are archived under `docs/worklog/`." | AGENTS.md:66-68 | REAL | worklog.md exists (2298 lines, 124 `Task ID:` sections, 247 Agent/Stage Summary fields — the §4 format followed exactly); docs/worklog/ has README + archive-2026-10.md (294 entries) + w24 review files |
| 13 | "Wave program: `docs/13-agent-wave-program.md`." (Task ID scheme source) | AGENTS.md:95 | REAL | file exists; IDs in live use (compat_matrix.rs:1 "card 8.B2"; ci.yml "card 1.E2"; worklog entries) |
| 14 | "One document / one crate / one bounded task per agent. … do not silently truncate." | AGENTS.md:99-102 | UNVERIFIED | process claim |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "No unsafe code in application or core crates" (AGENTS.md:33) | 11 non-platform files carry `#![allow(unsafe_code)]`, incl. 8 in `crates/codecs` and 2 in `apps/*` | The doc is the weaker side: `rg -l 'allow\(unsafe_code\)' crates/ apps/` lists them; codecs/apps are core/application crates. Each exception is documented in-file (SAFETY comments, sanctioned-exception headers), so the mechanism is honest — the rule text is not |
| (rule 3 has one exception clause: platform adapters + ADR) | codecs/src/video/mjpeg/idct_simd.rs:2-3: "the ONE sanctioned `unsafe` exception in the repo" vs codecs/src/video/av1/decoder.rs:10-11: "the repo's second sanctioned `unsafe` exception (the MJPEG SIMD kernel … is the first)" | The av1 comment is current; the idct_simd "the ONE" claim is stale — and both undercount: the h264 hw adapters, apps/web/abi.rs, desktop-tauri/main.rs and testkit/alloc_count.rs also re-allow unsafe (22-file inventory) |
| idct_simd.rs:2-3 sanctions itself by "contract `.agent-brief-w7.md`" | `ls .agent-brief-w7.md` → No such file or directory; only mention is docs/worklog/archive-2026-10.md ("Wrote .agent-brief-w7.md; dispatched Wave 7") | The sanctioning contract was never committed — a dangling evidence citation for one of the repo's unsafe exceptions |

## 4. Fabrication / inflation indicators
None material. The doc's checkable anchors all resolve (every referenced file exists; the worklog format, marker convention, dependency records and ADR register are real artifacts at scale). The single inflation is rule 3's absolutist phrasing against a default-deny-plus-sanctioned-exceptions reality — an overstatement of enforcement, not a fabricated capability.

## 5. Matrix rows
N/A (not a matrix chunk).
