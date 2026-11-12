import type { ReactElement } from "react";
import type { ComponentType } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b02";
import { GENERATED_AREA_IDS } from "../generated/Screens";
import { GENERATED_AREA_TITLES } from "../generated/Screens";
import { GENERATED_SCREENS } from "../generated/Screens";
import { SETTINGS_ENTRIES } from "../generated/schema";
import type { SettingEntry } from "../generated/schema";
import { SettingsSections } from "../SettingsSections";
import { schemaSectionsOf } from "../SettingsSections";
import { InvertedSwitchField } from "../SettingWidgets";
import { SettingsSliderField } from "../SettingWidgets";
import { StartupVolumeField } from "../SettingWidgets";
import { LocalePicker } from "./language/LocalePicker";
import { ThemeModePicker } from "./appearance/ThemeModePicker";
import { AccentColorPicker } from "./appearance/AccentColorPicker";
import { InterfaceArea } from "./interface";
import { AboutArea } from "./updates-about";
import { CodecsArea } from "./codecs";
import { EffectsArea } from "./effects";
import { VideoArea } from "./video";
import { AudioArea } from "./audio";
import { PrivacyArea } from "./privacy";
import { SubtitlesArea } from "./subtitles";
import { NetworkArea } from "./privacy-network";
// w26.b02 — sibling area components under the FIXED module names (wave-B
// lanes b07/b08 land them in parallel). If these imports show unresolved-
// module errors, the sibling files have not landed yet — EXPECTED during the
// wave (tsc TS2307 + vite import-analysis failures in every suite that
// mounts SettingsScreen), gone at merge (task w26.b02; see
// docs/waves/w26/logs/w26.b02.md).
//
// SIBLING CONTRACT (b07/b08) — `export function <Name>(): ReactElement`,
// zero props; the component OWNS its whole area screen and composes, in
// Figma frame order, its hand-authored sections WITH the schema-static
// sections generated from the w26.b02 seed:
//   • ControlsArea ("Settings - controls", C8): Keyboard Shortcuts (hand —
//     the hotkeys fold, HotkeysEditor) → Skip Amounts + Mouse (SCHEMA —
//     render `<GENERATED_SCREENS.controls />` from ../generated/Screens, or
//     SettingsSections with the SETTING_CONTROLS_* entries) → Remotes (hand).
//   • RecordingArea (C12): Recording (SCHEMA — GENERATED_SCREENS.recording:
//     directory + format) → Casting → Streaming out (hand).
//   • ExtensionsArea (C13): add-on LIST + toolbar (hand — dynamic data, not
//     [[setting]] rows) → "Load add-ons on startup" (SCHEMA —
//     GENERATED_SCREENS.extensions).
//   • LibraryArea (C9): pure hand (Opening Files + Metadata & Artwork; no
//     [[setting]] rows are seeded for library) — the playlist-services fold.
// The registry REPLACES the generated screen records for these areas (see
// SETTINGS_SCREENS below), so a sibling that skips the schema sections would
// leave seeded rows reachable only via search — don't.
import { ControlsArea } from "./controls";
import { RecordingArea } from "./recording";
import { ExtensionsArea } from "./extensions";
import { LibraryArea } from "./library";

// Settings AREAS registry (card 4.E2 + w26.b02 reshape) — the 13 Figma
// Settings areas (design/figma/Settings - *.svg, analysis tools-settings.md
// D3) are THE registry; the former flat list of 12 (5 generated + 7 hand)
// is gone. Merges (Lead decision, w26 wave plan):
//   hotkeys  → controls  (sibling ControlsArea hosts the hotkeys editor)
//   codecs   → video     (registry fold below)
//   playlist-services → library (sibling LibraryArea composes the content)
//   about    → global    (registry fold below)
//   advanced → in-flow Advanced disclosure per area (advanced=true entries
//              render behind each home screen's Advanced Options bar —
//              SettingsSections; the old advanced AREA retires, slug redirects)
//   effects  → audio     (registry fold below; EQ embeds per audio-1 frame)
//   appearance → interface, language → global (already the schema placement)
//
// MECHANISM (unchanged, card 4.E2): the generator emits what is schema-static
// (screens with [[setting]] rows); hand-authored areas and per-key special
// widgets live HERE and win over the generated screen records. The shell
// (SettingsScreen) imports this module, never generated/Screens directly.

// ---------------------------------------------------------------------------
// Pre-registration i18n seam (contract §4.5, the a10 `useShellMsg` pattern
// applied to registry data): the three hand-area title msgids that have no
// catalog entry yet (video/audio/subtitles) live in
// ui/src/i18n/keys/w26.b02.ts until the Lead registers them post-wave (b19).
// The generated titles for controls/recording/extensions flow through the
// generator's own msgid() seam (generated/Screens.tsx). After registration
// both paths carry the same literals — the casts become no-ops.
// ---------------------------------------------------------------------------
type LaneKey = keyof typeof laneKeys;

const laneMsg = (key: LaneKey): MessageKey => key as string as MessageKey;

