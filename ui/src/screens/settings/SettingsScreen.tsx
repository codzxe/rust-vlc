import { useEffect, useState } from "react";
import { Navigate, useNavigate, useParams } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { Dialog } from "../../components/surface/Surface";
import { useClient } from "../../client/ClientProvider";
import { isDesktopShell } from "../../client/app-transport";
import { createConfigSettingsPersistence } from "../../client/config-settings";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { SETTINGS_AREA_IDS } from "./areas";
import { SETTINGS_AREA_TITLES } from "./areas";
import { SETTINGS_SCREENS } from "./areas";
import type { SettingsAreaId } from "./areas";
import { SETTINGS_AREAS, SETTINGS_ENTRIES } from "./generated/schema";
import { localStorageSettingsPersistence } from "./persistence";
import { SettingsProvider } from "./SettingsProvider";
import { useSettings } from "./SettingsProvider";
import { useSettingsThemeSync } from "../../theme/settingsThemeSync";
import { SettingsSearchContext } from "./search";
import { searchSettings } from "./search";
import { SettingField } from "./SettingField";
import "./settings.css";

// SettingsScreen — card 4.E1 framework shell (doc 07 §5): schema-generated
// navigation + screens, cross-area search, dirty-state Apply/Revert bar,
// confirm-on-navigate when dirty (4.B4 Dialog), restart-required badges and
// per-screen reset-to-default. Content modules land with 4.E2–4.E4; the
// areas layer (./areas) merges the generated registry with the hand-written
// extensions (hotkeys) and per-key special widgets, and mounts the settings→
// ThemeProvider seam (card 4.E2).

const isAreaId = (area: string | undefined): area is SettingsAreaId =>
  area !== undefined && (SETTINGS_AREA_IDS as readonly string[]).includes(area);

function SettingsShell() {
  const m = useMsg();
  const navigate = useNavigate();
  const { area } = useParams<{ area: string }>();
  const settings = useSettings();
  // Card 4.E2: interface.theme draft → ThemeProvider → data-theme on <html>.
  useSettingsThemeSync();
  const [query, setQuery] = useState("");
  const [pendingArea, setPendingArea] = useState<SettingsAreaId | null>(null);

  // Solstice v2 (brief §1): settings is an APP-tier working screen — the
  // sunset is veiled to a calm warm wash while mounted (cleaned up on
  // unmount) so the anchored panels read as work surfaces, not islands.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "calm";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  if (!isAreaId(area)) return <Navigate to={`/settings/${SETTINGS_AREA_IDS[0]}`} replace />;

  const labelOf = (entry: { description: string }): string => m(entry.description as MessageKey);
  const matches = (entry: { description: string; featureExposed: boolean }): boolean =>
    query.trim() === "" || labelOf(entry).toLowerCase().includes(query.trim().toLowerCase());
  const searchValue: SettingsSearchContextValueLike = { query, setQuery, matches };
  const searching = query.trim() !== "";
  const hits = searching ? searchSettings(SETTINGS_AREAS, query, labelOf) : [];
  const Current = SETTINGS_SCREENS[area];

  const requestArea = (next: SettingsAreaId) => {
    if (next === area) return;
    if (settings.dirtyCount > 0) {
      setPendingArea(next);
      return;
    }
    navigate(`/settings/${next}`);
  };

  return (
    <SettingsSearchContext.Provider value={searchValue}>
      <main className="settings-screen" data-screen="settings">
        <aside className="settings-screen__nav" aria-label={m("settings.nav.title")}>
          <h1 className="settings-screen__title">{m("screen.settings")}</h1>
          <ul>
            {SETTINGS_AREA_IDS.map((id) => (
              <li key={id}>
                <button
                  type="button"
                  className={id === area ? "settings-nav__item settings-nav__item--active" : "settings-nav__item"}
                  aria-current={id === area ? "page" : undefined}
                  onClick={() => requestArea(id)}
                >
                  {m(SETTINGS_AREA_TITLES[id])}
                </button>
              </li>
            ))}
          </ul>
        </aside>
        <div className="settings-screen__body">
          {/* Area heading — anchors the content column (v2 audit: the body
              read as a floating toolbar over a void; the heading + shared
              grid give the panel a top edge to align with the nav rail). */}
          <h2 className="settings-screen__area-title">{m(SETTINGS_AREA_TITLES[area])}</h2>
          <div className="settings-screen__toolbar">
            <label className="settings-screen__search">
              <span className="settings-screen__search-label">{m("settings.search.label")}</span>
              <input
                type="search"
                value={query}
                placeholder={m("settings.search.label")}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
            {!searching ? (
              <button
                type="button"
                className="settings-screen__reset"
                onClick={() => settings.resetArea(area)}
                aria-label={m("settings.reset-screen")}
                title={m("settings.reset-screen")}
              >
                {m("settings.reset-screen")}
              </button>
            ) : null}
          </div>

          {searching ? (
            hits.length === 0 ? (
              <p className="settings-screen__empty" data-testid="settings-search-empty">
                {m("settings.search.no-results")}
              </p>
            ) : (
              <div className="settings-screen__results" data-testid="settings-search-results">
                {hits.map((hit) => (
                  <section key={`${hit.area.id}/${hit.sectionTitle}`} className="settings-section">
                    <h2 className="settings-section__title">
                      {m(SETTINGS_AREA_TITLES[hit.area.id as SettingsAreaId])} — {m(hit.sectionTitle as MessageKey)}
                    </h2>
                    {hit.entries.map((entry) => (
                      <SettingField key={entry.key} entry={entry} />
                    ))}
                  </section>
                ))}
              </div>
            )
          ) : (
            <Current />
          )}

          {settings.restartPending.length > 0 ? (
            <p className="settings-screen__restart" data-testid="settings-restart-notice" role="status">
              {m("settings.restart-applied")}
            </p>
          ) : null}

          {settings.dirtyCount > 0 ? (
            <div className="settings-applybar" data-testid="settings-applybar">
              <span className="settings-applybar__count">
                {m("settings.unsaved-changes")} ({settings.dirtyCount})
              </span>
              <Button variant="primary" onClick={() => settings.apply()}>
                {m("settings.apply")}
              </Button>
              <Button variant="default" onClick={() => settings.revert()}>
                {m("settings.revert")}
              </Button>
            </div>
          ) : null}
        </div>

        <Dialog open={pendingArea !== null} onClose={() => setPendingArea(null)} labelledBy="settings-confirm-title">
          <div className="settings-confirm">
            <h2 id="settings-confirm-title">{m("settings.confirm-leave.title")}</h2>
            <p>{m("settings.confirm-leave.body")}</p>
            <div className="settings-confirm__actions">
              <Button
                variant="primary"
                onClick={() => {
                  const next = pendingArea;
                  setPendingArea(null);
                  settings.revert(); // discard: draft ← persisted
                  if (next !== null) navigate(`/settings/${next}`);
                }}
              >
                {m("settings.confirm-leave.discard")}
              </Button>
              <Button variant="default" onClick={() => setPendingArea(null)}>
                {m("settings.confirm-leave.stay")}
              </Button>
            </div>
          </div>
        </Dialog>
      </main>
    </SettingsSearchContext.Provider>
  );
}

