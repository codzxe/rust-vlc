// AppShell tests — the W26 shell chrome composition (lane w26.a10):
// banner title bar (Player.svg topbar anatomy), icon rail, playlists
// sidebar (sidebar.svg — since w26.f08 backed by the REAL playlistsStore,
// e09-F5), the Panel/.lumen-pane adoption (w26.a08), the open-media event
// seam, the create-playlist flow (same NewPlaylistDialog as /playlist),
// window-control in-app actions, and the narrow-viewport auto-collapse.
// DOM-shape + behavior assertions; visual fidelity is proven by the
// dev-shots + VLM loop (lane log).
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { createPortal } from "react-dom";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import { OPEN_MEDIA_EVENT } from "../../screens/open-media";
import { PLAYER_TOGGLE_SIDEBAR_EVENT } from "../../screens/player";
// w26.f08 (e09-F5): the sidebar rows read the shared saved-playlists store.
import { resetPlaylistsForTests, type SavedPlaylist } from "../../screens/playlist/playlistsStore";
import { AppShell } from "./AppShell";
// w26.k05 (j05 P2-2): the strip-portal seam now has a runtime test — the
// consumer imports the context directly, exactly like PlayerScreen does.
import { useAppShellStrip } from "./stripContext";

/** The shell needs a client (open-media + window-close command seam). */
const renderShell = (
  ui: React.ReactElement = <AppShell><p>screen</p></AppShell>,
  initialEntry = "/player",
  /** Seed a stored OPEN preference so anatomy tests see the open sidebar
   * regardless of the route-aware default (w26.g03 P1-1: /player now
   * defaults CLOSED). Default-behavior tests pass false. */
  seedOpenPreference = true,
) => {
  // Seed ONLY when no preference exists — persistence tests that collapse
  // and re-mount must keep their stored "closed" (the seed never fights the
  // seam the test is proving).
  if (seedOpenPreference && window.localStorage.getItem("lumen.shell.sidebar.v1") === null) {
    window.localStorage.setItem("lumen.shell.sidebar.v1", "open");
  }
  return render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[initialEntry]}>{ui}</MemoryRouter>
    </ClientProvider>,
  );
};

beforeEach(() => {
  // The sidebar persists openness in localStorage — start every test clean.
  window.localStorage.clear();
  // The sidebar rows read the playlists STORE (w26.f08): re-hydrate it from
  // the now-clean localStorage so every test starts on the seed fixture
  // (Cool Playlist / Cool Songs / Anime to Watch — the frame rows).
  resetPlaylistsForTests();
});

afterEach(() => {
  vi.restoreAllMocks();
});

/** Seeds the playlists store via its persisted shape (lumen.playlists.v1). */
function seedPlaylistsStore(playlists: SavedPlaylist[]): void {
  window.localStorage.setItem("lumen.playlists.v1", JSON.stringify({ playlists }));
  resetPlaylistsForTests();
}

/**
 * The shell's banner. jsdom's role engine lacks the HTML-AAM scoping
 * exception, so the sidebar's internal <header> (inside the nav) also
 * surfaces as a banner — filter to the UNSCOPED one (same discipline as
 * PlayerScreen.test.tsx).
 */
function getShellBanner(): HTMLElement {
  const banner = screen
    .queryAllByRole("banner")
    .find((element) => element.closest("nav") === null);
  expect(banner).toBeDefined();
  return banner as HTMLElement;
}

/** The sidebar nav — queried with `hidden: true` so the collapsed state
 * (the `hidden` attribute removes it from the a11y tree) stays assertable;
 * the name is matched on the aria-label attribute because a hidden
 * element computes no accessible name in the testing-library engine. */
function getSidebar(): HTMLElement {
  const sidebar = screen
    .queryAllByRole("navigation", { hidden: true })
    .find((element) => element.getAttribute("aria-label") === "Playlists");
  expect(sidebar).toBeDefined();
  return sidebar as HTMLElement;
}

