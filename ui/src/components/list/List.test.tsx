import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import {
  ListEmpty,
  ListRow,
  ROW_HEIGHTS,
  naturalCompare,
  naturalSort,
  ScrollArea,
  Tab,
  Tabs,
  TabsList,
  TabsPanel,
  VirtualList,
  VirtualTable,
  computeRange,
} from "./index";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const listCss = readFileSync(`${dirname}/list.css`, "utf8");

const cssRule = (selector: string): string => {
  const match = listCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};

function TabbedScreen() {
  return (
    <Tabs defaultValue="library">
      <TabsList>
        <Tab value="home">Home</Tab>
        <Tab value="library">Library</Tab>
        <Tab value="network">Network</Tab>
      </TabsList>
      <TabsPanel value="home">Home panel</TabsPanel>
      <TabsPanel value="library">Library panel</TabsPanel>
      <TabsPanel value="network">Network panel</TabsPanel>
    </Tabs>
  );
}

describe("Tabs (doc 08 §4.5 anatomy)", () => {
  it("renders tablist/tab/tabpanel with aria wiring", () => {
    render(<TabbedScreen />);
    const tablist = screen.getByRole("tablist");
    expect(tablist).toHaveAttribute("aria-orientation", "horizontal");
    const library = screen.getByRole("tab", { selected: true, name: "Library" });
    expect(library).toHaveAttribute("aria-controls");
    const panel = screen.getByRole("tabpanel", { name: "Library" });
    expect(panel.id).toBe(library.getAttribute("aria-controls"));
    expect(library.getAttribute("aria-labelledby") ?? panel.getAttribute("aria-labelledby")).toBeTruthy();
  });

  it("uses roving tabindex: only the selected tab is tabbable", () => {
    render(<TabbedScreen />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.tabIndex)).toEqual([-1, 0, -1]);
  });

  it("shows only the selected panel", () => {
    render(<TabbedScreen />);
    expect(screen.getByText("Library panel")).toBeVisible();
    expect(screen.getByText("Home panel")).not.toBeVisible(); // hidden attr
    expect(screen.queryByText("Network panel")).toBeInTheDocument();
  });

  it("ArrowRight/ArrowLeft move selection and focus with wrapping", () => {
    render(<TabbedScreen />);
    const library = screen.getByRole("tab", { name: "Library" });
    library.focus();
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("Network");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "Network" }));
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("Home"); // wrapped
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("Network"); // wrapped back
  });

  it("Home/End jump to the first/last tab", () => {
    render(<TabbedScreen />);
    screen.getByRole("tab", { name: "Library" }).focus();
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "End" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("Network");
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "Home" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("Home");
  });

  it("vertical orientation uses Down/Up arrows", () => {
    render(
      <Tabs defaultValue="a" orientation="vertical">
        <TabsList>
          <Tab value="a">A</Tab>
          <Tab value="b">B</Tab>
        </TabsList>
        <TabsPanel value="a">Panel A</TabsPanel>
        <TabsPanel value="b">Panel B</TabsPanel>
      </Tabs>,
    );
    expect(screen.getByRole("tablist")).toHaveAttribute("aria-orientation", "vertical");
    screen.getByRole("tab", { name: "A" }).focus();
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowDown" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("B");
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowUp" });
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("A");
  });

  it("clicking a tab selects it", () => {
    render(<TabbedScreen />);
    fireEvent.click(screen.getByRole("tab", { name: "Network" }));
    expect(screen.getByRole("tab", { selected: true }).textContent).toBe("Network");
    expect(screen.getByText("Network panel")).toBeVisible();
  });
});

