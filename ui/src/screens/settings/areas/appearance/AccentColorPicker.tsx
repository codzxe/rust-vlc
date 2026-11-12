import { useContext, useEffect } from "react";
import type { ReactElement } from "react";
import { SwatchGroup } from "../../../../components/form/SwatchGroup";
import type { AccentId } from "../../../../tokens/generated/tokens";
import { useMsg } from "../../../../i18n/messages";
import type { MessageKey } from "../../../../i18n/messages";
import type { SettingEntry } from "../../generated/schema";
import { useSettings } from "../../SettingsProvider";
import { ThemeContext } from "../../../../theme/context";

// AccentColorPicker — the "Accent color" row control (w26.f03, e05 F2).
// Figma Settings - global.svg "Accent color" (Values): the 9-swatch group
// on a 320×36 rx6 track — orange (the W26 default accent) first, then red,
// yellow, green, teal(cyan), blue, purple(violet), magenta(pink), gray —
// the selected swatch in its accent-tinted slot. The existing SwatchGroup
// component carries exactly this grammar (radiogroup, roving tabindex,
// arrow keys with wrap-around); this picker wires it to the settings draft
// (dirty/apply contract) and previews the draft accent live through the
// ThemeContext seam (data-accent on <html>), extending the
// settingsThemeSync pattern to the accent — the gap note in
// ui/src/theme/settingsThemeSync.ts names this widget as its follow-up.
//
// LABELS reuse the registered onboarding accent vocabulary — the same nine
// names the onboarding appearance step shows (contract §4.5: reuse before
// new keys).

/** Frame swatch order (Settings - global.svg — orange first, the default). */
const FRAME_SWATCH_ORDER: readonly AccentId[] = [
  "orange",
  "red",
  "yellow",
  "green",
  "cyan",
  "blue",
  "violet",
  "pink",
  "gray",
];

/** Accessible name per accent — the onboarding vocabulary (registered). */
const ACCENT_LABEL_KEYS: Readonly<Record<AccentId, MessageKey>> = {
  orange: "onboarding.accent.orange",
  red: "onboarding.accent.red",
  yellow: "onboarding.accent.yellow",
  green: "onboarding.accent.green",
  cyan: "onboarding.accent.cyan",
  blue: "onboarding.accent.blue",
  violet: "onboarding.accent.violet",
  pink: "onboarding.accent.pink",
  gray: "onboarding.accent.gray",
};

const isAccentId = (v: unknown): v is AccentId =>
  typeof v === "string" && (FRAME_SWATCH_ORDER as readonly string[]).includes(v);

export function AccentColorPicker({ entry }: { entry: SettingEntry }): ReactElement {
  const m = useMsg();
  const { value, setValue } = useSettings();
  const theme = useContext(ThemeContext);

  // Orange is the W26 default accent (themes/tokens.json) — an unknown
  // draft value (stale override, enum drift) falls back to it rather than
  // rendering no selection (the swatch group's roving tabindex needs an
  // active swatch to stay keyboard reachable).
  const raw = value(entry.key);
  const current: AccentId = isAccentId(raw) ? raw : "orange";

  // Live preview seam: draft accent → ThemeProvider → data-accent on
  // <html> (same live-preview contract as the theme picker).
  useEffect(() => {
    if (theme !== null) theme.setAccent(current);
  }, [current, theme]);

  return (
    <div className="settings-field__special">
      <SwatchGroup
        label={m(entry.description as MessageKey)}
        options={FRAME_SWATCH_ORDER.map((accent) => ({
          value: accent,
          label: m(ACCENT_LABEL_KEYS[accent]),
        }))}
        value={current}
        onChange={(next) => setValue(entry.key, next)}
      />
    </div>
  );
}
