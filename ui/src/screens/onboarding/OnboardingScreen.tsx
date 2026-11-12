// Onboarding wizard (card 4.F1) — W26 Figma redesign (wave 26, lane w26.a12).
//
// The 11-frame step flow (design/figma/Onboarding*.svg — welcome folder
// picker, appearance ×9 accent variants, shortcuts tour) recomposed per the
// wave plan as ONE centered hero card on the near-black stage: the wizard
// window (surface-chrome #161616, r-xl, hairline border, overlay shadow)
// carries a dot step indicator, the step content, and a footer whose left
// slot holds step furniture (the 9 accent dots on the appearance step, the
// "full list" note on shortcuts) and whose right slot is the frame's button
// row: Back (panel), Skip (ghost — the Lead-mandated quiet exit), Next /
// Get started (accent, 100×32 r6 per Onboarding.svg "Frame 140").
//
// Honesty rules that survived the redesign:
//   · the three privacy choices are opt-IN switches, all OFF, none
//     pre-checked; "Skip" completes the wizard with the defaults UNCHANGED;
//   · every control on every step does something real — the appearance
//     step live-previews through ThemeProvider AND persists its picks
//     through the same SettingsProvider seam as its sibling steps
//     (interface.theme / global.accent-color — the overrides store
//     ThemeProvider bootstraps every route from; w26.gfix-D), the
//     playback/subtitles steps write the REAL settings schema
//     (SettingsProvider → the shared overrides store), the media step opens
//     through the ONE open-path funnel (client/open-path.ts), and the
//     shortcut tour's keycaps are the REAL keymap (keymaps/default.toml via
//     the generated bindings) — data, never hand-typed copy;
//   · hosts without a ThemeProvider render the appearance controls inert
//     (disabled, not dead controls lying about their effect).
//
// The Solstice stage (sunset backdrop tier, washi-taped note, keycap strip,
// Nunito) is fully retired; this screen paints tokens only.

import { Fragment, useCallback, useContext, useEffect, useRef, useState } from "react";
import type { FormEvent, KeyboardEvent as ReactKeyboardEvent, ReactElement, ReactNode } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Button } from "../../components/button";
import { NumberField, SegmentedControl, Switch, TextField } from "../../components/form";
import { useClient } from "../../client/ClientProvider";
import { openPathRequest } from "../../client/open-path";
import { parseSourceInput } from "../open-media/OpenMediaDialog";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.a12";
import { keys as laneKeysI01 } from "../../i18n/keys/w26.i01";
import { HOTKEY_BINDINGS } from "../settings/areas/hotkeys/generated/bindings";
import type { HotkeyBinding } from "../settings/areas/hotkeys/generated/bindings";
import {
  SETTING_PLAYBACK_DEFAULT_RATE,
  SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES,
  SETTING_PLAYBACK_START_PAUSED,
  SETTING_PLAYBACK_SUBTITLE_MODE,
} from "../settings/generated/schema";
import { SettingsProvider, useSettings } from "../settings/SettingsProvider";
import {
  ACCENT_SETTING_KEY,
  THEME_SETTING_KEY,
  useSettingsThemeSync,
} from "../../theme/settingsThemeSync";
import { ThemeContext } from "../../theme/ThemeProvider";
import type { ThemeModeSetting } from "../../theme/ThemeProvider";
import { ACCENTS, type AccentId } from "../../tokens/generated/tokens";
import { useOnboardingActions, useOnboardingState } from "./onboardingState";
import type { OnboardingChoiceKey } from "./onboardingState";
import "./onboarding.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead) —
 * w26.a12's original onboarding set + w26.i01's frame-parallel additions
 * (shortcut row labels, the library field label, Browse). */
type LaneKey = keyof typeof laneKeys | keyof typeof laneKeysI01;
const laneCatalog: Readonly<Record<string, string>> = { ...laneKeys, ...laneKeysI01 };

/**
 * useOnboardingMsg — `useMsg` extended with this lane's pre-registration
 * keys. Lane keys win while unregistered; once the Lead merges them both
 * paths return the same string. Every user-visible string flows through
 * here (doc 07 §6.5 — no literals).
 */
function useOnboardingMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

// ---------------------------------------------------------------------------
// The step flow (11 frames → 8 steps)
// ---------------------------------------------------------------------------

/** Wizard steps in order (wave plan: welcome → media/library → appearance →
 * playback → subtitles → privacy → shortcuts → finish). */
