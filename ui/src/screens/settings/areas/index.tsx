import type { ReactElement } from "react";
import type { ComponentType } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { keys as laneKeys } from "../../../i18n/keys/w26.b02";
import { GENERATED_AREA_IDS } from "../generated/Screens";
import { GENERATED_AREA_TITLES } from "../generated/Screens";
import { GENERATED_SCREENS } from "../generated/Screens";
import type { SettingEntry } from "../generated/schema";
import { LocalePicker } from "./language/LocalePicker";
import { ThemeModePicker } from "./appearance/ThemeModePicker";
import { AboutArea } from "./updates-about";
import { CodecsArea } from "./codecs";
import { EffectsArea } from "./effects";
import { VideoArea } from "./video";
import { AudioArea } from "./audio";
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

/** Global = generated language row + the folded About panel (about→global). */
const GeneratedGlobalScreen = GENERATED_SCREENS.global;

function GlobalAreaScreen(): ReactElement {
  return (
    <>
      <GeneratedGlobalScreen />
      <AboutArea />
    </>
  );
}

/** Video = VideoArea + the folded codecs rows (codecs→video). */
function VideoAreaScreen(): ReactElement {
  return (
    <>
      <VideoArea />
      <CodecsArea />
    </>
  );
}

/** Audio = AudioArea + the folded effects/EQ surface (effects→audio). */
function AudioAreaScreen(): ReactElement {
  return (
    <>
      <AudioArea />
      <EffectsArea />
    </>
  );
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
  // interface, privacy, playback, performance stay pure schema-generated.
  ...GENERATED_SCREENS,
  // global composes HERE (both halves already exist: generated language row
  // + the about fold) — no new sibling file for this area.
  global: GlobalAreaScreen,
  // controls / recording / extensions: the NEW sibling files own the whole
  // screen (frame order interleaves hand + schema sections — see the SIBLING
  // CONTRACT above); the generated records for these ids are replaced, not
  // stacked, so the siblings MUST embed GENERATED_SCREENS.<id> themselves.
  controls: ControlsArea,
  recording: RecordingArea,
  extensions: ExtensionsArea,
  // existing content files, registry-folded (siblings own the files):
  video: VideoAreaScreen, // VideoArea + codecs fold
  audio: AudioAreaScreen, // AudioArea + effects/EQ fold
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
 */
export const SPECIAL_FIELD_WIDGETS: Readonly<Record<string, ComponentType<{ entry: SettingEntry }>>> = {
  "interface.theme": ThemeModePicker,
  "global.language": LocalePicker,
};