describe("ScrollArea (thin-scrollbar spec, doc 08 §4)", () => {
  it("renders a labelled scroll region over native scrolling", () => {
    render(
      <ScrollArea label="Playlist scroll">
        <p>row</p>
      </ScrollArea>,
    );
    const region = screen.getByRole("region", { name: "Playlist scroll" });
    expect(region.className).toContain("lumen-scrollarea");
  });

  it("styles scrollbars with tokens (CSS source check, B1 pattern)", () => {
    const rule = cssRule(".lumen-scrollarea");
    expect(rule).toContain("overflow: auto"); // honest native scrolling
    expect(listCss).toContain("scrollbar-width: thin"); // thin-scrollbar spec
    expect(cssRule(".lumen-scrollarea::-webkit-scrollbar-thumb")).toContain("var(--color-surface-muted)");
  });
});

describe("virtualization math (computeRange)", () => {
  it("returns the visible window plus overscan", () => {
    // 10k rows × 28px, viewport 560px (= 20 rows), scrolled to row 500.
    const range = computeRange(10_000, 28, 500 * 28, 560, 5);
    expect(range.startIndex).toBe(495);
    expect(range.endIndex).toBe(524); // 500 + 20 + 5 - 1
    expect(range.totalHeight).toBe(280_000);
  });

  it("clamps at the top and bottom edges", () => {
    expect(computeRange(100, 10, 0, 100, 5)).toMatchObject({ startIndex: 0, endIndex: 14 });
    const bottom = computeRange(100, 10, 99 * 10, 100, 5);
    expect(bottom.startIndex).toBe(94);
    expect(bottom.endIndex).toBe(99);
  });

  it("clamps a stale scrollTop past a shrunken dataset back onto the data", () => {
    // 10k rows scrolled to row 9000, then filtered to 3 items — the offset
    // (9000×28) is past the new end; the window must re-attach to the data.
    expect(computeRange(3, 28, 9000 * 28, 560, 5)).toMatchObject({ startIndex: 0, endIndex: 2 });
    expect(computeRange(0, 28, 9000 * 28, 560, 5)).toMatchObject({ startIndex: 0, endIndex: -1 });
  });
});

const makeRows = (n: number) => Array.from({ length: n }, (_, i) => `Row ${i}`);

/** jsdom layout shim: the viewport must report a fixed clientHeight. */
function shimViewportHeight(container: HTMLElement, height: number) {
  Object.defineProperty(container, "clientHeight", { value: height, configurable: true });
}

