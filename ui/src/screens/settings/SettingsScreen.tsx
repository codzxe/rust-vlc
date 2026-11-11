import { useCallback, useEffect, useState } from "react";
import { Navigate, useNavigate, useParams } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { Dialog } from "../../components/surface/Surface";
import { Icon } from "../../components/icon/Icon";
import type { IconName } from "../../components/icon/icons";
import { useClient } from "../../client/ClientProvider";
import { isDesktopShell } from "../../client/app-transport";
import { createConfigSettingsPersistence } from "../../client/config-settings";
import { useMsg } from "../../i18n/messages";
import type { MessageKey } from "../../i18n/messages";
import { keys as laneKeys } from "../../i18n/keys/w26.b01";
import { keys as registryKeys } from "../../i18n/keys/w26.b02";
import { SETTINGS_AREA_IDS } from "./areas";
import { SETTINGS_AREA_TITLES } from "./areas";
import { SETTINGS_SCREENS } from "./areas";
import { resolveSettingsAreaSlug } from "./areas";
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

// SettingsScreen — the W26 Figma settings window (Settings - *.svg): a
// floating 1048×865 r14 #0E0E0E@95% window over the app, not a full-page
// route skin. Grammar (tools-settings.md §D2, measured from the frames):
//   * 50px top bar — search pill 200×34 r6 (magnifier + placeholder) at the
//     left inset, the area's WINDOW TITLE (e.g. "Global Settings") at the
//     content column's left edge, ✕ close at the right inset → /player;
//   * 216px sidebar column wrapping a 200px r6 #191919@95% panel with the
//     App / Media / System group labels and 188×40 r2 rows at 48px pitch
//     (active row: raised bg + orange icon stroke 2 + bright label);
//   * 784px content column — 16px section headers, label-left /
//     control-right rows (320px controls), in-flow "Advanced Options"
//     disclosure per area bottom (SettingsSections).
// Contracts kept (SettingsScreen.test.tsx): schema-driven nav via the area
// registry's CURRENT API, cross-area search, deep-link /settings/:area with
// unknown→first redirect, dirty-state Apply/Revert bar, confirm-on-navigate
// when dirty, restart badges + notices, per-screen reset-to-default, the
// settings→ThemeProvider seam, and the desktop config-bridge persistence.

type LaneKey = keyof typeof laneKeys;
type RegistryKey = keyof typeof registryKeys;
/** Pre-registration seam: this lane's keys + the registry lane's (b02) keys —
 * both resolve identically once the Lead registers them post-wave (b19). */
const laneCatalog: Readonly<Record<string, string>> = { ...registryKeys, ...laneKeys };

/**
 * useSettingsMsg — `useMsg` extended with the settings lanes' pre-registration
 * keys (same seam as w26.a10's useShellMsg): lane keys win while unregistered;
 * after the Lead merges them into the catalog both paths return the same
 * string. Every user-visible shell string flows through here.
 */
function useSettingsMsg() {
  const m = useMsg();
  return useCallback(
    (key: MessageKey | LaneKey | RegistryKey): string => m(key as MessageKey) ?? laneCatalog[key],
    [m],
  );
}

// ---------------------------------------------------------------------------
// Sidebar structure (Settings - global.svg Frame 260/261/263): three fixed
// groups, 4/5/4 rows in the frames. The AREAS come from the registry via its
// current API (SETTINGS_AREA_IDS / SETTINGS_AREA_TITLES / SETTINGS_SCREENS —
// the b02 reshape made it exactly the 13 Figma areas); this table supplies
// the grouping + iconography the registry does not export. Registry ids not
// listed anywhere fall into the LAST group, so a future registered area can
// never be unreachable from the nav.
// ---------------------------------------------------------------------------

interface SidebarGroup {
  readonly id: "app" | "media" | "system";
  readonly labelKey: LaneKey;
  readonly areas: readonly string[];
}

