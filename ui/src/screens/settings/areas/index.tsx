import type { ReactElement } from "react";
import { useState } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { useMsg } from "../../../i18n/messages";
import { keys as i13Keys } from "../../../i18n/keys/w26.i13";
import { GENERATED_AREA_IDS } from "../generated/Screens";
import { GENERATED_AREA_TITLES } from "../generated/Screens";
import { GENERATED_SCREENS } from "../generated/Screens";
import { AdvancedDisclosure } from "../SettingsSections";
import { SettingField } from "../SettingField";
import { partitionAdvanced } from "./shared";
import { schemaSectionViews } from "./shared";
import { InterfaceArea } from "./interface";
import { AboutArea } from "./updates-about";
import { EffectsArea } from "./effects";
import { VideoArea } from "./video";
import { CodecsFold } from "./video";
import { AudioArea } from "./audio";
import { PlaybackArea } from "./playback";
import { PrivacyArea } from "./privacy";
import { SubtitlesArea } from "./subtitles";
import { NetworkArea } from "./privacy-network";
import { PerformanceArea } from "./performance";
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
//              SettingsSections; the old advanced AREA retires, slug
//              redirects; the dead component was deleted w26.k13, j13 P2-5)
//   effects  → audio     (registry fold below; EQ embeds per audio-1 frame)
//   appearance → interface, language → global (already the schema placement)
//
// MECHANISM (unchanged, card 4.E2): the generator emits what is schema-static
// (screens with [[setting]] rows); hand-authored areas and per-key special
// widgets live in the leaf modules and win over the generated screen records
// — SPECIAL_FIELD_WIDGETS moved to ../specialWidgets (w26.k12, j12 P2-5, the
// module-cycle fix) and is re-exported below under its frozen public name.
// The shell (SettingsScreen) imports this module, never generated/Screens
// directly.
//
// w26.k13 (j13 P3-8): the pre-registration seam apparatus for the
// video/audio/subtitles titles is GONE — messages.ts registers all three
// ("settings.area.video"/"-audio"/"-subtitles", registered with the b19
// merge), so the former laneMsg cast + the w26.b02 laneKeys import were
// no-ops; the titles flow as plain MessageKeys. (Same retirement for the
// "Library" retitle comment — messages.ts already ships "Library".)

/** ⓘ hint msgids per global row (w26.i13, h08 #3 — Settings - global.svg
 * circles: Startup×2 + Diagnostics×4; Settings - global-1.svg adds the 4
 * advanced rows; the Appearance rows carry NO circle — honest absence). */
const GLOBAL_HINTS: Readonly<Record<string, keyof typeof i13Keys>> = {
  "global.show-dialogs": "settings.hint.global.show-dialogs",
  "global.update-checks": "settings.hint.global.update-checks",
  "global.developer-mode": "settings.hint.global.developer-mode",
  "global.save-log": "settings.hint.global.save-log",
  "global.log-file": "settings.hint.global.log-file",
  "global.log-details": "settings.hint.global.log-details",
  "global.show-minor-warnings": "settings.hint.global.show-minor-warnings",
  "global.extra-cli-options": "settings.hint.global.extra-cli-options",
  "global.log-format": "settings.hint.global.log-format",
  "global.collect-playback-statistics": "settings.hint.global.collect-playback-statistics",
};

/**
 * Global — hand-composed (w26.i13, h08 defects #1/#2 — the family's only NO
 * verdict). Frame truth (Settings - global.svg): the column is Appearance ·
 * Startup · Diagnostics · the Advanced Options bar at y645 LAST — no frame
 * state shows an About section. The g10-P2-3 `beforeAdvanced` placement put
 * 113px of About rows between Diagnostics and the bar, displacing it 141px
 * (y786 vs y645), so expanding revealed all 4 advanced rows below the 865px
 * window fold. Fix: the bar is the column's last element again; the About
 * fold rides BEHIND the disclosure (after the advanced rows — the
 * h08-sanctioned option), and expanding scrolls the revealed rows into view
 * (the i12 shell pattern).
 *
 * w26.k12 (j12 P2-6/P3-4): the disclosure is the shell's ONE exported
 * AdvancedDisclosure — the hand copy here was its second implementation.
 * w26.k13: the section split rides the shared schemaSectionViews/
 * partitionAdvanced helpers (j13 ruling 1).
 */
