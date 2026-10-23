import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Dialog, Sheet } from "./Surface";

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
