import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Card, Dialog, HeroPill, Panel, SectionHeader, Separator, Sheet, Well } from "./Surface";

/** Host harness mimicking a real opener button + dialog usage. */
function Harness({ closeOnOutsideClick = false }: { closeOnOutsideClick?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Open settings</button>
      <Dialog open={open} onClose={() => setOpen(false)} label="Settings" closeOnOutsideClick={closeOnOutsideClick}>
        <button>First action</button>
        <button>Second action</button>
        <button>Third action</button>
      </Dialog>
    </>
  );
}

afterEach(() => {
  // Reset focus between tests so opener-restore assertions stay independent.
  (document.activeElement as HTMLElement | null)?.blur();
});

describe("Dialog (focus trap, doc 08 §4.5)", () => {
  it("renders role=dialog with aria-modal and an accessible name", () => {
    render(
      <Dialog open onClose={() => {}} label="Network stream">
        <input aria-label="URL" />
      </Dialog>,
    );
    const dialog = screen.getByRole("dialog", { name: "Network stream" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
  });

  it("links aria-labelledby to a host-supplied title element", () => {
    render(
      <Dialog open onClose={() => {}} labelledBy="net-title">
        <h2 id="net-title">Open network stream</h2>
      </Dialog>,
    );
    const dialog = screen.getByRole("dialog", { name: "Open network stream" });
    expect(dialog).toHaveAttribute("aria-labelledby", "net-title");
  });

  it("renders nothing when closed", () => {
    const { container } = render(
      <Dialog open={false} onClose={() => {}} label="Empty">
        x
      </Dialog>,
    );
    expect(container.querySelector('[role="dialog"]')).toBeNull();
  });

  it("moves focus into the dialog on open and restores it to the opener on close", () => {
    render(<Harness />);
    const opener = screen.getByRole("button", { name: "Open settings" });
    opener.focus(); // jsdom: click alone does not focus (browsers do)
    fireEvent.click(opener);
    const first = screen.getByRole("button", { name: "First action" });
    expect(document.activeElement).toBe(first);

    fireEvent.keyDown(first, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it("traps Tab: cycling wraps from last back to first", () => {
    render(<Harness />);
    const opener = screen.getByRole("button", { name: "Open settings" });
    opener.focus();
    fireEvent.click(opener);
    const first = screen.getByRole("button", { name: "First action" });
    const last = screen.getByRole("button", { name: "Third action" });
    expect(document.activeElement).toBe(first);

    fireEvent.keyDown(document.activeElement!, { key: "Tab" }); // first -> second
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Second action" }));
    fireEvent.keyDown(document.activeElement!, { key: "Tab" }); // second -> third
    expect(document.activeElement).toBe(last);
    fireEvent.keyDown(document.activeElement!, { key: "Tab" }); // third -> wraps to first
    expect(document.activeElement).toBe(first);
  });

  it("traps Shift+Tab: wraps from first back to last", () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    const first = screen.getByRole("button", { name: "First action" });
    fireEvent.keyDown(first, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Third action" }));
  });

  it("Escape closes with reason 'escape'", () => {
    const onClose = vi.fn();
    render(
      <Dialog open onClose={onClose} label="Confirm">
        <button>OK</button>
      </Dialog>,
    );
    fireEvent.keyDown(screen.getByRole("button", { name: "OK" }), { key: "Escape" });
    expect(onClose).toHaveBeenCalledWith("escape");
  });

  it("scrim click closes only when closeOnOutsideClick is set", () => {
    const onClose = vi.fn();
    const { rerender } = render(
      <Dialog open onClose={onClose} label="Safe">
        <button>OK</button>
      </Dialog>,
    );
    // Clicks inside the panel never close (stopPropagation).
    fireEvent.click(screen.getByRole("button", { name: "OK" }));
    expect(onClose).not.toHaveBeenCalled();

    rerender(
      <Dialog open onClose={onClose} label="Safe" closeOnOutsideClick>
        <button>OK</button>
      </Dialog>,
    );
    fireEvent.click(screen.getByRole("dialog").parentElement!);
    expect(onClose).toHaveBeenCalledWith("outside");
  });
});

describe("Sheet (edge-anchored dialog variant, doc 08 §4)", () => {
  it("keeps dialog semantics with the side class", () => {
    render(
      <Sheet open onClose={() => {}} side="left" label="Queue">
        <button>Clear</button>
      </Sheet>,
    );
    const sheet = screen.getByRole("dialog", { name: "Queue" });
    expect(sheet).toHaveAttribute("aria-modal", "true");
    expect(sheet.className).toContain("lumen-sheet--left");
  });

  it("Escape closes and focus restores to the opener", () => {
    function SheetHarness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <button onClick={() => setOpen(true)}>Show queue</button>
          <Sheet open={open} onClose={() => setOpen(false)} side="bottom" label="Queue">
            <button>Clear queue</button>
          </Sheet>
        </>
      );
    }
    render(<SheetHarness />);
    const opener = screen.getByRole("button", { name: "Show queue" });
    opener.focus();
    fireEvent.click(opener);
    const clear = screen.getByRole("button", { name: "Clear queue" });
    expect(document.activeElement).toBe(clear);
    fireEvent.keyDown(clear, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);
  });

  it("defaults to the right edge", () => {
    render(
      <Sheet open onClose={() => {}} label="Panel">
        <span>content</span>
      </Sheet>,
    );
    expect(screen.getByRole("dialog").className).toContain("lumen-sheet--right");
  });
});

describe("Dialog/Sheet tones (W26 Figma grammar)", () => {
  it("Dialog defaults to the panel tone and marks the deep tone", () => {
    const { rerender } = render(
      <Dialog open onClose={() => {}} label="Settings window">
        <span>content</span>
      </Dialog>,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.className).toContain("lumen-dialog");
    expect(dialog).not.toHaveAttribute("data-tone");

    rerender(
      <Dialog open onClose={() => {}} label="Settings window" tone="deep">
        <span>content</span>
      </Dialog>,
    );
    expect(screen.getByRole("dialog")).toHaveAttribute("data-tone", "deep");
  });

  it("Sheet marks the deep tone alongside the side class", () => {
    render(
      <Sheet open onClose={() => {}} side="bottom" tone="deep" label="Queue">
        <span>content</span>
      </Sheet>,
    );
    const sheet = screen.getByRole("dialog");
    expect(sheet.className).toContain("lumen-sheet--bottom");
    expect(sheet).toHaveAttribute("data-tone", "deep");
  });
});

describe("Panel (square-edged app panel, W26 Figma grammar)", () => {
  it("renders a section with the pane surface class by default", () => {
    const { container } = render(<Panel>Sidebar</Panel>);
    const panel = container.querySelector("section.lumen-pane")!;
    expect(panel).toBeTruthy();
    expect(panel).not.toHaveAttribute("data-edges");
    expect(panel).not.toHaveAttribute("data-tone");
  });

  it("renders the requested semantic element", () => {
    const { container } = render(<Panel as="nav">Rail</Panel>);
    expect(container.querySelector("nav.lumen-pane")).toBeTruthy();
  });

  it("exposes hairline edge separators and the deep tone as data attributes", () => {
    const { container } = render(
      <Panel as="aside" edges="both" tone="deep">
        Sidebar
      </Panel>,
    );
    const panel = container.querySelector("aside.lumen-pane")!;
    expect(panel).toHaveAttribute("data-edges", "both");
    expect(panel).toHaveAttribute("data-tone", "deep");
  });
});

describe("Card (r12–14 bordered card, W26 Figma grammar)", () => {
  it("renders the flat card by default without a variant class", () => {
    const { container } = render(<Card>A playlist card</Card>);
    const card = container.querySelector("div.lumen-card")!;
    expect(card).toBeTruthy();
    expect(card.className).toBe("lumen-card");
  });

  it("applies variant classes and renders list-item cards", () => {
    const { container } = render(
      <>
        <Card variant="raised">Raised</Card>
        <Card variant="interactive">Interactive</Card>
        <Card variant="floating">Floating</Card>
        <Card as="li">Row</Card>
      </>,
    );
    expect(container.querySelector(".lumen-card--raised")).toBeTruthy();
    expect(container.querySelector(".lumen-card--interactive")).toBeTruthy();
    expect(container.querySelector(".lumen-card--floating")).toBeTruthy();
    expect(container.querySelector("li.lumen-card")).toBeTruthy();
  });
});

describe("Well (recessed inset surface)", () => {
  it("renders a div with the well class", () => {
    const { container } = render(<Well data-testid="well" />);
    expect(container.querySelector("div.lumen-well")).toBeTruthy();
  });
});

describe("Separator (1px hairline divider)", () => {
  it("is decorative (aria-hidden) and horizontal by default", () => {
    const { container } = render(<Separator />);
    const sep = container.querySelector("div.lumen-separator")!;
    expect(sep).toHaveAttribute("aria-hidden", "true");
    expect(sep).toHaveAttribute("data-orientation", "horizontal");
  });

  it("supports the vertical orientation", () => {
    const { container } = render(<Separator orientation="vertical" />);
    expect(container.querySelector(".lumen-separator")).toHaveAttribute(
      "data-orientation",
      "vertical",
    );
  });
});

describe("SectionHeader (15px semibold + optional thin rule)", () => {
  it("renders the title as a heading with no rule by default", () => {
    render(<SectionHeader>Appearance</SectionHeader>);
    const heading = screen.getByRole("heading", { name: "Appearance" });
    expect(heading.className).toContain("lumen-section-header__title");
    expect(heading.parentElement).not.toHaveAttribute("data-rule");
  });

  it("honours the host heading level, rule and trailing content", () => {
    render(
      <SectionHeader as="h3" rule trailing={<span>2 items</span>}>
        Startup
      </SectionHeader>,
    );
    const heading = screen.getByRole("heading", { level: 3, name: "Startup" });
    expect(heading.parentElement).toHaveAttribute("data-rule");
    expect(screen.getByText("2 items").parentElement!.className).toContain(
      "lumen-section-header__trailing",
    );
  });
});

describe("HeroPill (r30 hero surface)", () => {
  it("renders the pill surface without the ring by default", () => {
    const { container } = render(<HeroPill as="span">▶</HeroPill>);
    const pill = container.querySelector("span.lumen-hero-pill")!;
    expect(pill).toBeTruthy();
    expect(pill.className).toBe("lumen-hero-pill");
  });

  it("adds the accent selection ring class", () => {
    const { container } = render(<HeroPill ring />);
    expect(container.querySelector(".lumen-hero-pill--ring")).toBeTruthy();
  });
});