const SIDEBAR_GROUPS: readonly SidebarGroup[] = [
  // App: General(global) · Interface · Privacy · Controls — 4 rows.
  { id: "app", labelKey: "settings.group.app", areas: ["global", "interface", "privacy", "controls"] },
  // Media: Playback · Video · Audio · Subtitles · Library — 5 rows.
  { id: "media", labelKey: "settings.group.media", areas: ["playback", "video", "audio", "subtitles", "library"] },
  // System: Network · Performance · Recording · Extensions — 4 rows.
  { id: "system", labelKey: "settings.group.system", areas: ["network", "performance", "recording", "extensions"] },
];

/** Area id → sidebar glyph (Icon.svg registry names; active rows re-stroke). */
const AREA_ICONS: Readonly<Record<string, IconName>> = {
  global: "settings",
  interface: "interface",
  privacy: "privacy",
  controls: "keyboard-shortcuts",
  playback: "playback",
  video: "tv",
  audio: "audio",
  subtitles: "subtitles",
  library: "lists",
  network: "network-stream",
  performance: "wrench",
  recording: "record",
  extensions: "plugins",
};

const iconOf = (area: string): IconName => AREA_ICONS[area] ?? "settings";

/** Top-bar window title: the frames' verbatim copy, registry title fallback. */
const windowTitleKey = (area: SettingsAreaId): MessageKey | LaneKey =>
  (`settings.window-title.${area}` as LaneKey) in laneCatalog
    ? (`settings.window-title.${area}` as LaneKey)
    : SETTINGS_AREA_TITLES[area];

// ---------------------------------------------------------------------------

const isAreaId = (area: string | undefined): area is SettingsAreaId =>
  area !== undefined && (SETTINGS_AREA_IDS as readonly string[]).includes(area);
/** One sidebar group's resolved rows (registry ids in group order). */
function groupRows(group: SidebarGroup, registered: readonly SettingsAreaId[]): readonly SettingsAreaId[] {
  const known = group.areas.filter((id): id is SettingsAreaId =>
    (registered as readonly string[]).includes(id),
  );
  if (group.id !== "system") return known;
  // Last group also catches registry ids no group lists (future areas).
  const listed = new Set<string>([...SIDEBAR_GROUPS.flatMap((g) => g.areas)]);
  const orphans = registered.filter((id) => !listed.has(id));
  return [...known, ...orphans];
}

