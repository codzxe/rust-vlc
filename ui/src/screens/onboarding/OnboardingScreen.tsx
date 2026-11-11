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
//     step live-previews through ThemeProvider, the playback/subtitles
//     steps write the REAL settings schema (SettingsProvider → the shared
//     overrides store), the media step opens through the ONE open-path
//     funnel (client/open-path.ts), and the shortcut tour's keycaps are
//     the REAL keymap (keymaps/default.toml via the generated bindings) —
//     data, never hand-typed copy;
//   · hosts without a ThemeProvider render the appearance controls inert
//     (disabled, not dead controls lying about their effect).
//
// The Solstice stage (sunset backdrop tier, washi-taped note, keycap strip,
// Nunito) is fully retired; this screen paints tokens only.

import { Fragment, useCallback, useContext, useEffect, useRef, useState } from "react";
import type { FormEvent, ReactElement, ReactNode } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Button } from "../../components/button";
import { NumberField, SegmentedControl, Switch, TextField } from "../../components/form";
import { useClient } from "../../client/ClientProvider";
import { openPathRequest } from "../../client/open-path";
import { parseSourceInput } from "../open-media/OpenMediaDialog";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.a12";
import { HOTKEY_BINDINGS } from "../settings/areas/hotkeys/generated/bindings";
import type { HotkeyBinding } from "../settings/areas/hotkeys/generated/bindings";
import {
  SETTING_PLAYBACK_DEFAULT_RATE,
  SETTING_PLAYBACK_PREFERRED_SUBTITLE_LANGUAGES,
  SETTING_PLAYBACK_START_PAUSED,
  SETTING_PLAYBACK_SUBTITLE_MODE,
} from "../settings/generated/schema";
import { SettingsProvider, useSettings } from "../settings/SettingsProvider";
import { ThemeContext } from "../../theme/ThemeProvider";
import type { ThemeModeSetting } from "../../theme/ThemeProvider";
import { type AccentId } from "../../tokens/generated/tokens";
import { useOnboardingActions, useOnboardingState } from "./onboardingState";
import type { OnboardingChoiceKey } from "./onboardingState";
import "./onboarding.css";

// ---------------------------------------------------------------------------
// Lane i18n seam (contract §4.5) — copy with no catalog entry yet
// ---------------------------------------------------------------------------

/** Keys owned by this lane (registered into messages.ts by the Lead). */
type LaneKey = keyof typeof laneKeys;
const laneCatalog: Readonly<Record<string, string>> = laneKeys;

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
};

const keycapParts = (b: HotkeyBinding): readonly string[] =>
  b.key.split("+").map((part) => KEYCAP_GLYPHS[part] ?? part);

/** The tour rows — labels reuse the REAL hotkey catalog wherever a row has
 * one binding; pair rows (two bindings, one row) get a lane label. */
const SHORTCUT_ROWS: ReadonlyArray<{
  labelKey: MessageKey | LaneKey;
  bindingIds: readonly string[];
}> = [
  { labelKey: "hotkeys.play-pause", bindingIds: ["navigation.toggle-play-pause"] },
  {
    labelKey: "onboarding.shortcuts.seek",
    bindingIds: ["navigation.seek-backward", "navigation.seek-forward"],
  },
  {
    labelKey: "onboarding.shortcuts.item",
    bindingIds: ["navigation.prev-track", "navigation.next-track"],
  },
  {
    labelKey: "onboarding.shortcuts.frame",
    bindingIds: ["navigation.frame-prev", "navigation.frame-next"],
  },
  { labelKey: "hotkeys.subtitle-track-cycle", bindingIds: ["subtitles.subtitle-track-cycle"] },
  { labelKey: "hotkeys.audio-track-cycle", bindingIds: ["audio.audio-track-cycle"] },
  { labelKey: "hotkeys.fullscreen", bindingIds: ["video.fullscreen"] },
  {
    labelKey: "onboarding.shortcuts.volume",
    bindingIds: ["audio.volume-up", "audio.volume-down"],
  },
  { labelKey: "hotkeys.queue-toggle", bindingIds: ["playlist.queue-toggle"] },
  { labelKey: "hotkeys.file-browser-toggle", bindingIds: ["playlist.file-browser-toggle"] },
  { labelKey: "hotkeys.snapshot", bindingIds: ["video.snapshot"] },
  { labelKey: "hotkeys.record", bindingIds: ["video.record"] },
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
 * Auto-apply wrapper: wizard rows commit the moment they change (the step
 * copy says so). Mounted inside a fresh SettingsProvider whose persistence
 * is the REAL settings overrides store, so Settings reads the same values
 * later; nothing is staged behind an Apply bar the wizard never shows.
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
      <form className="onboarding__open-form" onSubmit={openMediaPath}>
        <TextField
          label={msg("open.path.label")}
          hideLabel
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
          {msg("open.action")}
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
                  <kbd className="onboarding__key" key={part}>
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
  // Optional theme access (settingsThemeSync pattern): the appearance step
  // live-previews through ThemeProvider when a host provides it and renders
  // inert (no dead controls lying about their effect) when not.
  const theme = useContext(ThemeContext);
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

          {step === "appearance" && (
            <div className="onboarding__theme-cards">
              {THEME_CARDS.map((card) => (
                <button
                  type="button"
                  key={card.setting}
                  className={[
                    "onboarding__theme-card",
                    `onboarding__theme-card--${card.choiceId}`,
                  ].join(" ")}
                  aria-pressed={theme ? theme.modeSetting === card.setting : undefined}
                  disabled={!theme}
                  onClick={() => theme?.setModeSetting(card.setting)}
                  data-testid={`onboarding-choice-${card.choiceId}`}
                >
                  <span className="onboarding__theme-swatch" aria-hidden="true" />
                  <span className="onboarding__theme-label">{msg(card.labelKey)}</span>
                </button>
              ))}
            </div>
          )}

          {step === "playback" && (
            <SettingsProvider>
              <AutoApplySettings>
                <PlaybackControls />
              </AutoApplySettings>
            </SettingsProvider>
          )}

          {step === "subtitles" && (
            <SettingsProvider>
              <AutoApplySettings>
                <SubtitleControls />
              </AutoApplySettings>
            </SettingsProvider>
          )}

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
                                  <kbd className="onboarding__key" key={`${id}-${part}`}>
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
         * row right (Back · Skip · Next/Get started). */}
        <footer className="onboarding__footer">
          <div className="onboarding__footer-aside">
            {step === "appearance" && (
              <div
                className="onboarding__accents"
                role="radiogroup"
                aria-label={msg("onboarding.appearance.accent")}
                data-testid="onboarding-accents"
              >
                {/* Per-accent raw values are not tokenized for direct
                 * consumption (they live under :root[data-accent=…] and
                 * redefine --color-accent), so the dot primaries are the
                 * frame-measured set (see onboarding.css) and the LIVE
                 * accent token ring is the selected-state preview. */}
                {ACCENT_ORDER.map((accent) => (
                  <button
                    type="button"
                    key={accent}
                    className="onboarding__accent"
                    data-accent={accent}
                    aria-pressed={theme ? theme.accent === accent : undefined}
                    aria-label={msg(ACCENT_LABEL_KEYS[accent])}
                    disabled={!theme}
                    onClick={() => theme?.setAccent(accent)}
                    data-testid={`onboarding-accent-${accent}`}
                  />
                ))}
              </div>
            )}
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
