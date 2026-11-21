# AUDIT: plugins/extensions/metadata-provider/manifest.toml
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Lines / functions | 27 / 10 config keys (id, kind, version, api_version, origin, capabilities.provides, permissions.fs-sandboxed-read, provenance.{publisher,source,hash,license}) |
| File verdict | REAL (parse-validated by the plugin-api manifest schema and executed through the full package→install→spawn→drive conformance cycle in baseline) |
| Reachable from shipped entry point? | NO. No shipped app embeds the plugin host or an extension store (Grep `plugin-host-wasm` over apps/ → zero embedders; baseline gap rows `plugins.loaded/failed/crashed not wired — handoff H-8/H-9/H-10`, orchrun log :3880-3882). The file IS load-bearing for real code that runs in CI: `crates/plugin-host-wasm/tests/plugin_examples_conformance.rs:80` reads exactly this path (`self.dir().join("manifest.toml")`, dir under `plugins/extensions/`), and the baseline ran that suite green (:4569-4578). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `Read` manifest + sibling plugin.wasm + README; `sha256sum plugin.wasm` (digest match vs README:107); `Grep` in plugin-api manifest.rs/metadata.rs (schema + token legality); `sed -n` package.rs:355-395 (placeholder-stamp contract); `Grep` conformance test paths; baseline log :4569-4578 (conformance 5/5). |

## 1. One-paragraph verdict
A 27-line example manifest that is exactly what it claims to be: every field parses against the plugin-api schema, the capability token and permission key are legal vocabulary, the provenance hash ships the mandated placeholder form the packager stamps over, and the whole file — with its sibling module — was packaged, installed, spawned under production resource ceilings, and driven through the extract protocol by a baseline-green conformance test. The committed `plugin.wasm` (764 B) hashes to exactly the digest the README documents. Nothing ships it: the app-side plugin registry does not exist (gap H-8/H-9/H-10), so this manifest's production audience is zero — an honest, tested example awaiting an embedder.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:4572-4574` → `committed_example_modules_match_regeneration ... ok` (the committed module is byte-exact what the deterministic emitter produces) and `metadata_provider_example_passes_conformance ... ok` (package → install with consent → load with digest re-validation → spawn under `ResourceLimits::for_kind` → `extract` protocol driven, capability honesty asserted: `provides = ["tags"]` is true because extract really returns the sidecar's tag text).
— verified by command: `sha256sum plugins/extensions/metadata-provider/plugin.wasm` → `ed7ec5bef684d20f36d0e0bb95ede58feb0caecd8c0777071cac6f6eea39ce82` — identical to the digest the README's `lumenext package` walkthrough documents (:107), corroborating the README's "real command output" claim.
— verified by read: the placeholder discipline is real — `stamp_manifest_hash` (package.rs:359-395) refuses any hash line that is neither the placeholder (`…` + `(64 hex)`, :382-387) nor already the module's true digest, refuses more than one hash line, and the manifest ships exactly the placeholder.
— verified by read: vocabulary legality — `kind = "metadata-provider"` is a `PluginKind` variant; `provides` is the capability key that kind requires (manifest.rs:517, allowed keys `["provides","sources"]` :545); `"tags"` is the documented token (metadata.rs:38); `fs-sandboxed-read` is the permission key the host's import fence enforces (manifest.rs:758); `origin = "extension"` makes `[provenance]` required (:203/:366) and all four provenance fields are present; `api_version = 1` matches the v1-only window (api.rs `version_window_is_v1_only_today`, baseline :4503).

## 3. Findings
None. What was checked, exhaustively: (a) schema conformance of all 10 keys against plugin-api/src/manifest.rs (id parse, kind enum, semver version, api window, origin→provenance requirement, kind-correct capability key, deny-by-default permission table); (b) token legality ("tags", "fs-sandboxed-read"); (c) placeholder-hash contract vs package.rs:359-395; (d) file↔README↔conformance-test consistency (paths, digests, capability claims); (e) execution evidence in baseline (5/5 conformance, including regeneration drift gate). No claim in the file or its header comments is false. The only limitation — nothing ships it — is a property of the absent embedder, recorded by the repo's own gap tests, not a defect of this file.

