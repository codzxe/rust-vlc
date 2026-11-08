// Onboarding wizard (card 4.F1) — Solstice redesign (2026-11-08): the Keyvox
// golden-hour scene. While mounted the screen flips the app backdrop to
// full-strength sunset (`html[data-backdrop="vivid"]`, removed on unmount),
// the step indicator and a decorative keycap strip sit directly ON the
// gradient, and every step's content floats on a single washi-taped
// sticky-note card (.solstice-note) that rises in on each step change.
// The shortcut tour is built from the REAL player keymap (keymaps/default.toml
// via the generated bindings module) — keycaps are data, not hand-typed copy.
//
// THE honest rule (unchanged by the redesign): the three privacy choices are
// opt-IN switches, all OFF, none pre-checked. "Skip" jumps straight to done
// with the defaults UNCHANGED — skipping is not consent either. Finishing
// commits whatever the user explicitly set, nothing more.

import { useContext, useEffect, useState, type ReactElement } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Button } from "../../components/button";
import { Switch } from "../../components/form";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { HOTKEY_BINDINGS } from "../settings/areas/hotkeys/generated/bindings";
import type { HotkeyBinding } from "../settings/areas/hotkeys/generated/bindings";
import { ThemeContext } from "../../theme/ThemeProvider";
import type { ThemeModeSetting } from "../../theme/ThemeProvider";
import { ACCENTS, type AccentId } from "../../tokens/generated/tokens";
import { useOnboardingActions, useOnboardingState } from "./onboardingState";
import type { OnboardingChoiceKey } from "./onboardingState";
import "./onboarding.css";

/** Wizard steps in order (design flow + the card's privacy step). */
const STEPS = ["welcome", "privacy", "appearance", "shortcuts", "done"] as const;
type StepId = (typeof STEPS)[number];

const STEP_TITLES: Record<StepId, MessageKey> = {
  welcome: "onboarding.welcome.title",
  privacy: "onboarding.privacy.title",
  appearance: "onboarding.appearance.title",
  shortcuts: "onboarding.shortcuts.title",
  done: "onboarding.done.title",
};

const STEP_BODIES: Record<StepId, MessageKey> = {
  welcome: "onboarding.welcome.body",
  privacy: "onboarding.privacy.body",
  appearance: "onboarding.appearance.body",
  shortcuts: "onboarding.shortcuts.body",
  done: "onboarding.done.body",
};

/** Washi-tape pin position per step — the card re-tapes itself as it moves. */
const STEP_TAPE: Record<StepId, "" | "left" | "right"> = {
  welcome: "",
  privacy: "left",
  appearance: "right",
  shortcuts: "left",
  done: "right",
};

/** Hero steps center their content; working steps read as notebook lines. */
const CENTERED_STEPS: ReadonlySet<StepId> = new Set(["welcome", "done"]);

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

// --- the REAL keymap (keymaps/default.toml via the 4.E2 generated module) ---

/** Binding lookup by dotted id (e.g. "navigation.toggle-play-pause"). */
const BINDING_BY_ID: ReadonlyMap<string, HotkeyBinding> = new Map(
  HOTKEY_BINDINGS.map((binding) => [binding.id, binding] as const),
);

/**
 * Keymap spellings → keycap glyphs. Arrow-key spellings render as arrow
 * symbols on the caps (the same artwork-not-copy convention as the ON/OFF
 * toggle glyphs); every other spelling shows verbatim keymap data.
 */
const KEYCAP_GLYPHS: Readonly<Record<string, string>> = {
  Down: "↓",
  Left: "←",
  Right: "→",
  Up: "↑",
};

const keycapText = (binding: HotkeyBinding): string =>
  KEYCAP_GLYPHS[binding.key] ?? binding.key;

/** The shortcut tour: one pre-seeded hint per row, keycaps from the keymap. */
const SHORTCUT_TOUR: ReadonlyArray<{
  hint: MessageKey;
  bindingIds: readonly string[];
}> = [
  { hint: "onboarding.shortcuts.hint-play-pause", bindingIds: ["navigation.toggle-play-pause"] },
  {
    hint: "onboarding.shortcuts.hint-seek",
    bindingIds: ["navigation.seek-backward", "navigation.seek-forward"],
  },
  { hint: "onboarding.shortcuts.hint-volume", bindingIds: ["audio.volume-up", "audio.volume-down"] },
  { hint: "onboarding.shortcuts.hint-mute", bindingIds: ["audio.mute"] },
  { hint: "onboarding.shortcuts.hint-fullscreen", bindingIds: ["video.fullscreen"] },
  { hint: "onboarding.shortcuts.hint-playlist", bindingIds: ["playlist.queue-toggle"] },
];

/** The decorative bottom strip: Space / ← / → / F / M straight from the map. */
const KEYCAP_STRIP: readonly string[] = [
  "navigation.toggle-play-pause",
  "navigation.seek-backward",
  "navigation.seek-forward",
  "video.fullscreen",
  "audio.mute",
];

const stripBindings: readonly HotkeyBinding[] = KEYCAP_STRIP.map((id) =>
  BINDING_BY_ID.get(id),
).filter((binding): binding is HotkeyBinding => binding !== undefined);