describe("VirtualList — 10k-row AC (doc 08 §4 virtualization)", () => {
  const data = makeRows(10_000);

  it("renders a bounded DOM window, not 10k rows (< 100 nodes)", () => {
    const { container } = render(
      <VirtualList data={data} itemHeight={28} label="Big list" renderItem={(row) => <span>{row}</span>} />,
    );
    // jsdom clientHeight is 0 → only overscan rows render; still bounded.
    const rows = container.querySelectorAll('[role="option"]');
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).toBeLessThan(100);
    expect(container.querySelectorAll("*").length).toBeLessThan(100);
  });

  it("spacer height = count × itemHeight (exact scrollbar geometry)", () => {
    const { container } = render(
      <VirtualList data={data} itemHeight={28} label="Big list" renderItem={(row) => <span>{row}</span>} />,
    );
    const spacer = container.querySelector<HTMLElement>(".lumen-vlist__spacer")!;
    expect(spacer.style.height).toBe("280000px");
  });

  it("scrolling renders exactly the right contiguous window — no gaps, no dupes", () => {
    const { container } = render(
      <VirtualList data={data} itemHeight={28} overscan={5} label="Big list" renderItem={(row) => <span>{row}</span>} />,
    );
    const viewport = container.querySelector<HTMLElement>('[role="listbox"]')!;
    shimViewportHeight(viewport, 560); // 20 rows visible

    viewport.scrollTop = 500 * 28; // jump to row 500
    fireEvent.scroll(viewport);

    const rows = Array.from(container.querySelectorAll<HTMLElement>('[role="option"]'));
    expect(rows.length).toBe(30); // 20 visible + 2×5 overscan
    const indexes = rows.map((row) => parseInt(row.style.top, 10) / 28);
    // Contiguity = no gaps: sorted indexes form an unbroken run.
    const sorted = [...indexes].sort((a, b) => a - b);
    for (let i = 1; i < sorted.length; i++) expect(sorted[i]!).toBe(sorted[i - 1]! + 1);
    expect(sorted[0]).toBe(495);
    expect(sorted.at(-1)).toBe(524);
    // Every rendered row shows the correct data for its position.
    expect(rows.find((r) => r.style.top === `${500 * 28}px`)!.textContent).toBe("Row 500");
  });

  it("scrollToIndex lands the target row at the viewport start", () => {
    const { container, rerender } = render(
      <VirtualList data={data} itemHeight={28} overscan={3} label="Big list" scrollToIndex={{ index: 2000 }} renderItem={(row) => <span>{row}</span>} />,
    );
    const viewport = container.querySelector<HTMLElement>('[role="listbox"]')!;
    shimViewportHeight(viewport, 280); // 10 rows
    rerender(
      <VirtualList data={data} itemHeight={28} overscan={3} label="Big list" scrollToIndex={{ index: 2000 }} renderItem={(row) => <span>{row}</span>} />,
    );
    expect(viewport.scrollTop).toBe(2000 * 28);
    const rows = container.querySelectorAll('[role="option"]');
    expect(rows[0]!.textContent).toBe("Row 1997"); // 2000 - overscan 3
    expect(screenByTop(container, 2000 * 28)).toBe("Row 2000");
  });

  it("scrollToIndex clamps out-of-range targets", () => {
    const { container, rerender } = render(
      <VirtualList data={data} itemHeight={28} label="Big list" scrollToIndex={{ index: 99_999 }} renderItem={(row) => <span>{row}</span>} />,
    );
    rerender(
      <VirtualList data={data} itemHeight={28} label="Big list" scrollToIndex={{ index: 99_999, align: "end" }} renderItem={(row) => <span>{row}</span>} />,
    );
    const viewport = container.querySelector<HTMLElement>('[role="listbox"]')!;
    shimViewportHeight(viewport, 280);
    rerender(
      <VirtualList data={data} itemHeight={28} label="Big list" scrollToIndex={{ index: 99_999, align: "start" }} renderItem={(row) => <span>{row}</span>} />,
    );
    expect(viewport.scrollTop).toBe(9_999 * 28); // clamped to last row
  });

  it("never renders past the data when the dataset shrinks mid-window", () => {
    // Deep scroll → stale window ~rows 495–524, then the filter shrinks the
    // dataset to 3 items before any scroll event fires (AppShell's guard
    // case — the list lane owns the clamp, not the consumer).
    const { container, rerender } = render(
      <VirtualList data={data} itemHeight={28} overscan={5} label="Big list" renderItem={(row) => <span>{row}</span>} />,
    );
    const viewport = container.querySelector<HTMLElement>('[role="listbox"]')!;
    shimViewportHeight(viewport, 560);
    viewport.scrollTop = 500 * 28;
    fireEvent.scroll(viewport);
    expect(container.querySelectorAll('[role="option"]').length).toBeGreaterThan(20);
    rerender(
      <VirtualList data={["A", "B", "C"]} itemHeight={28} overscan={5} label="Big list" renderItem={(row) => <span>{row}</span>} />,
    );
    const rows = Array.from(container.querySelectorAll<HTMLElement>('[role="option"]'));
    expect(rows.length).toBe(3); // clamped to the shrunken data, not the stale window
    expect(rows.map((r) => r.textContent)).toEqual(["A", "B", "C"]);
    rerender(<VirtualList data={[]} itemHeight={28} overscan={5} label="Big list" renderItem={(row) => <span>{row}</span>} />);
    expect(container.querySelectorAll('[role="option"]').length).toBe(0); // empty → zero rows
  });
});

