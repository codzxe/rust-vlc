# CLAIM AUDIT: docs/security/sweep-supply-chain.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 12 |
| REAL 5 · REAL-UNPROVEN 3 · SHALLOW 0 · STUB 0 · PAPER-ONLY 0 · FALSE-on-current-tree 2 · UNVERIFIED 1 · consistent-drift 1 |
| Internal contradictions found | 1 (graph inventory names crates absent from the lock / not admitted) |

## 1. Blunt verdict
A mostly rigorous sweep record whose checkable specifics survive: the tool pin matches ci.yml byte-for-byte, the duplicate-version taxonomy (getrandom 0.2.17/0.4.3, syn 2.0.119/3.0.9) is exactly what today's lock contains, the RUSTSEC `paste` 1.0.15-via-rav1d-1.1.0 chain has both crates in the lock, and the deny.toml comment-rot it flags is real and still unfixed. But the §3 "graph posture" inventory is sloppy: it names `oxideav` and `cpal` as crates in the non-dev Linux graph — oxideav is in NO lock (its admission was BLOCKED) and cpal is in NO lock (allowlist §5 itself says "NOT admitted"); plus the lock has moved 156→158 packages since the sweep. The tool-run claims (five gates green) cannot be re-executed offline.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | cargo-deny tarball "sha256 9f12ed4c...c8d5f ✓ (re-downloaded this sweep, matches the ci.yml pin)" | :17-18 | REAL | .github/workflows/ci.yml:484 `sha256 9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` — identical pin |
| 2 | "`advisories ok, bans ok, licenses ok, sources ok` ... Zero errors" (at 784c6e6) | :31-33 | REAL-UNPROVEN | no cargo-deny binary/network in sandbox; corroborated at the later phase7 gate (phase7-exit.md §2 #15 PASS, same four "ok"s, 2026-11-10) |
| 3 | "29 `wildcard` warnings across 25 crates — every warned crate is a workspace member; zero registry `*` requirements" | :38-41 | REAL-UNPROVEN | needs a run; consistent with 39 path-dep members; deny.toml wildcard posture as described |
| 4 | "2 `duplicate` warnings: `getrandom` 0.2.17 ... vs 0.4.3; `syn` 2.0.119 ... vs 3.0.9" | :49-52 | REAL | all four versions confirmed in root Cargo.lock (grep: getrandom 0.2.17 + 0.4.3; syn 2.0.119 + 3.0.9) — the dichotomy exists exactly as documented |
| 5 | "single RustSec hit ... RUSTSEC-2024-0436 (`paste` 1.0.15 via rav1d 1.1.0 — unmaintained ...)" | :56-58 | REAL-UNPROVEN | Cargo.lock: `paste` 1.0.15 and `rav1d` 1.1.0 both present (grep -A1) — chain plausible; the advisory hit itself not re-queried offline |
| 6 | "`yanked = \"deny\"` held: no locked crate is yanked" | :59 | REAL | deny.toml:49 `yanked = "deny"` (config as claimed); no-yanked needs the DB — config half verified |
| 7 | "`Cargo.lock`: 156 packages = 38 workspace members ... + 118 registry entries" | :63-64 | FALSE-on-current-tree (snapshot) | current: 158 total = 119 registry + 39 local (grep counts) — +2 since the sweep (rust_h264 admission + 1) |
| 8 | "every `source` field is registry+... — zero git dependencies, zero path-source externals, no vendored tarballs" | :64-67 | REAL | `grep -c 'git+' Cargo.lock` -> 0; 119/119 registry sources |
| 9 | graph inventory: "rav1d + symphonia + filmcraft/oxideav/ruopus/rusty_vp9 decoder set, cpal audio" | :70-71 | FALSE-on-current-tree (oxideav, cpal) | lock greps: `oxideav` -> 0 entries, `cpal` -> 0 entries; oxideav admissions BLOCKED (h264-software-decoder-audits.md / oxideav-mpeg4video record); allowlist.md §5: cpal "NOT admitted — ADR-005 input". filmcraft-dnx/ruopus/rusty_vp9/symphonia family all present (1 entry each) |
| 10 | "every external crate of consequence has an admission record under `docs/dependencies/records/` (21 files)" | :75-76 | REAL-at-capture / drift | 22 files today (rust_h264-0.6.1.md added post-sweep — the +1 matches claim #7's drift) |
| 11 | doc-drift note: deny.toml comment "the only two expected warnings are the two workspace path edges" has rotted | :42-48 | REAL | deny.toml:160-161 comment verbatim as quoted; still unfixed (no reword found) |
| 12 | "cargo-audit — [PROPOSAL] only. Not run (no pinned binary, no Lead admission)" | :78-80 | REAL | honest deferral with a reasoned one-tool stance; no cargo-audit anywhere in CI (grep ci.yml -> only cargo-deny) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "cpal audio" is in the non-dev Linux graph (:71) | allowlist.md:153-155 "cpal-0.18.2.md (P0 verification, conditional-E2, NOT admitted — ADR-005 input)" | The lock: cpal = 0 entries — NOT in the graph. The sweep's inventory names an unadmitted crate |
| "filmcraft/oxideav/ruopus/rusty_vp9 decoder set" (:70) | oxideav-mpeg4video record verdict: BLOCKED; root lock: 0 oxideav entries | the lock — oxideav never entered the workspace graph (its only lockfile is tools/vta/mpeg4's, a quarantined non-workspace tool) |
| "156 packages = 38 + 118" (:63) | 158 = 39 + 119 today | the lock (snapshot drift) |

## 4. Fabrication / inflation indicators
Low. The record distinguishes verified facts from tool output, records its own doc-drift finding against deny.toml, and defers cargo-audit honestly. The inventory sloppiness (oxideav/cpal named as graph members) reads as copy-from-records, not deliberate inflation — but it IS the kind of sentence a reader quotes as "the graph is clean and known," and it is wrong for two of its named crates.

## 5. Matrix rows
Not a matrix chunk. Related: SBOM claims (issue #52) — the SBOM generator (xtask `sbom --check`) exists per ci.yml:527-529 (verified this wave); per-platform finalization deferred to 8.E3 as stated.
