# CLAIM AUDIT: docs/formats/sub-cc.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 7 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 REAL / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Every claim verified, both gate commands RE-EXECUTED by this auditor with exact matching
numbers (28/0 lib cc, 11/0 cc_acceptance). The module's honesty table (IN/OUT split, open
5.F1 seam) matches the matrix row notes. This is what a records-consolidation doc should
look like — including its own admission that no live H.264 SEI feed reaches it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "row status `implemented` on win/linux/macos, `not_started` on web" | L10-12 | REAL | feature-matrix.toml:1030 status implemented×3, web not_started (rg-verified) |
| 2 | "self_built = [`subtitles::cc` (`cc_data`/`eia608`/`cea708`/`render`/`fuzz` + CcDecoder/CcCueBuilder …)" | L23-24 | REAL | ls crates/subtitles/src/cc/ → cc_data.rs cea708.rs eia608.rs fuzz.rs mod.rs render.rs |
| 3 | "`cargo test --locked -p subtitles --lib cc` — 28 passed / 0 failed" | L54-55 | REAL (EXECUTED) | I ran it (PATH=/tmp/my-project/.cargo/bin): `test result: ok. 28 passed; 0 failed; 0 ignored; … 239 filtered out; finished in 0.01s` |
| 4 | "`cargo test --locked -p subtitles --test cc_acceptance` — 11 passed / 0 failed" | L56-57 | REAL (EXECUTED) | I ran it: `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s` |
| 5 | "no `fuzz_sub_cc` target exists in `fuzz/` yet" | L15-17 | REAL | ls fuzz/fuzz_targets/: fuzz_sub_ass/srt/vtt only — no fuzz_sub_cc |
| 6 | "crates = [] — self-built, std-only, no `unsafe`" | L18 | REAL | subtitles Cargo.toml: "Std-only by design (doc 11: zero external dependencies…)"; rg unsafe in cc/*.rs → 3 hits, all doc-comments denying unsafe; zero unsafe blocks |
| 7 | "Open seam … the feed from a live H.264 decode is not wired … No end-to-end proof from a real H.264 SEI stream yet" | L84-86 | REAL (honest limit) | matrix notes:1032 "the codecs-side SEI/user-data FEED wiring (H.264 5.F1 consume seam) … remain open" — consistent with W9 finding (subtitles unreachable from shipped entry points) |

## 3. Contradictions
None. Matrix note tells the same seam story as the doc (notes: "Row was STALE not_started
x4 … flipped 2026-10-26 as a RECORDS CORRECTION only" — a status flip WITH matching code,
the opposite of the hotkey-251 pattern).

## 4. Fabrication / inflation indicators
None. Gates were re-run by this auditor and match to the digit. The `implemented` matrix
status coexists with honest OUT-list (708 native window model "walked + op-recorded, never
applied") — the doc explicitly does not claim 708-native completeness. Reachability
caveat: the module itself is real + tested, but nothing in the shipped CLI feeds it
SEI/user-data (open 5.F1 seam) — end-to-end is UNREACHABLE today, which the doc states.
