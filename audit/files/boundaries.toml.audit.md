# AUDIT: boundaries.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / config keys | 708 / [manifest].version, [rules].same_layer_pairs/dev_only/static_mut_allow, 39 × [crates.*] (5 keys each; 2 entries missing `external_dev_allowlist`) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev/CI config consumed by `cargo xtask check-architecture` (CI gate G5, full_gate.sh step 4); not part of any shipped binary. Enforced live: I ran `./target/debug/xtask check-architecture` -> exit 1, matches baseline full_gate.txt:44-56 exactly. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | `./target/debug/xtask check-architecture` -> exit 1 (1 EDGE violation, 4/5 checks pass, 39 nodes / 143 edges); python3 tomllib cross-check of all 39 entries vs member Cargo.tomls (incl. target-gated tables); `sed`/`grep` on apps/cli, apps/web, crates/device-discovery, root Cargo.toml, tools/vta/*, tools/bench-vs-vlc |

## 1. One-paragraph verdict
The manifest is real, machine-enforced (I ran the gate live), and 37 of 39 entries match their Cargo.tomls exactly — but the ONE that doesn't is the merge gate's current red light: `apps/cli/Cargo.toml` depends on `streaming`, and [crates.lumen-cli].internal_deps (L514) does not list it, violating the manifest's own update rule ("same change as the crate graph it permits, Lead review"). Beyond that live drift, the rules encode three structural holes: (1) stale declared-but-removed edges are tolerated forever by design ("the manifest may be ahead of the tree") and one is rotting right now (device-discovery's plugin-api, removed from its Cargo.toml on 2026-11-17, still declared, with a comment asserting "Deps exactly the registry row" that is now false); (2) dev-dependency edges are exempt from layer ordering, an exemption already load-bearing for an UPWARD edge (streaming L2 dev→ codecs L3); (3) the gate's "single source of truth" covers only the 39 root-workspace members — the actual desktop product (apps/desktop-tauri, ADR-004-excluded) and seven standalone-workspace crates that path-depend on engine crates (tools/bench-vs-vlc, tools/vta/{ac3,dnx,mpeg4,ruopus,vp9}) are invisible to it, so `[workspace]` in a Cargo.toml is an unchecked escape hatch from the entire boundary system.

