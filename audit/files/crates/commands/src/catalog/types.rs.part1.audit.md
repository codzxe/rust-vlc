# AUDIT: crates/commands/src/catalog/types.rs (part1, lines 1–455)
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2026-11-21 |
| Lines / functions | 455 audited (of 908; chunk part1 = 1–453, extended to 455 to close `StepDirection`) / 27 items (1 method, 20 type defs, 2 aliases, 2 id newtypes, module doc) |
| File verdict | REAL (wire vocabulary, faithfully derived from doc 05; several types unserved by any shipped host — by-design vocabulary with typed refusals) |
| Reachable from shipped entry point? | YES for the core: `lumen` CLI serves SessionOpen/PlaybackSeek/MediaGetInfo/TracksList/… (apps/cli/src/dispatch.rs `route()`, 16 commands) → these types are the args/results. NO for Volume/Rate/AspectRatioSpec/CropSpec/StepDirection/Bookmark/AudioDevice/PlaylistItemId surfaces — commands unserved, typed `UnsupportedForHost` refusal (verified in dispatch tests). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg "commands" apps/*/Cargo.toml` → cli, desktop-tauri, control-server, web, desktop, control-protocol, playback-engine all depend on it. `rg "can_transition_to"` → only tests (playback-engine model_check.rs/table.rs) + in-file tests. `rg "AspectRatioSpec|CropSpec|VideoSetAspectRatio"` → only catalog files (ids/command/spec), zero engine consumers. `sed -n` on every quoted range. Doc-05 §2.2/§2.3/§3.3 cross-read (lines 183-227, 405-411). product.toml/product.schema.json searched for `max-volume`/`max-rate` → zero hits. |

## 1. One-paragraph verdict
This is a clean, spec-faithful wire-type module: serde shapes match doc 05 §5.1 exactly (camelCase structs, kebab-case internally-tagged enums, `deny_unknown_fields` on every plain struct — I checked all ten), `PlaybackState::can_transition_to` is a line-for-line faithful rendering of doc 05 §3.3's transition table (cross-verified edge by edge), and the µs/ms unit split (`Timestamp` µs vs `SeekTarget::Relative.delta_ms`) matches the spec and its wire example. The defects are contract-vs-host drift: `OpenOptions` declares five options (rate, three track selectors, forced_subtitle_only) that the only shipped host silently drops — accepted-with-ignored-options, no typed refusal; the `Volume`/`Rate` doc comments cite config keys that exist nowhere in the repo and a rate ceiling (16.0) that contradicts the shipped schema (0.25..=4.0); and a handful of vocabulary types (`AspectRatioSpec`, `CropSpec`, …) back commands no shipped host serves, refused typed — honest but paper-thin availability.

## 2. What actually works (verified)
— Wire shapes: `SeekTarget::Relative` serializes as `{"kind":"relative","deltaMs":…}` (tag="kind", `rename_all_fields="camelCase"` at 188-192) — matches doc 05 line 471's example verbatim. All 10 plain structs in part1 carry `deny_unknown_fields` (113, 129, 145, 160, 241, 266, 343, 357, 372, 397), satisfying the module doc's "V1 closed shapes" claim (13-16).
— `can_transition_to` (321-338) vs doc 05 §3.3 (line 411): every documented edge present (idle→opening; opening→{buffering,error,idle,stopped}; buffering→{playing,paused,stopped,error}; playing↔paused; playing→buffering; playing/paused→{stopped,ended,error}; stopped/ended/error→idle; self-transitions), and the doc's "no direct active→idle edge on the wire" holds — no (Playing|Paused|Buffering, Idle) arm. Verified by reading both side by side.
— Bookmark doc claim "matches the keymap's 1-based bookmark slots" — verified: `keymaps/default.toml:238-242` defines `set-bookmark-1..5`.
— Unit conventions match doc 05 line 183 (`Timestamp(pub i64)` µs, negative only in Relative) and line 211 (`Relative { delta_ms }`).

## 3. Findings

### F-001 [S2]
Where: crates/commands/src/catalog/types.rs:242-261 (cross-site: apps/cli/src/dispatch.rs:83-88, apps/cli/src/host.rs:890-896)
Code:
```
pub struct OpenOptions {
    /// Start at this position (µs) instead of 0.
    pub start_time: Option<Timestamp>,
    /// Open paused instead of playing.
    pub start_paused: bool,
    /// Initial playback rate.
    pub rate: Option<Rate>,
    /// Disable the video track.
    pub no_video: bool,
    /// Disable the audio track.
    pub no_audio: bool,
    /// Pre-select a video track.
    pub video_track: Option<TrackSelector>,
    /// Pre-select an audio track.
    pub audio_track: Option<TrackSelector>,
    /// Pre-select a subtitle track.
    pub subtitle_track: Option<TrackSelector>,
    /// Only select forced subtitles.
    pub forced_subtitle_only: bool,
}
```
What is wrong: Doc 05 §2.3 (line 227) declares these options "schema-validated, unknown option = invalid-argument" — i.e., a KNOWN option is meaningful. The only shipped host silently drops 5 of the 10 fields: `apps/cli/src/dispatch.rs:83-88` calls `host.open_session(source, options.start_time…, options.no_video, options.no_audio)` and `open_session` (apps/cli/src/host.rs:890-896) takes exactly those four parameters — `rate`, `video_track`, `audio_track`, `subtitle_track`, `forced_subtitle_only` are never read. The desktop host delegates to this same dispatcher (apps/desktop/src/dispatch.rs:216 `lumen_cli::dispatch::lumen_dispatch`), so no shipped entry point honors them.
Why it matters: A control host sends `session.open` with `{"rate": 2.0}` or a subtitle pre-selection, gets `Accepted`/`Completed` success, and nothing happens — an accepted-but-ignored option. This is exactly the class of silent contract violation the doc's own truth rule (§2.13 rule 1: "a failed open or seek is never reported as success") exists to prevent. The repo itself already contains the correct pattern: the web host refuses these same options typed — `apps/web/src/host.rs:1478-1490` returns `UnsupportedForHost` "an initial rate is not served by the web v1 host (rate is card 3.A1)" and "track pre-selection is card 3.A3" — so the CLI/desktop lane is the inconsistent, dishonest one. The type's `[VERIFY]` marker (236-237) covers only the track-selector *rendering*, not the drop.
What correct looks like: The web host's exact refusal, ported to `apps/cli/src/dispatch.rs`'s SessionOpen arm: refuse opens carrying the five unserved options with a typed error naming the option and owner card.
Confidence: HIGH — Verification: read + signatures of both cross-sites quoted and line-verified + web-host contrast quoted (apps/web/src/host.rs:1478-1490).

### F-002 [S3]
Where: crates/commands/src/catalog/types.rs:51-57
Code:
```
/// Linear volume, `0.0..=audio.max-volume` (config, default 1.25)
/// (§2.2, `[PROPOSAL]` range per doc 05).
pub type Volume = f32;

/// Playback rate, `(0.0, playback.max-rate] (config, default 16.0)`
/// (§2.2, `[PROPOSAL]` range per doc 05).
pub type Rate = f32;
```
What is wrong: The cited config keys exist nowhere: repo-wide search for `max-volume`/`max-rate` returns zero hits in product.toml, product.schema.json, docs/07, and the product-config crate. Worse, the rate claim contradicts the shipped schema: `product.schema.json:180` bounds `default_rate` to `"minimum": 0.25, "maximum": 4.0` and `product.toml:193` ships `default_rate = 1.0 # schema 0.25..=4.0 in v1` — the wire type's doc says the ceiling is 16.0.
Why it matters: Two contradictory range contracts for the same knob (16.0 here vs 4.0 in the schema) is drift a host author will trip over; referencing a nonexistent config key (`audio.max-volume`) makes the doc unactionable. The `[PROPOSAL]` marker admits the numbers are proposals, but the shipped schema has already decided differently.
What correct looks like: Point the doc comments at the real keys (`playback.default_rate`, 0.25..=4.0 per schema) or add the proposed keys and reconcile the ceilings.
Confidence: HIGH — Verification: read + `rg` over product.toml/product.schema.json/docs (zero hits for the cited keys; schema line 180 quoted).

### F-003 [S4]
Where: crates/commands/src/catalog/types.rs:277-278
Code:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum VolumeKind {
```
What is wrong: `deny_unknown_fields` on a unit-only enum is a serde no-op (unit variants serialize as plain strings; unknown strings are rejected by deserialization regardless). Sibling unit enums (`TrackKind` 70, `TrackOrigin` 100, `PlaybackState` 292, `StepDirection` 448, `SeekKind` 458) correctly omit it — this one is inconsistent dead attribute noise implying a guarantee that isn't a thing.
Why it matters: Cargo-cult attributes mislead the next reader into thinking unit enums need it (or that it does something), and reviewers waste time on the diff anomaly.
What correct looks like: Drop the attribute.
Confidence: HIGH — Verification: read + serde semantics of unit-only enums.

### F-004 [S4]
Where: crates/commands/src/catalog/types.rs:405-444 (and their commands)
Code:
```
pub enum AspectRatioSpec {
    /// The track's display aspect ratio.
    Default,
    /// An explicit ratio.
    Ratio {
        /// Ratio width.
        w: u32,
        /// Ratio height.
        h: u32,
    },
}
```
What is wrong: `AspectRatioSpec`/`CropSpec` (and the `video.set-aspect-ratio`/`video.set-crop` commands they parameterize) have zero consumers outside the catalog itself (`rg` hits only ids.rs:126-127, command.rs:219-226, spec.rs:367/375) — no engine, renderer, or host code reads them; no shipped host serves the commands (CLI `route()` serves neither; unserved ⇒ typed `UnsupportedForHost`). Also nothing documents that `Ratio { w: 0 }`/`{ h: 0 }` is invalid — moot while unconsumed, a divide-by-zero foot-gun the day a renderer consumes it.
Why it matters: Vocabulary-only surface: the catalog *advertises* renderer commands that cannot execute anywhere in the repo. The refusal is typed and honest (control-server/dispatch doc states the served subset), so this is availability theatre, not fraud — but auditors of the feature matrix must not count these as working video commands.
What correct looks like: Keep only if the doc-05 full-vocabulary stance is intentional (it appears to be — "the wire never changes shape under it (§6)"); then document at the types that v1 hosts refuse them, and add a nonzero validation contract for `Ratio` before any consumer lands.
Confidence: HIGH — Verification: read + repo-wide consumer sweep (documented above).

### F-005 [S4]
Where: crates/commands/src/catalog/types.rs:228-231
Code:
```
    /// A network URL; scheme support is feature-gated (§6.4).
    Url {
        /// The URL, userinfo-free on the wire (V8).
        url: String,
    },
```
What is wrong: "Userinfo-free on the wire" is a sender-side promise nothing enforces at the command boundary: the field is a bare `String`, and the shipped open lane (apps/cli/src/demux_host.rs:404-423 `SourceSpec::Url` arm) parses and dispatches by scheme without inspecting userinfo — a `user:pass@host` URL flows into the streaming crate, which by design *accepts, strips, and uses* userinfo as credentials (crates/streaming/src/auth.rs:17, tests/auth_e2e.rs:99-119).
Why it matters: The V8 redaction guarantee (no credential leakage in errors/events) is real, but the wire-level claim on this type suggests the command layer rejects userinfo URLs; it does not — it silently credentializes them. Doc says one thing, boundary does another (mild, since V8's actual rule is about redaction of outgoing text, which is enforced elsewhere).
What correct looks like: Either reject userinfo-carrying URLs at the command boundary with `invalid-argument`, or reword the doc to "userinfo is stripped by the access layer (streaming auth) and never re-emitted (V8)".
Confidence: MEDIUM — Verification: read + traced the open lane and streaming auth docs/tests; did not execute a userinfo open end-to-end.

## 4. Per-function table (types file: one row per type + the one method)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-18 | REAL | — | claims verified: camelCase/kebab-case/deny_unknown_fields all hold in part1 |
| `Timestamp` (µs newtype) | 30-33 | REAL | — | live (CLI served commands; doc 05:183 matches) |
| `Duration` enum | 35-49 | REAL | — | internally tagged `kind`; Known/Unknown/Infinite — no sentinels, as claimed |
| `Volume` alias | 51-53 | REAL (unserved surface) | S3 | F-002 phantom config key |
| `Rate` alias | 55-57 | REAL (unserved surface) | S3 | F-002 contradicts schema 0.25..=4.0 |
| `TrackId` | 59-62 | REAL | — | live via TracksList/MediaGetInfo |
| `PlaylistItemId` | 64-66 | REAL (vocabulary-only) | — | playlist commands unserved by CLI route() |
| `TrackKind` | 68-80 | REAL | — | kebab-case unit enum; live |
| `TrackSelector` | 82-96 | REAL (vocabulary-only via OpenOptions) | — | dropped by shipped host (F-001); tracks.set-*-track unserved |
| `TrackOrigin` | 98-106 | REAL | — | part of TrackInfo (live via get-info) |
| `CodecDescriptor` | 108-123 | REAL | — | all-Option; `[VERIFY]` marker on granularity (honest) |
| `VideoTrackInfo` | 125-141 | REAL | — | no Eq/Hash (f32) — documented reason |
| `AudioTrackInfo` | 143-155 | REAL | — | |
| `TrackInfo` | 157-182 | REAL | — | live; `language: Option<String>` — note demux garbage "voe" flows in from engine side (baseline INDEX correction §, not this file's defect) |
| `SeekTarget` | 184-215 | REAL | — | live via PlaybackSeek; wire example verified against doc 05:471; µs/ms split per spec |
| `SourceSpec` | 217-233 | REAL | — | live via SessionOpen (File and Url lanes both implemented in demux_host.rs); F-005 doc nuance |
| `OpenOptions` | 235-261 | REAL (contract violated host-side) | S2 | F-001 |
| `AudioDevice` | 263-274 | REAL (vocabulary-only) | — | audio.list-devices unserved |
| `VolumeKind` | 276-288 | REAL | S4 | F-003 no-op attribute; consumed by catalog/event.rs:303 |
| `PlaybackState` | 290-310 | REAL | — | live (state events; baseline smoke shows the state cycle) |
| `PlaybackState::can_transition_to` | 312-339 | REAL | — | faithful to doc 05 §3.3 (edge-by-edge verified); callers: tests only (playback-engine model_check/table keep engine model in sync with wire) |
| `Chapter` | 341-353 | REAL | — | structurally live via MediaInfo.chapters (MediaGetInfo served) |
| `Bookmark` | 355-367 | REAL (vocabulary-only) | — | media.*-bookmark unserved; keymap claim verified real |
| `MediaInfo` | 369-392 | REAL | — | live via MediaGetInfo |
| `TimeRange` | 394-403 | REAL (vocabulary-only) | — | buffered-ranges query unserved |
| `AspectRatioSpec` | 405-418 | REAL (vocabulary-only) | S4 | F-004 |
| `CropSpec` | 420-444 | REAL (vocabulary-only) | S4 | F-004 |
| `StepDirection` | 446-454 | REAL (vocabulary-only) | — | playback.step-frame unserved; `[03]` marker on Previous |

## 5. Test honesty
No tests in lines 1-455 (test module starts at 789, part2/A05 territory). The in-file tests at 877-904 exercise `can_transition_to` (assert accepted/rejected edge sets + self-transitions) — from the visible assertions (877: accepted edges asserted with names, 891: rejected edges asserted, 904: self-transitions) they would fail if the table were wrong; full audit of those tests belongs to part2.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`Timestamp` is integer microseconds… `Rate`/`Volume` are `f32` wire scalars, `Duration` has an `Infinite` variant" | module doc 7-9; doc 05:183-187 | REAL | code 30-57 matches |
| "plain structs carry `deny_unknown_fields`" | module doc 15 | REAL | all 10 plain structs in part1 carry it |
| `OpenOptions`: "schema-validated, unknown option = invalid-argument", fields incl. rate/track selectors/forced_subtitle_only | doc 05:227 + types.rs 242-261 | REAL type / FALSE availability | shipped host drops 5 of 10 fields (F-001) |
| Volume range `0.0..=audio.max-volume (config, default 1.25)` | types.rs 51-52 | FALSE | no such config key anywhere (F-002) |
| Rate range `(0.0, playback.max-rate] (config, default 16.0)` | types.rs 55-56 | FALSE | no such key; shipped schema caps rate at 4.0 (product.schema.json:180) |
| transition table §3.3 | doc 05:411 ↔ types.rs 321-338 | REAL | edge-by-edge match (§2 above) |
| Bookmark ids "matches the keymap's 1-based bookmark slots, keymaps/default.toml `set-bookmark-N`" | types.rs 359-361 | REAL | keymaps/default.toml:238-242 |
| "userinfo-free on the wire (V8)" | types.rs 230 | SHALLOW | unenforced at boundary; streaming layer credentializes userinfo (F-005) |

## 7. Dead / duplicated / unreachable code
- `AspectRatioSpec`, `CropSpec`, `StepDirection`, `Bookmark`, `AudioDevice`, `TimeRange`, `PlaylistItemId`, `Volume`/`Rate` (as command payloads): zero served-command reachability — vocabulary only (F-004 and per-type rows). Not removable by design (full-vocabulary catalog, §6 forward-compat), but must not be counted as capability.
- `device-discovery` has its own `VolumeKind` (crates/device-discovery) — concept duplicated across wire and device layers; intentional layering (wire vs engine vocabulary) but a mapping seam that will need an audit when devices events go live.
- No dead code within part1 beyond the above.

## 8. Dependencies & flags
serde + schemars (JsonSchema) as declared in crates/commands/Cargo.toml (wire crate); `use crate::envelope::{Generation, SessionId}` (27-28) unused within part1 but consumed in part2 at lines 571/576 (SessionContext-like struct) — not dead. No feature flags gate any part1 type. No unsafe. No cfg(test) in part1.

## 9. Open questions for the next agent
- A05 (part2, lines 454-908): starts at `}` 454 / `SeekKind` 456 — I covered through 455 (StepDirection complete). `Generation`/`SessionId` imports are used at part2's lines 571/576 — not dead.
- Whoever audits apps/cli/src/dispatch.rs: F-001's ignoring site (83-88) needs its own finding row there. `forced_subtitle_only`/track-selector senders exist (apps/cli/src/parse.rs:530 hardcodes `false`/`None` for the CLI's own flag surface; apps/web/src/host.rs:1484 refuses them typed) — no test sends them through the CLI dispatcher expecting refusal; the drop is unpinned.
- A01/A03 (command.rs parts): `playback.set-rate` is the canonical unserved example — confirm the catalog spec (spec.rs) marks its availability honestly.
- apps/web/src/host.rs:1478-1490 is the honest-refusal reference implementation for F-001's fix.
