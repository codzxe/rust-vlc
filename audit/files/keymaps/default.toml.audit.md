# AUDIT: keymaps/default.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-21 |
| Lines / keys | 281 / 150 binding rows (141 main + 9 platform overrides) + 4 non-binding `jump-size-*-ms` keys across 9 sections |
| File verdict | SHALLOW (complete, provenance-disciplined data — wired to display only; the real shipped keymap is ~8 bindings embedded in TypeScript, two of which contradict this file) |
| Reachable from shipped entry point? | INDIRECT-AS-DATA: no shipped binary reads this file at runtime; `ui/scripts/gen-hotkeys-data.mjs` reads it at build time into committed `bindings.ts`, which the UI ships for menu accelerators, the hotkeys editor, and onboarding. The `lumen` CLI has no keymap reader. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 4 S4: 2 |
| Commands run | `rg -o 'section: "…"' …/generated/bindings.ts \| sort \| uniq -c` (141 rows: media 18, navigation 35, audio 7, video 27, subtitles 12, playlist 8, window 12, advanced 22); `rg -c "label-msgid" keymaps/default.toml` (151 = 150 rows + 1 comment); `rg -c 'disabled = true'` (0); `rg -n "hotkeys\." locales/en-US.toml` (0) / `ui/src/i18n/messages.ts` (291); cross-check loop of all 14 `command =` ids against `crates/commands/src` (14/14 exist); read `ui/src/screens/player/keymap.ts`, `variants/variantKeymap.ts`, `PlayerScreen.tsx:755-862`, `ui/scripts/gen-hotkeys-data.mjs`, `hotkeysData.test.ts`; `sed -n` on all quoted ranges |

## 1. One-paragraph verdict
This is the best-documented data file in the repo — 150 bindings, every row carrying source provenance and conflict resolutions, all 14 engine `command` ids verified to exist in the commands crate, all `when` tokens from the documented set — and it is not the keymap the product runs. No shipped binary loads it: the `lumen` CLI has no reader, and the UI's actual dispatch comes from hand-copied TOML excerpts embedded in TypeScript (`ui/src/screens/player/keymap.ts:68-90` — "The keymap LOADING pipeline has not landed … seeded from a verbatim excerpt"), plus a variant seed (`variantKeymap.ts:20-21`) that binds **B** and **G** — keys this file already claims for `audio-track-cycle` (line 147) and `subtitle-delay-down` (line 189). So the hotkeys settings screen, generated from this file, tells the user "B — Cycle Audio Track" while pressing B in the shipped player toggles the miniplayer. On top of that: the `disabled` data channel the file's own notes rely on ("binding ships disabled-from-data", line 53) is used by zero rows, two macOS label msgids resolve in no catalog anywhere, Up/Down/Left/Right are each claimed twice in the same mode with no machine-readable gate, and the four `jump-size-*-ms` keys are read by nothing while their values are re-hard-coded as `delta_ms` literals twelve rows below.

## 2. What actually works (verified)
— verified by baseline #24 (`audit/runs/baseline/ui-hotkeys_check.log.txt`, EXIT=0): `gen-hotkeys-data --check: OK (141 bindings, 9 platform overrides)` — the TOML → `bindings.ts` generator pipeline is drift-guarded and the committed artifact matches the file (141 bindings + 9 overrides, exactly my section census).
— verified by my rg loop: every `command = "…"` value (playback.seek, playback.step-frame, playback.set-rate, playlist.*, video.take-screenshot, video.set-zoom, subtitle.set-scale — 14/14) exists in `crates/commands/src/catalog/command.rs` — no dead engine-command references in the `command` field.
— verified by `rg -o 'when = "…"' | sort | uniq -c`: only has-media (39), has-video (25), has-chapters (2) — all documented tokens (keymaps/default.toml:33-34).
— verified by cross-check: 133 of 135 unique label-msgids exist in `ui/src/i18n/messages.ts` (the two exceptions are F-004).

## 3. Findings

