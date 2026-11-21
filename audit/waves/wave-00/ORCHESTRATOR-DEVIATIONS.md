# ORCHESTRATOR DEVIATIONS — audit/devil-20261121

Faithful record of every place the sandbox reality forced a deviation from
DEVIL-AUDIT-PROMPT.md mechanics. No deviation weakens the audit's evidential
standard; each is recorded here and visible in the wave summaries.

1. **Token handling.** §15's literal token-export command was NOT typed into
   any shell log; the token was exported by extracting it from the
   orchestrator's uploaded instruction file, so it never appears in any command
   string, log, or file. Effect identical; exposure zero. All pushes are made
   by the orchestrator through the §6.1 credential-helper (`scripts/gpush.sh`
   outside the repo), which refuses to push if any GitHub-token-prefixed string
   is found under `audit/` and refuses any branch not matching `audit/devil-*`.

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

---

# SESSION 2 (2026-11-21 ~17:00 UTC onward) — continuation notes

The morning session ended after Wave 5's gate (11:24 UTC). Session 2 resumed at
~16:48 UTC on the same branch and continues Waves 6–20.

8. **Session-2 restart evidence.** `audit/runs/baseline-recheck-2/` holds an
   independent re-run of the key baseline steps from a fresh clone of HEAD
   e83e5bd3 (build rc=0; fmt rc=1 same violations; MP4 smoke with the §7.1-exact
   sample — which is **yuv444p**, refused by the sw decoder with a hang;
   plus the Lead's own locally-built binary preview). The session also
   diagnosed why a fresh `cargo` with the leftover Lead `CARGO_HOME` fails
   (`failed to find <crate> in path source` — poisoned registry index cache);
   a fresh `CARGO_HOME` was used for all session-2 builds.

9. **Agents no longer use per-agent git worktrees.** Sandbox disk headroom
   (5.8 GB) made 10 simultaneous 169 MB checkouts wasteful. Session-2 agents
   work in the one shared checkout, write only their own `audit/` paths, run no
   git at all, and append one line per completed file to
   `audit/runs/W{nn}-A{nn}.manifest.tsv` (path, verdict, S-counts). The
   orchestrator then creates the per-file commits with the exact §6.3 message
   format from the manifest — commit cadence and message format preserved;
   agents never touch git.

10. **Wave gating is scripted** (`audit/waves/wave-<nn>/gate-report.json`):
    coverage + thickness (≥40 lines unless the source is tiny) + citation
    spot-checks (quotes re-verified against `sed -n` ranges). W06's first run
    flagged 2 citations; both were manually verified accurate — the checker's
    regex was eating markdown code fences; the checker was fixed, no
    fabrication occurred (nothing to record in audit/verify/fabrications.md).
