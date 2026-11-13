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

/**
 * w26.k04 (j04-1 P1 + j04-3) — Escape ownership: OPEN popups yield, in-flow
 * listboxes never do. The harness reproduces both DOM shapes of the real
 * app inside ONE dialog: a combobox popup (trigger + conditionally rendered
 * listbox — the Dropdown pattern) and a persistent in-flow listbox (the
 * SubtitlePickList pattern of the AddSubtitlesDialog Folder-2/3 candidate
 * states, which is always present and never an open popup).
 */
function PickerHarness({ onClose }: { onClose: () => void }) {
  const [open, setOpen] = useState(false);
  const [listOpen, setListOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Open picker</button>
      <Dialog
        open={open}
        onClose={() => {
          onClose();
          setOpen(false);
        }}
        label="Picker"
      >
        <button aria-haspopup="listbox" aria-expanded={listOpen} onClick={() => setListOpen((v) => !v)}>
          Destination
        </button>
        {listOpen ? (
          <div
            role="listbox"
            aria-label="Destination"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                setListOpen(false);
              }
            }}
          >
            <button role="option" aria-selected="true">To This File</button>
          </div>
        ) : null}
        {/* Persistent in-flow listbox — present with candidates, never an
         * open popup. This is the node that used to wedge Escape shut. */}
        <div role="listbox" aria-label="Candidates">
          <button role="option">one.srt</button>
          <button role="option">two.srt</button>
        </div>
        <button>Save</button>
      </Dialog>
    </>
  );
}