describe("AppShell — title bar (Player.svg topbar, banner landmark)", () => {
  it("renders the banner with the full topbar anatomy: app dot, sidebar toggle, breadcrumb, centered wordmark, window controls", () => {
    const { container } = renderShell();
    const banner = getShellBanner();
    expect(banner).toHaveClass("app-shell__banner");
    // orange app dot (decorative)
    expect(container.querySelector(".app-shell__appdot")).not.toBeNull();
    // sidebar-toggle wired to the sidebar region
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    expect(toggle).toHaveAttribute("aria-controls", "lumen-app-sidebar");
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    // breadcrumb carries the current section (Player)
    expect(within(banner).getByText("Player")).toBeInTheDocument();
    // centered wordmark
    expect(within(banner).getByText("Lumen")).toHaveClass("app-shell__wordmark");
    // window controls group (in-app actions)
    const group = within(banner).getByRole("group", { name: "Window controls" });
    expect(within(group).getByRole("button", { name: "Minimize to miniplayer" })).toBeInTheDocument();
    expect(within(group).getByRole("button", { name: "Toggle fullscreen" })).toBeInTheDocument();
    expect(within(group).getByRole("button", { name: "Stop and return to the player" })).toBeInTheDocument();
  });

  it("badges the chrome while on the demo transport", () => {
    renderShell(<AppShell demo><p>screen</p></AppShell>);
    expect(screen.getByText("Demo")).toBeInTheDocument();
  });

  it("does not badge the chrome on a real transport", () => {
    renderShell();
    expect(screen.queryByText("Demo")).toBeNull();
  });
});