const STEPS = ["welcome", "library", "appearance", "playback", "subtitles", "privacy", "shortcuts", "done"] as const;
type StepId = (typeof STEPS)[number];

/** Step titles. Welcome/done are the cataloged hero copy; the working steps
 * carry the frame headings where a frame exists (appearance, shortcuts). */
const STEP_TITLES: Record<StepId, MessageKey | LaneKey> = {
  welcome: "onboarding.welcome.title",
  library: "onboarding.library.title",
  appearance: "onboarding.appearance.heading",
  playback: "onboarding.playback.title",
  subtitles: "onboarding.subtitles.title",
  privacy: "onboarding.privacy.title",
  shortcuts: "onboarding.shortcuts.heading",
  done: "onboarding.done.title",
};

const STEP_BODIES: Record<StepId, MessageKey | LaneKey | null> = {
  welcome: "onboarding.welcome.body",
  library: "onboarding.library.body",
  // Frame: Onboarding-1.svg — heading goes straight to the theme cards.
  appearance: null,
  playback: "onboarding.playback.body",
  subtitles: "onboarding.subtitles.body",
  privacy: "onboarding.privacy.body",
  shortcuts: null, // Frame: Onboarding-2.svg — heading goes straight to the grid.
  done: "onboarding.done.body",
};

/** Hero steps center their content; working steps read top-down. */
const CENTERED_STEPS: ReadonlySet<StepId> = new Set(["welcome", "done"]);

// ---------------------------------------------------------------------------
// Privacy (card 4.F1 honest rule — unchanged by the redesign)
// ---------------------------------------------------------------------------

const PRIVACY_ROWS: ReadonlyArray<{
  key: OnboardingChoiceKey;
  labelKey: MessageKey;
  descriptionKey: MessageKey;
}> = [
  {
    key: "telemetry",
    labelKey: "onboarding.privacy.telemetry.label",
    descriptionKey: "onboarding.privacy.telemetry.description",
  },
  {
    key: "updateChecks",
    labelKey: "onboarding.privacy.update-checks.label",
    descriptionKey: "onboarding.privacy.update-checks.description",
  },
  {
    key: "onlineSuggestions",
    labelKey: "onboarding.privacy.online-suggestions.label",
    descriptionKey: "onboarding.privacy.online-suggestions.description",
  },
];

// ---------------------------------------------------------------------------
// Appearance (frame: Onboarding-1.svg / -3..10 — theme cards + 9 accent dots)
// ---------------------------------------------------------------------------

const THEME_CARDS: ReadonlyArray<{
  setting: ThemeModeSetting;
  choiceId: "light" | "dark" | "system";
  labelKey: MessageKey;
}> = [
  { setting: "light", choiceId: "light", labelKey: "onboarding.appearance.theme.light" },
  { setting: "dark", choiceId: "dark", labelKey: "onboarding.appearance.theme.dark" },
  { setting: "auto", choiceId: "system", labelKey: "onboarding.appearance.theme.system" },
];

const ACCENT_LABEL_KEYS: Record<AccentId, MessageKey> = {
  blue: "onboarding.accent.blue",
  cyan: "onboarding.accent.cyan",
  gray: "onboarding.accent.gray",
  green: "onboarding.accent.green",
  orange: "onboarding.accent.orange",
  pink: "onboarding.accent.pink",
  red: "onboarding.accent.red",
  violet: "onboarding.accent.violet",
  yellow: "onboarding.accent.yellow",
};

/** The footer dot row's order — frame: Onboarding-1.svg (orange first, the
 * default accent, then the design-language band set; the generated ACCENTS
 * array is alphabetical, the frame is not). */
