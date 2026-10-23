import type { ReactElement } from "react";
import type { ComponentType } from "react";
import type { MessageKey } from "../../../i18n/messages";
import { GENERATED_AREA_IDS } from "../generated/Screens";
import { GENERATED_AREA_TITLES } from "../generated/Screens";
import { GENERATED_SCREENS } from "../generated/Screens";
import type { SettingEntry } from "../generated/schema";
import { HotkeysEditor } from "./hotkeys/HotkeysEditor";
import { LocalePicker } from "./language/LocalePicker";
import { ThemeModePicker } from "./appearance/ThemeModePicker";
import { CodecsArea } from "./codecs";
import { PlaylistServicesArea } from "./playlist-services";
import { NetworkArea } from "./privacy-network";
import { AboutArea } from "./updates-about";
import { AdvancedArea } from "./advanced";

// Settings AREAS layer (card 4.E2) — the content/extension half of the 4.E1
// framework. MECHANISM (recorded per the card): the generator only emits what
// is schema-static; dynamic areas (hotkeys — one row per keymaps/default.toml
// binding, deliberately NOT [[setting]] entries) and per-key special widgets
// are hand-written HERE and merged with the generated registry. The shell
// (SettingsScreen) imports from this module instead of generated/Screens
// directly, so the generator stays untouched and E3/E4 extend the same way
// (append to the records below — their schema-backed areas appear via the
// generator once the schema owner seeds entries).

/** Hotkeys — dynamic data area (no schema entries; see module header). */
function HotkeysAreaScreen(): ReactElement {
  return <HotkeysEditor />;
}

/** Card 4.E4 group-3 areas — hand-authored, registered like hotkeys. */
function CodecsAreaScreen(): ReactElement {
  return <CodecsArea />;
}
function PlaylistServicesAreaScreen(): ReactElement {
  return <PlaylistServicesArea />;
}
function NetworkAreaScreen(): ReactElement {
  return <NetworkArea />;
}
function AboutAreaScreen(): ReactElement {
  return <AboutArea />;
}
function AdvancedAreaScreen(): ReactElement {
  return <AdvancedArea />;
}

export type SettingsAreaId =
  | (typeof GENERATED_AREA_IDS)[number]
  | "hotkeys"
  | "codecs"
  | "library"
  | "network"
  | "about"
  | "advanced";

export const SETTINGS_AREA_IDS: readonly SettingsAreaId[] = [
  ...GENERATED_AREA_IDS,
  "hotkeys",
  "codecs",
  "library",
  "network",
  "about",
  "advanced",
];

export const SETTINGS_AREA_TITLES: Record<SettingsAreaId, MessageKey> = {
  ...GENERATED_AREA_TITLES,
  hotkeys: "settings.area.hotkeys",
  codecs: "settings.area.codecs",
  library: "settings.area.library",
  network: "settings.area.network",
  about: "settings.area.about",
  advanced: "settings.area.advanced",
};

export const SETTINGS_SCREENS: Record<SettingsAreaId, () => ReactElement> = {
  ...GENERATED_SCREENS,
  hotkeys: HotkeysAreaScreen,
  codecs: CodecsAreaScreen,
  library: PlaylistServicesAreaScreen,
  network: NetworkAreaScreen,
  about: AboutAreaScreen,
  advanced: AdvancedAreaScreen,
};

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
