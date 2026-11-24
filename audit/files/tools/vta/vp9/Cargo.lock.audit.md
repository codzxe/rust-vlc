# AUDIT: tools/vta/vp9/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 209 / n-a (cargo lockfile; same 5-registry-dep scaffold as ruopus lock) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone VTA harness lock (own `[workspace]` root); the candidate it pins IS shipped: root Cargo.lock rusty_vp9 == this lock's |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Identical scaffold to tools/vta/ruopus/Cargo.lock (diff = 18 lines: candidate block + package name only). It pins rusty_vp9 0.1.3 with checksum `9bf32821db20be4ea56ce1f8feb757bf55f146f2685ba98822c3bd2d43a33889` — byte-identical to the root workspace lock's entry — and the root workspace ships rusty_vp9 as the default `codec-vp9` feature (crates/codecs/Cargo.toml:81,145, default list L58). The differential evidence chain (docs/dependencies/records/rusty_vp9-0.1.3.md) therefore covers the exact shipped bytes. Note: `rusty_vp9 = "=0.1.3"` here is pinned WITHOUT default-features=false (unlike ruopus's posture) — matches the shipped pin at codecs/Cargo.toml:145 (`optional = true`, no feature restriction), so harness posture == shipped posture.

## 2. What actually works (verified)
— Checksum cross-check (ran): root lock rusty_vp9 `9bf32821...` == this lock's.
— Admission wiring (ran, read): crates/codecs/Cargo.toml:138 `# 6.M-vp9 (Lead admission, docs/dependencies/records/rusty_vp9-0.1.3.md)` + :145 `rusty_vp9 = { version = "=0.1.3", optional = true }`; default feature `codec-vp9` at L58; apps/cli depends on codecs with default features — release lane builds it in.
— `diff ruopus-lock vp9-lock` -> only candidate + name differ (18 lines): no scaffold drift between harnesses.

## 3. Findings
None. Checked: exact-pin `=0.1.3` matches shipped pin exactly; all registry entries checksummed; path deps correct (`../../../crates/{demux,plugin-api}`); lock v4; no git/patch sections.

## 4. Per-function table
N/A (data). Inventory: vta-vp9 (root), 4 repo path deps, rusty_vp9 0.1.3 (candidate), serde/thiserror/indexmap+equivalent+hashbrown transitive.

## 5. Test honesty
N/A — the enabled differential record exists (rusty_vp9-0.1.3.md); admission is real (feature wiring verified). Feature-matrix vp9 row's evidence cites "rusty_vp9 =0.1.3 (card 6.M-vp9 ...)" — consistent with this lock.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decodes an .ivf VP9 file through OUR ivf demuxer into rusty_vp9 0.1.3" | tools/vta/vp9/README.md:1-3 | REAL | Cargo.toml pin `=0.1.3` + path dep to crates/demux (read) |
| "Evidence lives in docs/dependencies/records/rusty_vp9-0.1.3.md" | README:7 | REAL | record exists (the mpeg4 record references it as "the closest analogue") |

## 7. Dead / duplicated / unreachable code
Harness-only lock by design; candidate is shipped.

## 8. Dependencies & flags
rusty_vp9 =0.1.3 exact; no default-features override (posture parity with the shipped pin).

## 9. Open questions for the next agent
1. The feature-matrix vp9 row (windows=implemented, linux=partial, web=not_started) — the "1 corpus track refused" linux downgrade is a registry/build question in codecs domain, not this lock.