const ACCENT_ORDER: readonly AccentId[] = [
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

/** Draft-value guards — settingsThemeSync's discipline: an unknown/stale
 * persisted value never guesses; the schema default renders instead. */
const isThemeModeDraft = (v: unknown): v is ThemeModeSetting =>
  v === "dark" || v === "light" || v === "auto";

const isAccentDraft = (v: unknown): v is AccentId =>
  typeof v === "string" && (ACCENTS as readonly string[]).includes(v);

// ---------------------------------------------------------------------------
// The shortcut tour (frame: Onboarding-2.svg — two columns × six rows)
// ---------------------------------------------------------------------------

/** Binding lookup by dotted id (e.g. "navigation.toggle-play-pause"). */
const BINDING_BY_ID: ReadonlyMap<string, HotkeyBinding> = new Map(
  HOTKEY_BINDINGS.map((binding) => [binding.id, binding] as const),
);

const binding = (id: string): HotkeyBinding | undefined => BINDING_BY_ID.get(id);

/**
 * Keymap spellings → keycap glyphs. Arrow-key spellings render as arrow
 * symbols on the caps (artwork, not copy); everything else shows the
 * keymap's own data, one cap per key — the frame renders combos as
 * ADJACENT caps with no joiner glyph (Onboarding-2.svg: "SHIFT" chip +
 * "S" chip, 4px apart), so no "+" is drawn between them.
 */
const KEYCAP_GLYPHS: Readonly<Record<string, string>> = {
  Down: "↓",
  Left: "←",
  Right: "→",
  Up: "↑",
  // frame: Onboarding-2.svg renders the modifier cap "SHIFT" (all caps) —
  // cap artwork, not copy: the keymap's own spelling stays "Shift".
  Shift: "SHIFT",
};

const keycapParts = (b: HotkeyBinding): readonly string[] =>
  b.key.split("+").map((part) => KEYCAP_GLYPHS[part] ?? part);

/** Symbol caps (arrows, <, >, comma, period) carry the frame's tight side
 * padding; letter/word caps (V, Q, SHIFT, Space) the fatter one — measured
 * from Onboarding-2.svg: arrow cap 46 = 16px glyph + 30, "," cap 35 = 5 + 30,
 * V cap 57 = 11 + 46, SHIFT cap 92 = 44 + 48 (one 0 22px padding rule made
 * single-glyph caps ~6px/side too fat — h02 #4). */
const isSymbolKeycap = (part: string): boolean =>
  part.length === 1 && !/[a-z0-9]/i.test(part);

const keycapClass = (part: string): string =>
  ["onboarding__key", !isSymbolKeycap(part) && "onboarding__key--text"]
    .filter(Boolean)
    .join(" ");

/** The tour rows — labels are the Onboarding-2.svg row strings, verbatim
 * (w26.i01: one casing convention, frame-parallel — the hotkey catalog's
 * Title-Case spellings are settings-side copy, not this frame's); each
 * row's keycaps stay the REAL keymap (keymaps/default.toml via the
 * generated bindings) — data, never hand-typed copy. */
const SHORTCUT_ROWS: ReadonlyArray<{
  labelKey: MessageKey | LaneKey;
  bindingIds: readonly string[];
}> = [
  { labelKey: "onboarding.shortcuts.row.play-pause", bindingIds: ["navigation.toggle-play-pause"] },
  {
    labelKey: "onboarding.shortcuts.row.seek",
    bindingIds: ["navigation.seek-backward", "navigation.seek-forward"],
  },
  {
    labelKey: "onboarding.shortcuts.row.item",
    bindingIds: ["navigation.prev-track", "navigation.next-track"],
  },
  {
    labelKey: "onboarding.shortcuts.row.frame",
    bindingIds: ["navigation.frame-prev", "navigation.frame-next"],
  },
  { labelKey: "onboarding.shortcuts.row.subtitle-cycle", bindingIds: ["subtitles.subtitle-track-cycle"] },
  { labelKey: "onboarding.shortcuts.row.audio-cycle", bindingIds: ["audio.audio-track-cycle"] },
  { labelKey: "onboarding.shortcuts.row.fullscreen", bindingIds: ["video.fullscreen"] },
  {
    labelKey: "onboarding.shortcuts.row.volume",
    bindingIds: ["audio.volume-up", "audio.volume-down"],
  },
  { labelKey: "onboarding.shortcuts.row.queue", bindingIds: ["playlist.queue-toggle"] },
  { labelKey: "onboarding.shortcuts.row.file-browser", bindingIds: ["playlist.file-browser-toggle"] },
  { labelKey: "onboarding.shortcuts.row.snapshot", bindingIds: ["video.snapshot"] },
  { labelKey: "onboarding.shortcuts.row.record", bindingIds: ["video.record"] },
];

/** Frame grammar: six rows per column, read down the left column first. */
const SHORTCUT_COLUMN_LENGTH = 6;

// ---------------------------------------------------------------------------
// The cone mark (frame: Onboarding.svg "Icon/Frame 4" — hero logo)
// ---------------------------------------------------------------------------

/** The Lumen cone mark — frame-exact port of the Onboarding.svg hero logo
 * (traffic-cone glyph: base plinth, band, tip as three stacked paths,
 * `fill: currentColor` so the accent token drives ink — frames 3..10 recolor
 * the mark with the selected accent). Geometry is frame-measured. */
function ConeMark({ className }: { className?: string }): ReactElement {
  return (
    // frame: Onboarding.svg (Icon/Frame 4) — three-segment cone, 107×126 art box
    <svg
      className={className}
      viewBox="492 122 107 126"
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      <path
        d="M573.648 200.666L575.932 207.52C569.082 212.546 558.58 217.116 545.795 217.116C532.553 217.116 522.508 212.546 515.659 207.52L517.942 200.666H507.44C505.156 200.666 503.787 202.036 502.873 203.864L492.828 242.247C491.915 244.989 494.198 248.188 497.394 248.188H593.739C596.936 248.188 598.762 245.446 598.306 242.247L587.803 203.864C587.347 202.036 585.52 200.666 583.237 200.666H573.648Z"
        fill="currentColor"
      />
      <path
        d="M521.138 187.413C526.618 191.526 534.837 195.181 545.339 195.181C555.841 195.181 564.06 191.069 569.539 187.413L563.147 165.48C559.037 168.222 553.101 170.049 545.339 170.049C537.576 170.049 531.64 167.765 527.531 165.48L521.138 187.413Z"
        fill="currentColor"
      />
      <path
        d="M533.466 144.461L537.575 130.296C539.858 122.985 550.361 122.985 552.644 130.296L556.753 144.461C553.557 145.375 549.904 146.289 545.338 146.289C540.772 146.289 537.119 145.375 533.466 144.461Z"
        fill="currentColor"
      />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Settings-backed steps (playback / subtitles) — real schema writes
// ---------------------------------------------------------------------------

/**
 * Auto-apply wrapper: wizard settings commit the moment they change (the
 * step copy says so). The ONE SettingsProvider at the screen root is the
 * REAL settings overrides store, so Settings reads the same values later
 * AND the appearance step's picks survive reload (w26.gfix-D — g02 P1-2);
 * nothing is staged behind an Apply bar the wizard never shows.
 */
function AutoApplySettings({ children }: { children: ReactNode }): ReactElement {
  const settings = useSettings();
  useEffect(() => {
    if (settings.dirtyCount > 0) settings.apply();
  }, [settings]);
  return <>{children}</>;
}

/** One settings row for controls that carry no visible label of their
 * own: label left, control right (the Settings row grammar). */
function SettingsRow({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor?: string;
  children: ReactNode;
}): ReactElement {
  return (
    <div className="onboarding__setting-row">
      <span className="onboarding__setting-label" id={htmlFor}>
        {label}
      </span>
      {children}
    </div>
  );
}

/** Playback step controls (schema: playback.start-paused, default-rate).
 * The NumberField renders its own (accessible) label; onboarding.css lays
 * the field out in the row grammar. */
function PlaybackControls(): ReactElement {
  const m = useOnboardingMsg();
  const settings = useSettings();
  const startPaused = settings.value(SETTING_PLAYBACK_START_PAUSED.key) === true;
  const rate = settings.value(SETTING_PLAYBACK_DEFAULT_RATE.key);
  return (
    <div className="onboarding__settings">
      <SettingsRow label={m("settings.playback.start-paused")} htmlFor="onboarding-start-paused">
        <Switch
          checked={startPaused}
          onChange={(checked) => settings.setValue(SETTING_PLAYBACK_START_PAUSED.key, checked)}
          aria-labelledby="onboarding-start-paused"
          data-testid="onboarding-setting-start-paused"
        />
      </SettingsRow>
      <NumberField
        className="onboarding__setting-field"
        label={m("settings.playback.default-rate")}
        min={SETTING_PLAYBACK_DEFAULT_RATE.min}
        max={SETTING_PLAYBACK_DEFAULT_RATE.max}
        step={0.05}
        value={typeof rate === "number" ? rate : Number(SETTING_PLAYBACK_DEFAULT_RATE.defaultValue)}
        onChange={(event) => {
          const next = Number(event.target.value);
          if (Number.isFinite(next)) settings.setValue(SETTING_PLAYBACK_DEFAULT_RATE.key, next);
        }}
        data-testid="onboarding-setting-default-rate"
      />
    </div>
  );
}

/** Subtitles step controls (schema: playback.subtitle-mode, preferred
 * subtitle languages). The segmented group's accessible name is its own
 * `label`; the visible row label is the SettingsRow one. */
function SubtitleControls(): ReactElement {
  const m = useOnboardingMsg();
  const settings = useSettings();
  const mode = String(settings.value(SETTING_PLAYBACK_SUBTITLE_MODE.key));
  const languages = settings.value(SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES.key);
  const languageList = Array.isArray(languages) ? languages : [];
  return (
    <div className="onboarding__settings">
      <SettingsRow
        label={m("settings.playback.subtitle-mode")}
        htmlFor="onboarding-subtitle-mode"
      >
        <SegmentedControl
          className="onboarding__setting-segmented"
          label={m("settings.playback.subtitle-mode")}
          options={SETTING_PLAYBACK_SUBTITLE_MODE.enumeration.map((option) => ({
            value: option.value,
            label: m(option.msgid as MessageKey),
          }))}
          value={mode}
          onChange={(next) => settings.setValue(SETTING_PLAYBACK_SUBTITLE_MODE.key, next)}
        />
      </SettingsRow>
      <TextField
        className="onboarding__setting-field"
        label={m("settings.playback.preferred-subtitle-languages")}
        value={languageList.join(", ")}
        placeholder="en, de"
        onChange={(event) =>
          settings.setValue(
            SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES.key,
            event.target.value
              .split(",")
              .map((part) => part.trim())
              .filter((part) => part !== ""),
          )
        }
        data-testid="onboarding-setting-subtitle-languages"
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Appearance step (frames: Onboarding-1 / -3..10) — picks that persist
// ---------------------------------------------------------------------------

/**
 * Theme cards (frames Onboarding-1/-3..10 — 100×100 r12 depictions at 156px
 * pitch). w26.gfix-D (g02 P1-2): the cards bind the SETTINGS draft
 * (`interface.theme`) on the same SettingsProvider + auto-apply seam the
 * playback/subtitles steps use, so the first-run answer lands in the
 * overrides store (`lumen.settings.overrides.v1`) ThemeProvider bootstraps
 * every route from — the pick survives reload instead of evaporating.
 * useSettingsThemeSync keeps the live preview (the frame recolors while
 * picking); hosts without a ThemeProvider still render the controls inert
 * (disabled, not dead controls lying about their effect).
 */
function AppearanceCards(): ReactElement {
  const msg = useOnboardingMsg();
  const settings = useSettings();
  const theme = useContext(ThemeContext);
  // Pushes the draft (mode + accent) into ThemeProvider — the live preview.
  useSettingsThemeSync();
  // Unknown/stale draft → no card reads selected (never guess).
  const rawMode = settings.value(THEME_SETTING_KEY);
  const mode = isThemeModeDraft(rawMode) ? rawMode : null;
  return (
    <div className="onboarding__theme-cards">
      {THEME_CARDS.map((card) => (
        <button
          type="button"
          key={card.setting}
          className={[
            "onboarding__theme-card",
            `onboarding__theme-card--${card.choiceId}`,
          ].join(" ")}
          aria-pressed={theme ? mode === card.setting : undefined}
          disabled={!theme}
          onClick={() => settings.setValue(THEME_SETTING_KEY, card.setting)}
          data-testid={`onboarding-choice-${card.choiceId}`}
        >
          <span className="onboarding__theme-swatch" aria-hidden="true" />
          <span className="onboarding__theme-label">{msg(card.labelKey)}</span>
        </button>
      ))}
    </div>
  );
}

/**
 * The footer accent dots (frame: Onboarding-1.svg footer row — 9 primaries
 * at 80px pitch, the selected one carrying the 60×60 rx30 ring). Same seam
 * as the theme cards: the radiogroup binds the `global.accent-color` draft
 * and commits immediately; the sync mounted in the cards previews the pick
 * live. Roving tabindex + arrow keys (w26.d1) drive the DRAFT — selection,
 * focus and the recolor land together.
 */
function AppearanceAccents(): ReactElement {
  const msg = useOnboardingMsg();
  const settings = useSettings();
  const theme = useContext(ThemeContext);
  // Unknown/stale draft falls back to the orange default — the roving
  // tabindex needs an anchored selection to stay keyboard reachable (the
  // AccentColorPicker's contract on the settings side).
  const raw = settings.value(ACCENT_SETTING_KEY);
  const accent: AccentId = isAccentDraft(raw) ? raw : "orange";

  /**
   * Arrow-key roving for the accent radiogroup (w26.d1 — the SwatchGroup
   * keyboard contract: arrows move focus AND selection with wrap-around;
   * the selected swatch is the group's single tab stop). WAI-ARIA
   * radiogroup pattern; inert hosts (no ThemeProvider → disabled buttons)
   * consume the key without side effects.
   */
  const onAccentKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const vertical = event.key === "ArrowDown" || event.key === "ArrowUp";
    const horizontal = event.key === "ArrowRight" || event.key === "ArrowLeft";
    if (!vertical && !horizontal) return;
    event.preventDefault();
    if (!theme) return;
    const current = ACCENT_ORDER.indexOf(accent);
    const delta = event.key === "ArrowUp" || event.key === "ArrowLeft" ? -1 : 1;
    const next = ACCENT_ORDER[(current + delta + ACCENT_ORDER.length) % ACCENT_ORDER.length];
    if (next === undefined) return; // unreachable (accent ids are total) — typed guard
    settings.setValue(ACCENT_SETTING_KEY, next);
    // Focus follows selection immediately (all 9 swatches exist in the DOM —
    // no re-render round-trip needed).
    event.currentTarget
      .querySelector<HTMLButtonElement>(`[data-accent="${next}"]`)
      ?.focus();
  };

  return (
    <div
      className="onboarding__accents"
      role="radiogroup"
      aria-label={msg("onboarding.appearance.accent")}
      data-testid="onboarding-accents"
      onKeyDown={onAccentKeyDown}
    >
      {/* Per-accent raw values are not tokenized for direct
       * consumption (they live under :root[data-accent=…] and
       * redefine --color-accent), so the dot primaries are the
       * frame-measured set (see onboarding.css) and the LIVE
       * accent token ring is the selected-state preview.
       * Radio semantics + roving tabindex (w26.d1): the selected
       * accent is the single tab stop; arrows roam (see
       * onAccentKeyDown). */}
      {ACCENT_ORDER.map((id) => (
        <button
          type="button"
          key={id}
          className="onboarding__accent"
          data-accent={id}
          role="radio"
          aria-checked={theme ? accent === id : undefined}
          aria-label={msg(ACCENT_LABEL_KEYS[id])}
          tabIndex={theme !== null && accent === id ? 0 : -1}
          disabled={!theme}
          onClick={() => settings.setValue(ACCENT_SETTING_KEY, id)}
          data-testid={`onboarding-accent-${id}`}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Library step — the REAL open-path funnel (client/open-path.ts)
// ---------------------------------------------------------------------------

/**
 * The media step body. Owns the client seam so the wizard's other steps
 * mount fine on hosts without a transport (the app entry always provides
 * one; the funnel needs it only here). No navigation — the wizard owns this
 * route until the user finishes; what was opened is waiting on the player.
 */
function LibraryStep(): ReactElement {
  const msg = useOnboardingMsg();
  const client = useClient();
  const [pathValue, setPathValue] = useState("");
  const [pathInvalid, setPathInvalid] = useState(false);
  const [openNotice, setOpenNotice] = useState<"idle" | "opening" | "opened" | "failed">("idle");
  const nextRequestId = useRef(0);

  const openMediaPath = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const source = parseSourceInput(pathValue);
    if (source === null) {
      setPathInvalid(true);
      setOpenNotice("idle");
      return;
    }
    setPathInvalid(false);
    setOpenNotice("opening");
    nextRequestId.current += 1;
    void client
      .send(openPathRequest(source, nextRequestId.current))
      .then((response) => {
        setOpenNotice("error" in response.result ? "failed" : "opened");
      })
      .catch(() => {
        setOpenNotice("failed");
      });
  };

  return (
    <>
      {/* frame: Onboarding.svg — a VISIBLE field label above the inset
       * field ("Choose a Media Folder…", recomposed honestly for the real
       * open-path funnel — file/playlist/stream, so "media"), the inset
       * #090909 rx5 field with its 2px stroke, and the 114×35 #1B1B1B
       * "Browse" submit — h01 #1, the twice-verified #1 shame. */}
      <form className="onboarding__open-form" onSubmit={openMediaPath}>
        <TextField
          label={msg("onboarding.library.field-label")}
          placeholder={msg("open.path.placeholder")}
          value={pathValue}
          error={pathInvalid ? msg("open.path.invalid") : undefined}
          onChange={(event) => {
            setPathValue(event.target.value);
            if (pathInvalid) setPathInvalid(false);
          }}
          data-testid="onboarding-open-path"
        />
        <Button
          type="submit"
          className="onboarding__open-action"
          loading={openNotice === "opening"}
          data-testid="onboarding-open-submit"
        >
          {msg("onboarding.library.browse")}
        </Button>
      </form>
      {/* Honest status line — the funnel's real outcome, announced. */}
      <p className="onboarding__open-notice" aria-live="polite" data-testid="onboarding-open-notice">
        {openNotice === "opened" && msg("onboarding.library.opened")}
        {openNotice === "failed" && msg("onboarding.library.failed")}
      </p>
      {/* Real keymap hints — the same caps as the shortcuts tour. */}
      <ul className="onboarding__open-hints">
        {["media.open-file", "media.open-network-stream"].map((id) => {
          const b = binding(id);
          return b ? (
            <li key={id} className="onboarding__open-hint">
              <span className="onboarding__shortcut-label">
                {msg(b.labelMsgid as MessageKey)}
              </span>
              <span className="onboarding__shortcut-keys">
                {keycapParts(b).map((part) => (
                  <kbd className={keycapClass(part)} key={part}>
                    {part}
                  </kbd>
                ))}
              </span>
            </li>
          ) : null;
        })}
      </ul>
    </>
  );
}

// ---------------------------------------------------------------------------
// The screen
// ---------------------------------------------------------------------------

export function OnboardingScreen(): ReactElement {
  const msg = useOnboardingMsg();
  const navigate = useNavigate();
  const { choices } = useOnboardingState();
  const { setChoice, complete } = useOnboardingActions();
  const [stepIndex, setStepIndex] = useState<number>(0);
  const step: StepId = STEPS[stepIndex] ?? "welcome";
  const isDone = step === "done";

  const next = (): void => setStepIndex((i) => Math.min(i + 1, STEPS.length - 1));
  const back = (): void => setStepIndex((i) => Math.max(i - 1, 0));
  // Skip = jump to done with the defaults UNCHANGED. It completes the
  // wizard (so first-run gating opens) but never touches a privacy flag.
  const skip = (): void => {
    complete();
    navigate("/player");
  };
  const finish = (): void => {
    complete();
    navigate("/player");
  };

  const stepLabel = `${msg("onboarding.progress.label")} ${stepIndex + 1} ${msg(
    "onboarding.progress.of",
  )} ${STEPS.length}`;

  const bodyKey = STEP_BODIES[step];

  return (
    <main className="onboarding" data-testid="onboarding-root">
      {/* ONE settings seam for the whole wizard (w26.gfix-D): every step's
       * settings commit the moment they change — playback/subtitles rows and
       * the appearance step's interface.theme / global.accent-color picks
       * (the keys ThemeProvider bootstraps from on every fresh load, so the
       * first-run answer survives reload — g02 P1-2). */}
      <SettingsProvider>
        <AutoApplySettings>
          <section
            className="onboarding__window"
            aria-labelledby="onboarding-title"
            data-testid="onboarding-window"
          >
            {/* Step indicator — the frames' quiet grammar: a row of dots, the
             * current one a wider accent pill. The accessible count lives on
             * the list itself (announced once). */}
            <ol
              className="onboarding__progress"
              aria-label={stepLabel}
              data-testid="onboarding-progress"
            >
              {STEPS.map((id, index) => (
                <li
                  key={id}
                  className={[
                    "onboarding__progress-dot",
                    index < stepIndex && "onboarding__progress-dot--done",
                    index === stepIndex && "onboarding__progress-dot--current",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  aria-current={index === stepIndex ? "step" : undefined}
                >
                  <span className="visually-hidden">{`${msg("onboarding.progress.label")} ${index + 1}`}</span>
                </li>
              ))}
            </ol>

            <div
              key={stepIndex}
              className={[
                "onboarding__step",
                CENTERED_STEPS.has(step) && "onboarding__step--center",
              ]
                .filter(Boolean)
                .join(" ")}
              data-testid={`onboarding-step-${step}`}
            >
              {(step === "welcome" || step === "done") && (
                <ConeMark className="onboarding__cone" />
              )}

              <h1 className="onboarding__title" id="onboarding-title">
                {msg(STEP_TITLES[step])}
              </h1>
              {bodyKey !== null && <p className="onboarding__body">{msg(bodyKey)}</p>}

              {step === "library" && <LibraryStep />}

              {step === "appearance" && <AppearanceCards />}

              {step === "playback" && <PlaybackControls />}

              {step === "subtitles" && <SubtitleControls />}

              {step === "privacy" && (
                <>
                  <ul className="onboarding__privacy-list">
                    {PRIVACY_ROWS.map((row) => (
                      <li className="onboarding__privacy-row" key={row.key}>
                        <span className="onboarding__privacy-text">
                          <span className="onboarding__privacy-label">{msg(row.labelKey)}</span>
                          <span className="onboarding__privacy-description">
                            {msg(row.descriptionKey)}
                          </span>
                        </span>
                        <Switch
                          checked={choices[row.key]}
                          onChange={(value) => setChoice(row.key, value)}
                          aria-label={msg(row.labelKey)}
                          data-testid={`onboarding-choice-${row.key}`}
                        />
                      </li>
                    ))}
                  </ul>
                  <p className="onboarding__privacy-note">{msg("onboarding.privacy.note")}</p>
                </>
              )}

              {step === "shortcuts" && (
                <div className="onboarding__shortcut-grid">
                  {[0, 1].map((column) => (
                    <ul className="onboarding__shortcut-column" key={column}>
                      {SHORTCUT_ROWS.slice(
                        column * SHORTCUT_COLUMN_LENGTH,
                        (column + 1) * SHORTCUT_COLUMN_LENGTH,
                      ).map((row) => (
                        <li className="onboarding__shortcut-row" key={row.labelKey}>
                          <span className="onboarding__shortcut-label">{msg(row.labelKey)}</span>
                          <span className="onboarding__shortcut-keys">
                            {row.bindingIds.map((id, index) => {
                              const b = binding(id);
                              if (!b) return null;
                              return (
                                <Fragment key={id}>
                                  {index > 0 && (
                                    <span className="onboarding__shortcut-sep" aria-hidden="true">
                                      /
                                    </span>
                                  )}
                                  <span className="onboarding__shortcut-pair">
                                    {keycapParts(b).map((part) => (
                                      <kbd className={keycapClass(part)} key={`${id}-${part}`}>
                                        {part}
                                      </kbd>
                                    ))}
                                  </span>
                                </Fragment>
                              );
                            })}
                          </span>
                        </li>
                      ))}
                    </ul>
                  ))}
                </div>
              )}
            </div>

            {/* Footer — the frames' anatomy: step furniture left (accent dots on
             * the appearance step, the full-list note on shortcuts), the button
             * row right (Back · Skip · Next/Get started — Skip is the
             * Lead-mandated quiet exit; frames Onboarding-1/-6..10 show
             * Back+Next / Next only, a documented deviation). */}
            <footer className="onboarding__footer">
              <div className="onboarding__footer-aside">
                {step === "appearance" && <AppearanceAccents />}
                {step === "shortcuts" && (
                  <p className="onboarding__footer-note">{msg("onboarding.shortcuts.more")}</p>
                )}
              </div>
              <div className="onboarding__footer-actions">
                <Button
                  onClick={back}
                  className={["onboarding__back", stepIndex === 0 && "onboarding__hidden"]
                    .filter(Boolean)
                    .join(" ")}
                >
                  {msg("onboarding.action.back")}
                </Button>
                {!isDone && (
                  <Button
                    variant="ghost"
                    onClick={skip}
                    data-testid="onboarding-skip"
                  >
                    {msg("onboarding.action.skip")}
                  </Button>
                )}
                {isDone ? (
                  <Button
                    variant="primary"
                    onClick={finish}
                    data-testid="onboarding-get-started"
                  >
                    {msg("onboarding.action.get-started")}
                  </Button>
                ) : (
                  <Button variant="primary" onClick={next} data-testid="onboarding-next">
                    {msg("onboarding.action.next")}
                  </Button>
                )}
              </div>
            </footer>
          </section>
        </AutoApplySettings>
      </SettingsProvider>
    </main>
  );
}

/** Route entry: a completed wizard never shows again (the /player gate is
 * in App.tsx; this one keeps /onboarding itself honest after completion). */
export function OnboardingEntry(): ReactElement {
  const { complete } = useOnboardingState();
  if (complete) return <Navigate to="/player" replace />;
  return <OnboardingScreen />;
}

/** App-level first-run gate: /player (and only /player, per the card) is
 * unreachable until the wizard is finished or skipped. */
export function RequireOnboarding({ children }: { children: ReactElement }): ReactElement {
  const { complete } = useOnboardingState();
  if (!complete) return <Navigate to="/onboarding" replace />;
  return children;
}