export function OnboardingScreen(): ReactElement {
  const msg = useMsg();
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

  // The Keyvox scene needs the full-strength sunset: drop the mode veil for
  // as long as this screen is mounted, and ONLY that long — leaving it on
  // would strip the veil from the whole app.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "vivid";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

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

  return (
    <main className="onboarding" data-testid="onboarding-root">
      {/* Step indicator sits ON the gradient, top-left. The visible caption
       * is aria-hidden because the progress list below already carries the
       * exact same words as its accessible name (announced once, not twice). */}
      <div className="onboarding__stage-meta">
        <p className="onboarding__stepcount" aria-hidden="true">
          {stepLabel}
        </p>
        <ol
          className="onboarding__progress"
          aria-label={stepLabel}
          data-testid="onboarding-progress"
        >
          {STEPS.map((id, index) => (
            <li
              key={id}
              className={[
                "onboarding__progress-bar",
                index < stepIndex && "onboarding__progress-bar--done",
                index === stepIndex && "onboarding__progress-bar--current",
              ]
                .filter(Boolean)
                .join(" ")}
              aria-current={index === stepIndex ? "step" : undefined}
            />
          ))}
        </ol>
      </div>

      <div className="onboarding__note-wrap">
        {/* The single sticky-note card. Washi tape re-pins per step; the
         * step content is keyed on the index so .solstice-rise replays on
         * every step change. */}
        <section
          className="onboarding__note solstice-note"
          data-tape={STEP_TAPE[step]}
          aria-labelledby="onboarding-title"
          data-testid="onboarding-window"
        >
          <div
            key={stepIndex}
            className={[
              "onboarding__step",
              "solstice-rise",
              CENTERED_STEPS.has(step) && "onboarding__step--center",
            ]
              .filter(Boolean)
              .join(" ")}
            data-testid={`onboarding-step-${step}`}
          >
            {step === "welcome" && (
              <span className="onboarding__sun solstice-sun" aria-hidden="true" />
            )}

            <h1 className="onboarding__title" id="onboarding-title">
              {msg(STEP_TITLES[step])}
            </h1>
            <p className="onboarding__body">{msg(STEP_BODIES[step])}</p>

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

            {step === "appearance" && (
              <>
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
                <p className="onboarding__group-label">{msg("onboarding.appearance.accent")}</p>
                <div className="onboarding__accents">
                  {/* Per-accent raw values are not tokenized for direct
                      consumption (they live under :root[data-accent=…] and
                      redefine --color-accent). So every swatch renders with
                      the tinted token and the LIVE accent token fills the
                      selected one — the ring is the live preview. */}
                  {ACCENTS.map((accent) => (
                    <button
                      type="button"
                      key={accent}
                      className="onboarding__accent"
                      aria-pressed={theme ? theme.accent === accent : undefined}
                      aria-label={msg(ACCENT_LABEL_KEYS[accent])}
                      disabled={!theme}
                      onClick={() => theme?.setAccent(accent)}
                      data-testid={`onboarding-accent-${accent}`}
                    />
                  ))}
                </div>
              </>
            )}

            {step === "shortcuts" && (
              <>
                <ul className="onboarding__shortcut-list">
                  {SHORTCUT_TOUR.map((row) => (
                    <li className="onboarding__shortcut-row" key={row.hint}>
                      <span className="onboarding__shortcut-label">{msg(row.hint)}</span>
                      <span className="onboarding__shortcut-keys">
                        {row.bindingIds
                          .map((id) => BINDING_BY_ID.get(id))
                          .filter((binding): binding is HotkeyBinding => binding !== undefined)
                          .map((binding) => (
                            <kbd className="solstice-key solstice-key--sm" key={binding.id}>
                              {keycapText(binding)}
                            </kbd>
                          ))}
                      </span>
                    </li>
                  ))}
                </ul>
                <hr className="solstice-hr" />
                <p className="onboarding__shortcut-more">{msg("onboarding.shortcuts.hint-more")}</p>
              </>
            )}
          </div>

          <hr className="solstice-hr onboarding__rule" />

          <footer className="onboarding__footer">
            <Button
              onClick={back}
              className={[
                "onboarding__back",
                stepIndex === 0 && "onboarding__hidden",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              {msg("onboarding.action.back")}
            </Button>
            <span className="onboarding__footer-group">
              {!isDone && (
                <button
                  type="button"
                  className="onboarding__skip"
                  onClick={skip}
                  data-testid="onboarding-skip"
                >
                  {msg("onboarding.action.skip")}
                </button>
              )}
              {isDone ? (
                <Button
                  variant="primary"
                  size="lg"
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
            </span>
          </footer>
        </section>
      </div>

      {/* Ambient keycap strip on the gradient — pure scene-setting (the
       * shortcuts step teaches these keys accessibly), so it is decorative. */}
      <div className="onboarding__keys" aria-hidden="true">
        {stripBindings.map((binding) => (
          <span className="onboarding__keys-group" key={binding.id}>
            <span className="solstice-key solstice-key--sm">{keycapText(binding)}</span>
            <span className="onboarding__keys-label">{msg(binding.labelMsgid as MessageKey)}</span>
          </span>
        ))}
      </div>
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
