# Web wasm size report — the 7.K1 tree-shake analysis (card 7.K1, issue #49)

Machine record: `docs/web/size-record.toml` (schema
`lumen.web.size-record/1`). This report is the human analysis of the
same measurements; the tree-shake builds below were real
`cargo build -p lumen-web --target wasm32-unknown-unknown --release
--locked` runs executed 2026-11-11 on branch `w24/a` at commit
`30153face2d9956415a3460da18e74d6532a6e2c` (the per-codec feature-gate
commit whose sizes this is). Doc 09 §2.4's deliverable: the first real
per-configuration size table for the engine-in-Worker wasm module.

## Environment (labeled, per AGENTS §3.4)

| Input | Value |
|---|---|
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Target | wasm32-unknown-unknown |
| Profile | cargo `release` — workspace default (opt-level 3; no LTO or codegen-units override exists in the workspace root) |
| Compressor | `gzip -n -c` (gzip 1.13, default level, name/timestamp stripped — the CI canary's convention) |
| wasm-opt | **NOT RUN** — binaryen/wasm-opt is absent from this build environment; every number below is raw rustc codegen. Doc 09 §2.3/§2.5 item 4's `wasm-opt` pass is a [PROPOSAL] for when binaryen is admitted (AGENTS §3.2 dependency approval); until then these numbers are the honest upper bound. |
| Lockfile | `--locked` (doc 11 §3.7 — CI builds never float) |
| Machine | the w24a lane sandbox (shared PolarFS target dir, warm artifact cache; cold-build times were not re-measured — only warm rebuild times are quotable: floor 12.6 s, external-all 40.1 s) |
| Date | 2026-11-11 |

## The per-configuration size table

Five real builds of the SAME crate tree, differing only in
`--features` (the 7.K1 per-codec gates in `apps/web/Cargo.toml` +
`crates/codecs`):

| Config | Cargo features | Decoders linked | raw bytes | gzip -n bytes | Δ raw vs floor | Δ gz vs floor |
|---|---|---|---:|---:|---:|---:|
| `floor` | *(none)* | none (the shipped decoder-less module) | 1,294,311 (~1.23 MiB) | 300,250 (~0.29 MiB) | — | — |
| `builtin-inhouse` | `builtin-codecs` | mjpeg, pcm, adpcm (in-house) | 1,387,526 (~1.32 MiB) | 334,759 (~0.32 MiB) | +93,215 | +34,509 |
| `flac` | `codec-flac` | + flac (claxon) | 1,429,280 (~1.36 MiB) | 349,963 (~0.33 MiB) | +134,969 | +49,713 |
| `mp3` | `codec-mp3` | + mp3, mp2 (symphonia-bundle-mp3) | 1,529,340 (~1.46 MiB) | 385,300 (~0.37 MiB) | +235,029 | +85,050 |
| `external-all` | `builtin-codecs-external` | + aac, alac, dnx, flac, mp3/mp2, opus, vorbis, vp9 (all 8 wasm-buildable external lanes) | 2,439,192 (~2.33 MiB) | 704,996 (~0.67 MiB) | +1,144,881 | +404,746 |

`codec-av1` (rav1d) is absent from every row by measurement, not
oversight: rav1d 1.1.0 does not compile for wasm32-unknown-unknown
(39 errors, measured 2026-11-10; the record lives in the codecs crate's
feature table and this card's worklog).

## The tree-shake proof (doc 09 §2.4's symbol-scan check)

Occurrences of each external engine crate's name in the built module
(`grep -a -o <name> <module> | wc -l` over the release artifacts — the
names ride the binary through panic locations and error strings, so
zero means the engine's code is absent, which is the anti-bloat claim
doc 09 §2.4 asks CI to assert):

| Config | claxon | symphonia | ruopus | rusty_vp9 | filmcraft | rav1d |
|---|---:|---:|---:|---:|---:|---:|
| `floor` | 0 | 0 | 0 | 0 | 0 | 1\* |
| `builtin-inhouse` | 0 | 0 | 0 | 0 | 0 | 1\* |
| `flac` | 21 | 0 | 0 | 0 | 0 | 1\* |
| `mp3` | 0 | 107 | 0 | 0 | 0 | 1\* |
| `external-all` | 18 | 397 | 116 | 115 | 33 | 1\* |

\* The single `rav1d` occurrence in EVERY config is the honest
typed-refusal reason string in the capability snapshot
(`apps/web/src/capabilities.rs`: "…the wasm module ships the engine
core decoder-less… rav1d/AV1 is not wasm32-buildable…") — a runtime
string, never linked rav1d code. That is the honest-refusal design
working, not a leak.

Read of the table: enabling codec *X* brings in X's engine and nothing
else's (flac adds claxon only; mp3 adds symphonia only); the
decoder-less floor carries zero external engines; the full
wasm-buildable set costs ~2.3 MiB raw / ~0.67 MiB gzipped over the
floor. Dead-code elimination did its job on the disabled engines.

## Budget context (doc 09 §2.4 — budgets are CONFIG, not prose)

The 3 MiB / 6 MiB baselines live in `product.toml`
`[web.size_budgets]` (validated by `product.schema.json`), still
carrying doc 09 §2.4's `[PROPOSAL]` label until the Lead pins them
(doc 09 §12 Q5). Against those values:

| Config | Budget class | gzipped | % of budget |
|---|---|---:|---:|
| `floor` | phase2-vertical (3 MiB) | 300,250 | 9.5% |
| `builtin-inhouse` | phase2-vertical (3 MiB) | 334,759 | 10.6% |
| `flac` | full-p0 (6 MiB) | 349,963 | 5.6% |
| `mp3` | full-p0 (6 MiB) | 385,300 | 6.1% |
| `external-all` | full-p0 (6 MiB) | 704,996 | 11.2% |

`cargo xtask web-budget --check` (doc 09 §2.4's regression gate)
enforces exactly this: budget breach or >5% growth on the floor row
fails the gate with a "justify or optimize" message.

## Honest limits of this measurement

- **No wasm-opt** (see the environment table): numbers are raw rustc
  codegen; the shipped pipeline's post-wasm-opt sizes can only be
  smaller, but they are NOT measured here and are not claimed.
- **The UI bundle is not in these numbers.** Doc 09 §2.4's budget is
  "engine + UI + host, gzipped"; this table measures the engine host
  module only (`lumen_web.wasm`). The hand-written glue
  (`worker.js` 7,073 B + `audio-worklet.js` 9,012 B = 16,085 B raw,
  measured 2026-11-11) is trivially inside any budget; the `ui/`
  React app has no production bundle build in this tree yet — when it
  does, its artifact joins the record and the budget arithmetic turns
  real. Until then the budget headroom is engine-side only, labeled
  as such.
- **Single environment**: one machine, one toolchain, one day. The
  numbers are deterministic for that environment (no timestamps in
  the record; same tree → same sha256); cross-platform size variance
  for wasm codegen is expected to be nil (same target), but only this
  environment is claimed.
- **Warm-cache build times only** — cold-build cost was not measured.
- **Debug-profile sizes are not tabulated**: the CI canary
  (`ci.yml` wasm-build job) keeps measuring those for theme-tokens;
  this report is the release-profile lane doc 09 §2.5 item 4 asked
  for.

## Regenerating / verifying

```sh
cargo xtask web-budget            # rebuild + re-measure the floor row, rewrite the record
cargo xtask web-budget --check    # gate: budgets + >5% floor growth (exit 1 on failure)
```

Both are wired into `scripts/full_gate.sh` (step 10) and the CI
`wasm-build` job. The per-codec rows above are regenerable by hand:
`cargo build -p lumen-web --target wasm32-unknown-unknown --release
--locked --features <set>` and the measure pipeline from the
environment table; the record's `sha256` fields make every row
auditable against its artifact.
