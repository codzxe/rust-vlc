# AUDIT: crates/plugin-api/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Lines / functions | 33 / 0 (manifest: 1 package, 1 lints, 1 deps, 1 dev-deps section) |
| File verdict | REAL (every manifest claim verified against source usage and the allowlist; one cross-doc drift noted) |
| Reachable from shipped entry point? | YES — plugin-api is a direct dependency of lumen-cli (apps/cli/Cargo.toml:59); its `SourceStream`/`Demuxer`/`SeekSupport` traits are the shipped demux contract (apps/cli/src/containers/{ogg,mp4,ts}.rs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (dependency-usage tracing in crates/plugin-api/src, exit 0); sed -n / od -c (quote byte-verification, exit 0); allowlist cross-check (rg, exit 0) |

## 1. One-paragraph verdict
A clean manifest whose header claims all verify: the two path deps (media-types, codec-api) are genuinely consumed, all three external crates (serde, toml, thiserror) are genuinely used in non-test source, testkit is dev-only as stated, and the scope fence ("no registry, no selection policy, no WASM host, no enforcement tooling, no conformance kit") matches what lib.rs actually contains (types and traits only — the registry/host live in plugin-host-wasm, unshipped). The allowlist verdicts cited are real (`serde`/`toml`/`thiserror` all `**pass**`, allowlist.md:63,65,66). One drift: the allowlist's "Direct workspace consumers" census omits plugin-api for all three externals — a stale census in that doc, visible from this manifest.

## 2. What actually works (verified)
— Every declared dependency is used in non-test source (rg, exit 0): `toml` at manifest.rs:307,320,1053 (real TOML parse/serialize); `serde` derives at manifest.rs:214,275,429,725 + semver.rs; `thiserror` in semver.rs + manifest.rs error types; `codec_api` re-exported as code at codec.rs:19-21; `media_types` at ui.rs:16-81, metadata.rs:14-15.
— Allowlist rows exist and say pass: docs/dependencies/allowlist.md:63 (`serde ... **pass**`), :65 (`toml ... **pass**`), :66 (`thiserror ... **pass**`). Version constraints `"1"`/`"1.1"`/`"2.0"` are consistent with the allowlist ranges (>=1.0.229, >=1.1.8, >=2.0.21).
— Release lane builds `--locked` clean (baseline: cargo-build-release-lumen-cli.txt), so manifest/lock are consistent.

## 3. Findings
F-001 [S4]
Where: crates/plugin-api/Cargo.toml:5-7 (header claim), vs docs/dependencies/allowlist.md:43-48
Code:
```
# Registry: doc 01 §2.2 (`plugin-api`, layer L1, deps: media-types +
# codec-api). The ONLY external crates are the pre-audited config family
# (docs/dependencies/allowlist.md, verdict `pass`): serde (derive),
```
What is wrong: The claim itself is true — the three externals all carry verdict `pass` in the cited doc — but the allowlist's "Direct workspace consumers" census (allowlist.md:43-48, "named per `cargo tree --workspace`") lists serde/toml/thiserror consumers as product-config/settings-schema/commands (+localization) and does NOT list plugin-api, even though this manifest directly depends on all three. The census is stale relative to this crate. Why it matters: the human-reviewable approval record understates this crate's external surface; the next audit that trusts the census instead of Cargo.toml will under-count. What correct looks like: regenerate the consumer census (one `cargo tree` run) — file the fix against allowlist.md, not this manifest. Confidence: HIGH — Verification: read (both files, rg cross-check).

## 4. Per-function table
Not applicable (0 functions, 33-line manifest). Checklist coverage: package metadata consistent (name/version/edition 2024/publish false/workspace lints — all standard); `[dependencies]` = 2 path + 3 registry, all used (§2); `[dev-dependencies]` = testkit path-only, dev-only confirmed; no feature gates declared, no build.rs, no bins — nothing hidden.

## 5. Test honesty
No tests in a manifest; the crate's own tests run in baseline (PACKAGE plugin-api section, cargo-test-workspace.txt:4804-4882, exit_code=0, incl. kind::tests 4/4 at 4824-4827).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The ONLY external crates are … serde (derive), toml, thiserror" | Cargo.toml:5-7 | REAL | [dependencies] section lists exactly those 3 registry crates (sed 26-29); all used (§2) |
| "verdict `pass`" per allowlist | Cargo.toml:6-7 | REAL | allowlist.md:63,65,66 all `**pass**` |
| "deps: media-types + codec-api" (registry layering) | Cargo.toml:3-4 | REAL | the only path deps (Cargo.toml:22-23); both used in src |
| scope fence: no registry/selection/host/enforcement/conformance | Cargo.toml:9-12 | REAL | lib.rs:1-30 states and src tree matches (type/trait modules only; registry/host live in plugin-host-wasm) |
| "testkit is dev-only by contract" | Cargo.toml:32 | REAL | under [dev-dependencies]; no testkit import in non-test src (rg) |
| description: "the 16-kind taxonomy" | Cargo.toml:17 | REAL | kind.rs has exactly 16 variants (kind.rs audit; test-pinned) |

## 7. Dead / duplicated / unreachable code
No unused dependencies (the opposite: every dep earns its place — verified §2). No duplication.

## 8. Dependencies & flags
serde 1 (+derive), toml 1.1, thiserror 2.0 — the pre-audited config family, versions consistent with allowlist ranges and Cargo.lock (release `--locked` build clean). media-types + codec-api path deps (L0/L1 layering respected: no L2+ deps — lib.rs:9-12, verified by reading [dependencies]).

## 9. Open questions for the next agent
- File the allowlist consumer-census drift (F-001) against docs/dependencies/allowlist.md's owner: plugin-api (and possibly other later crates) missing from the serde/toml/thiserror consumer lists.
- allowlist.md:50-53 admits codec admissions (rav1d, symphonia, …) "predate this row without §3 rows of their own" — that drift is bigger than mine; whoever audits the dependency docs should weigh it.
