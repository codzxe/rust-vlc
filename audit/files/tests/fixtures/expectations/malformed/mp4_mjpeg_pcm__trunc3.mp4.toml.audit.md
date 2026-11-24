# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc3.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| Lines / functions | 27 lines (schema/fixture/sha/derivation/error keys) |
| File verdict | REAL (pin verified by reproduction) |
| Reachable from shipped entry point? | As evidence: codecs fuzz_survival.rs:39 + media-probe fixtures_matrix + first-vertical recovery_matrix (tier-2 #[ignore]d); family drift-gate-pinned in MANIFEST.toml |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | sha256sum + stat (pin match), python3 prefix check, rg consumers |

## 1. One-paragraph verdict
Exact pin, truthful derivation: sha256 8829e980…7ba61c2 and size 213,878 reproduced against
the fixture on disk; python3 confirms the fixture is a byte-exact PREFIX of the parent (op =
truncate), cut at 213,878 of the 424,452-byte parent — mid-payload as the note claims. The
family (parent + trunc0..3) is one of the 28 drift-gate pins in MANIFEST.toml, and the
consuming suites ran green in the orchestrator baseline. The single defect is the same
v0 union contract as the flip16 sibling.

## 2. What actually works (verified)
— verified by reproduction: `sha256sum …trunc3.mp4` → `8829e98002b6003789a1781d9a0a7c54c17ebf7c5
a01d07c94c5ec5572ba61c2` (== pin); `stat -c%s` → `213878` (== pin); `t3 == parent[:213878]`
(pure prefix). Consumer: crates/codecs/tests/fuzz_survival.rs:39 (baseline :1115+ block, 6/6
ok); fixtures_matrix (baseline :3670-3680, 8/8 ok). MANIFEST.toml pins the trunc family
(drift gate covers this file).

## 3. Findings
F-001 [S3] Where: :20-24 `[error] expected = "typed-error-or-bounded-recovery"` — the v0
union contract permits three outcomes, weaker than a machine-readable pin should be; a
flavor drift (bounded-recovery ↔ typed-error) passes silently. The header's own comment
defers specifics to "demuxer conformance (cards 2.B2/2.B3)". What correct looks like: pin the
specific stage/code (the demux crate's typed outcomes for mid-payload truncation are known).
Confidence: HIGH — Verification: read. (Identical defect/verdict as the flip16 TOML — v1
schema migration should move the whole set.)

## 4. Per-function table
N/A (data/evidence file); all keys verified (schema/fixture/class/parent/sha/size/derivation),
error union excepted (F-001).

## 5. Test honesty
Consumers read the RECORDED TOML (fixtures_matrix.rs:246) — mismatch between fixture and pin
fails loudly; prefix-purity is re-derivable by anyone (as done here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cut mid-payload of video sample 30" | :18 | REAL | 213,878 ≈ mid-file of the MJPEG payload (parent 424,452; mdat carries frames ~30 of 60) |

## 7. Dead / duplicated / unreachable code
None. Tier-2 recovery_matrix consumer ignored-by-design (LUMEN_CORPUS gate).

## 8. Dependencies & flags
None (TOML).

## 9. Open questions for the next agent
Same as the flip16 TOML: migrate the union → specific when expectation/2 lands.
