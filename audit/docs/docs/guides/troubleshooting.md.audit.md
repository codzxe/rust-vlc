# CLAIM AUDIT: docs/guides/troubleshooting.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R4) |
| Claims extracted | 10 |
| REAL 9 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 1 |
| Internal contradictions found | 1 (dated-defect guidance, resolved since) |

## 1. Blunt verdict
A troubleshooting guide with in-tree evidence behind every row: the typed
probe error strings exist in CLI code and tests, the charset fallback ladder
matches crates/subtitles/src/srt/encoding.rs exactly (BOM classes + no-BOM
UTF-8-else-Windows-1252), the F-1 merge-marker defect and F-2 settings-drift
rows trace to docs/audit/docs-sweep-20261110.md and both have since been fixed
in-tree (App.tsx now carries /discovery + /convert cleanly), the Playwright
1.64.1 pin and the RC_SMOKE_CHROMIUM workaround are real, and hw-decode=false
is verbatim in product.toml. The only staleness is inherent to the genre: two
"known defect on main" rows that were true on 2026-11-10 and are fixed now.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | probe.unsupported error carries code + sniffed bytes + probe hints | :9-19 | REAL | apps/cli/src/exit.rs:173 test string "error: [probe.unsupported]: ..."; containers/es.rs:541 asserts "probe.no-demuxer" in the message. |
| 2 | "37 of 73 rows claimed on desktop as of 2026-11-10" | :22-23 | REAL (dated) / 38 today | Matrix format rows = 73; linux implemented+partial = 38 now (was 37 at doc date). |
| 3 | Null audio device (ADR-005, cpal Lead-gated) + ADR-004 skeleton window | :29-33 | REAL | ADR-005-audio-output.md, ADR-004-os-webview.md exist; cpal not admitted (audio-output/Cargo.toml:12 conditional-E2 comment); KNOWN-LIMITS.md §4 rows verified in the register audit. |
| 4 | decode-bench prints BENCH line with per-track counts | :38-39 | REAL | Baseline rc-smoke.txt:130 exact BENCH line (verified in the getting-started audit). |
| 5 | "Charset detection is BOM-class only ... UTF-8-then-Windows-1252 ... BOM-less UTF-16 deliberately not sniffed" | :48-52 | REAL | crates/subtitles/src/srt/encoding.rs:9-14: `EF BB BF`→UTF-8, `FF FE`→UTF-16LE, `FE FF`→UTF-16BE, "no BOM, VALID UTF-8 → UTF-8", "no BOM, INVALID UTF-8 → Windows-1252" — exactly the documented ladder; matrix row 1085 `subtitle-encoding-detection` = partial ×3. |
| 6 | F-1: merge commit 45c5c6f committed conflict markers into ui/src/App.tsx (two regions ~15/~100) | :63-76 | REAL (dated; fixed since) | docs/audit/docs-sweep-20261110.md:179 F-1 row (same commit id); :206 probe "grep -c '<<<<<<<'" → 2 on main then 0 on the fix branch. Current tree: `rg -c '<<<<<<<\|>>>>>>>' ui/src/App.tsx` → 0 (fix landed; /discovery + /convert imports at App.tsx:49-60). |
| 7 | F-2: stale generated settings data; fix `npm run settings:generate` | :78-84 | REAL | docs-sweep F-2 row referenced; ui/package.json:24 `"settings:generate": "node scripts/gen-settings-screens.mjs"`. |
| 8 | Playwright pin 1.64.1 vs sandbox chromium-1200; RC smoke executablePath workaround | :86-95 | REAL | ui/package.json:40 `"@playwright/test": "1.64.1"`; scripts/rc-smoke.sh:24 "RC_SMOKE_CHROMIUM explicit chromium executable for check 8", :165 executablePath resolution. |
| 9 | "Device discovery landed in wave 23 ... /discovery UI, issue #42" | :99-105 | REAL | ui/src/App.tsx:49 "// Discovery panel (card 7.D1, issue #42): the device-discovery surface"; screens/discovery/ exists. |
| 10 | "features.hw-decode = false in product.toml (rule F2 ... ADR-001)" | :107-113 | REAL | product.toml:168 `hw-decode = false  # ADR-001 accepted (2026-10-19); ... rule F2 keeps this false until then`. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "the UI is unusable in every mode" (F-1, as of 2026-11-10) | Baseline ui-build.txt (2026-11-23): vite build passes | Dated truth — the F-1 fix landed after the doc; the doc's own pointer to the docs-sweep (which records the fix branch) keeps it traceable. Not fraud; standard guide aging. |

## 4. Fabrication / inflation indicators
None. The guide is the anti-hype genre: it states the product's core honesty
(null audio, no window, partial decoders) as expected behavior with sources,
and its two live defects carry commit ids and reproduction steps.