function screenByTop(container: HTMLElement, top: number): string | null {
  return container.querySelector<HTMLElement>(`[role="option"][style*="top: ${top}px"]`)?.textContent ?? null;
}

describe("ListRow — Figma row grammar (W25 §2: playlist.svg / media.svg / menu.svg)", () => {
  it("renders the anatomy: icon · title/meta · duration · trailing · kebab", () => {
    const { container } = render(
      <ListRow
        icon={<span data-testid="row-icon" />}
        title="Cool Playlist"
        meta="15 items"
        duration="23:06"
        trailing="Ctrl + O"
        kebab={{ label: "Row actions", onClick: () => {} }}
      />,
    );
    expect(screen.getByText("Cool Playlist")).toBeInTheDocument();
    expect(screen.getByText("15 items")).toBeInTheDocument();
    expect(screen.getByText("23:06")).toBeInTheDocument();
    expect(screen.getByTestId("row-icon")).toBeInTheDocument();
    expect(screen.getByText("Ctrl + O")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Row actions" })).toBeInTheDocument();
    // no drag handle unless the row is draggable (queue rows only)
    expect(container.querySelector(".lumen-lrow__handle")).toBeNull();
  });

  it("defaults to comfortable density and exposes the data-density hook", () => {
    render(<ListRow title="Row" />);
    const row = screen.getByText("Row").closest(".lumen-lrow");
    expect(row).toHaveClass("lumen-lrow--comfortable");
    expect(row).toHaveAttribute("data-density", "comfortable");
  });

  it("compact density swaps to the menu/settings pitch", () => {
    render(<ListRow density="compact" title="General" />);
    const row = screen.getByText("General").closest(".lumen-lrow");
    expect(row).toHaveClass("lumen-lrow--compact");
    expect(row).toHaveAttribute("data-density", "compact");
  });

  it("selected rows carry the selected class + data-selected", () => {
    render(<ListRow selected title="General" />);
    const row = screen.getByText("General").closest(".lumen-lrow");
    expect(row).toHaveClass("lumen-lrow--selected");
    expect(row).toHaveAttribute("data-selected", "true");
  });

  it("kebab click fires without bubbling to the row", () => {
    const onRowClick = vi.fn();
    const onKebab = vi.fn();
    render(<ListRow title="Row" onClick={onRowClick} kebab={{ label: "More", onClick: onKebab }} />);
    fireEvent.click(screen.getByRole("button", { name: "More" }));
    expect(onKebab).toHaveBeenCalledTimes(1);
    expect(onRowClick).not.toHaveBeenCalled();
  });

  it("custom actions replace the default kebab", () => {
    const { container } = render(<ListRow title="Row" actions={<button type="button">Custom</button>} />);
    expect(screen.getByRole("button", { name: "Custom" })).toBeInTheDocument();
    expect(container.querySelector(".lumen-lrow__kebab")).toBeNull();
  });

  it("drag-handle rows: the handle renders only while draggable", () => {
    const { container, rerender } = render(<ListRow title="Row" draggable />);
    expect(container.querySelector(".lumen-lrow__handle")).not.toBeNull();
    rerender(<ListRow title="Row" />);
    expect(container.querySelector(".lumen-lrow__handle")).toBeNull();
  });

  it('Booleanish edge: draggable="false" never shows the handle', () => {
    const { container } = render(<ListRow title="Row" draggable="false" />);
    expect(container.querySelector(".lumen-lrow__handle")).toBeNull();
  });

  it("passes role/aria through for listbox wiring", () => {
    render(<ListRow role="option" aria-selected title="Item" />);
    expect(screen.getByRole("option", { selected: true })).toBeInTheDocument();
  });

  it("composes inside VirtualList at the comfortable pitch", () => {
    const { container } = render(
      <VirtualList
        data={["A", "B", "C"]}
        itemHeight={ROW_HEIGHTS.comfortable}
        label="Rows"
        renderItem={(item) => <ListRow title={item} />}
      />,
    );
    const row = container.querySelector<HTMLElement>(".lumen-lrow");
    expect(row).not.toBeNull();
    expect(row!.closest('[role="option"]')).not.toBeNull();
  });
});

describe("row grammar CSS (source check — B1 pattern)", () => {
  it("comfortable rows are ~66px with r10 rounding (playlist.svg)", () => {
    expect(cssRule(".lumen-lrow--comfortable")).toContain(`height: ${ROW_HEIGHTS.comfortable}px`);
    expect(cssRule(".lumen-lrow")).toContain("border-radius: 10px");
  });

  it("compact rows are 36–44px with the r6 control rounding (menus/settings)", () => {
    expect(cssRule(".lumen-lrow--compact")).toContain(`height: ${ROW_HEIGHTS.compact}px`);
    expect(cssRule(".lumen-lrow--compact")).toContain("var(--radius-control)");
  });

  it("hover: surface fill + bright text (playlist.svg hover state)", () => {
    const hover = cssRule(".lumen-lrow:hover");
    expect(hover).toContain("var(--color-surface-hover)");
    expect(hover).toContain("var(--color-content-bright)");
  });

  it("selected: surface-selected fill + orange icon (Settings active row)", () => {
    expect(cssRule(".lumen-lrow--selected")).toContain("var(--color-surface-selected)");
    expect(cssRule(".lumen-lrow--selected .lumen-lrow__icon")).toContain("var(--color-accent-icon)");
  });

  it("focus-visible ring uses the accent (W25 §2 focus floor)", () => {
    expect(cssRule(".lumen-lrow:focus-visible")).toContain("var(--color-accent)");
  });

  it("durations render with tabular numerals (W25 §1)", () => {
    expect(cssRule(".lumen-lrow__duration")).toContain("tabular-nums");
  });

  it("kebab + drag handle stay hidden until hover/focus (quiet rows)", () => {
    expect(cssRule(".lumen-lrow__actions")).toContain("opacity: 0");
    expect(cssRule(".lumen-lrow__handle")).toContain("opacity: 0");
    expect(listCss).toContain(".lumen-lrow:hover .lumen-lrow__actions");
  });

  it("virtualized rows carry the frame rounding too (.lumen-vrow)", () => {
    expect(cssRule(".lumen-vrow")).toContain("border-radius: 10px");
  });
});

describe("ListEmpty — quiet empty-list state", () => {
  it("renders title/body/action without artwork", () => {
    const { container } = render(
      <ListEmpty
        title="Nothing in the queue"
        body="Files you open are listed here."
        action={<button type="button">Open media</button>}
      />,
    );
    expect(screen.getByText("Nothing in the queue")).toBeVisible();
    expect(screen.getByText("Files you open are listed here.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Open media" })).toBeVisible();
    // quiet: no images/SVG art in the empty state
    expect(container.querySelector("img, svg")).toBeNull();
  });

  it("body and action are optional", () => {
    const { container } = render(<ListEmpty title="No items" />);
    expect(screen.getByText("No items")).toBeVisible();
    expect(container.querySelector(".lumen-empty__body")).toBeNull();
    expect(container.querySelector(".lumen-empty__action")).toBeNull();
  });

  it("CSS: secondary title + tertiary body (source check)", () => {
    expect(cssRule(".lumen-empty__title")).toContain("var(--color-content-secondary)");
    expect(cssRule(".lumen-empty__body")).toContain("var(--color-content-tertiary)");
  });
});