function SettingsShell() {
  const m = useMsg();
  const t = useSettingsMsg();
  const navigate = useNavigate();
  const { area } = useParams<{ area: string }>();
  const settings = useSettings();
  // Card 4.E2: interface.theme draft → ThemeProvider → data-theme on <html>.
  useSettingsThemeSync();
  const [query, setQuery] = useState("");
  const [pending, setPending] = useState<{ area: SettingsAreaId } | { close: true } | null>(null);

  if (!isAreaId(area)) {
    // Deep-link canonicalisation (registry API): retired slugs (hotkeys,
    // codecs, about, advanced, effects — the b02 merges) redirect to their
    // merged home via resolveSettingsAreaSlug; unknown slugs to the first
    // area. Nothing 404s.
    const target = resolveSettingsAreaSlug(area) ?? SETTINGS_AREA_IDS[0];
    return <Navigate to={`/settings/${target}`} replace />;
  }

  // Search labels resolve through the SAME seam the UI renders with (b01 +
  // b02 lane keys pre-registration), so what matches is what the user sees.
  // Unresolved labels (mid-wave catalog gaps) read as empty — the search
  // path never crashes; an entry without a resolved label cannot match.
  const labelOf = (entry: { description: string }): string => {
    const text = t(entry.description as MessageKey);
    return typeof text === "string" ? text : "";
  };
  const matches = (entry: { description: string; featureExposed: boolean }): boolean =>
    query.trim() === "" || labelOf(entry).toLowerCase().includes(query.trim().toLowerCase());
  const searchValue: SettingsSearchContextValueLike = { query, setQuery, matches };
  const searching = query.trim() !== "";
  const hits = searching ? searchSettings(SETTINGS_AREAS, query, labelOf) : [];
  const Current = SETTINGS_SCREENS[area];

  // Dirty-gated navigation (4.B4): area switches AND the window's ✕ both
  // route through the same confirm dialog; "discard" reverts and continues.
  const requestArea = (next: SettingsAreaId) => {
    if (next === area) return;
    if (settings.dirtyCount > 0) {
      setPending({ area: next });
      return;
    }
    navigate(`/settings/${next}`);
  };
  const requestClose = () => {
    if (settings.dirtyCount > 0) {
      setPending({ close: true });
      return;
    }
    navigate("/player");
  };
  const discardAndContinue = () => {
    const next = pending;
    setPending(null);
    settings.revert(); // discard: draft ← persisted
    if (next !== null && "area" in next) navigate(`/settings/${next.area}`);
    else navigate("/player");
  };

  return (
    <SettingsSearchContext.Provider value={searchValue}>
      <main className="settings-screen" data-screen="settings">
        {/* Window-accessible name (the frames carry no "Settings" wordmark;
            the top bar shows the AREA title). Kept as a real heading so the
            screen keeps its h1 anchor. */}
        <h1 className="settings-vh">{m("settings.nav.title")}</h1>
        <div className="settings-window">
          <header className="settings-topbar">
            <label className="settings-topbar__search">
              <Icon name="search" size={15} className="settings-topbar__search-icon" />
              <input
                type="search"
                value={query}
                placeholder={m("settings.search.label")}
                aria-label={m("settings.search.label")}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
            <h2 className="settings-topbar__title">{t(windowTitleKey(area))}</h2>
            {!searching ? (
              <button
                type="button"
                className="settings-topbar__reset"
                onClick={() => settings.resetArea(area)}
                aria-label={m("settings.reset-screen")}
                title={m("settings.reset-screen")}
              >
                {m("settings.reset-screen")}
              </button>
            ) : null}
            <button
              type="button"
              className="settings-topbar__close"
              aria-label={t("settings.close")}
              title={t("settings.close")}
              onClick={requestClose}
            >
              <Icon name="close" size={14} />
            </button>
          </header>
          <div className="settings-window__body">
            <aside className="settings-sidebar" aria-label={m("settings.nav.title")}>
              <div className="settings-sidebar__panel">
                {SIDEBAR_GROUPS.map((group) => {
                  const rows = groupRows(group, SETTINGS_AREA_IDS);
                  if (rows.length === 0) return null;
                  return (
                    <section key={group.id} className="settings-sidebar__group">
                      <h3 className="settings-sidebar__group-label">{t(group.labelKey)}</h3>
                      <ul>
                        {rows.map((id) => (
                          <li key={id}>
                            <button
                              type="button"
                              className={
                                id === area
                                  ? "settings-nav__item settings-nav__item--active"
                                  : "settings-nav__item"
                              }
                              aria-current={id === area ? "page" : undefined}
                              onClick={() => requestArea(id)}
                            >
                              <Icon
                                name={iconOf(id)}
                                size={18}
                                variant={id === area ? "strong" : "default"}
                                className="settings-nav__icon"
                              />
                              <span className="settings-nav__label">
                                {t(SETTINGS_AREA_TITLES[id])}
                              </span>
                            </button>
                          </li>
                        ))}
                      </ul>
                    </section>
                  );
                })}
              </div>
            </aside>
            <div className="settings-content">
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
                          {t(SETTINGS_AREA_TITLES[hit.area.id as SettingsAreaId])} —{" "}
                          {t(hit.sectionTitle as MessageKey)}
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
          </div>
        </div>

        <Dialog open={pending !== null} onClose={() => setPending(null)} labelledBy="settings-confirm-title">
          <div className="settings-confirm">
            <h2 id="settings-confirm-title">{m("settings.confirm-leave.title")}</h2>
            <p>{m("settings.confirm-leave.body")}</p>
            <div className="settings-confirm__actions">
              <Button variant="primary" onClick={discardAndContinue}>
                {m("settings.confirm-leave.discard")}
              </Button>
              <Button variant="default" onClick={() => setPending(null)}>
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
