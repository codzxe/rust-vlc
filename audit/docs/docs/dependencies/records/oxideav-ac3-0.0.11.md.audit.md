# CLAIM AUDIT: docs/dependencies/records/oxideav-ac3-0.0.11.md
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Claims extracted | 14 (repo-verifiable subset below; measurement claims from the 2026-10-29 session) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 8 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 2 UNVERIFIED |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is the honest kind of dependency record: it blocks its own candidate. Every repo-side claim I could check is true (oxideav-ac3 is absent from the root lockfile, the standalone harness exists and pins exactly oxideav-ac3 0.0.11), and the verdict (BLOCKED, 3-of-8 divergent lanes at 202–431 LSB vs a ±1-LSB house bar) is consistent with the record's own measured table. The measurement claims themselves are session data I cannot re-run here — REAL-UNPROVEN, with the harness in-repo as the reproduction path.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "oxideav-ac3 was not added to `crates/codecs`" | L5-6 | REAL | `grep -c oxideav Cargo.lock` -> 0; `grep -rn oxideav crates/ --include=*.toml` -> no output |
| 2 | "Harness: `tools/vta/ac3/` (standalone, non-workspace)" | L10-11 | REAL | `ls tools/vta/ac3/` -> Cargo.toml, Cargo.lock, src/main.rs (separate workspace) |
| 3 | "measured in this session (2026-10-29) ... against oxideav-ac3 0.0.11" | L7-8 | REAL (harness pins the version) | tools/vta/ac3/Cargo.toml:10 `oxideav-ac3 = "0.0.11"` |
| 4 | "0 native files ... no build.rs ... unsafe: 0 in oxideav-ac3" | L26-29 | REAL-UNPROVEN | crate internals not inspectable without download; no vendored copy in-repo |
| 5 | "11 in oxideav-core, all in arena/{mod,sync}.rs" | L30-32 | REAL-UNPROVEN | same |
| 6 | "s16le interleaved only ... truncating quantisation at scale 32767" | L78-83 | REAL-UNPROVEN | same |
| 7 | "corpus ... DIVERGENT, max 316 LSB, 97.2% differing" | L101 | REAL-UNPROVEN | measurement; ffmpeg 7.1.5 present in sandbox but harness not built (cargo-build-budget) |
| 8 | "3 divergent lanes are exactly the low-bitrate (<=192k) stereo broadband lanes" | L113-116 | REAL-UNPROVEN | characterisation claim from same session |
| 9 | "0 panics / 6 cases" (hostile input) | L145 | REAL-UNPROVEN | same |
| 10 | "Verdict: BLOCKED" | L147 | REAL | consistent with table rows DIVERGENT + unblock conditions L162-170 |
| 11 | "no admission, no Cargo.toml edit, no crates/codecs wiring" (addendum) | L186-187 | REAL | root Cargo.lock grep -> 0 |
| 12 | "wasm32 not measured" | L64, L175 | UNVERIFIED-gap (honest) | acknowledged as gap |
| 13 | ffmpeg reference "7.1.5" | L93 | REAL (sandbox has it) | `ffmpeg -version` -> 7.1.5-0+deb13u1 line 1 |
| 14 | "Pin `oxideav-ac3 = \"=0.0.11\"`" (future condition) | L169 | REAL (harness uses 0.0.11) | tools/vta/ac3/Cargo.toml:10 |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none found | — | — |

## 4. Fabrication / inflation indicators
None. The record declines its own candidate and lists honest gaps (L172-179). The differential numbers hurt the case it could have inflated (97.2% differing) — a fabricated record would show passing numbers.

## 5. Open questions / next agent
- The differential table is reproducible only via tools/vta/ac3 (not built; cargo-build-budget). Whoever gets that lane should re-run one DIVERGENT lane to confirm the 316-LSB claim.