function GlobalAreaScreen(): ReactElement {
  const m = useMsg();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const { sections, advanced } = partitionAdvanced(schemaSectionViews("global"));

  return (
    <div className="settings-sections" data-settings-area="global">
      {sections.map(({ section, entries }) => (
        <section key={section.id} className="settings-section" data-section={section.id}>
          <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
          {entries.map((entry) => (
            <SettingField key={entry.key} entry={entry} hint={GLOBAL_HINTS[entry.key]} />
          ))}
        </section>
      ))}
      {advanced.length > 0 ? (
        <AdvancedDisclosure open={advancedOpen} onToggle={() => setAdvancedOpen((open) => !open)}>
          {advanced.map((entry) => (
            <SettingField key={entry.key} entry={entry} hint={GLOBAL_HINTS[entry.key]} />
          ))}
        </AdvancedDisclosure>
      ) : null}
      {/* w26.i13 (h08 #2): the About fold renders BEHIND the disclosure —
       * reachable, honest, and out of the frame-true column. */}
      {advancedOpen ? <AboutArea /> : null}
    </div>
  );
}

/** Video = VideoArea (the frame's 4 sections + Reset picture, areas/video)
 * + the folded codecs rows behind an in-flow disclosure (codecs→video,
 * w26.i14 — g12-P0-1 composition + the h09 defect #5 fold honesty). */
function VideoAreaScreen(): ReactElement {
  return (
    <>
      <VideoArea />
      <CodecsFold />
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
 * Playback rows the frame's Tracks section does not carry live in
 * areas/playback.tsx (g11-P2-2) — the screen composes there, hand-rendered
 * with per-row ⓘ hints (w26.i14, h09 #2).
 */

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
  video: "settings.area.video", // registered (messages.ts, b19 merge)
  audio: "settings.area.audio", // registered (messages.ts, b19 merge)
  subtitles: "settings.area.subtitles", // registered (messages.ts, b19 merge)
  library: "settings.area.library", // registered — "Library" (messages.ts:1887)
  network: "settings.area.network",
};

export const SETTINGS_SCREENS: Record<SettingsAreaId, () => ReactElement> = {
  // interface + playback + performance render through their area files
  // (interface flat — its frame carries no Advanced bar; playback and
  // performance hand-render for per-row ⓘ hints, w26.i14 / w26.k13).
  ...GENERATED_SCREENS,
  // w26.gfix-B compositions (the frame inventories interleave hand rows
  // with the schema sections — the helpers at the top of this file):
  global: GlobalAreaScreen, // schema sections + the Advanced bar LAST (frame); About behind the disclosure (w26.i13)
  // w26.gfix-B (g11-P2-2) + w26.i14 (h09 #2): playback composes in
  // areas/playback — the frame's 4-row Tracks section with per-row ⓘ hints;
  // the non-frame language/mode rows render via search/onboarding only.
  playback: PlaybackArea,
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
  video: VideoAreaScreen, // generated frame sections + Reset picture + codecs fold (disclosure)
  audio: AudioAreaScreen, // generated frame sections + Output device row + effects/EQ fold
  subtitles: SubtitlesArea,
  library: LibraryArea, // playlist-services fold (b07/b08)
  network: NetworkArea,
  // w26.k13 (j13 P1-1): performance hand-renders for the frame's 10 ⓘ —
  // the GlobalAreaScreen precedent (per-row hints through SettingField).
  performance: PerformanceArea,
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
 * Per-setting-key special widgets (card 4.E2 group 1) — the registry every
 * key MUST be a real schemas/settings.toml key of (AC-asserted in
 * areas.test.tsx). w26.k12 (j12 P2-5) moved the registry to the
 * ../specialWidgets LEAF (SettingField imports it from there — the former
 * areas/index ⇄ SettingField module cycle is gone); re-exported here under
 * its frozen public name/shape for the registry API.
 */
export { SPECIAL_FIELD_WIDGETS } from "../specialWidgets";

/**
 * Compound-widget keys (g12-P1-4): rendered INSIDE another key's special
 * widget (the Values-1 startup-volume control owns the mode enum), never as
 * a standalone row — SettingField returns null for them and the composed
 * screens filter them out of the section arrays. Defined in SettingWidgets
 * (beside the compound widget); re-exported here for the registry API.
 */
export { WIDGET_GROUPED_KEYS } from "../SettingWidgets";
