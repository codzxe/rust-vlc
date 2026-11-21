# AUDIT: plugins/extensions/ui-panel/manifest.toml
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Lines / functions | 27 / 10 config keys (id, kind, version, api_version, origin, capabilities.surfaces, permissions.ui-surface, provenance.{publisher,source,hash,license}) |
| File verdict | REAL (parse-validated by the plugin-api manifest schema and executed through the full package→install→spawn→register conformance cycle in baseline) |
| Reachable from shipped entry point? | NO. Same as its sibling: no app embeds the plugin host (Grep `plugin-host-wasm` over apps/ → zero; baseline gap rows `plugins.* not wired — handoff H-8/H-9/H-10`, orchrun log :3880-3882). Load-bearing for CI code: `plugin_examples_conformance.rs:80` reads this exact file; `ui_panel_example_passes_conformance` (:4575) ran green in baseline. The PanelRecord the plugin registers is consumed by NO shipped UI — the app-side composition card does not exist (README "Honest limitations" :207-209 records this). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `Read` manifest + sibling plugin.wasm + README; `sha256sum plugin.wasm` (digest match vs README:113); `Grep` in plugin-api manifest.rs/ui.rs (schema, surface token legality, permission key); `sed -n` package.rs:355-395 (placeholder contract, same as sibling); `sed -n` plugin_examples_conformance.rs:321-330 (the panel test's spawn + assertions); baseline log :4569-4578. |

## 1. One-paragraph verdict
The second example manifest, as clean as the first: schema-conformant, legally vocabulary'd, placeholder-hash-disciplined, and executed end-to-end by the baseline-green conformance kit — packaged, installed with consent, spawned under production ceilings with EMPTY host services, its `init` verified to register exactly one panel whose descriptor the host surfaces as a PanelRecord. The committed module's sha256 matches the README's documented digest to the byte. Its one structural truth is bigger than the file: the `surfaces = ["side-panel"]` capability is declarative-only end to end — the host records the panel and dispatches `on_command`, but no shipped application composes any PanelRecord into any layout, so even in a hypothetical embedding the panel would never draw. The repo documents this limitation honestly; the manifest makes no false claim.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt:4575` → `ui_panel_example_passes_conformance ... ok`: the test (read at conformance :321-330) spawns the installed example with `HostServices::empty()`, asserts the manifest handshake, and verifies `init` registers exactly ONE panel — the manifest's `ui-surface` permission is what gates the `lumen:v1.ui::ui_register_panel` import the guest calls.
— verified by command: `sha256sum plugins/extensions/ui-panel/plugin.wasm` → `acfe368e199b9fc774a9fec9b0e074646e718e45fc55352414b819c7451e4d37` — byte-identical to the README's packaged digest (:113).
— verified by read: `kind = "ui-panel"` maps to the `surfaces` capability key (manifest.rs:518, allowed `["surfaces"]` :546); `"side-panel"` is the documented surface token (plugin-api ui.rs:32, and manifest.rs's own round-trip test :1199 uses the same value); `ui-surface` is the permission key that fences the UI import family (manifest.rs:767); `origin = "extension"` requires `[provenance]` — all four fields present; the hash ships the exact placeholder `stamp_manifest_hash` demands (package.rs:382-387).
— verified by read: README table row (:12) is field-for-field identical to the file (id, kind, capabilities, permissions).

## 3. Findings
None. What was checked, exhaustively: (a) all 10 keys against the plugin-api schema (id/kind/version/api_version/origin parses; kind→capability-key mapping; provenance-required-for-extension rule satisfied); (b) token legality (`"side-panel"` surface, `ui-surface` permission); (c) the placeholder-hash contract vs package.rs:355-395; (d) file↔README↔conformance consistency including the digest; (e) baseline execution of the whole cycle (5/5). The declarative-only nature of the panel surface is a limitation the repo itself documents (README :207-209), not a manifest misclaim — the manifest asserts the plugin REGISTERS a side-panel, which is exactly and only what it does.

## 4. Per-key table
| Key | Value | Verdict | Notes |
|---|---|---|---|
| `id` | `ext.example.panel` | REAL | Parses; example prefix. |
| `kind` | `ui-panel` | REAL | Legal PluginKind; `surfaces` is its capability key (manifest.rs:518). |
| `version` | `0.1.0` | REAL | Semver; matches the packaged filename (README :114). |
| `api_version` | `1` | REAL | v1-only window (baseline :4503). |
| `origin` | `extension` | REAL | Provenance required — present. |
| `capabilities.surfaces` | `["side-panel"]` | REAL | Legal token; the panel registers exactly one such surface (conformance assertion). Declarative-only downstream — no shipped UI consumes PanelRecords (documented limitation). |
| `permissions.ui-surface` | `true` | REAL | Gates `lumen:v1.ui::ui_register_panel` — the import the guest actually calls; enforced at instantiation (undeclared imports refuse — README :181-182, the §2.2 fence). |
| `provenance.publisher` | `Example Extensions` | REAL | Honest free text. |
| `provenance.source` | `https://example.com/…lumenext` | REAL | Reserved host; never fetched by any code. |
| `provenance.hash` | `sha256:…(64 hex)` | REAL | Mandated placeholder; stamp-verified. |
| `provenance.license` | `MIT` | REAL | Consistent. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the manifest were broken? | Reason |
|---|---|---|
| `committed_example_modules_match_regeneration` (:4572) | YES (module side) | Byte-exact drift gate. |
| `ui_panel_example_passes_conformance` (:4575) | YES | The manifest drives the permission fence (a wrong permission key would refuse the guest's `ui_register_panel` import at instantiation), the kind→capability mapping, and the packaging stamp. |
| `lumenext_cli_round_trip_packages_and_inspects_both_examples` (:4576) | YES | Package+inspect round-trip prints this manifest's identity (README :126-131 output matches the fields). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The `[provenance] hash` ships the doc 06 §2.3 placeholder form; `lumenext package` stamps the real digest" (header :5-9) | manifest.toml:5-9 | REAL | package.rs:359-395. |
| README: `ui_register_panel`, §5.8 protocol, "ONE registered panel" | README:74-81, conformance :326-327 | REAL | The conformance test asserts exactly one panel; baseline green. |
| "The panel surface is declarative-only in v1: the host records `PanelRecord`s … actual UI composition into the layout system is the app-side card" | README:207-209 | REAL | Grep over apps/ finds no PanelRecord consumer; the app-side card does not exist. Honest limitation, correctly scoped. |
| README table row for ui-panel (:12) | README:12 | REAL | Field-for-field match. |

## 7. Dead / duplicated / unreachable code
- Not applicable as code. Test/dev-tooling-only reachability, same as the sibling; the panel's end-to-end lane stops at PanelRecord (no UI consumer anywhere in the tree).

## 8. Dependencies & flags
- Pure TOML; sibling `plugin.wasm` 764 B, deterministic, digest-pinned (verified). No flags.

## 9. Open questions for the next agent
- plugin-api ui.rs auditor (A02): is `side-panel` a CLOSED vocabulary (enum) or free string? manifest.rs:1199 and ui.rs:32 both use it, but I found no exhaustive surface list — if it is open, a future panel can claim arbitrary surface names and the host has no refusal.
- The app-side composition card (the one that would render PanelRecords) is unassigned in this wave — when it lands, re-run the reachability chain for this file: manifest → PanelRecord → layout slot.