describe("naturalCompare / naturalSort (2026-11-09 v2 flow fix: numeric order)", () => {
  it('"Bookmark 1, 2, 10" — never the lexical "1, 10, 2"', () => {
    expect(naturalSort(["Bookmark 10", "Bookmark 2", "Bookmark 1"])).toEqual([
      "Bookmark 1",
      "Bookmark 2",
      "Bookmark 10",
    ]);
  });

  it("episode/file numbers compare by value across runs", () => {
    expect(naturalSort(["E10", "E9", "E2"])).toEqual(["E2", "E9", "E10"]);
    expect(
      naturalSort(["The Wand S01E02.mp4", "The Wand S01E10.mp4", "The Wand S01E01.mp4"]),
    ).toEqual(["The Wand S01E01.mp4", "The Wand S01E02.mp4", "The Wand S01E10.mp4"]);
  });

  it("numeric value beats leading zeros; fewer zeros break ties", () => {
    expect(naturalCompare("a02", "a10")).toBeLessThan(0); // 2 < 10 numerically
    expect(naturalCompare("a1", "a01")).toBeLessThan(0); // same value, fewer zeros first
    expect(naturalCompare("a10", "a9")).toBeGreaterThan(0);
  });

  it("text runs compare case-insensitively with a deterministic tie-break", () => {
    expect(naturalCompare("bookmark 2", "Bookmark 10")).toBeLessThan(0); // 2 < 10, case aside
    expect(naturalCompare("Bookmark 2", "bookmark 2")).toBeLessThan(0); // stable, not 0-equal
    expect(naturalCompare("bookmark 2", "bookmark 2")).toBe(0);
  });

  it("prefixes sort before their numbered extensions", () => {
    expect(naturalCompare("Track", "Track 1")).toBeLessThan(0);
    expect(naturalCompare("Track 2", "Track 10a")).toBeLessThan(0);
  });

  it("sorts via a key extractor and never mutates the input", () => {
    const input = [{ n: "Item 10" }, { n: "Item 2" }];
    const sorted = naturalSort(input, (i) => i.n);
    expect(sorted.map((i) => i.n)).toEqual(["Item 2", "Item 10"]);
    expect(input.map((i) => i.n)).toEqual(["Item 10", "Item 2"]);
  });

  it("sort is stable for equal keys", () => {
    const items = [
      { id: 1, n: "same" },
      { id: 2, n: "same" },
      { id: 3, n: "same" },
    ];
    expect(naturalSort(items, (i) => i.n).map((i) => i.id)).toEqual([1, 2, 3]);
  });
});

