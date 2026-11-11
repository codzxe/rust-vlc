// AppShell tests — the W26 shell chrome composition (lane w26.a10):
// banner title bar (Player.svg topbar anatomy), icon rail, playlists
// sidebar (sidebar.svg), the Panel/.lumen-pane adoption (w26.a08), the
// open-media event seam, window-control in-app actions, and the
// narrow-viewport auto-collapse. DOM-shape + behavior assertions; visual
// fidelity is proven by the dev-shots + VLM loop (lane log).
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import { OPEN_MEDIA_EVENT } from "../../screens/open-media";
import { AppShell } from "./AppShell";

/** The shell needs a client (open-media + window-close command seam). */
const renderShell = (
  ui: React.ReactElement = <AppShell><p>screen</p></AppShell>,
  initialEntry = "/player",
) =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[initialEntry]}>{ui}</MemoryRouter>
    </ClientProvider>,
  );

beforeEach(() => {
  // The sidebar persists openness in localStorage — start every test clean.
  window.localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

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

describe("AppShell — playlists sidebar (sidebar.svg)", () => {
  it("renders the header, meta row, create row and the fixture rows through the list primitive", () => {
    renderShell();
    const sidebar = getSidebar();
    // header controls
    expect(within(sidebar).getByRole("button", { name: "Back" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "More playlist options" })).toBeDisabled();
    expect(within(sidebar).getByRole("button", { name: "Search playlists" })).toBeInTheDocument();
    // meta row: count + sort/list/grid modes
    expect(within(sidebar).getByText("2 Items")).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "Sort playlists alphabetically" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "List view" })).toBeInTheDocument();
    expect(within(sidebar).getByRole("button", { name: "Grid view" })).toBeInTheDocument();
    // create row is disabled-honest (no engine command behind it)
    expect(within(sidebar).getByRole("button", { name: "Create playlist" })).toBeDisabled();
    // rows render through VirtualList's listbox API
    const list = within(sidebar).getByRole("listbox", { name: "Playlists" });
    expect(within(list).getByRole("option", { name: /Cool Songs/ })).toBeInTheDocument();
    expect(within(list).getByRole("option", { name: /Anime to Watch/ })).toBeInTheDocument();
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
    const list = within(sidebar).getByRole("listbox", { name: "Playlists" });
    expect(within(list).getByRole("option", { name: /Cool Songs/ })).toBeInTheDocument();
    expect(within(list).queryByRole("option", { name: /Anime to Watch/ })).toBeNull();
    // closing search drops the filter (no stale hidden rows)
    fireEvent.click(within(sidebar).getByRole("button", { name: "Search playlists" }));
    const listAfter = within(sidebar).getByRole("listbox", { name: "Playlists" });
    expect(within(listAfter).getByRole("option", { name: /Anime to Watch/ })).toBeInTheDocument();
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

  it("sorts rows alphabetically via the meta-row sort toggle", () => {
    renderShell();
    const sidebar = getSidebar();
    const sort = within(sidebar).getByRole("button", { name: "Sort playlists alphabetically" });
    expect(sort).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(sort);
    expect(sort).toHaveAttribute("aria-pressed", "true");
    const list = within(sidebar).getByRole("listbox", { name: "Playlists" });
    const labels = within(list)
      .getAllByRole("option")
      .map((option) => option.textContent);
    expect(labels).toEqual(["Anime to Watch", "Cool Songs"]);
  });

  it("switches to the 2-up grid view through the scroll primitive", () => {
    renderShell();
    const sidebar = getSidebar();
    fireEvent.click(within(sidebar).getByRole("button", { name: "Grid view" }));
    const grid = within(sidebar).getByRole("region", { name: "Playlists" });
    expect(within(grid).getAllByRole("link", { name: /Cool Songs|Anime to Watch/ })).toHaveLength(2);
    // and back to the windowed list
    fireEvent.click(within(sidebar).getByRole("button", { name: "List view" }));
    expect(within(sidebar).getByRole("listbox", { name: "Playlists" })).toBeInTheDocument();
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

describe("AppShell — narrow viewport (1024×768 graceful collapse)", () => {
  type MatchMediaFactory = typeof window.matchMedia;

  function installMatchMedia(matches: boolean): void {
    const listeners: Array<(event: { matches: boolean }) => void> = [];
    const factory = vi.fn((query: string) => ({
      matches: matches && query === "(max-width: 1100px)",
      addEventListener: (_: string, cb: (event: { matches: boolean }) => void) => {
        listeners.push(cb);
      },
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatch: (event: { matches: boolean }) => {
        listeners.forEach((listener) => listener(event));
      },
    })) as unknown as MatchMediaFactory;
    Object.defineProperty(window, "matchMedia", { value: factory, configurable: true });
  }

  it("auto-collapses the sidebar inside the narrow band; the toggle stays live", () => {
    installMatchMedia(true);
    renderShell();
    const sidebar = getSidebar();
    expect(sidebar).toHaveAttribute("hidden"); // collapsed, nothing clips
    // the user can still re-open it (the icon rail keeps destinations too)
    fireEvent.click(screen.getByRole("button", { name: "Toggle playlists sidebar" }));
    expect(sidebar).not.toHaveAttribute("hidden");
  });

  it("leaves the sidebar open on wide viewports", () => {
    installMatchMedia(false);
    renderShell();
    expect(getSidebar()).not.toHaveAttribute("hidden");
  });
});
