# WAVE 17 SUMMARY — scripts, workflows, packaging, SBOM, release evidence

| Field | Value |
|---|---|
| Agents | A01-A04, A06, A07 complete; A05/A08/A09/A10 recovered via R1/R2; 100% coverage |
| Files audited | 216 files (244 rows) |
| Findings | S0:2 S1:0 S2:15+ S3:~70 S4:~130 |
| Tree purity | clean |

## THE PIPELINE ANSWER (§8.17)
The release workflow is REAL and CAN fail (toolchain asserts, sha256-verified
tool pins, if-no-files-found: error, real Windows launch smoke) — and it
DID execute: six in-tree run IDs (43378757248, 43464162696, 43729218652...)
with post-mortems disprove the "never executed / billing-blocked" claims
still carried in three docs. Unsigned binaries are DISCLOSED (.UNSIGNED.txt
sidecars), never presented as signed. BUT:
1. [S0] 23_alloc_budget.sh: a CI gate whose sys.exit(1) is swallowed by
   trailing `say`s — proven by executed repro (script-exit=0). A CI gate
   that cannot fail (W17-A03).
2. [S0-class] rust-clippy.yml can NEVER fail on lint findings
   (continue-on-error: true, no -D warnings) (W17-A04).
3. [S2] 26_complexity.sh: ratchet baseline never committed — every CI run
   is a "first run", the gate can never fail (W17-A04).
4. [S2] The committed gate-evidence logs are STALE-GREEN: wave3-merge log
   shows 0 failed (1718/0) while today's baseline is 4744/2 FAILED;
   phase7-exit-fmt.log FMT_EXIT=0 vs baseline fmt FAILED — the tree
   regressed after the gates were cut (W17-A04/A06).
5. [S2] SBOM stale by exactly one edge (device-discovery->plugin-api);
   drift gate red yet shipped; 4 "per-artifact" SBOMs byte-identical x4
   (W17-A02/A06).
6. [S2] astgrep_summary.py has ZERO callers — root cause of 22_attack_
   surface's all-zero panic columns (false clean) (W17-R1).
7. [S2] resolve_w2_scheduler.py BOOBY TRAP: rerunning the one-shot splice
   on today's tree silently deletes 9 code lines with exit 0 (empirical)
   (W17-R2).
8. [S2] oracle_check.sh exit-1 today (6 unpinned fixtures); wired into
   nothing so permanently invisible (W17-A04).
9. [KEY HONESTY FINDING] Phase-exit evidence culture: phase1-exit.md's
   PASS has zero committed evidence; phase3-exit.md quotes the passing
   middle of a log that ends EXIT_CODE:101 twice; phase3-exit.md's
   signature is BACKDATED (says 2026-10-24, own square-up says signed
   2026-11-13) and claims a 4h-soak debt "discharged" while the cited
   soak report says never taken (W17-A03/A06).
10. [S2/S3 cluster] dependabot.yml is a STUB (package-ecosystem: "");
    codeql actions unpinned (@v7/@v4) vs the repo's own SHA-pin
    convention; root package-lock.json is INVALID JSON with committed
    merge-conflict markers; quality scripts are J-reporters whose exit
    codes never propagate; `|| true` on miri/asan/ubsan lanes; mutation/
    fuzz evidence entirely uncommitted; loom=0 shuttle=0 under a
    "concurrency model-checking" name; 25_fuzz_campaign fail-open.
11. [VERIFIED GOOD] rc-smoke.sh honest (7/8 PASS, wasm env-gap, honest
    exit); gen-wxs self-test executed PASS; vta tooling has real external
    oracles (vp9 = the only bit-exact VTA verdict, re-verified today);
    dependency records are honest (BLOCKED states match the lock exactly);
    release.yml is not a paper pipeline.

## Verdict
The CI/release story splits in two: the release pipeline is real and
disciplined; the QUALITY-gate layer is largely theatre (gates that cannot
fail, stale-green evidence, rubber-stamp reporters), and the committed
"exit" evidence systematically overstates the tree's current health.