describe("Dialog Escape ownership (w26.g19 root fix + w26.k04 j04-1)", () => {
  it("first Escape closes only the open popup — the surface survives (j04-3)", () => {
    const onClose = vi.fn();
    render(<PickerHarness onClose={onClose} />);
    const opener = screen.getByRole("button", { name: "Open picker" });
    opener.focus(); // jsdom: click alone does not focus (browsers do)
    fireEvent.click(opener);
    fireEvent.click(screen.getByRole("button", { name: "Destination" }));
    // The trigger now reports the open popup; the popup node is in the DOM.
    expect(screen.getByRole("button", { name: "Destination" })).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("listbox", { name: "Destination" })).not.toBeNull();

    // Trap must YIELD: the popup's own handler consumes the key. This exact
    // assertion failed before w26.k04 whenever an in-flow listbox was also
    // present — the bare-role selector could not tell the two apart.
    fireEvent.keyDown(screen.getByRole("listbox", { name: "Destination" }), { key: "Escape" });
    expect(screen.queryByRole("listbox", { name: "Destination" })).toBeNull();
    expect(screen.getByRole("dialog", { name: "Picker" })).not.toBeNull();
    expect(onClose).not.toHaveBeenCalled();

    // With no popup open, the next Escape closes the dialog (and restores
    // focus to the opener).
    fireEvent.keyDown(screen.getByRole("button", { name: "Save" }), { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Open picker" }));
  });

  it("a persistent in-flow listbox never wedges Escape (j04-1 P1 regression)", () => {
    const onClose = vi.fn();
    render(<PickerHarness onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Open picker" }));
    // No popup is open — only the in-flow candidates listbox (plus the
    // SubtitlePickList-shaped DOM the trap used to yield to forever).
    expect(screen.getByRole("listbox", { name: "Candidates" })).not.toBeNull();

    // (i) focus outside the listbox: Escape closes on the FIRST press.
    fireEvent.keyDown(screen.getByRole("button", { name: "Save" }), { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);

    // (ii) focus INSIDE the in-flow listbox (user tabbed into the options):
    // still no yield — an in-flow listbox does not handle Escape itself.
    fireEvent.click(screen.getByRole("button", { name: "Open picker" }));
    const option = screen.getByRole("option", { name: "one.srt" });
    option.focus();
    fireEvent.keyDown(option, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("a focused popup outside the surface subtree owns the first Escape (portal/fixed popups)", () => {
    const onClose = vi.fn();
    render(<PickerHarness onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Open picker" }));

    // A second RTL render mounts OUTSIDE the dialog's DOM tree — the honest
    // shape of a portal/fixed popup (context menus, ported dropdowns).
    const portal = render(
      <div role="menu" aria-label="Canvas menu">
        <button>Play</button>
      </div>,
    );
    const portalItem = screen.getByRole("button", { name: "Play" });
    portalItem.focus();

    // While focus lives in the out-of-tree popup, the trap yields.
    fireEvent.keyDown(screen.getByRole("button", { name: "Save" }), { key: "Escape" });
    expect(screen.getByRole("dialog", { name: "Picker" })).not.toBeNull();
    expect(onClose).not.toHaveBeenCalled();

    // Once focus returns inside the surface, Escape closes the dialog.
    screen.getByRole("button", { name: "Save" }).focus();
    fireEvent.keyDown(screen.getByRole("button", { name: "Save" }), { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
    portal.unmount();
  });

  it("Escape during IME composition is ignored (j04-11)", () => {
    const onClose = vi.fn();
    render(<PickerHarness onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Open picker" }));
    fireEvent.keyDown(screen.getByRole("button", { name: "Save" }), {
      key: "Escape",
      isComposing: true,
    });
    expect(screen.getByRole("dialog", { name: "Picker" })).not.toBeNull();
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(screen.getByRole("button", { name: "Save" }), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("Dialog trap edge cases (w26.k04 — review §4 gaps)", () => {
  it("skips [hidden] subtrees while cycling Tab", () => {
    function HiddenHarness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <button onClick={() => setOpen(true)}>Open</button>
          <Dialog open={open} onClose={() => setOpen(false)} label="Hidden">
            <button>First</button>
            <div hidden>
              <button>Hidden action</button>
            </div>
            <button>Last</button>
          </Dialog>
        </>
      );
    }
    render(<HiddenHarness />);
    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    fireEvent.keyDown(document.activeElement!, { key: "Tab" });
    // Skips the hidden subtree entirely.
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Last" }));
    fireEvent.keyDown(document.activeElement!, { key: "Tab" });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "First" }));
  });

  it("with zero focusables the surface itself holds focus through Tab", () => {
    const { container } = render(
      <Dialog open onClose={() => {}} label="Empty trap">
        <span>no controls here</span>
      </Dialog>,
    );
    const dialog = screen.getByRole("dialog", { name: "Empty trap" }) as HTMLDivElement;
    expect(document.activeElement).toBe(dialog);
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(document.activeElement).toBe(dialog);
    expect(container.querySelector("[hidden]")).toBeNull();
  });

  it("cycles to natively focusable [contenteditable]/summary nodes (j04-11)", () => {
    const { container } = render(
      <Dialog open onClose={() => {}} label="Rich">
        <button>First</button>
        <div contentEditable tabIndex={-1} role="textbox" aria-label="Note" />
        <details>
          <summary>More</summary>
          <p>detail</p>
        </details>
      </Dialog>,
    );
    const first = screen.getByRole("button", { name: "First" });
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(first, { key: "Tab" });
    expect(document.activeElement).toBe(screen.getByRole("textbox", { name: "Note" }));
    fireEvent.keyDown(document.activeElement!, { key: "Tab" });
    expect(document.activeElement).toBe(container.querySelector("summary"));
  });

  it("renders an empty sr-title and warns in dev when neither labelledBy nor label is set (j04-13)", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { container } = render(
      <Dialog open onClose={() => {}}>
        <button>OK</button>
      </Dialog>,
    );
    const srTitle = container.querySelector(".lumen-surface__sr-title")!;
    expect(srTitle).not.toBeNull();
    expect(srTitle.textContent).toBe("");
    expect(warn).toHaveBeenCalledTimes(1);
    warn.mockRestore();
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
