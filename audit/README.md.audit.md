# CLAIM AUDIT: README.md (L1-147)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 1 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The README is the most honest top-level status page I have audited in this ledger: its headline capability number ("38 of the 73 format rows… zero on web") is EXACTLY reproducible from feature-matrix.toml (38 = 14 containers + 7 video + 9 audio + 8 subtitle; web 0 — recomputed with python3 over all 252 rows), it repeats the 0/252 verified-parity number that matches the orchestrator's parity-report, and it volunteers the three ugliest product truths (no sound on any OS, no video in the desktop app, URL sources refused). Nothing inflated. The unverifiable remainder (release history, phase gates, wave numbers) is git-history claims I cannot check without git.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "38 of the 73 format rows… claimed (implemented or partial) on desktop (14 containers, 7 video codecs, 9 audio codecs, 8 subtitle formats) — zero on web" | :41-43 | REAL (executed) | python3 recount of feature-matrix.toml: format rows=73 (21/16/19/17); windows=linux=macos=38 claimed (14/7/9/8), web=0. Exact match. |
| 2 | "Nothing is claimed implemented or verified until feature-matrix.toml records automated evidence" | :53-56 | REAL (policy) | Corroborated by matrix header R4 rules + parity 0/252. |
| 3 | "verified parity is still 0/252" | :132 | REAL | Baseline: audit/runs/baseline/xtask-parity-report.txt:61 "parity: rows=252 windows=66/252 linux=61/252 macos=66/252 web=0/252" (strict verified = 0). |
| 4 | "audio output is the ADR-005 null device — no sound on any OS yet" | :45 | REAL | apps/cli/Cargo.toml comment: "the audio path over the null device — no audio hardware in this sandbox, ADR-005 era"; engine-info features list "audio.null-device" (host.rs features vec). |
| 5 | "the desktop app… displays no video yet… canvas path is built but unwired" | :46-48 | REAL-UNPROVEN | Consistent with docs and honest-limitation framing; desktop-tauri build is env-blocked in this sandbox (baseline: gdk-sys missing) — cannot execute to confirm. |
| 6 | "URL sources (http/hls) are refused with a clear message (the streaming engine exists)" | :48-49 | REAL | crates/streaming exists (real crate); refusal claim consistent with ADR-011 admission state (see ADR-011 audit). |
| 7 | "unsafe code denied in application/core crates" | :123 | REAL (with documented codec-FFI exceptions) | Actual unsafe blocks: playback-engine 0, demux 0, media-types 0, input 0, media-probe 0, audio-engine 0, subtitles 0 (rg "unsafe (fn|{|impl|extern)"). codecs has 155 unsafe sites — the C-FFI seams documented as sanctioned exceptions (crates/codecs/src/video/av1/decoder.rs:4-10 "The unsafe seam (and why it exists)… second sanctioned unsafe exception"). Wording is loose (codecs is arguably core), but the exceptions are documented, not hidden. |
| 8 | Phases 0–6 complete; Phase 7 landed waves 20–23; "Releases v0.1.0, v0.1.1 and v0.1.2 are published" | :18-29 | UNVERIFIED | Requires git/release history; referenced records exist (docs/gates/phase5-exit.md, phase6-exit.md, docs/RELEASE.md — all present). |
| 9 | "Phase 8 — in progress (2026-11-15, wave 30…)" | :16-17 | UNVERIFIED | Wave numbering (30) vs this audit's wave-19 context — cannot verify; date-plausible. |
| 10 | Cited files: RELEASE.md, KNOWN-LIMITS.md, ROADMAP.md, NEXT-STEPS.md, guides/getting-started.md, release-notes/skeleton-v0.1.3.md, PARITY-REPORT.md, ADR-006 | :50-63,142-146 | REAL | All exist (ls-verified). |
| 11 | "Build from source… `cargo build --release --locked -p lumen-cli -p desktop`" | :82-83 | REAL | Baseline: release lane builds clean, exit 0 (cargo-build-release-lumen-cli.txt, cargo-build-release-desktop.txt). |
| 12 | "Working name 'Lumen'… fully rebrandable via product.toml" | :11-12 | REAL | product.toml + product.schema.json exist; config-validator baseline shows 4 missing brand assets (a defect in the direction of NOT-yet-rebrandable — recorded honestly as validator FAILURE in baseline). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none material) | — | — |

## 4. Fabrication / inflation indicators
None. The status claims under- rather than over-state (0/252, no sound, no video). The one asymmetry: "zero on web" is stated as a capability fact while the wasm module does build and decode refusal is typed (compat-matrix) — the README wording is still the conservative truth.
Commands run: python3 matrix recount (73/38/14-7-9-8/web-0), rg unsafe-block counts per crate, ls existence checks. Exit 0.
Findings: S2: 0 · S3: 0 · S4: 1 (F-016 "unsafe denied in application/core crates" silently scopes out the codecs crate where 155 unsafe sites live — documented exceptions, but the README sentence alone would mislead a casual reader).
