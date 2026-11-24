# CLAIM AUDIT: docs/security/sweep-parsers.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL: 5 REAL-UNPROVEN: 3 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 1 UNREACHABLE: 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
A serious, mostly-honest parser security sweep — but its headline claim
("zero panic-capable calls in non-test parser code") is FALSE by exactly one
live site I re-derived myself: `codecs/src/audio/pcm/decoder.rs:330` carries
a live `unreachable!` in the cold path of the PCM convert rotation (a parser
crate named in the sweep's own scope). Also its stated verification of the
input `contract.rs` hits cites the wrong gate (line 537 instead of the
module-level `#[cfg(any(test, feature = "contract"))]` at lib.rs:64). The
rest of the sweep checks out: lint policy real, fuzz targets/corpus/registry
script real, three of the seven suite counts byte-exact today.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "**Result: zero panic-capable calls in non-test parser code.**" | :28 | FALSE | my re-run of the sweep's own rg method (refined classifier over 7 crates): one LIVE hit — crates/codecs/src/audio/pcm/decoder.rs:330 `None => unreachable!("fresh SharedBytes has refcount 1"),` (cold path of SharedBytes rotation; pcm is in the sweep's listed scope :6). Invariant-guarded, but panic-capable and outside any cfg(test) |
| 2 | "input's live-looking hits at buffered.rs:461 ... below #[cfg(test)] at line 419" | :29-32 | REAL | rg: buffered.rs `#[cfg(test)]` at :419; hits at :461/482/693/785 all below |
| 3 | "contract.rs hits are under its line-537 gate" | :31-32 | PARTLY FALSE-as-stated | input/contract.rs hits at :249-531 sit ABOVE the file's `#[cfg(test)]` at :537; the actual gate is the module attribute lib.rs:64 `#[cfg(any(test, feature = "contract"))]` — conclusion (not in release) holds, cited verification is wrong |
| 4 | "unwrap_used/expect_used/panic ... deny in the root Cargo.toml [workspace.lints]" | :22-26 | REAL | Cargo.toml:60-66 `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`, `todo`, `unimplemented`, `dbg_macro`, `await_holding_lock` — note `unreachable!` is NOT denied (why the live site at claim 1 compiles clean; baseline clippy 0 warnings) |
| 5 | "35/35 targets, exit 0, zero panics/hangs, ~165.7 M total executions, peak RSS 27–38 Mb" | :104-106 | REAL-UNPROVEN | past run; corroborated structurally: fuzz/fuzz_targets/ has 35 entries, fuzz/corpus/ has 35 dirs, scripts/fuzz_registry_check.sh exists (all ls-verified). Not re-runnable here (no toolchain) |
| 6 | "demux 454+8 doc-tests, codecs 461, playlists 134, subtitles 335, media-probe 92, input 46, streaming 242 — all green" | :111-113 | REAL (3 exact today) | my baseline tally: playlists 134 ✓, subtitles 335 ✓, media-probe 92 ✓ exact; demux 606 / codecs 701 / input 47 / streaming 252 today (tree grew after the sweep) |
| 7 | "57 panic-capable sites ... across 14 files" (fixes landed) | :36-44 | REAL-UNPROVEN | the fix table's files exist (ogg/mod.rs, es/mp3.rs, ogg/page.rs, av1/decoder.rs, ivf/mod.rs, adts/mod.rs, annexb.rs, mkv/cluster.rs, ts/mod.rs, alac/decoder.rs all present); site counts need git history — UNVERIFIED (git-forbidden) |
| 8 | "`clippy::indexing_slicing` ... demux 461, codecs 540, subtitles 378, streaming 66, media-probe 40, playlists 18, input 6 — ~1 509 sites" | :75-77 | REAL-UNPROVEN | needs a clippy run (no toolchain); the described guard style is the observed house style in audited files |
| 9 | "`overflow-checks` is on in dev/test, **wraps silently in release**" | :83-84 | REAL | root Cargo.toml has NO [profile.*] sections (rg exit 1) → default release profile has overflow-checks=false |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "zero panic-capable calls in non-test parser code" (:28) | pcm/decoder.rs:330 live `unreachable!` (my rg + file read) | the code — the sweep's own method finds the site |
| "contract.rs hits are under its line-537 gate" (:31) | hits at :249-531 precede :537; the module gate is lib.rs:64 | the module gate is what saves the conclusion, not line 537 |

## 4. Fabrication / inflation indicators
None detected beyond the two inaccuracies above — the sweep discloses its
limits honestly (uninstrumented random mutation is "shallower" :127-129;
deep review deferred to 8.A1 :125-137). The fuzz headline (165.7 M
executions) is a self-report with no committed per-target logs inside this
doc (it points to "the sweep's worklog entry" — not in this file).

## 5. Matrix rows
N/A.

## 6. What actually works (verified)
— Re-implemented the sweep's rg inventory with a cfg(test)/module-gate/doc-
comment-aware classifier over all 7 crates: 1559 test-gated hits, exactly ONE
live site (claim 1). lint deny list read; fuzz targets/corpus/registry
ls-verified; baseline per-crate test tallies computed.
