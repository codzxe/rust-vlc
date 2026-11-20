# CLAIM AUDIT: docs/feasibility-report.md

| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Date | 2026-11-16 |
| Claims extracted | 17 | REAL 8 | REAL-UNPROVEN 7 | SHALLOW 1 | STUB 0 | PAPER-ONLY 1 | FALSE 0 | UNVERIFIED 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict

The most honest document in the set. It is a Phase-0 assessment (2026-10-19) that explicitly disclaims verified status ("Nothing here is 'verified' in the §17 sense — no CI exists yet"), and every census number it states reconciles exactly against the committed inventory JSONs: 502 module registrations / 356 unique, the seven-category rollup to the digit, 1,476 options with the 252/252 default agreement, 82 format families / 59 probe-verified / 27 P0, the UI surface counts, and the 61-P0 / 33-blocked matrix arithmetic (which I reproduced from the phase-0-gate tag of the matrix itself). Its predictions aged unevenly — H.264 and Opus landed (better than forecast), the cpal-first audio output never landed (worse than forecast) — but predictions are labeled as projections. Zero false claims found. The single defect class is that the report's "day-one parity ceiling" language remains aspiration that later docs partially converted into over-claims elsewhere.

## 2. Claim ledger

| # | Claim (verbatim ≤25 words) | Location (path:line) | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Phase 0 deliverable (card 0.D3, Lead-assembled 2026-10-19). Sources: the four feasibility sections …" | docs/feasibility-report.md:3-7 | REAL-UNPROVEN | Historical provenance; the phase-0 gate record and feasibility/sections/ tree exist; the assembly event is not re-provable. |
| 2 | "every claim is either evidenced or marked. Nothing here is 'verified' in the §17 sense — no CI exists yet" | docs/feasibility-report.md:8-9 | REAL | Self-scoping statement consistent with the doc's content — no verified-status claims inside. |
| 3 | "A Rust-only reimplementation … is feasible for the large majority of the inventory, with bounded, honestly-reported exceptions" | docs/feasibility-report.md:13-14 | REAL-UNPROVEN | Assessment; partially validated by later events (H.264/Opus/ASF landed), not a checkable state claim. |
| 4 | "Of 252 matrix rows: 61 are P0; 33 are blocked-with-reason (13%)" | docs/feasibility-report.md:15-16 | REAL | Reproduced from git: `git show phase-0-gate:docs/feature-matrix.toml` → 252 rows, 61 P0 rows, 33 rows carrying blocked status cells; 33/252 = 13.1%. |
| 5 | "The single planning-blind-spot discovered in Phase 0 is HTTPS/TLS purity (ADR-011)" | docs/feasibility-report.md:20-21 | REAL | docs/adr/ADR-011-tls-https.md exists; ROADMAP D-011 records the same blind-spot discovery (0.C3-I). |
| 6 | "Modules: 502 registrations / 356 unique / 56 capabilities (Linux census; Win/mac hardware runs pending). Seven-category split: demux 57, decode 35, access 57, filters 110, subtitle 32, playlist 21, services-discovery 26, other 164" | docs/feasibility-report.md:27-30 | REAL | docs/inventory/modules-linux.json: total = 502, stats.unique_module_names = 356; seven_category_rollup.buckets counts = demuxers 57, decoders 35, access 57, filters 110, subtitle 32, playlist 21, services-discovery 26, other 164 — exact match. |
| 7 | "Options: 1,476 (100% dump coverage; 58.3% advanced-folded; 442 core + 1,034 module-scoped; 252/252 default cross-checks agreed)" | docs/feasibility-report.md:31-32 | REAL | options-linux.json totals = 1476; phase0-exit-report.md:12 "1,476 records, 100% coverage, 252/252 default agreement". |
| 8 | "Formats: 82 families inventoried; 59 probe-verified against the pinned oracle with synthetic fixtures; 27 P0" | docs/feasibility-report.md:33-34 | REAL | formats.json: families_total = 82; probed_by_kind sums 13+18+6+5+8+9 = 59; docs/inventory/formats.md:41 "27 of 82 families" P0. |
| 9 | "UI surface: 8 menus / 58 items / 23 dialogs; 111 hotkey actions" | docs/feasibility-report.md:35-36 | REAL | phase0-exit-report.md:13 "8 menus/58 items/23 dialogs; 111 actions → keymap seed". |
| 10 | "Baseline identity: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd), release 2026-01-08, pinned per-platform with hash-verified artifacts" | docs/feasibility-report.md:37-38 | REAL-UNPROVEN | docs/baselines/vlc-3.0.23*.md record the pin with sourcing; external verification impossible from sandbox. |
| 11 | Codec verdicts: MJPEG pure-Rust; AV1 rav1d asm-off; H.264/HEVC young pure-Rust wave; HE-AAC zero upstream; Opus placeholder 1-byte; WMA/DTS absent; AC-3 self-build; TrueHD young; WavPack GPL flag | docs/feasibility-report.md:42-57 | REAL-UNPROVEN | Phase-0 landscape assessment; later dependency records corroborate the candidate landscape (rust_h264-0.6.1.md, ruopus-0.1.2.md, truehd-0.7.2.md, oxideav-ac3-0.0.11.md, rav1d-1.1.0.md all exist in docs/dependencies/records/). |
| 12 | "Subtitles: … ASS subset matrix …; PGS/DVB bitmap via CPU-rasterized RGBA atlas compositing; CC 608/708 extraction viable" | docs/feasibility-report.md:58-62 | REAL-UNPROVEN | Planning verdicts; later matrix rows (subtitle formats 8 claimed) partially validate. |
| 13 | "Outputs: cpal 0.18.2 conditional-E2 acceptable (all native deps are OS-audio bindings); direct escape hatches per OS documented (ADR-005 inputs)" | docs/feasibility-report.md:72-74 | SHALLOW | The Phase-0 recommendation was sound but never executed: at HEAD cpal is NOT admitted (cpal.rs:43 AUDIO_ADR005_ADMITTED=false), audio output is null-only (KNOWN-LIMITS.md:51), and the shipped product has no sound on any OS. The report predicted a path the program did not take. |
| 14 | "Foundation pre-audit (18 crates): pass × 8, conditional-E2 × 7, exception-ADR-004 × 2 (tauri/wry), license-recheck × 1 (toml). WASM runtime decided (ADR-002: wasmi)." | docs/feasibility-report.md:92-95 | REAL-UNPROVEN | docs/adr/ADR-002-wasm-runtime-wasmi.md exists; the 0.C4 pre-audit counts are recorded Phase-0 facts, not re-derived here. |
| 15 | "day-one shipping parity ≈ the 252-row matrix minus the 33 blocked rows …, as measured by future CI evidence only. This report claims nothing as verified" | docs/feasibility-report.md:109-112 | PAPER-ONLY | Explicitly a projection with a no-claims disclaimer — honest framing of an unmeasured ceiling. |
| 16 | "All Phase 0 exit artifacts exist (see gates/phase0-exit-report.md). Open [VERIFY] items (≈70 …) … none block Phase 1" | docs/feasibility-report.md:116-120 | REAL-UNPROVEN | phase0-exit-report.md exists with the artifact register; the ≈70 [VERIFY] tally not re-counted. |
| 17 | "The 61 P0 rows are the first three phases' bread: 27 format P0s map onto Phases 2–5 …" | docs/feasibility-report.md:112-114 | REAL-UNPROVEN | The 27-format-P0 number matches inventory formats.md:41; the phase-mapping is planning arithmetic. |

