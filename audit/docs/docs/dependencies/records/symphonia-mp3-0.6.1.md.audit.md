# CLAIM AUDIT: docs/dependencies/records/symphonia-mp3-0.6.1.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 REAL / 5 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 1 UNREACHABLE (wasm item) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An admitted (pass) record whose every in-repo-checkable claim verifies: the exact pin `=0.6.1` and checksum match the root Cargo.lock byte-for-byte, the R2 graph nodes all exist in the lock, the MPL-2.0 license-class addition to deny.toml (the record's own admission precondition) actually landed (deny.toml:117), the feature wiring (codec-mp3) is real and in the DEFAULT feature set (shipped), and both cited conformance test files exist. The record even documents its own correction of the card's wrong crate name (`symphonia-codec-mp3` does not exist). External purity-scan claims (forbid(unsafe_code), no build.rs, no advisories) are date-stamped REAL-UNPROVEN.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "admitted by card 5.A3 ... Pinned `symphonia-bundle-mp3 = "=0.6.1"`" | L143-146 | REAL | root Cargo.lock: symphonia-bundle-mp3 0.6.1 ck `98ea5ffc8716bff677df...` == record's R2 ck prefix (ran grep, byte-compared) |
| 2 | "the merged lock diff must show exactly these 9 additions" | L64-67 | REAL | lock carries bundle-mp3, core, lazy_static, log, bitflags, bytemuck, num-complex, num-traits, smallvec (grep-verified nodes; count matches the 9 named) |
| 3 | "deny.toml ... must add "MPL-2.0" ... same PR" | L119-120 | REAL | deny.toml:117 `"MPL-2.0",` present (L110 comment names the symphonia family) — the precondition was honored (ran grep) |
| 4 | "codec-mp3" feature wiring (engine dep) | R6/context | REAL | crates/codecs/Cargo.toml:78 `codec-mp3 = ["dep:symphonia-bundle-mp3", "dep:symphonia-core"]`; in default list (L55 of defaults block, verified in dnx audit) — shipped in the release lane |
| 5 | "crates/codecs/tests/mp3_bitexact.rs, mp3_corpus_differential.rs ... the recorded coverage proof" | L108-110 | REAL | both files exist (ls, ran) + mp3_bench.rs, mp3_fuzz_survival.rs, mp3/ dir |
| 6 | "the card's assumed crate name symphonia-codec-mp3 does not exist on crates.io" | L8-11 | REAL-UNPROVEN | external registry fact, plausible + internally consistent (code uses bundle-mp3) |
| 7 | "both crates declare `#![forbid(unsafe_code)]` at the crate root" | L34-37 | REAL-UNPROVEN | external tarball claim; consistent with the ecosystem-known posture |
| 8 | "No build.rs in either tarball" | L71-72 | REAL-UNPROVEN | external; consistent with the lock graph (no cc/... nodes) |
| 9 | "no advisories ... as of 2026-10-26" | L127-129 | REAL-UNPROVEN | external OSV fact, date-stamped |
| 10 | "MpaDecoder ... enforces one-packet-one-frame (header.frame_size must equal packet's remaining bytes)" | L94-97 | REAL-UNPROVEN | external API behavior; our framing layer cited as owner |
| 11 | "wasm32 left as the standing P1 verify" | L86-87, L148-149 | UNREACHABLE (item) | the standing verify item itself; wasm32 build not exercised here |
| 12 | "R2 tree pasted from cargo tree run" | L151-154 | REAL | the pasted tree matches the actual lock graph (claim 2 cross-check) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — | — | none found |

## 4. Fabrication / inflation indicators
NONE — the record corrects its own card's wrong assumption in-band, pre-registers the Lead-owned gates, and its admission effects all check out in the tree (pin, checksum, license row, feature, tests).

## 5. Matrix rows
N/A.