## 4. Per-key table
| Key | Value | Verdict | Notes |
|---|---|---|---|
| `id` | `ext.example.metadata` | REAL | Parses as PluginId; example-TLD prefix — honest naming for an example. |
| `kind` | `metadata-provider` | REAL | Legal PluginKind; capability key mapping verified (manifest.rs:517). |
| `version` | `0.1.0` | REAL | Semver; matches the packaged filename in the README walkthrough. |
| `api_version` | `1` | REAL | The only accepted major today (api window test, baseline :4503). |
| `origin` | `extension` | REAL | Triggers the provenance-required rule — satisfied. |
| `capabilities.provides` | `["tags"]` | REAL | Legal token; capability-honesty asserted by the conformance run (extract returns real tag text). |
| `permissions.fs-sandboxed-read` | `true` | REAL | Declared; jail roots remain embedder-granted (deny-by-default documented in state.rs) — the conformance kit grants a media dir and drives real jailed reads. |
| `provenance.publisher` | `Example Extensions` | REAL | Free text; honest. |
| `provenance.source` | `https://example.com/…lumenext` | REAL | Reserved example host; provenance URL is metadata, never fetched by anything (verified: no consumer of `source` in the host crates). |
| `provenance.hash` | `sha256:…(64 hex)` | REAL | The mandated placeholder; stamped at package time, verified at load time. |
| `provenance.license` | `MIT` | REAL | Consistent with the repo license posture. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the manifest were broken? | Reason |
|---|---|---|
| `committed_example_modules_match_regeneration` (baseline :4572) | YES (module side) | Byte-exact drift gate on the sibling plugin.wasm. |
| `metadata_provider_example_passes_conformance` (:4574) | YES | The manifest IS an input: package stamps it, install re-validates it (digest, kind, permissions), spawn enforces its permission fence, and the capability claim is behaviorally asserted. A wrong key, illegal token, or lying capability fails the run. |
| `lumenext_cli_round_trip_packages_and_inspects_both_examples` (:4576) | YES | Round-trips this manifest through package+inspect. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The `[provenance] hash` ships the doc 06 §2.3 placeholder form; `lumenext package` stamps the real digest" (header comment :5-9) | manifest.toml:5-9 | REAL | package.rs:359-395 verified; placeholder present. |
| "any diverging full digest is the tamper refusal" (:8-9) | manifest.toml:8-9 | REAL | package.rs:383-387 refuses non-placeholder diverging digests at packaging; load re-validates (conformance kit description, README :140-142). |
| README table row: capabilities `provides = ["tags"]`, permissions `fs-sandboxed-read` | README:11 | REAL | Field-for-field identical to the file. |
| "Two real, loadable example extensions" (README:3) | README:3 | REAL with scope | Real and loadable — by the conformance kit and dev bin; NOT by any shipped app (gap tests :3880-3882). The README's own walkthrough only ever exercises the kit. |

## 7. Dead / duplicated / unreachable code
- Not applicable as code. The file is reachable only by test/dev tooling (see Reachability) — consistent with the repo's own gap-test record.

## 8. Dependencies & flags
- Pure TOML; consumed by plugin-api's std-only parser. No feature flags. The sibling `plugin.wasm` is 764 bytes, deterministic, drift-gated.

## 9. Open questions for the next agent
- install.rs / package.rs auditors: confirm the install-time digest re-validation path (the "tamper refusal" the header claims at load time) — I verified the packaging-side refusal only.
- Whoever audits the app-side plugin registry gap (H-8): when an embedder lands, this manifest's `source` URL becomes user-visible provenance — decide whether example.com URLs should be filtered from any "installable from" listing.