describe("AppShell — icon rail (Player.svg rail, Panel edges adoption)", () => {
  it("renders the nav landmarks with square-pane chrome + hairline edges (w26.a08 Panel)", () => {
    renderShell();
    const rail = screen.getByRole("navigation", { name: "Primary navigation" });
    // Panel mounts .lumen-pane; the rail is the panel tone with the end hairline
    expect(rail).toHaveClass("lumen-pane", "app-shell__rail");
    expect(rail).toHaveAttribute("data-edges", "end");
    expect(rail).not.toHaveAttribute("data-tone");
    // the sidebar is the DEEP tone (sidebar.svg #0E0E0E) with its own hairline
    const sidebar = screen.getByRole("navigation", { name: "Playlists" });
    expect(sidebar).toHaveClass("lumen-pane", "app-shell__sidebar");
    expect(sidebar).toHaveAttribute("data-tone", "deep");
    expect(sidebar).toHaveAttribute("data-edges", "end");
  });

  it("exposes every rail stop with an accessible name; the current route is the active stop", () => {
    renderShell();
    const rail = screen.getByRole("navigation", { name: "Primary navigation" });
    for (const name of ["Player", "Playlist", "Media info", "Discovery", "Shortcuts", "Settings"]) {
      expect(within(rail).getByRole("link", { name })).toBeInTheDocument();
    }
    const active = within(rail).getByRole("link", { name: "Player" });
    expect(active).toHaveClass("app-shell__rail-link--active");
    // every stop is a ≥44px hit frame (IconWrapper lg — W26 contract)
    for (const link of within(rail).getAllByRole("link")) {
      const hit = link.querySelector(".lumen-icon-wrapper--lg");
      expect(hit).not.toBeNull();
    }
    // the pinned foot stop opens the shell's open-media dialog
    expect(within(rail).getByRole("button", { name: "Add media" })).toBeInTheDocument();
  });

  it("opens the open-media dialog from the pinned rail stop", () => {
    renderShell();
    fireEvent.click(screen.getByRole("button", { name: "Add media" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("listens for the shell-wide lumen:open-media event (the ONE listener contract)", () => {
    renderShell();
    act(() => {
      window.dispatchEvent(new Event(OPEN_MEDIA_EVENT));
    });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});

describe("AppShell — playlists sidebar (sidebar.svg, store-backed since w26.f08)", () => {
  it("defaults the sidebar CLOSED on /player (one-sidebar rule, w26.g03 P1-1)", () => {
    renderShell(<AppShell><p>screen</p></AppShell>, "/player", false);
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  it("defaults the sidebar OPEN on non-player routes (w26.g03 P1-1 route-aware default)", () => {
    renderShell(<AppShell><p>screen</p></AppShell>, "/playlist", false);
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
  });

  it("w26.i20 (h20 flow notes): defaults the sidebar CLOSED on /player/music — the music dock is the one open column", () => {
    // Player - Music-1.svg composes the 377px playlist column as the player's
    // OWN left dock; the shell's playlists sidebar starts closed so the two
    // never stack (one-sidebar rule, extended to the whole player family).
    renderShell(<AppShell><p>screen</p></AppShell>, "/player/music", false);
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  it("w26.i20 (i04 h06-#5 durable fix): defaults the sidebar CLOSED on /miniplayer — the mini card is the whole surface", () => {
    // Miniplayer.png floats a rail-less card; live keeps the shell RAIL (the
    // documented w26 one-shell ruling, App.tsx flow map) but variants.css
    // suppresses the playlists column while the mini variant is up — so the
    // route-aware default is CLOSED and the toggle's aria-expanded stays
    // honest (it used to read "open" over a display:none column — i04's
    // logged ask, closed here).
    renderShell(<AppShell><p>screen</p></AppShell>, "/miniplayer", false);
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(getSidebar()).toHaveAttribute("hidden");
  });

  it("w26.i20: route change INTO /miniplayer collapses the open sidebar (session-only, honest aria)", () => {
    // The real user path to the mini window: the title-bar minimize control.
    // The collapse is session-only — nothing is persisted, so the stored
    // preference (had there been one) survives for other routes.
    renderShell(<AppShell><p>screen</p></AppShell>, "/playlist", false);
    expect(getSidebar()).not.toHaveAttribute("hidden");
    fireEvent.click(screen.getByRole("button", { name: "Minimize to miniplayer" }));
    expect(getShellBanner().textContent).toContain("Miniplayer");
    expect(getSidebar()).toHaveAttribute("hidden");
    expect(window.localStorage.getItem("lumen.shell.sidebar.v1")).toBeNull();
  });

  it("w26.k05 (j05 P1-1): ON /miniplayer the toggle is an honest no-op — no aria flip, no un-hide, no phantom persistence", () => {
    // j05's live repro (the exact gap the old suite missed — it covered the
    // route-entry collapse but not the state while RESIDENT on the route):
    // the mini variant suppresses the column via variants.css :has(), yet
    // the toggle still flipped aria-expanded to true, dropped `hidden`, and
    // persisted "open" — a dead control lying to AT with a preference
    // stored for a toggle that did nothing. The gate keeps every signal
    // honest; this regression test pins all three.
    renderShell(<AppShell><p>screen</p></AppShell>, "/miniplayer", false);
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    const sidebar = getSidebar();
    // pre-toggle (the review's live probe): collapsed + suppressed
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(sidebar).toHaveAttribute("hidden");
    fireEvent.click(toggle);
    // post-toggle: STILL collapsed — aria never claimed the suppressed column…
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(sidebar).toHaveAttribute("hidden");
    // …and nothing was persisted for the no-op (no phantom "open")
    expect(window.localStorage.getItem("lumen.shell.sidebar.v1")).toBeNull();
    // a second click keeps every signal honest (no ratchet, no flicker)
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(sidebar).toHaveAttribute("hidden");
    expect(window.localStorage.getItem("lumen.shell.sidebar.v1")).toBeNull();
  });

  it("w26.k05 (j05 P1-1): a stored OPEN preference never surfaces over the suppressed /miniplayer column (and survives it untouched)", () => {
    // Deep-link carrying the preference another route stored: the column is
    // display:none on /miniplayer (variants.css :has()), so the honest
    // initial state is collapsed — loadSidebarOpen now checks the mini
    // route BEFORE the stored preference, so not even the first committed
    // frame claims open. The stored preference itself is read-only here and
    // must survive untouched for the routes where the column is real.
    window.localStorage.setItem("lumen.shell.sidebar.v1", "open");
    renderShell(<AppShell><p>screen</p></AppShell>, "/miniplayer", false);
    expect(screen.getByRole("button", { name: "Toggle playlists sidebar" }))
      .toHaveAttribute("aria-expanded", "false");
    expect(getSidebar()).toHaveAttribute("hidden");
    // the preference survives the visit (loadSidebarOpen only reads)…
    expect(window.localStorage.getItem("lumen.shell.sidebar.v1")).toBe("open");
    // …and the gated toggle still persists nothing over the suppressed column
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    expect(window.localStorage.getItem("lumen.shell.sidebar.v1")).toBe("open");
  });

  it("renders the header, meta row, create row and the store rows through the list primitive", () => {
    renderShell();
    const sidebar = getSidebar();
    // header controls
    expect(within(sidebar).getByRole("button", { name: "Back" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "More playlist options" })).toBeDisabled();
    expect(within(sidebar).getByRole("button", { name: "Search playlists" })).toBeInTheDocument();
    // meta row: count + sort/list/grid modes — the store seed (frame rows)
    expect(within(sidebar).getByText("3 Items")).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "Sort playlists alphabetically" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "List view" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "Grid view" })).toBeInTheDocument();
    // create row is LIVE (w26.f08, e09-F5): opens the shared create dialog
    expect(within(sidebar).getByRole("button", { name: "Create playlist" })).toBeEnabled();
    // rows render through VirtualList's GRID API (w26 Lead a11y fix: sidebar rows
    // contain links, so the widget is role="grid" with row/gridcell rows)
    const list = within(sidebar).getByRole("grid", { name: "Playlists" });
    expect(within(list).getByRole("row", { name: /Cool Playlist/ })).toBeInTheDocument();
    expect(within(list).getByRole("row", { name: /Cool Songs/ })).toBeInTheDocument();
    expect(within(list).getByRole("row", { name: /Anime to Watch/ })).toBeInTheDocument();
  });

  it("collapses and re-opens via the title-bar toggle (aria-expanded follows)", () => {
    renderShell();
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    fireEvent.click(toggle);
    const sidebar = getSidebar();
    expect(sidebar).toHaveAttribute("hidden");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(toggle);
    expect(sidebar).not.toHaveAttribute("hidden");
    expect(toggle).toHaveAttribute("aria-expanded", "true");
  });

  it("persists the collapsed state across mounts (localStorage seam)", () => {
    const { unmount } = renderShell();
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    unmount();
    renderShell();
    expect(getSidebar()).toHaveAttribute("hidden");
  });

  it("filters rows through the revealed search field and drops the filter on close", () => {
    renderShell();
    const sidebar = getSidebar();
    fireEvent.click(within(sidebar).getByRole("button", { name: "Search playlists" }));
    const input = within(sidebar).getByRole("searchbox");
    fireEvent.change(input, { target: { value: "cool" } });
    const list = within(sidebar).getByRole("grid", { name: "Playlists" });
    expect(within(list).getByRole("row", { name: /Cool Songs/ })).toBeInTheDocument();
    expect(within(list).queryByRole("row", { name: /Anime to Watch/ })).toBeNull();
    // closing search drops the filter (no stale hidden rows)
    fireEvent.click(within(sidebar).getByRole("button", { name: "Search playlists" }));
    const listAfter = within(sidebar).getByRole("grid", { name: "Playlists" });
    expect(within(listAfter).getByRole("row", { name: /Anime to Watch/ })).toBeInTheDocument();
  });

  it("shows the honest empty message when the search matches nothing", () => {
    renderShell();
    const sidebar = getSidebar();
    fireEvent.click(within(sidebar).getByRole("button", { name: "Search playlists" }));
    fireEvent.change(within(sidebar).getByRole("searchbox"), {
      target: { value: "nope" },
    });
    expect(within(sidebar).getByText("No playlists match your search.")).toBeInTheDocument();
  });

  it("sorts rows naturally by default (library order) and strictly alphabetically via the meta-row toggle", () => {
    // Numbered titles expose the difference: natural reads "Playlist 2"
    // before "Playlist 10" (the v2 flow-brief fix), strict alpha is lexical.
    seedPlaylistsStore([
      { id: "p10", title: "Playlist 10", createdAt: 0, itemNames: [] },
      { id: "p2", title: "Playlist 2", createdAt: 0, itemNames: [] },
    ]);
    renderShell();
    const sidebar = getSidebar();
    const list = () => within(sidebar).getByRole("grid", { name: "Playlists" });
    const labels = () => within(list()).getAllByRole("row").map((row) => row.textContent);
    expect(labels()).toEqual(["Playlist 2", "Playlist 10"]);
    const sort = within(sidebar).getByRole("button", { name: "Sort playlists alphabetically" });
    expect(sort).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(sort);
    expect(sort).toHaveAttribute("aria-pressed", "true");
    expect(labels()).toEqual(["Playlist 10", "Playlist 2"]);
  });

  it("switches to the 2-up grid view through the scroll primitive", () => {
    renderShell();
    const sidebar = getSidebar();
    fireEvent.click(within(sidebar).getByRole("button", { name: "Grid view" }));
    const grid = within(sidebar).getByRole("region", { name: "Playlists" });
    expect(within(grid).getAllByRole("link")).toHaveLength(3); // store seed rows
    // and back to the windowed list
    fireEvent.click(within(sidebar).getByRole("button", { name: "List view" }));
    expect(within(sidebar).getByRole("grid", { name: "Playlists" })).toBeInTheDocument();
  });
});

describe("AppShell — create-playlist flow (w26.f08, e09-F5: no dead chrome)", () => {
  it("opens the SAME create dialog /playlist uses and lands on the library route", () => {
    renderShell();
    fireEvent.click(screen.getByRole("button", { name: "Create playlist" }));
    // the create flow belongs to the library surface — the crumb follows it
    expect(getShellBanner().textContent).toContain("Playlist");
    // the playlist core's own dialog (NewPlaylistDialog), not a shell copy
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByRole("heading", { name: "New playlist" })).toBeInTheDocument();
    // Cancel closes without creating anything
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(within(getSidebar()).getByText("3 Items")).toBeInTheDocument();
  });

  it("navigating is a no-op when already on /playlist (no history spam)", () => {
    renderShell(<AppShell><p>screen</p></AppShell>, "/playlist");
    fireEvent.click(screen.getByRole("button", { name: "Create playlist" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(getShellBanner().textContent).toContain("Playlist");
  });

  it("Save persists through the shared playlistsStore — the row appears in the sidebar AND the store", () => {
    renderShell();
    fireEvent.click(screen.getByRole("button", { name: "Create playlist" }));
    const dialog = screen.getByRole("dialog");
    fireEvent.change(within(dialog).getByRole("textbox"), { target: { value: "Road Trips" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    // the sidebar reflects the shared store immediately (count + row)…
    expect(within(getSidebar()).getByText("4 Items")).toBeInTheDocument();
    const list = within(getSidebar()).getByRole("grid", { name: "Playlists" });
    expect(within(list).getByRole("row", { name: /Road Trips/ })).toBeInTheDocument();
    // …and it truly persisted (the same seam /playlist hydrates from)
    const saved: { playlists?: Array<{ title?: string }> } = JSON.parse(
      window.localStorage.getItem("lumen.playlists.v1") ?? "{}",
    );
    expect(saved.playlists?.some((playlist) => playlist.title === "Road Trips")).toBe(true);
  });
});

describe("AppShell — one-sidebar composition seam (lumen:toggle-sidebar)", () => {
  /** Listens on the player's window-event seam and records dispatches. */
  function recordToggleEvents(): { seen: string[]; stop: () => void } {
    const seen: string[] = [];
    const listener = (event: Event): void => {
      seen.push(event.type);
    };
    window.addEventListener(PLAYER_TOGGLE_SIDEBAR_EVENT, listener);
    return { seen, stop: () => window.removeEventListener(PLAYER_TOGGLE_SIDEBAR_EVENT, listener) };
  }

  it("dispatches lumen:toggle-sidebar when the player's docked sidebar is open (Player.png one-column rule)", () => {
    // The player publishes its dock state as data-sidebar on .lumen-player
    // (PlayerScreen.tsx); the shell reads it, never guesses.
    // w26.g03 P1-1: entering /player with BOTH sidebars open now AUTO-COLLAPSES
    // the shell sidebar on mount (route-change effect) — so this test starts
    // from the collapsed state and toggles OPEN: the convergence dispatch
    // (collapse the player's dock) must still fire.
    const { container } = renderShell(
      <AppShell>
        <div className="lumen-player" data-sidebar="open" />
      </AppShell>,
    );
    // the auto-collapse ran on mount: one column on screen
    expect(getSidebar()).toHaveAttribute("hidden");
    const recorder = recordToggleEvents();
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    recorder.stop();
    expect(recorder.seen).toEqual([PLAYER_TOGGLE_SIDEBAR_EVENT]);
    // the shell sidebar opened (the dispatch collapses the PLAYER's dock)
    expect(getSidebar()).not.toHaveAttribute("hidden");
    expect(container.querySelector(".lumen-player")?.getAttribute("data-sidebar")).toBe("open");
  });

  it("stays silent when the player's dock is closed or no player is mounted", () => {
    const docked = renderShell(
      <AppShell>
        <div className="lumen-player" data-sidebar="closed" />
      </AppShell>,
    );
    const recorder = recordToggleEvents();
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    expect(recorder.seen).toEqual([]);
    recorder.stop();
    // and on screens with no player at all (no listener, no dispatch source)
    docked.unmount();
    renderShell();
    const quiet = recordToggleEvents();
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    quiet.stop();
    expect(quiet.seen).toEqual([]);
  });

  it("w26.i20: ROUTE CHANGE into the player family collapses the shell sidebar when the dock is open (session-only)", () => {
    // gfix-L5 verified as a ROUTE-CHANGE behavior, not just on mount: start
    // on /playlist (route-aware default OPEN), navigate through the shell's
    // own rail (the real user path), and the one-sidebar rule collapses the
    // shell column for the visit without touching the stored preference.
    renderShell(
      <AppShell>
        <div className="lumen-player" data-sidebar="open" />
      </AppShell>,
      "/playlist",
      false,
    );
    expect(getSidebar()).not.toHaveAttribute("hidden");
    const rail = screen.getByRole("navigation", { name: "Primary navigation" });
    fireEvent.click(within(rail).getByRole("link", { name: "Player" }));
    expect(getSidebar()).toHaveAttribute("hidden");
    // session-only: nothing was persisted for other routes.
    expect(window.localStorage.getItem("lumen.shell.sidebar.v1")).toBeNull();
  });
});

describe("AppShell — window controls (in-app actions, honest labels)", () => {
  it("minimize routes to the miniplayer", () => {
    renderShell();
    fireEvent.click(screen.getByRole("button", { name: "Minimize to miniplayer" }));
    // the crumb follows the route (breadcrumb seam)
    expect(getShellBanner().textContent).toContain("Miniplayer");
  });

  it("maximize toggles document fullscreen and reflects the pressed state", () => {
    const request = vi.fn().mockResolvedValue(undefined);
    const exit = vi.fn().mockResolvedValue(undefined);
    // requestFullscreen lives on the element being promoted (the shell
    // promotes documentElement); exitFullscreen lives on the document.
    Object.defineProperty(document.documentElement, "requestFullscreen", {
      value: request,
      configurable: true,
    });
    Object.defineProperty(document, "exitFullscreen", { value: exit, configurable: true });
    Object.defineProperty(document, "fullscreenElement", { value: null, configurable: true });

    renderShell();
    const maximize = screen.getByRole("button", { name: "Toggle fullscreen" });
    expect(maximize).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(maximize);
    expect(request).toHaveBeenCalledTimes(1);

    act(() => {
      Object.defineProperty(document, "fullscreenElement", {
        value: document.documentElement,
        configurable: true,
      });
      document.dispatchEvent(new Event("fullscreenchange"));
    });
    expect(maximize).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(maximize);
    expect(exit).toHaveBeenCalledTimes(1);
  });

  it("close stops playback (a real command) and returns home", () => {
    const transport = new MockTransport();
    render(
      <ClientProvider transport={transport}>
        <MemoryRouter initialEntries={["/playlist"]}>
          <AppShell><p>screen</p></AppShell>
        </MemoryRouter>
      </ClientProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Stop and return to the player" }));
    expect(transport.sent.some((request) => request.cmd === "playback.stop")).toBe(true);
    expect(getShellBanner().textContent).toContain("Player");
  });
});

describe("AppShell — main region ownership", () => {
  it("provides the <main> landmark by default (exactly one)", () => {
    renderShell();
    expect(screen.getAllByRole("main")).toHaveLength(1);
  });

  it("defers to screens that render their own <main> (provideMain=false)", () => {
    const { container } = renderShell(
      <AppShell provideMain={false}><p>screen</p></AppShell>,
    );
    expect(screen.queryByRole("main")).toBeNull();
    expect(container.querySelector(".app-shell__main")).not.toBeNull();
  });
});

describe("AppShell — strip portal seam (w26.g03 P1-2, AppShellStripContext; runtime test w26.k05/j05 P2-2)", () => {
  /** Minimal consumer of the strip contract (PlayerScreen is the real
   * one): portals its band content into the shell-provided host, or renders
   * in place when no shell mounted (the context's null default). */
  function StripConsumer() {
    const host = useAppShellStrip();
    if (host === null) return <p data-testid="strip-inplace">in place</p>;
    return createPortal(<footer data-testid="strip-footer">band</footer>, host);
  }

  it("portals the consumer's content into the full-window strip band — outside <main>, on the player-variant host", () => {
    renderShell(
      <AppShell>
        <StripConsumer />
      </AppShell>,
    );
    const footer = screen.getByTestId("strip-footer");
    // the portal target is the shell's strip band (the full-window slot
    // BELOW the frame row — rail + sidebar end above it, Player.svg)
    const strip = footer.closest(".app-shell__strip");
    expect(strip).not.toBeNull();
    // full-window band: the portaled content lives OUTSIDE the main region
    expect(footer.closest("main")).toBeNull();
    expect(footer.closest(".app-shell__main")).toBeNull();
    // the host carries the classes the portaled controls' CSS scopes to
    // (player.css's neutralized --strip variant)
    expect(strip).toHaveClass("lumen-player", "lumen-player--strip");
    // and the in-place fallback is NOT rendered alongside the portal
    expect(screen.queryByTestId("strip-inplace")).toBeNull();
  });

  it("keeps the strip band EMPTY when no consumer portals (the :empty collapse seam)", () => {
    const { container } = renderShell();
    const strip = container.querySelector(".app-shell__strip");
    expect(strip).not.toBeNull();
    // `.app-shell__strip:empty { display: none }` collapses the band on
    // non-player routes; childElementCount 0 is the DOM-semantic proxy for
    // :empty (jsdom computes no CSS) — a moved provider or a dropped
    // ref={setStripHost} now fails HERE instead of only in pixel-land.
    expect(strip?.childElementCount).toBe(0);
  });

  it("renders the consumer IN PLACE when no shell provides a strip host (the null default)", () => {
    // Isolated host (exactly the contract PlayerScreen's footer relies on
    // in its own tests): the context default is null → no portal, the
    // consumer keeps its own tree.
    render(<StripConsumer />);
    expect(screen.getByTestId("strip-inplace")).toBeInTheDocument();
    expect(screen.queryByTestId("strip-footer")).toBeNull();
  });
});

describe("AppShell — narrow viewport (1024×768 graceful collapse)", () => {
  type MatchMediaFactory = typeof window.matchMedia;
  const realMatchMedia = window.matchMedia;

  /** Installs a controllable matchMedia mock. Returns the driver for the
   * NARROW query's `change` event — w26.k05 (j05 P3-6): the change path
   * (viewport crossing into the band mid-session) is now DRIVEN; the old
   * mock's `dispatch` helper was dead test code. */
  function installMatchMedia(matches: boolean): {
    dispatchIntoNarrow: (event: { matches: boolean }) => void;
  } {
    // One fake MediaQueryList per matchMedia() call, like the real engine —
    // listeners stay isolated per query (the shell is not the only caller).
    const listenersByQuery = new Map<string, Array<(event: { matches: boolean }) => void>>();
    const factory = vi.fn((query: string) => {
      const listeners: Array<(event: { matches: boolean }) => void> = [];
      listenersByQuery.set(query, listeners);
      return {
        matches: matches && query === "(max-width: 1100px)",
        addEventListener: (_: string, cb: (event: { matches: boolean }) => void) => {
          listeners.push(cb);
        },
        removeEventListener: () => {},
        addListener: () => {},
        removeListener: () => {},
      };
    }) as unknown as MatchMediaFactory;
    Object.defineProperty(window, "matchMedia", { value: factory, configurable: true });
    return {
      dispatchIntoNarrow: (event) => {
        listenersByQuery.get("(max-width: 1100px)")?.forEach((listener) => listener(event));
      },
    };
  }

  afterEach(() => {
    // Restore the REAL matchMedia — the Object.defineProperty mock used to
    // leak into every later test in the file (the narrow suite was last
    // only by luck of describe order; the hygiene half of j05 P3-6).
    Object.defineProperty(window, "matchMedia", { value: realMatchMedia, configurable: true });
  });

  it("auto-collapses the sidebar inside the narrow band; the toggle stays live", () => {
    installMatchMedia(true);
    renderShell();
    const sidebar = getSidebar();
    expect(sidebar).toHaveAttribute("hidden"); // collapsed, nothing clips
    // the user can still re-open it (the icon rail keeps destinations too)
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    expect(sidebar).not.toHaveAttribute("hidden");
  });

  it("collapses the sidebar when the viewport crosses INTO the narrow band mid-session (the `change` path, j05 P3-6)", () => {
    const { dispatchIntoNarrow } = installMatchMedia(false);
    renderShell();
    // wide start: the sidebar is up (seeded preference)
    expect(getSidebar()).not.toHaveAttribute("hidden");
    // the window shrinks past 1100px (devtools opened, snap resize…): the
    // MediaQueryList fires `change` — the listener collapses the column
    act(() => {
      dispatchIntoNarrow({ matches: true });
    });
    expect(getSidebar()).toHaveAttribute("hidden");
    // the toggle stays live inside the band (nothing clips either way)
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    expect(getSidebar()).not.toHaveAttribute("hidden");
  });

  it("rescues focus to the toggle when the sidebar auto-collapses while focused inside it (j05 P3-8)", () => {
    const { dispatchIntoNarrow } = installMatchMedia(false);
    renderShell();
    const sidebar = getSidebar();
    // keyboard user working inside the sidebar (the revealed search field)
    fireEvent.click(within(sidebar).getByRole("button", { name: "Search playlists" }));
    const input = within(sidebar).getByRole("searchbox");
    input.focus();
    expect(document.activeElement).toBe(input);
    // the viewport crosses into the narrow band → the column collapses…
    act(() => {
      dispatchIntoNarrow({ matches: true });
    });
    expect(sidebar).toHaveAttribute("hidden");
    // …and focus followed the collapse to the toggle, not stranded on <body>
    const toggle = screen.getByRole("button", { name: "Toggle playlists sidebar" });
    expect(document.activeElement).toBe(toggle);
  });

  it("leaves the sidebar open on wide viewports", () => {
    installMatchMedia(false);
    renderShell(
      <AppShell><p>screen</p></AppShell>,
      "/player",
      true,
    );
    expect(getSidebar()).not.toHaveAttribute("hidden");
  });
});
