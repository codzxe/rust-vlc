import { fireEvent, render, screen } from "@testing-library/react";
// @ts-expect-error node:fs is untyped in ui/ (no @types/node; zero-new-deps rule, card 4.B1) — vitest runs on Node.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ScrollArea, Tab, Tabs, TabsList, TabsPanel, VirtualList, VirtualTable, computeRange } from "./index";

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
});

function screenByTop(container: HTMLElement, top: number): string | null {
  return container.querySelector<HTMLElement>(`[role="option"][style*="top: ${top}px"]`)?.textContent ?? null;
}

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
