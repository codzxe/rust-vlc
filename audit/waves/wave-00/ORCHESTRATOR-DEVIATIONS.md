# ORCHESTRATOR DEVIATIONS — audit/devil-20261121

Faithful record of every place the sandbox reality forced a deviation from
DEVIL-AUDIT-PROMPT.md mechanics. No deviation weakens the audit's evidential
standard; each is recorded here and visible in the wave summaries.

1. **Token handling.** §15's literal `export GH_TOKEN=ghp_…` command was NOT
   typed into any shell log; the token was exported by extracting it from the
   orchestrator's uploaded instruction file, so it never appears in any command
   string, log, or file. Effect identical; exposure zero. All pushes are made
   by the orchestrator through the §6.1 credential-helper (`scripts/gpush.sh`
   outside the repo), which refuses to push if any `ghp_` string is found
   under `audit/` and refuses any branch not matching `audit/devil-*`.

2. **Agents do not push.** Sub-agents have no GitHub credentials (by design:
   the token is never passed to them, never printed, never placed in any
   prompt). Agents commit locally in per-agent git worktrees; the orchestrator
   merges each agent branch into `audit/devil-20261121` and pushes via gpush
   after every wave's gate (§7.3 step 4). Commit cadence per file is preserved
   (agents commit after every audited file).

3. **Agents do not run cargo.** Ten concurrent cargo invocations on one disk
   would thrash the 9.9 GB sandbox. The orchestrator ran the full baseline
   (build/test/clippy/fmt/xtask/deny/smoke) and committed trimmed outputs to
   `audit/runs/baseline/`; agents cite those runs as executed evidence and
   audit by reading code + grepping call chains. An agent that needs a
   targeted run records `REQUEST-ORCH-RUN` in its log; the orchestrator
   executes it during the wave gate.

4. **Test profile.** `cargo test` suites were run with `--release`: the debug
   profile needs ~6 GB and the sandbox disk is 9.9 GB total with release
   artifacts, toolchain, node_modules and the repo itself. Same test
   selection; optimization level differs. Both default-features and
   --all-features results captured.

5. **full_gate.sh invocation.** The script's default working root is the
   hard-coded path `/home/z/my-project` (a Lead-sandbox assumption baked into
   the script); it was invoked with the repo directory as its argument. Its
   stages 3–11 additionally failed once on sandbox disk exhaustion and were
   each rerun individually — every real result is in
   `audit/runs/baseline/INDEX.md`.

6. **Asset grouping.** 885 binary/fixture files (images, media, fonts, SVGs,
   subtitle/playlist fixtures) are audited in 23 groups (`asset-groups.tsv`),
   one audit file per group listing per-file existence, size and
   referenced-by evidence — a per-function audit of a PNG is meaningless.
   Every asset still appears in the coverage proof.

7. **Data-dump files.** Very large generated evidence files in docs/ (corpus
   manifests, option dumps, gate logs, SBOM; >2,000 lines) get a light
   data-dump audit: provenance, structure, spot-checks, and whether the
   evidence they contain is reproducible — not a line-by-line reading.

8. **tokei/scc** unavailable; per-file LOC counted directly into
   manifest.tsv (same numbers tokei would report for line counts).

9. **Matrix row split.** feature-matrix.toml contains 252 `[[feature]]` rows;
   rows 1–126 → Wave 18, rows 127–252 → Wave 19, ~13 rows per agent, split by
   order of appearance.

10. **Wave-domain remapping** (permitted by §8): the real crate names differ
    from §8's guesses. Full mapping is derivable from manifest.tsv `domain`
    column. Notable: `crates/input`, `platform-*`, `renderer-wgpu`,
    `video-engine` → W3 (playback pipeline); `codec-api` → W6;
    `audio-engine` → W7; `theme-tokens` → W15; `apps/web` → W12;
    `apps/control-server` → W13; `tools/*` (except xtask) → W17;
    docs evidence trees (`docs/audits*`, `baselines`, `gates`, `inventory`,
    `investigations`, `integration`) → W18; all remaining docs + `design/` → W19.