### F-001 [S2]
Where: keymaps/default.toml:1-45 (header) + ui/src/screens/player/keymap.ts:5-11
Code:
```
# Task 0.B3 (doc 13 §3) — G6 deliverable. Generated 2026-10-19.
```
```
// Keymap seeds live in keymaps/default.toml at repo root (binding-id keyed
// inline tables: `fullscreen = { key = "F", recipe = "fullscreen-toggle",
// mode = "player-focused", … }`). The keymap LOADING pipeline has not landed
```
What is wrong: The file's declared role — the product's keymap, doc 05 §8.1 — is not its real role. At runtime the shipped UI dispatches from: `DEFAULT_KEYMAP_TOML` (3 rows, hand-copied, keymap.ts:68-73), `DIALOG_KEYMAP_TOML` (3 rows, hand-copied, keymap.ts:85-90), `VARIANT_KEYMAP_TOML` (2 rows, keymap.ts's sibling variantKeymap.ts:17-22), and `CUSTOMIZE_DIALOG_KEYMAP` (1 row filtered out of the generated `bindings.ts`, PlayerScreen.tsx:191-192). That is 9 dispatchable bindings out of 150 declared. Space, S, N, P, arrows, volume keys, M, rate keys — none of them dispatch anything; they appear only as menu-accelerator display strings and hotkeys-editor rows (PlayerScreen.test.tsx:941-960 asserts exactly that: "accelerators are display strings from keymap DATA").
Why it matters: The declared-vs-shipped gap is the Wave-8 disease in the input layer: a settings screen that lists 150 shortcuts whose overwhelming majority do nothing. A user reading the hotkeys editor believes pressing `T` shows position; nothing listens.
What correct looks like: land the keymap loader the code itself references (keymap.ts:11 "When the loader lands, swap DEFAULT_KEYMAP_TOML for the loaded source") so `bindings.ts` is consumed by the dispatcher, or mark the undelivered rows `disabled = true` so the editor says so honestly.
Confidence: HIGH — Verification: read (PlayerScreen.tsx:755-862 dispatch flow traced end to end) + baseline #24.

### F-002 [S2]
Where: keymaps/default.toml:147, 189 vs ui/src/screens/player/variants/variantKeymap.ts:13-14, 20-21
Code:
```
audio-track-cycle = { key = "B", label-msgid = "hotkeys.audio-track-cycle", recipe = "cycle-audio-track", mode = "player-focused", when = "has-media", …
```
```
// and the media keys); final keys belong to the keymap data file, not code.
…
miniplayer-toggle = { key = "B", recipe = "miniplayer-toggle", mode = "player-focused" }
music-mode-toggle = { key = "G", recipe = "music-mode-toggle", mode = "player-focused" }
```
What is wrong: The shipped player binds B → miniplayer-toggle and G → music-mode-toggle (`PLAYER_SHELL_KEYMAP = merge(DEFAULT_KEYMAP, VARIANT_KEYMAP)`, variantKeymap.ts:44-46, dispatched at PlayerScreen.tsx:821). This file declares B → audio-track-cycle (line 147) and G → subtitle-delay-down (line 189), both mode "player-focused". variantKeymap.ts's header claim "Keys avoid every key claimed in keymaps/default.toml's player-focused mode (Space/S/N/P/M/E/F/I/Esc and the media keys)" is false — B and G are claimed (the author's own exclusion list even misses that M is claimed by `mute`).
Why it matters: User-visible contradiction: the hotkeys settings screen (generated from this file) shows "B — Cycle Audio Track"; pressing B in the player switches to the miniplayer. When the loader lands, `mergeKeymaps` rejects duplicate *ids*, not duplicate *keys* — the collision then becomes a double-dispatch.
What correct looks like: pick keys genuinely unclaimed in the TOML (e.g. Shift+M / Shift+N), or move the variant rows into this file and retire one side; make `mergeKeymaps` reject duplicate key specs in the same mode.
Confidence: HIGH — Verification: read both files + dispatch trace.

### F-003 [S3]
Where: keymaps/default.toml:53
Code:
```
open-disc = { key = "Ctrl+D", label-msgid = "hotkeys.open-disc", ui_action = "open-disc", mode = "player-focused", source = "design-package", note = "menu.svg; identical to VLC Qt Ctrl+D. v1: SourceSpec::Disc is future/feature-gated (doc 05 §2.2) — binding ships disabled-from-data" }
```
What is wrong: The note asserts "binding ships disabled-from-data" — but `rg -c 'disabled = true'` over the file returns **zero**; the token `disabled` occurs only inside that note string. The generator's disabled channel (`row["disabled"] === true`, gen-hotkeys-data.mjs:56) is therefore always false: committed `bindings.ts` shows `media.open-disc … disabled: false`, `media.convert … disabled: false`, `window.vlm-configurator … disabled: false`. Every binding whose note says "v1: feature-gated" (open-disc, open-capture-device, disc-menu, deinterlace, record, wallpaper, viewpoint-*) or "not-adopted-v1" (convert, vlm-configurator, dock-playlist) ships as enabled data.
Why it matters: The file contradicts itself in the one machine-readable channel it has for disposition; the hotkeys editor presents not-adopted features (VLM configurator!) as live bindings.
What correct looks like: `disabled = true` on every row whose note claims feature-gated/not-adopted, plus a generator warning for notes matching /feature-gated|not-adopted/ without the flag (note-text ↔ data agreement).
Confidence: HIGH — Verification: read + rg count + bindings.ts rows.

### F-004 [S3]
Where: keymaps/default.toml:276-277
Code:
```
seek-backward-short-macos = { key = "Cmd+Ctrl+Left", label-msgid = "hotkeys.seek-backward-short", command = "playback.seek", args = { target = { kind = "relative", delta_ms = -10000 } }, mode = "player-focused", when = "has-media", …
```
What is wrong: `hotkeys.seek-backward-short` and `hotkeys.seek-forward-short` resolve in **no** catalog in the repository: `locales/en-US.toml` has zero `hotkeys.*` keys (16 keys total, listed lines 70-97), and `ui/src/i18n/messages.ts` — which has `hotkeys.seek-backward/-extrashort/-medium/-long` — lacks both `-short` variants (verified by exact-string rg). Every other of the 135 unique label-msgids resolves in the UI catalog.
Why it matters: Two declared references are broken everywhere; nothing catches it because the Rust rule-4 audit never scans keymaps (my refs.rs audit F-001/F-003) and the UI generator strips `label-msgid` from platform rows (gen-hotkeys-data.mjs:70), so the defect is invisible until someone surfaces macOS override labels — then macOS users get missing labels.
What correct looks like: add the two keys to `ui/src/i18n/messages.ts` (and the reference catalog when it becomes real), or reuse the existing `hotkeys.seek-backward` msgid like the Linux rows do.
Confidence: HIGH — Verification: read + rg exact-string cross-check against both catalogs.

### F-005 [S3]
Where: keymaps/default.toml:134-137, 144-145, 101-102
Code:
```
nav-up = { key = "Up", label-msgid = "hotkeys.nav-up", ui_action = "nav-up", mode = "player-focused", …
nav-down = { key = "Down", label-msgid = "hotkeys.nav-down", ui_action = "nav-down", mode = "player-focused", …
```
```
volume-up = { key = "Up", label-msgid = "hotkeys.volume-up", recipe = "volume-up", mode = "player-focused", …
volume-down = { key = "Down", label-msgid = "hotkeys.volume-down", recipe = "volume-down", mode = "player-focused", …
```
What is wrong: Four keys are claimed twice inside the same mode: Up → `volume-up` + `nav-up` (both unconditional, both "player-focused"); Down → `volume-down` + `nav-down`; Left → `seek-backward` (when has-media) + `nav-left`; Right → `seek-forward` + `nav-right` (lines 136-137, notes say "Q4 — conflicts with design seek-backward/forward"). The intended discriminator (`in-disc-menu`) is a proposed token that appears nowhere in the data — `when` values in the file are only has-media/has-video/has-chapters.
Why it matters: In a data-driven dispatcher both rows match one keypress (`matchBindings` returns every hit); the resolution exists only in prose notes. No keymap schema exists (F-008) to flag same-mode duplicate keys, so this shipped silently.
What correct looks like: `when = "in-disc-menu"` on the nav-* rows the moment the token is real, and a validator that rejects duplicate key specs per (mode, when) pair.
Confidence: HIGH — Verification: read + when-token census by rg.

### F-006 [S3]
Where: keymaps/default.toml:78-81, 101-108
Code:
```
jump-size-extrashort-ms = 3000
jump-size-short-ms = 10000
jump-size-medium-ms = 60000
jump-size-long-ms = 300000
```
What is wrong: These four non-binding config keys are consumed by nothing — `rg -ln "jump-size"` over non-audit code hits only docs (worklog, feature-matrix, ui-inventory, VLC baselines) and this file. Their values are simultaneously duplicated as literals in the twelve seek bindings (`delta_ms = -10000` in seek-backward, line 101, right below `jump-size-short-ms = 10000`; ±3000/±60000/±300000 likewise) — two unlinked copies of the same tuning numbers.
Why it matters: Textbook declared-but-never-read config: change `jump-size-short-ms` and nothing in the product changes; the seek bindings carry their own hard-coded deltas. The keys are drift bait.
What correct looks like: either delete the jump-size keys or make the seek bindings reference them (`args = { target = { kind = "jump-size-short" } }` resolved by the loader), one source of truth.
Confidence: HIGH — Verification: read + consumer grep.

### F-007 [S4]
Where: keymaps/default.toml:267-281 vs ui/scripts/gen-hotkeys-data.mjs:70
Code:
```
        platformKeys.push({ platform: platformId, id, key: row["key"] });
```
What is wrong: The 9 `[platform_overrides.macos]` rows carry full binding payloads (key, label-msgid, command/recipe, args, mode, when, source, note) but the only consumer keeps `{platform, id, key}` and discards the rest. The override ids (`seek-backward-medium-macos`) also match no base binding id (base ids are section-qualified: `navigation.seek-backward-medium`), and no production component applies the overrides — the sole importer is `hotkeysData.test.ts:45`, which asserts only non-emptiness. The macOS keymap is decorative at every layer.
Why it matters: Rich data with no consumer and an id shape that can't join to the base rows; if overrides ever become real, the id mismatch makes them dangle.
What correct looks like: either strip the platform rows to key-only in the TOML too, or emit the full override rows and key them by the base binding id they patch.
Confidence: HIGH — Verification: read + importer grep.

### F-008 [S4]
Where: keymaps/default.toml:28-32
Code:
```
# FORMAT FLAG (for the Lead): doc 05 §8.1 specifies `config/keymaps/default.toml`
# with `[[binding]]` rows (`schema = "keymap.v1"`); this seed follows card 0.B3
# and D-003 (`keymaps/*.toml`, section-grouped `binding-id = { … }` entries with
```
What is wrong: Self-flagged format deviation from the spec: different path (`keymaps/` vs doc 05's `config/keymaps/`), different shape (section-grouped inline tables vs `[[binding]]`), no `schema = "keymap.v1"` key anywhere in the file (rg for `^schema` → no match). `schemas/keymap.schema.json`, which the header says the reconciliation must precede, does not exist (`ls schemas/` → commands.schema.json, settings.toml, generated/). The file therefore has no format validator — the lenient generator parse plus byte-compare drift guard is the only structural check, and it validates only the fields the generator reads.
Why it matters: An unvalidated config file accepts any drift (F-003/F-005 class defects are invisible by construction); the reconciliation "open question Q1" has been open since the seed landed.
What correct looks like: land `schemas/keymap.schema.json` in the reconciled shape and wire it into the config-validator gate, or amend doc 05 §8.1 to bless this shape.
Confidence: HIGH — Verification: read + `ls schemas/` + rg.

## 4. Per-key table (sections; binding rows enumerated by count, verified against generated bindings.ts)
| Section / key group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Header / provenance comments | 1-45 | REAL | — | Exemplary sourcing (VLC 3.0.23 oracle, design SVGs, conflict ledger); honest FORMAT FLAG (F-008) |
| `[media]` — 18 bindings (open-* 8, quit, convert, media-key-* 8, browser-* 2) | 46-72 | REAL as data / display-only | S2 (F-001) | open-disc note's disabled claim false (F-003); convert/vlm not-adopted but enabled |
| `[navigation]` — 4 `jump-size-*-ms` keys | 78-81 | REAL, UNREAD | S3 (F-006) | Zero consumers; values duplicated as delta_ms literals |
| `[navigation]` — 35 bindings (transport 6, frame-step 2, seek 8, rate 5, position/time 2, chapter/title/program 7, disc-menu, nav-* 5) | 83-137 | REAL as data | S3 (F-005) | Left/Right/Up/Down double-claimed with [audio]/[video] rows; rate-* semantics unresolved (Q5, recipes) |
| `[audio]` — 7 bindings | 140-150 | REAL as data | S2 (F-002 via B) | `audio-track-cycle` claims B, contradicted by shipped miniplayer binding |
| `[video]` — 27 bindings | 153-181 | REAL as data | — | All 4 `command =` ids exist (video.take-screenshot, video.set-zoom ×4 verified); wallpaper/deinterlace/record/crop-px rows feature-gated in notes only (F-003) |
| `[subtitles]` — 12 bindings | 184-196 | REAL as data | S2 (F-002 via G) | `subtitle-delay-down` claims G, contradicted by shipped music-mode binding |
| `[playlist]` — 8 bindings | 201-212 | REAL as data | — | `advanced-controls` (Ctrl+A) is the ONE row that flows TOML→generated data→live dispatch (PlayerScreen.tsx:191) |
| `[window]` — 12 bindings | 215-231 | REAL as data | S3 (F-003) | vlm-configurator not-adopted yet enabled; history-forward honestly left unbound |
| `[advanced]` — 22 bindings (set/play-bookmark 1-10, viewpoint 2) | 234-261 | REAL as data | — | Bookmark slots referenced by playback-engine comments only (bookmarks.rs:17) |
| `[platform_overrides.macos]` — 9 rows | 267-281 | REAL as data / STRIPPED | S3+S4 (F-004, F-007) | Full payloads discarded by sole consumer; 2 label-msgids resolve nowhere |

## 5. Test honesty (tests covering this file's data)
| Test | Would it fail if the data were broken? | Reason |
|---|---|---|
| `npm run hotkeys:check` (gen-hotkeys-data --check; baseline #24 PASS) | YES (for what it sees) | Byte-compare of regenerated vs committed bindings.ts — catches key/label-msgid/command drift in main rows |
| hotkeysData.test.ts:38-45 | PARTIAL | Asserts >100 rows, section set, one command present, macos overrides non-empty — cannot catch F-003/F-004/F-007 (disabled flags, override semantics, msgid resolvability) |
| PlayerScreen.test.tsx:941-960 | YES for display | Asserts exact accelerator strings from HOTKEY_BINDINGS rows |
| PlayerScreen keydown tests (via useKeymap seam) | YES for the 9 embedded bindings | But they test the TS seeds, NOT this file — the 141 non-embedded rows have zero dispatch tests (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| doc 05 §8.1: keymap at `config/keymaps/default.toml`, `[[binding]]` rows, `schema = "keymap.v1"` | doc 05 §8.1 | FALSE vs file | File is `keymaps/default.toml`, section-grouped, no schema key (F-008) |
| "every msgid referenced from … `keymaps/*` … resolves to a reference-catalog key" | doc 07 §6.4 rule 4 | FALSE | 2/135 unique label-msgids resolve nowhere (F-004); Rust audit never scans keymaps (refs.rs audit F-001) |
| "binding ships disabled-from-data" (open-disc) | keymaps/default.toml:53 note | FALSE | Zero rows carry `disabled = true` (F-003) |
| Generator header: "keymaps/default.toml (card 0.B3 G6 seed — READ-ONLY source of truth)" | ui/scripts/gen-hotkeys-data.mjs:4 | SHALLOW | Source of truth for display strings only; dispatch truth is 9 TS-embedded bindings, 2 contradicting it (F-001/F-002) |
| "Keys avoid every key claimed in keymaps/default.toml's player-focused mode" | variantKeymap.ts:13-14 | FALSE | B and G are claimed (lines 147, 189) (F-002) |
| VLC-parity provenance per row | notes column throughout | REAL-UNPROVEN | Internal consistency checks out (dates, wiki refs, oracle dump); the pinned-VLC cross-check itself is docs-only, not re-runnable here |

## 7. Dead / duplicated / unreachable code
- `jump-size-*-ms` (78-81): dead config (F-006).
- `[platform_overrides.macos]` semantics (label-msgid/command/args/mode/when): dead data (F-007).
- `disabled` channel: defined by the generator, used by zero rows (F-003).
- TOML dispatch role: duplicated (and partly contradicted) by keymap.ts / variantKeymap.ts seeds (F-001/F-002).

## 8. Dependencies & flags
- Pure data file; no feature flags. Consumers: `ui/scripts/gen-hotkeys-data.mjs` (build time → committed bindings.ts, drift-guarded, baseline #24), `ui/src/screens/player/PlayerScreen.tsx:191` (one row via generated data), docs/inventory/ui-inventory.md (provenance ledger). No Rust consumer exists.

## 9. Open questions for the next agent
- Keymap loader card (referenced by keymap.ts:11): owner and ETA? Until it lands, F-001/F-002 stand.
- Lead item Q1 (ui-inventory §9): which keymap format wins — doc 05 §8.1 or the 0.B3 seed? schemas/keymap.schema.json is the deliverable.
- recipes (`volume-up`, `cycle-audio-track`, `subsync-*`, ~50 ids): are they registered anywhere (doc 05 §2.7 recipe registry) or free-text? Not checked here — recipe ids were out of my file's blast radius.
- Whoever owns `ui/src/i18n/messages.ts`: add or alias the two `-short` hotkeys msgids (F-004).