/** Global = schema sections + the About fold BEFORE the Advanced bar (g10-P2-3). */
function GlobalAreaScreen(): ReactElement {
  return (
    <SettingsSections
      area="global"
      sections={schemaSectionsOf("global")}
      beforeAdvanced={<AboutArea />}
    />
  );
}

/** Video = VideoArea (the frame's 4 sections + Reset picture, areas/video)
 * + the folded codecs rows (codecs→video) — g12-P0-1. */
function VideoAreaScreen(): ReactElement {
  return (
    <>
      <VideoArea />
      <CodecsArea />
    </>
  );
}

/** Audio = AudioArea (the frame's 3 sections with the capability-backed
 * Output device row interleaved, areas/audio) + the folded effects/EQ
 * surface last — g12-P0-2. */
function AudioAreaScreen(): ReactElement {
  return (
    <>
      <AudioArea />
      <EffectsArea />
    </>
  );
}

/**
 * Playback rows the frame's Tracks section does not carry (g11-P2-2): the
 * three language/mode rows stay REAL, LIVE settings (the onboarding wizard
 * writes them; search still finds them) — the playback screen's own
 * composition just hides them from the frame's 4-row Tracks section.
 */
const PLAYBACK_HIDDEN_KEYS: ReadonlySet<string> = new Set([
  "playback.preferred-audio-languages",
  "playback.preferred-subtitle-languages",
  "playback.subtitle-mode",
]);

/** Playback = the generated frame sections minus the non-frame Tracks rows. */
function PlaybackAreaScreen(): ReactElement {
  const sections = schemaSectionsOf("playback").map((section) => ({
    ...section,
    entries: section.entries.filter(
      (row) => !("key" in row && PLAYBACK_HIDDEN_KEYS.has(row.key)),
    ),
  }));
  return <SettingsSections area="playback" sections={sections} />;
}

export type SettingsAreaId =
  | (typeof GENERATED_AREA_IDS)[number]
  | "video"
  | "audio"
  | "subtitles"
  | "library"
  | "network";

/**
 * Exactly the 13 Figma areas, in the Settings sidebar order (App 4 · Media 5
 * · System 4): global, interface, privacy, controls · playback, video, audio,
 * subtitles, library · network, performance, recording, extensions.
 */
export const SETTINGS_AREA_IDS: readonly SettingsAreaId[] = [
  "global",
  "interface",
  "privacy",
  "controls",
  "playback",
  "video",
  "audio",
  "subtitles",
  "library",
  "network",
  "performance",
  "recording",
  "extensions",
];

// Contract guard: every schema-backed area (GENERATED_AREA_IDS — a screen with
// at least one ui=true [[setting]] row) must appear in the 13-area registry;
// a seeded screen without a registry row would be an unreachable route. Grows
// only with a Lead decision (a 14th area), never silently.
const MISSING_GENERATED_AREAS = (GENERATED_AREA_IDS as readonly string[]).filter(
  (id) => !(SETTINGS_AREA_IDS as readonly string[]).includes(id),
);
if (MISSING_GENERATED_AREAS.length > 0) {
  throw new Error(
    `settings registry (areas/index.tsx) is missing schema areas: ${MISSING_GENERATED_AREAS.join(", ")}`,
  );
}

export const SETTINGS_AREA_TITLES: Record<SettingsAreaId, MessageKey> = {
  ...GENERATED_AREA_TITLES,
  video: laneMsg("settings.area.video"),
  audio: laneMsg("settings.area.audio"),
  subtitles: laneMsg("settings.area.subtitles"),
  library: "settings.area.library", // registered ("Playlist & Services" — retitle to "Library" is a messages.ts change for the Lead/b19)
  network: "settings.area.network",
};

export const SETTINGS_SCREENS: Record<SettingsAreaId, () => ReactElement> = {
  // interface + performance stay pure schema-generated.
  ...GENERATED_SCREENS,
  // w26.gfix-B compositions (the frame inventories interleave hand rows
  // with the schema sections — the helpers at the top of this file):
  global: GlobalAreaScreen, // schema sections + About fold BEFORE the Advanced bar
  // w26.gfix-B (g11-P2-2): playback composes HERE (the frame's 4-row Tracks
  // section — the non-frame language/mode rows render via search/onboarding
  // only, PLAYBACK_HIDDEN_KEYS above).
  playback: PlaybackAreaScreen,
  // w26.f03 (e05 F3/F7): interface is HAND-AUTHORED — the Figma frame
  // (Settings - interface.svg) carries no "Advanced Options" bar, so the
  // schema sections render flat (areas/interface.tsx); sections stay
  // schema-driven for the w26.gfix-B seed.
  interface: InterfaceArea,
  // w26.gfix-B (g11-P1-1): privacy owns its composition (areas/privacy —
  // schema sections + the Clear-history action row in History).
  privacy: PrivacyArea,
  // controls / recording / extensions: the NEW sibling files own the whole
  // screen (frame order interleaves hand + schema sections — see the SIBLING
  // CONTRACT above); the generated records for these ids are replaced, not
  // stacked, so the siblings MUST embed GENERATED_SCREENS.<id> themselves.
  controls: ControlsArea,
  recording: RecordingArea,
  extensions: ExtensionsArea,
  // existing content files, registry-folded (siblings own the files):
  video: VideoAreaScreen, // generated frame sections + Reset picture + codecs fold
  audio: AudioAreaScreen, // generated frame sections + Output device row + effects/EQ fold
  subtitles: SubtitlesArea,
  library: LibraryArea, // playlist-services fold (b07/b08)
  network: NetworkArea,
};

