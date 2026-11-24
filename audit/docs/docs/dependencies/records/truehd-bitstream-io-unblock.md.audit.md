# CLAIM AUDIT: docs/dependencies/records/truehd-bitstream-io-unblock.md
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 11 |
| Verdict counts | TRUE: 7 (5 of them verified byte-for-byte against the parent record) TRUE-in-scratch/UNVERIFIED-here: 1 UNVERIFIED: 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A clean decision record: it changes nothing in the tree (no truehd, no
bitstream-io, no [patch.crates-io], no allowlist row — all verified absent),
and every number it cites is byte-faithful to its parent record
(truehd-0.7.2.md R6/R10/R11): the dmesg line, the =4.10.0 pin, the
huffman.rs:129+ root cause, and the bit-exact scratch differential. The
three options are argued with real cons, the recommendation is honestly
marked [PROPOSAL — Lead], and the predicted end-state matches reality: the
matrix row is still `blocked` on all four platforms with this record's
unblock path embedded in its blocked_reason. The only unverifiable items are
the branch/commit provenance (git prohibited), the crates.io release-date
claim (no network), and the scratch substitution itself (no cargo; rebuilding
truehd would also re-trigger the OOM — out of budget by design).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "this record changes no Cargo.toml, no code, no allowlist" | :5-6 | TRUE | rg -i truehd/bitstream over Cargo.toml, Cargo.lock, crates/codecs/Cargo.toml -> no hits (only filmcraft-bitstream, an unrelated crate); rg 'patch.crates-io' Cargo.toml -> NO-PATCH-SECTION; rg bitstream/truehd deny.toml -> NO-ROWS |
| 2 | "Parent record: `truehd-0.7.2.md` (purity PASS, admission BLOCKED at R10; root cause isolated at R11)" | :6-7 | TRUE | truehd-0.7.2.md:172-174 R10 "**blocked (purity PASS — F1–F7 all NO, no build scripts, 2 `unsafe`**" + :189+ "## R11 — The blocker, isolated" |
| 3 | "the OOM fires at ~3.4–3.6 GB anon-RSS ... (dmesg: `total-vm:20.6 GB, anon-rss:3.58 GB`)" | :15-17 | TRUE (byte-match to parent) | truehd-0.7.2.md:195-196 "`Out of memory: Killed process … (rustc) total-vm:20.6 GB, anon-rss:3.58 GB` (host: 4 GB RAM, no swap)" |
| 4 | "the transitive `bitstream-io = \"=4.10.0\"` ... `src/huffman.rs:129+`" | :13-15 | TRUE (byte-match) | truehd-0.7.2.md:203 "`bitstream-io = \"=4.10.0\"` and invokes" + :208-209 "(`compile_write_tree_nodes!`, bitstream-io 4.10.0 `src/huffman.rs:129+`)" |
| 5 | "decode-quality ... answered YES in scratch: four-stream ffmpeg differential, authored streams bit-exact (R6, MD5s quoted there)" | :19-22 | TRUE (in parent) | truehd-0.7.2.md:112-141 R6 bit-exact section incl. :128 "reproduces the s24le MD5 `3d092c4c77896b975338d1a08137dfba`" |
| 6 | "(b) ... exactly the substitution already proven in scratch: full `truehd 0.7.2` compiles clean" | :47-49 | TRUE-as-recorded / UNVERIFIED-here | scratch evidence lives in the parent record only; this sandbox has no cargo, and reproducing would re-trigger the documented rustc OOM (load discipline prohibits) |
| 7 | "4.10.0 is the newest release, 2026-04-14" | :36-37 | UNVERIFIED | network claim; parent R11:213 repeats "bitstream-io 4.10.0 is the newest release" (same-source corroboration, not independent) |
| 8 | "the `codec-audio-truehd-mlp` matrix row stays `blocked` with this record linked as the unblock plan" | :110-112 | TRUE | docs/feature-matrix.toml:779-790: status = { windows="blocked", linux="blocked", macos="blocked", web="blocked" }; blocked_reason = "...bitstream-io 4.10.0 ToBits macro OOM root-caused; scratch differential 4/4 bit-exact (4,065,640 samples)... unblock = upstream fix or ~60-line vendor patch" |
| 9 | "Date: 2026-11-09, branch w15/07-codec-admissions at main 62453fb" | :8 | UNVERIFIED (git) / corroborated | worklog.md:354 "b900c0b: records added — ... truehd-bitstream-io-unblock (+117)" — the +117 line count matches this file exactly (117 loc); commit hashes not checkable |
| 10 | "no new measurements in this record — every number and claim above is cited from the parent record ... or the in-tree codecs suite evidence" | :114-117 | TRUE (structural) | every quantitative string I checked (3.58 GB, 20.6 GB, =4.10.0, huffman.rs:129+, MD5, 4/4) byte-matches truehd-0.7.2.md |
| 11 | "[PROPOSAL — Lead] ... Adopt (b) ..." | :82-84 | TRUE-as-proposal / outcome open | the tree shows option (b) NOT implemented (no patch, no admission — claim 1 evidence), consistent with "declines (b): stays honestly BLOCKED"; worklog.md:357 "TrueHD unblocked via the bitstream-io decision" refers to the decision RECORD landing (merge 55a14f9), not to admission |

## 3. Contradictions
None within the doc. Nuance, not contradiction: worklog.md:357's phrase
"TrueHD unblocked via the bitstream-io decision" could be misread as
admission; the manifests and the matrix row prove nothing was admitted —
the record's own "declines (b)" branch is the live state.

## 4. Fabrication / inflation indicators
NONE. The doc claims LESS than its parent record holds (it defers every
measurement), marks itself [PROPOSAL — Lead], pre-declares the re-scan
obligations a (b) adoption would trigger (fork re-scan + sha256 at :98-100),
and predicts the honest end-state that the matrix now shows.

## 5. Matrix rows
N/A — decision record, not a matrix chunk. (Cross-ref: matrix row
codec-audio-truehd-mlp verified under claim 8.)
