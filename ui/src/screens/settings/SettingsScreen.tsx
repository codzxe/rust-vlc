import { useCallback, useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
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
import { searchSettings } from "./search";
import { useSettingsMsg } from "./settingsMsg";
import { settingsLaneCatalog } from "./settingsMsg";
import type { SettingsMsgKey } from "./settingsMsg";
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
// when dirty, restart badges + notices, the settings→ThemeProvider seam, and
// the desktop config-bridge persistence. w26.f03: the top-bar reset ghost is
// GONE (e05 F10 — the per-field resets keep the reset-to-default contract);
// focus starts on the sidebar's active row and Tab wraps inside the window
// (e06 F8).
//
// w26.k12 (j12 P2-2): the search CONTEXT is gone — while a query is live the
// content column renders the cross-area results INSTEAD of the current area
// (the tested W26 contract), so an in-area filter could never execute; the
// dead context/plumbing is deleted (see search.ts + SettingsSections).
// w26.k12 (j12 P3-5): the shell's lane-seam resolver is the shared settingsMsg
// module (was this file's private useSettingsMsg copy with its own
// precedence).

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
  readonly labelKey: "settings.group.app" | "settings.group.media" | "settings.group.system";
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

/** Area id → sidebar glyph (Icon.svg registry names; active rows re-stroke).
 * w26.f03 (e05 F11): General (global) carries the sliders glyph — the frame
 * row is 3 vertical sliders + offset nubs (Settings - global.svg "Icon_3"),
 * the Icon.svg "name=preferences" sheet = the `equalizer` glyph. The gear
 * (`settings`) is the frames' MENU row glyph, not this row's. */
const AREA_ICONS: Readonly<Record<string, IconName>> = {
  global: "equalizer",
  interface: "interface",
  privacy: "privacy",
  controls: "keyboard-shortcuts",
  playback: "playback",
  video: "tv",
  audio: "audio",
  subtitles: "subtitles",
  // w26.gfix-B: the frames' Library row is the FOLDER glyph (global.svg
  // "setting category_9" = the Icon.svg folder sheet's Vector 23 shape;
  // vision-compared twice) — not the lists glyph.
  library: "folder",
  network: "network-stream",
  performance: "wrench",
  recording: "record",
  extensions: "plugins",
};

const iconOf = (area: string): IconName => AREA_ICONS[area] ?? "settings";

/** Top-bar window title: the frames' verbatim copy, registry title fallback.
 * w26.k12 (j12 P3-5): membership probe + resolution through the shared
 * settingsMsg catalog (ONE precedence rule for the whole shell). */
const windowTitleKey = (area: SettingsAreaId): SettingsMsgKey =>
  (`settings.window-title.${area}` as SettingsMsgKey) in settingsLaneCatalog
    ? (`settings.window-title.${area}` as SettingsMsgKey)
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

/** Focus-trap candidate selector (w26.f03, e06 F8) — natural tab stops inside
 * the settings window; roving-tabindex controls (segmented cells, swatches,
 * dropdown options) surface their active item at tabIndex 0.
 * w26.k12 (j12 P3-9): EXPORTED — the trap test imports the same selector so
 * the test's first/last stops can never drift from the trap's definition
 * (the old test copy omitted select/textarea/a[href]). */
export const FOCUSABLE =
  'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])';

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

  // w26.k12 (j12 P1-1): the search query must NOT survive navigation.
  // requestArea used to navigate while `searching` kept the content column
  // in cross-area results mode — the URL, window title and `aria-current`
  // moved to the target area, but the column kept rendering the STALE hits
  // and the target screen was unreachable until the pill was cleared. One
  // effect on the ROUTE PARAM clears the pill on every area change; all
  // navigation paths funnel through :area (sidebar row, the dirty-dialog's
  // discard-and-continue, deep-link/slug redirects), so one hook covers
  // them all. First mount is a no-op (the pill starts empty).
  useEffect(() => {
    setQuery("");
  }, [area]);

  // w26.f03 (e06 F8) — the settings window is a floating surface: focus
  // starts INSIDE it, on the sidebar's active area row, and Tab wraps within
  // the window (the app chrome behind it is never a tab stop while it is
  // up). Open overlays keep their own traps: the confirm Dialog renders
  // OUTSIDE this subtree, and dropdown popups handle their own Tab/Escape
  // (the handler skips already-prevented events).
  const windowRef = useRef<HTMLDivElement | null>(null);
  const activeNavRef = useRef<HTMLButtonElement | null>(null);
  // Focus lands on the ACTIVE area row when it MOUNTS (settings data is
  // async — the nav list may not exist on first paint, so a [area] effect
  // alone races it). A callback ref fires exactly at mount, including the
  // async arrival AND every area switch (the active element identity
  // changes), and never re-fires while the user types in search (same
  // element re-render ≠ remount).
  //
  // w26.gfix-B: the mount focus must not PAINT the keyboard ring at rest —
  // the frames' active row shows the #222222 fill only, and Chromium matches
  // :focus-visible for script focus before any user interaction, so the
  // focused row is marked data-mount-focus (ring suppressed, see settings.css)
  // and the FIRST keydown anywhere clears the marker — real keyboard use gets
  // the 2px accent ring back (the e2e tab audit passes through it).
  const focusActiveNav = useCallback((el: HTMLButtonElement | null): void => {
    activeNavRef.current = el;
    if (el !== null) {
      el.dataset.mountFocus = "";
      el.focus();
    }
  }, []);

  // Clears every mount-focus marker on the first keydown (capture phase —
  // runs before the focus moves, so the newly focused stop paints its ring).
  useEffect(() => {
    const clearMountFocus = (): void => {
      for (const el of document.querySelectorAll("[data-mount-focus]")) {
        el.removeAttribute("data-mount-focus");
      }
    };
    window.addEventListener("keydown", clearMountFocus, true);
    return () => window.removeEventListener("keydown", clearMountFocus, true);
  }, []);

  const onWindowKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "Tab" || event.defaultPrevented) return;
    const root = windowRef.current;
    if (root === null) return;
    const focusables = Array.from(
      root.querySelectorAll<HTMLElement>(FOCUSABLE),
    ).filter((el) => el.getAttribute("aria-hidden") !== "true");
    if (focusables.length === 0) return;
    const first = focusables[0] as HTMLElement;
    const last = focusables[focusables.length - 1] as HTMLElement;
    const active = document.activeElement;
    const inside = active instanceof Node && root.contains(active);
    if (!event.shiftKey && (!inside || active === last)) {
      event.preventDefault();
      first.focus();
    } else if (event.shiftKey && (!inside || active === first)) {
      event.preventDefault();
      last.focus();
    }
  };

  // w26.d1 — Escape closes the settings window (the frames imply dismissal:
  // Settings - *.svg are windowed with a ✕ close; VLC parity) through the
  // SAME seam as the ✕ (dirty state → the confirm dialog). Open overlays
  // (the confirm Dialog, menus, dropdown POPUPS) own Escape first — the
  // guard keeps this handler silent while any of them is up. NOTE: the
  // guard matches the Dropdown POPUP class, not every role="listbox" (the
  // shell's sidebar VirtualList is a persistent listbox, never an overlay).
  // Declared before the area-guard early return (Rules of Hooks); the ref
  // carries the fresh requestClose closure (updated post-commit below).
  const requestCloseRef = useRef<() => void>(() => {});
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      if (event.defaultPrevented) return;
      if (document.querySelector('[role="dialog"], [role="menu"], .lumen-dropdown__list') !== null) {
        return; // an open overlay's focus trap handles this key
      }
      requestCloseRef.current();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  // Dirty-gated navigation (4.B4): area switches AND the window's ✕ both
  // route through the same confirm dialog; "discard" reverts and continues.
  // Declared BEFORE the area-guard early return so the ref-updating effect
  // below can close over it unconditionally (Rules of Hooks).
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

  // w26.k12 (j12 P3-8): the Escape seam's fresh-closure write moved OUT of
  // the render path into this no-deps effect (runs after every commit, so
  // the listener always reads the latest closure — the previous render-phase
  // ref write was equivalent but unsanctioned shape).
  useEffect(() => {
    requestCloseRef.current = requestClose;
  });

  if (!isAreaId(area)) {
    // Deep-link canonicalisation (registry API): retired slugs (hotkeys,
    // codecs, about, advanced, effects — the b02 merges) redirect to their
    // merged home via resolveSettingsAreaSlug; unknown slugs to the first
    // area. Nothing 404s.
    const target = resolveSettingsAreaSlug(area) ?? SETTINGS_AREA_IDS[0];
    return <Navigate to={`/settings/${target}`} replace />;
  }

  // Search labels resolve through the SAME seam the UI renders with (the
  // shared settingsMsg catalog), so what matches is what the user sees.
  // Unresolved labels (mid-wave catalog gaps) read as empty — the search
  // path never crashes; an entry without a resolved label cannot match.
  const labelOf = (entry: { description: string }): string => {
    const text = t(entry.description as MessageKey);
    return typeof text === "string" ? text : "";
  };
  const searching = query.trim() !== "";
  const hits = searching ? searchSettings(SETTINGS_AREAS, query, labelOf) : [];
  const Current = SETTINGS_SCREENS[area];

  return (
    <main className="settings-screen" data-screen="settings">
      {/* Window-accessible name (the frames carry no "Settings" wordmark;
          the top bar shows the AREA title). Kept as a real heading so the
          screen keeps its h1 anchor. */}
      <h1 className="settings-vh">{m("settings.nav.title")}</h1>
      <div className="settings-window" ref={windowRef} onKeyDown={onWindowKeyDown}>
        <header className="settings-topbar">
          <label className="settings-topbar__search">
            <Icon name="search" size={15} className="settings-topbar__search-icon" />
            <input
              type="search"
              value={query}
              placeholder={t("settings.search.placeholder")}
              aria-label={m("settings.search.label")}
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          <h2 className="settings-topbar__title">{t(windowTitleKey(area))}</h2>
          {/* w26.f03 (e05 F10): the top bar is frame-verbatim — search pill ·
              window title · ✕ only; the extra per-screen reset ghost is
              gone (per-field resets + the provider's resetArea seam keep the
              behavior contract). */}
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
                            ref={id === area ? focusActiveNav : undefined}
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
  );
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