/**
 * Retired settings slugs → merged home (the w26 merges above). SPA
 * 301-equivalent: NOTHING may 404. The route shell (SettingsScreen.tsx,
 * shell lane b01 — LANDED) consumes this via resolveSettingsAreaSlug():
 * `const target = resolveSettingsAreaSlug(area) ?? SETTINGS_AREA_IDS[0]`
 * → retired slugs deep-link to their merged home, unknown slugs to the
 * first area. [Lead: confirm `advanced` → `performance`; the Advanced area
 * dissolves into the per-area in-flow disclosure, so `performance`
 * (Decoding/Smooth Playback/Developer) is the nearest home.]
 */
export const SETTINGS_AREA_REDIRECTS: Readonly<Record<string, SettingsAreaId>> = {
  hotkeys: "controls", // controls area hosts the hotkeys editor (sibling ControlsArea)
  codecs: "video", // codecs rows fold under video (this registry)
  about: "global", // about panel folds under global (this registry)
  advanced: "performance", // advanced = per-area in-flow disclosure; nearest home
  effects: "audio", // effects/EQ fold under audio (this registry)
};

/**
 * Resolve a `:area` route slug: the 13-area id if valid, else the merged
 * home of a retired slug, else null (caller falls back to the first area).
 */
export function resolveSettingsAreaSlug(slug: string | undefined): SettingsAreaId | null {
  if (slug === undefined) return null;
  if ((SETTINGS_AREA_IDS as readonly string[]).includes(slug)) return slug as SettingsAreaId;
  return SETTINGS_AREA_REDIRECTS[slug] ?? null;
}

/**
 * Per-setting-key special widgets (card 4.E2 group 1). SettingField consults
 * this registry BEFORE its type-based default renderer — the framework seam.
 * Every key MUST be a real schemas/settings.toml key (AC-asserted in
 * areas.test.tsx).
 *
 * w26.f03 (e05 F2/F4): the accent picker wires wherever the seed carries the
 * accent key (`global.accent-color` per the finding; `interface.accent`/
 * `global.accent` per the settingsThemeSync gap note), and the theme picker
 * follows the theme key if the seed moves it from `interface.theme` to
 * `global.theme` (e05 F3). Both registrations are DERIVED from the schema so
 * the AC (every key real) holds at every merge state of the parallel f01
 * seed lane.
 */
const THEME_SETTING_KEYS: readonly string[] = ["interface.theme", "global.theme"];
const ACCENT_SETTING_KEYS: readonly string[] = ["global.accent-color", "interface.accent", "global.accent"];

const themeEntry = SETTINGS_ENTRIES.find((e) => THEME_SETTING_KEYS.includes(e.key));
const accentEntry = SETTINGS_ENTRIES.find((e) => ACCENT_SETTING_KEYS.includes(e.key));

// w26.gfix-B: the frame's slider rows (g11-P2-3 / g12-P1-3), the inverted
// start-playing switch (g11-P2-2) and the Values-1 startup-volume control
// (g12-P1-4) — every key a real schema key (AC below).
const SLIDER_WIDGET_KEYS: readonly string[] = [
  "video.hue",
  "video.brightness",
  "video.contrast",
  "video.saturation",
  "video.gamma",
  "playback.default-rate",
  "audio.maximum-volume",
  "interface.hide-controls-after",
];

export const SPECIAL_FIELD_WIDGETS: Readonly<Record<string, ComponentType<{ entry: SettingEntry }>>> = {
  ...(themeEntry !== undefined ? { [themeEntry.key]: ThemeModePicker } : {}),
  ...(accentEntry !== undefined ? { [accentEntry.key]: AccentColorPicker } : {}),
  "global.language": LocalePicker,
  "playback.start-paused": InvertedSwitchField,
  "audio.startup-volume": StartupVolumeField,
  ...Object.fromEntries(SLIDER_WIDGET_KEYS.map((key) => [key, SettingsSliderField])),
};

/**
 * Compound-widget keys (g12-P1-4): rendered INSIDE another key's special
 * widget (the Values-1 startup-volume control owns the mode enum), never as
 * a standalone row — SettingField returns null for them and the composed
 * screens filter them out of the section arrays. Defined in SettingWidgets
 * (beside the compound widget); re-exported here for the registry API.
 */
export { WIDGET_GROUPED_KEYS } from "../SettingWidgets";