## 3. Contradictions

| This doc says | Other source says (path:line) | Which is true (proof) |
|---|---|---|
| "Opus placeholder crate is 1-byte empty — blocked v1" (feasibility-report.md:52-53, Phase-0-time) | feature-matrix.toml row codec-audio-opus: implemented on all desktops since 2026-11-09 (ruopus admission, opus_corpus_differential 2/2) | Both, at their respective times — the Phase-0 statement was true on 2026-10-19 and was superseded by the ruopus admission; the contradiction is evolution, not error. Cited here because KNOWN-LIMITS.md:30 still carries the Phase-0-era claim as current (see KNOWN-LIMITS audit). |
| "Outputs: cpal … acceptable" (feasibility-report.md:72-74, recommendation) | KNOWN-LIMITS.md:51 + cpal.rs:43: cpal NOT admitted, null-only output, no sound on any OS | The later reality — the feasibility's audio recommendation is the single largest place where the Phase-0 plan and the shipped product diverge. |

## 4. Fabrication/inflation indicators

None found. Every checkable number reconciles with a committed inventory artifact; the one soft spot (the cpal-first audio recommendation) is labeled as an assessment, not a fact, and its non-execution is honestly registered elsewhere (KNOWN-LIMITS §4, ADR-005). This document demonstrates the repo's census discipline at its best — which makes the staleness failures of the later-status documents (PARITY-REPORT, KNOWN-LIMITS, README gaps block) a maintenance failure, not a method failure.