## 2. What actually works (verified)
- `./target/debug/xtask check-architecture` -> exit 1, output identical to baseline full_gate.txt:44-56 (edges FAIL on exactly the streaming edge; cycles 39 nodes/143 edges/0 cycles; unsafe-lints 39/39; static-mut 0; testkit-direction PASS).
- python3 cross-check: manifest edge sum = 97 prod + 46 dev = 143 == checker's "143 internal edges (normal + dev)" (numbers reconcile through the two known discrepancies: +streaming undeclared, −plugin-api stale).
- All 4 same_layer_pairs are genuinely same-layer (computed: plugin-api/codec-api L1, desktop/lumen-cli L6, streaming/playlists L2, control-protocol/commands L1); 0 upward PROD edges across all 37 in-stack entries.
- 37/39 entries byte-accurate against their Cargo.tomls including external allowlists and target-gated tables (lumen-web's codecs dep is correctly declared target-gated).

## 3. Findings
F-001 [S2]
Where: boundaries.toml:512-517 vs apps/cli/Cargo.toml:62
Code:
```
[crates.lumen-cli]
layer = 6
internal_deps = ["audio-engine", "audio-output", "codec-api", "codecs", "commands", "demux", "input", "media-clock", "media-probe", "media-types", "playback-engine", "plugin-api", "renderer-wgpu", "settings-schema", "video-engine"]
```
(and apps/cli/Cargo.toml:62: `streaming = { path = "../../crates/streaming" }`)
What is wrong: The CLI app gained a production dependency on `streaming` (L2) without the Lead-reviewed manifest update this file's own header demands (L18-19: "Updating this file: same change as the crate graph it permits, Lead review"). Why it matters: architecture gate G5 is red on the merge lane (verified live, exit 1); either the edge is unratified or the manifest edit was forgotten — both break the file's core contract. What correct looks like: add "streaming" to internal_deps with the Lead-review record, or revert the Cargo.toml edge.
Confidence: HIGH — Verification: ran (check-architecture exit 1 names this exact edge) + python3 diff.

F-002 [S3]
Where: boundaries.toml:272-284
Code:
```
# SSDP, which is UDP multicast). Deps exactly the registry row:
# media-types (SourceUri/EngineError) + plugin-api (the doc 06 §1.2
...
[crates.device-discovery]
layer = 2
internal_deps = ["media-types", "plugin-api"]
```
What is wrong: The comment claims the deps are "exactly the registry row" including plugin-api, but crates/device-discovery/Cargo.toml removed plugin-api on 2026-11-17 (its own NOTE: the dep was dead weight, never imported). The stale edge is never flagged because the checker tolerates declared-but-absent edges ("Missing allowed-edges are fine", L11-12) — stale-edge rot is invisible by design. Why it matters: the manifest is drifting out of sync with the graph it claims to mirror, and the only drift the gate catches is the under-declaring direction. What correct looks like: remove "plugin-api" from the entry and fix the comment; optionally have the checker warn on declared-but-absent edges older than one change.
Confidence: HIGH — Verification: read + python3 diff + live gate run (this stale edge produced NO violation).

F-003 [S3]
Where: boundaries.toml:3-4 (coverage claim) vs root Cargo.toml:53-60 (exclude) and tools/{bench-vs-vtc,vta/*}
Code:
```
# THE single source of truth for allowed workspace structure, consumed by
# `cargo xtask check-architecture` (doc 01 §5.2, CI gate G5 per doc 10 §4.6).
```
What is wrong: "Allowed workspace structure" is enforced for exactly the 39 root members. Outside it, with path edges into engine crates and external dependencies, sit: apps/desktop-tauri (the real desktop product, workspace-EXCLUDED per ADR-004 — root Cargo.toml:53-60), tools/bench-vs-vlc (standalone `[workspace]`, path-deps codec-api/codecs/demux/input with a non-default `features = ["contract"]` on input), and tools/vta/{ac3,dnx,mpeg4,ruopus,vp9} (5 standalone crates path-depending on demux/plugin-api/media-types and pulling oxideav-ac3, oxideav-mpeg4video, filmcraft-dnx, ruopus, rusty_vp9). None of this is recorded in boundaries.toml. Why it matters: any crate escapes the whole boundary system by declaring its own `[workspace]`; the shipped desktop app's crate graph is entirely unchecked; the vta harnesses enable non-default features (`input/contract`) on engine crates that no manifest entry governs. What correct looks like: a manifest section (or checker check) enumerating the sanctioned escapees with their Lead-ratified records, so the exclusion list is itself reviewed.
Confidence: HIGH — Verification: read + `ls`/`grep` of tools/vta/*, tools/bench-vs-vlc/Cargo.toml, root Cargo.toml.

F-004 [S3]
Where: boundaries.toml:14-16 (rule) and 213-218 (the load-bearing use)
Code:
```
# pair (see [rules].same_layer_pairs). Dev edges ([dev-dependencies]) are
# allowlisted separately and are exempt from layer ordering — doc 01's own
# design has L0 `media-types` dev-depend on the dev crate `testkit`.
```
What is wrong: The exemption is justified by the testkit case (layer-less dev crate) but is written as a blanket exemption, and it is already used for a genuine UPWARD layer edge: streaming (L2) dev-depends on codecs (L3) — a lower layer linking a higher one. Computed: it is the only upward edge in the manifest (prod or dev). Why it matters: the layering guarantee ("edges point downward") does not hold for test graphs; an L0 crate could dev-depend on an L6 app and stay legal, so layer discipline is unenforced exactly where integration tests link the real stack. Cycles are still caught (dev edges included in the cycle check), which bounds the damage. What correct looks like: scope the exemption to layer-less targets (testkit), not to arbitrary cross-layer edges.
Confidence: HIGH — Verification: python3 layer computation over all manifest edges.

F-005 [S4]
Where: boundaries.toml:589-590 and 616-617
Code:
```
external_allowlist = ["wasmi", "toml"]
```
(then the next `[crates.lumen-web]` header — no `external_dev_allowlist` key in either plugin-host-wasm or lumen-web; the other 37 entries have it)
What is wrong: Two of 39 entries omit `external_dev_allowlist`; the checker defaults the missing key to an empty allowlist (check_architecture.rs:1222), so the check still runs — but the schema is inconsistent and a reviewer diffing entries cannot tell "no dev externals allowed" from "forgot to fill in". Why it matters: schema drift in the file whose whole value is being a precise, reviewable allowlist. What correct looks like: add `external_dev_allowlist = []` to both entries.
Confidence: HIGH — Verification: read + python3 key scan + checker source read.

F-006 [S4]
Where: boundaries.toml:45
Code:
```
same_layer_pairs = ["plugin-api->codec-api", "desktop->lumen-cli", "streaming->playlists", "control-protocol->commands"]
```
What is wrong: "desktop->lumen-cli" encodes app-wrapping-app (L6→L6) as permanently legal — the desktop composition root depends on the CLI app crate, which forces lumen-cli to remain a lib+bin and couples the two shipped apps' graphs; the comment itself admits it is a backfill of "the wrapper edge the skeleton always had" (L37-38). Not a current violation of anything, but it is the skeleton's shape ratified as architecture. What correct looks like: extract the shared host seam into a crate both apps depend on, then drop the pair.
Confidence: MEDIUM — Verification: read (architecture judgment; the pair is Lead-ratified and documented).

## 4. Per-entry table (all 39 [crates.*] entries; "OK" = manifest matches Cargo.toml incl. target-gated tables, verified by script)
| Entry | Layer | prod/dev/ext | Status |
|---|---|---|---|
| media-types | 0 | 0/1/0 | OK |
| codec-api | 1 | 1/0/0 | OK |
| plugin-api | 1 | 2/1/3 | OK |
| localization | 1 | 0/0/2 | OK |
| product-config | 1 | 0/0/4 | OK |
| settings-schema | 1 | 0/0/3 | OK |
| theme-tokens | 1 | 0/0/1 | OK |
| commands | 1 | 0/0/3 | OK |
| control-protocol | 1 | 1/0/2 | OK |
| input | 2 | 2/1/0 | OK |
| media-clock | 2 | 1/1/0 | OK |
| packet-queue | 2 | 1/1/0 | OK |
| demux | 2 | 2/2/0 | OK |
| streaming | 2 | 3/4/1 | OK (dev edge codecs = upward, F-004) |
| playlists | 2 | 0/0/1 | OK |
| media-probe | 2 | 2/2/0 | OK |
| subtitles | 2 | 1/2/0 | OK |
| device-discovery | 2 | 2/1/0 | STALE: declares plugin-api absent from Cargo.toml (F-002) |
| playback-engine | 4 | 6/4/0 | OK |
| codecs | 3 | 2/4/11 | OK (11 Lead-admitted decoder externals, records cited) |
| video-engine | 3 | 2/1/0 | OK |
| audio-engine | 3 | 2/1/0 | OK |
| transcode | 3 | 4/3/0 | OK |
| audio-output | 4 | 3/1/0 | OK |
| renderer-wgpu | 4 | 2/2/0 | OK (no wgpu import — documented, F-006-adjacent) |
| platform-windows | 5 | 2/1/0 | OK |
| platform-linux | 5 | 2/1/0 | OK |
| platform-macos | 5 | 2/1/0 | OK |
| lumen-cli | 6 | 15/1/2 | DRIFT: streaming undeclared (F-001) |
| desktop | 6 | 5/2/0 | OK (same-layer pair to lumen-cli, F-006) |
| plugin-host-wasm | 6 | 2/6/2 | missing external_dev_allowlist (F-005) |
| lumen-web | 6 | 11/1/2 | missing external_dev_allowlist (F-005) |
| control-server | 6 | 3/0/2 | OK |
| first-vertical | — | 2/1/0 | OK (layer-less test harness) |
| testkit | — | 0/0/0 | OK (dev_only rule, enforced) |
| lumen-mem-audit | — | 7/0/0 | OK (layer-less dev tool) |
| lumen-sweep | — | 7/0/0 | OK (layer-less dev tool) |
| template | — | 0/0/0 | OK (in no internal_deps list — enforced by absence) |
| xtask | — | 0/0/0 | OK |

## 5. Test honesty
No tests in this file. The enforcement tests live in tools/xtask/tests/check_architecture.rs (another agent's file). The gate itself was executed live by me (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "THE single source of truth for allowed workspace structure" | L3-4 | OVERSTATED | 7 standalone-workspace crates + excluded apps/desktop-tauri compile against engine crates outside it (F-003) |
| "Missing allowed-edges are fine (the manifest may be ahead of the tree)" | L11-12 | REAL | Checker enforces only under-declaration (live run: stale plugin-api edge unflagged) |
| "Adding a crate without a [crates.<name>] entry FAILS the check; a stale entry for a removed crate fails it too" | L20-21 | REAL | check_architecture.rs:1363-1372 (undeclared member) and :1491-1502 (stale crate entry) |
| "Deps exactly the registry row: media-types + plugin-api" (device-discovery) | L272-273 | FALSE | Cargo.toml removed plugin-api 2026-11-17 (F-002) |
| "Updating this file: same change as the crate graph it permits, Lead review" | L18-19 | VIOLATED IN TREE | lumen-cli->streaming edge without manifest update (F-001) |

## 7. Dead / duplicated / unreachable code
`[rules].static_mut_allow = []` is live config (check 4 scans 632 files for 0 occurrences — enforced). No dead keys; checker rejects unknown keys in [crates.*] loudly (check_architecture.rs:1242-1248).

## 8. Dependencies & flags
Parsed by tools/xtask/src/cmd/check_architecture.rs via toml_lite (std-only subset: no multi-line arrays — root Cargo.toml:44-46 documents that constraint shaping the single-line members array). No feature flags involved.

## 9. Open questions for the next agent
- Who added apps/cli's streaming dependency and was there a Lead review? (worklog/card reference absent from both the manifest comment and apps/cli/Cargo.toml context.)
- tools/vta/* pull oxideav-ac3 and oxideav-mpeg4video — neither is in any external_allowlist; are they admitted decoders hiding outside the boundary system, or pending candidates? Cross-check with deny.toml and codecs' registry claims (mp4-smoke baseline: engine-info says only mjpeg/pcm_s16le landed).
- Does any gate cover apps/desktop-tauri's dependency graph at all (its own Cargo.lock is unaudited by CI's cargo-deny --workspace)?
