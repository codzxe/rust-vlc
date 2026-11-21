# WAVE 1 — SUMMARY

Date: 2026-11-21 · Agents W01-A01…A10 · Domain: workspace root, Cargo manifests,
toolchain pins, deny/product/boundaries config, tools/xtask scanners.
Gate: **PASS — 53/53 assignments audited, 0 missing, 0 thin.**

## Verdicts (file-level, from gate-report.json)
REAL 39 · REAL-UNPROVEN 1 (desktop-tauri) · UNREACHABLE 10 (crate-level:
_template, plugin-host-wasm, device-discovery, platform-linux, control-protocol,
transcode, platform-windows, platform-macos …) · FALSE 2 (product.toml,
.env.example) · STUB 1 (package-lock.json — recorded as file-level REAL-STUB mix;
see A08 log)

## Findings
**S0:4 S1:0 S2:11 S3:43 S4:63 — total 121**

## Worst findings (citations verified at gate)
1. **[S0] deny.toml:3-6** — header claims enforcement ("any lane that runs
   enforces this file") but no CI lane invokes cargo deny at all. (W01-A06 F-001;
   orchestrator re-run: `cargo deny check licenses sources bans advisories` →
   **advisories FAILED** — yanked `wasmi 2.0.0` + `wasmi_core 2.0.0` via
   plugin-host-wasm, stale RUSTSEC-2024-0370 ignore; bans/licenses/sources ok.
   audit/runs/baseline/orchrun-cargo-deny-full.log.txt)
2. **[S0] product.toml:4-7** — "build-breaking error" claim is false: the
   product-config validator is not wired into any shipped path; the file ships
   failing (4 missing brand assets) with no build break. (W01-A08 F-001)
3. **[S0] product.toml:135-140** — rule F1 ("cannot name unimplemented
   features") is false: schema allows naming and enabling unimplemented
   features. (W01-A08 F-002)
4. **[S0] product.toml:150** — `audio-output = true # default audio device
   output` while the shipped binary registers only the null device
   (apps/cli/src/host.rs:79,1639). (W01-A08 F-003)
5. **[S2] .gitignore:31-38** — global `*.log` rule silently excluded the
   program's own baseline evidence from git (why INDEX-referenced .log runs were
   missing; worked around by committing them as `.log.txt`). (W01-A03 F-001)
6. **[S2] docs/security/sbom/cyclonedx.json:10-13 vs Cargo.toml:51** — product
   SBOM omits the entire shipped desktop-tauri dependency closure; `sbom
   --check` re-run confirms DRIFT. (W01-A04 F-001;
   orchrun-xtask-sbom-check.log.txt)
7. **[S2] boundaries.toml:512-517 vs apps/cli/Cargo.toml:62** — committed
   drift: lumen-cli's real `streaming` edge is undeclared; the architecture
   gate is RED on main. (W01-A05 F-001, corroborated W01-A01 F-001)
8. **[S2] crates/platform-windows/Cargo.toml:20-24** — claimed cross-target
   verification has no execution record anywhere in the repo; identical claim
   shape was fabricated before (OF-5). Orchestrator re-ran
   `cargo check -p platform-windows --target x86_64-pc-windows-msvc`: exit 0 —
   the code compiles, the *claim of a recorded verification* stands refuted.
   (W01-A03 F-001; orchrun-check-platform-windows.log.txt)
9. **[S2] product.schema.json:5,164 + dual validators** — schema and two
   divergent `pascal_case` implementations (xtask vs crate) give opposite
   verdicts for dot-containing names; gate tests a hand-mirrored SCHEMA
   fixture, not the committed schema. (W01-A07 F-003)
10. **[S2] apps/web/Cargo.toml:1-2 vs ui/src/client/app-transport.ts:73-79** —
    header claim contradicts the UI's transport reality. (W01-A10 F-001)

## REQUEST-ORCH-RUN outcomes (orchestrator, during gate)
- W01-A03 `cargo check -p platform-windows --target x86_64-pc-windows-msvc` →
  **exit 0** (orchrun-check-platform-windows.log.txt)
- W01-A08 `cargo run -p xtask -- codegen --check` → **no drift**, 65 commands /
  26 events (orchrun-xtask-codegen-check.log.txt)
- W01-A04 `cargo run -p xtask -- sbom --check` → **DRIFT confirmed** (158
  components, desktop-tauri closure absent) (orchrun-xtask-sbom-check.log.txt)
- W01-A06 `cargo deny check licenses sources bans advisories` → **advisories
  FAILED** (2 yanked wasmi crates, 1 stale ignore, wildcard warnings,
  unused-wrapper warning); bans/licenses/sources ok
  (orchrun-cargo-deny-full.log.txt, orchrun-cargo-deny-advisories.log.txt)
- W01-A09 4:2:0 interactive open session + decode-bench → already executed in
  Wave-0 correction v2 (mp4-smoke-open-session-420.txt, mp4-smoke-bench-420.txt)
- W01-A02 desktop-tauri build/test → **UNVERIFIED (environmental)**: sandbox has
  no gtk/webkit system libraries (baseline #2, exit 101).
- W01-A10 wasm32 `lumen-web` build (optional) → **UNVERIFIED (environmental)**:
  897 MB free disk; deferred to Wave 12 for re-request.

## Citation spot-check (§7.3 step 4)
14 audit files sampled (3 citations each): PASS 1 · PARTIAL 1 · FAIL 2 →
both FAILs manually adjudicated as **extractor false-positives** (compound
`Where: A; B` citations where the code block quotes source B; quotes verified
verbatim against deny.toml:28-30, tools/xtask/src/cmd/config_validator.rs:607-612,
crates/product-config/src/semantic.rs:128-131, product.schema.json:21-25).
**Fabrications: 0** — record in audit/verify/fabrications.md.

## Blocked / UNVERIFIED carried forward
- lumen-tauri build+test (needs webview host) — permanent environmental.
- wasm32 lane — deferred to W12.
- Yanked-registry status of the other 117 pinned versions — UNVERIFIED (wasmi
  pair confirmed yanked via cargo-deny; full sweep needs `cargo deny check
  advisories` DB, done, but per-version registry dates not enumerated).
- GitHub Actions run history — UNVERIFIED (no API access from audit sandbox).
