// Onboarding wizard (card 4.F1) — first-run flow per the design exports
// (doc 08 §1.6 "onboarding ×11": welcome → … → theme+accent → shortcuts →
// finish). The design package has NO privacy artboard, so the privacy step
// copy here is an honest [PROPOSAL] placeholder built from doc 07 §5.4 —
// every string flows through i18n keys regardless (RULES).
//
// THE honest rule: the three privacy choices are opt-IN switches, all OFF,
// none pre-checked. "Skip" jumps straight to done with the defaults
// UNCHANGED — skipping is not consent either. Finishing commits whatever
// the user explicitly set, nothing more.

import { useContext, useState, type ReactElement } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Button } from "../../components/button";
import { Switch } from "../../components/form";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
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

const THEME_CARDS: ReadonlyArray<{ setting: ThemeModeSetting; labelKey: MessageKey }> = [
  { setting: "light", labelKey: "onboarding.appearance.theme.light" },
  { setting: "dark", labelKey: "onboarding.appearance.theme.dark" },
  { setting: "auto", labelKey: "onboarding.appearance.theme.system" },
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

  return (
    <main className="onboarding" data-testid="onboarding-root">
      <section
        className="onboarding__window"
        aria-labelledby="onboarding-title"
        data-testid="onboarding-window"
      >
        <header className="onboarding__header">
          <h1 className="onboarding__title" id="onboarding-title">
            {msg(STEP_TITLES[step])}
          </h1>
          <ol
            className="onboarding__progress"
            aria-label={`${msg("onboarding.progress.label")} ${stepIndex + 1} ${msg(
              "onboarding.progress.of",
            )} ${STEPS.length}`}
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
        </header>

        <div className="onboarding__step" data-testid={`onboarding-step-${step}`}>
          <p className="onboarding__body">{msg(STEP_BODIES[step])}</p>

          {step === "privacy" && (
            <>
              <ul className="onboarding__privacy-list">
                {PRIVACY_ROWS.map((row) => (
                  <li className="onboarding__privacy-row" key={row.key}>
                    <span className="onboarding__privacy-text">
                      <span className="onboarding__privacy-label">{msg(row.labelKey)}</span>
                      <p className="onboarding__privacy-description">{msg(row.descriptionKey)}</p>
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
                      `onboarding__theme-card--${card.setting === "auto" ? "auto" : card.setting}`,
                    ].join(" ")}
                    aria-pressed={theme ? theme.modeSetting === card.setting : undefined}
                    disabled={!theme}
                    onClick={() => theme?.setModeSetting(card.setting)}
                  >
                    <span className="onboarding__theme-swatch" aria-hidden="true" />
                    {msg(card.labelKey)}
                  </button>
                ))}
              </div>
              <div
                className="onboarding__accents"
                aria-label={msg("onboarding.appearance.accent")}
              >
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
            <ul className="onboarding__shortcut-list">
              <li>{msg("onboarding.shortcuts.hint-play-pause")}</li>
              <li>{msg("onboarding.shortcuts.hint-seek")}</li>
              <li>{msg("onboarding.shortcuts.hint-fullscreen")}</li>
              <li>{msg("onboarding.shortcuts.hint-more")}</li>
            </ul>
          )}
        </div>

        <footer className="onboarding__footer">
          <Button onClick={back} className={stepIndex === 0 ? "onboarding__hidden" : undefined}>
            {msg("onboarding.action.back")}
          </Button>
          <span className="onboarding__footer-group">
            {!isDone && (
              <Button onClick={skip} data-testid="onboarding-skip">
                {msg("onboarding.action.skip")}
              </Button>
            )}
            {isDone ? (
              <Button variant="primary" onClick={finish} data-testid="onboarding-get-started">
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
