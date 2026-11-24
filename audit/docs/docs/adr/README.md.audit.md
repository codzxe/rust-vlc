# CLAIM AUDIT: docs/adr/README.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL 7 / FALSE 0 / UNVERIFIED 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The register is exactly what it claims to be: an index that quotes statuses rather than
re-deciding them. Every row I cross-checked matches its ADR's own Status: line; the
numbering gaps (007/008/009 do not exist) are real and their dangling references are
real; the one drift flag (ADR-013 proposed while VA-API code landed) is factually
accurate — the vaapi module is in the tree. Honest, current, self-limiting.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Statuses are quoted from each ADR's own Status: line — register never re-decides | :7 | REAL | Spot-checks: ADR-012:3 "ACCEPTED — Lead signature 2026-10-24" == register's "accepted (Lead signature)"; ADR-005:3 "**accepted for the adapter-seam architecture**" == register; ADR-013:3 "**proposed** (drafted 2026-11-09, issue #2...)" == register; ADR-011:3 "**accepted (2026-11-09)**" == register's accepted row. |
| 2 | Register covers 11 ADR files (001..006, 010..013 + evidence pack) | table | REAL | ls docs/adr -> ADR-001..006, ADR-010..013, ADR-002-evidence.md, README.md — 11 records + register; every file listed has a row. |
| 3 | ADR-007, ADR-008, ADR-009 do not exist; sequence jumps 006→010 | :25-27 | REAL | ls docs/adr | rg 'ADR-00[789]' -> zero matches. |
| 4 | ADR-008 referenced as future owner (optical disc + WebCodecs) but never opened | :28-31 | REAL | docs/formats/disc-dvd-video.md:52-53 "scope ownership to 'ADR-008 (optical disc scope)' — **ADR-008 is not [open]**"; docs/formats/video-hevc.md:27 "WebCodecs is ADR-008 territory" — both dangling references exist as quoted. |
| 5 | ADR-009 referenced once ("crate registry, ADR-009") in doc 09 header | :32 | REAL | docs/09-web-wasm-target.md:8 "(crate registry, ADR-009)" — single reference, byte-exact. |
| 6 | ADR-013 drift: ADR says decision due before VA-API code lands, yet wave-23 landed VA-API internals (compile-verified) | :23 | REAL | crates/codecs/src/video/h264/hw/vaapi/ exists (device.rs, dl.rs, ffi.rs, library.rs) while ADR-013:3 still reads "**proposed**" — the drift flag is accurate and unresolved. |
| 7 | ADR-005 note: cpal NOT admitted (conditional-E2), every platform refuses real output through a typed marker | :18 | REAL | crates/audio-output/Cargo.toml:12 "cpal is conditional-E2" (comment-gated, not a shipped dep); the audio-output stub/null device lanes are prior-wave-audited reality. |

## 3. Contradictions
None found. The register's self-description (quotes-only, Lead-owned status changes,
append-only) matches AGENTS.md's discipline and the observed files.

## 4. Fabrication / inflation indicators
None — the register's notable content is mostly the disclosure of its own gaps
(numbering holes, one honest drift flag with a named next step).

Commands run: rg 'Status:' across ADR-005/012/013/011; ls docs/adr (+ absence check 007-009); rg ADR-008/ADR-009 references; ls crates/codecs/src/video/h264/hw/vaapi; rg cpal in audio-output Cargo.toml.