describe("VirtualTable (windowed table semantics)", () => {
  const data = makeRows(1000);
  const columns = [
    { id: "action", header: "Action", width: 200, renderCell: (row: string) => row },
    { id: "hotkey", header: "Hotkey", renderCell: (_: string, i: number) => `Ctrl+${i}` },
  ];

  it("renders table/row/columnheader/cell roles with sticky head", () => {
    const { container } = render(<VirtualTable data={data} itemHeight={32} columns={columns} label="Shortcuts" />);
    expect(screen.getByRole("table", { name: "Shortcuts" })).toHaveAttribute("aria-rowcount", "1000");
    expect(container.querySelectorAll('[role="columnheader"]')).toHaveLength(2);
    expect(container.querySelector('[role="row"]')!.className).toContain("lumen-vtable__head");
  });

  it("keeps the window bounded and correct while scrolling 1000 rows", () => {
    const { container } = render(<VirtualTable data={data} itemHeight={32} columns={columns} label="Shortcuts" />);
    const body = container.querySelector<HTMLElement>(".lumen-vtable__body")!;
    shimViewportHeight(body, 320); // 10 rows
    body.scrollTop = 100 * 32;
    fireEvent.scroll(body);
    const rows = Array.from(container.querySelectorAll('[role="row"]')).filter((r) => !r.className.includes("head"));
    expect(rows.length).toBeLessThan(100);
    expect(rows[0]!.textContent).toContain("Row 95"); // 100 - overscan 5
    expect(rows[0]!.textContent).toContain("Ctrl+95");
    expect(container.querySelector<HTMLElement>(".lumen-vlist__spacer")!.style.height).toBe("32000px");
  });
});