/** Minimal structural type for the context value (keeps search.ts import-free). */
interface SettingsSearchContextValueLike {
  readonly query: string;
  setQuery(query: string): void;
  matches(entry: { description: string; featureExposed: boolean }): boolean;
}

/** Route element: /settings/:area */
export function SettingsScreen() {
  // App-layer wiring (issue #3 — the 4.H1 handoff d): in the desktop
  // shell, Apply round-trips the REAL engine config over the client
  // bridge (config.set-setting / config.reset-setting; hydration via
  // config.get-setting) and `config.setting-changed` events keep the
  // store live. The browser dev shell keeps the localStorage seam —
  // the demo transport honestly answers feature-not-exposed for
  // config.*.
  const m = useMsg();
  const client = useClient();
  const [config] = useState(() =>
    isDesktopShell()
      ? createConfigSettingsPersistence(client, SETTINGS_ENTRIES)
      : null,
  );
  const [hydrated, setHydrated] = useState(config === null);
  useEffect(() => {
    if (config === null) return;
    let cancelled = false;
    void config.hydrate().finally(() => {
      if (!cancelled) setHydrated(true);
    });
    return () => {
      cancelled = true;
      config.dispose();
    };
  }, [config]);

  if (config !== null && !hydrated) {
    // Hydration gate: never render schema defaults over engine values
    // that have not been read yet (one honest loading pass).
    return (
      <main className="settings-screen" data-screen="settings">
        <p className="settings-screen__empty" data-testid="settings-hydrating">
          {m("settings.loading")}
        </p>
      </main>
    );
  }
  return (
    <SettingsProvider
      persistence={config ?? localStorageSettingsPersistence}
      subscribe={(listener) => client.subscribe(listener)}
    >
      <SettingsShell />
    </SettingsProvider>
  );
}
