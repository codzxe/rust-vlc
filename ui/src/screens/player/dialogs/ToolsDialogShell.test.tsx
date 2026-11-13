// ToolsDialogShell tests (w26.a18) — pins the generic shell the wave-c
// audio/video tool lanes mount into: dialog chrome (role, labelled title,
// Escape + scrim-click + ✕ close), the body/footer slots, and the tool
// card skeleton (collapsed by default per the frames, aria wiring, the
// label-left/control-right row and the value chip it exposes).
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ToolsCard, ToolsDialogShell, ToolsRow, ToolsValueChip } from "./ToolsDialogShell";
// w26.k09: the shell's copy renders through useMsg (player.tools.* are
// central); TOOLS_DIALOG_COPY consolidated into useDialogsMsg.ts.
import { TOOLS_DIALOG_COPY } from "./useDialogsMsg";

function renderShell(props: Partial<Parameters<typeof ToolsDialogShell>[0]> = {}) {
  const onClose = vi.fn();
  render(
    <ToolsDialogShell
      open
      onClose={onClose}
      title="Equalizer"
      subtitle="Shape the audio bands."
      {...props}
    >
      <p>body-slot</p>
    </ToolsDialogShell>,
  );
  return { onClose };
}

describe("ToolsDialogShell — dialog chrome", () => {
  it("renders the labelled dialog with body + subtitle + footer slots", () => {
    renderShell({ footer: <button type="button">Reset</button> });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Equalizer" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Shape the audio bands.")).toBeInTheDocument();
    expect(screen.getByText("body-slot")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reset" })).toBeInTheDocument();
  });

  it("Escape closes through the shared Dialog focus trap", () => {
    const { onClose } = renderShell();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledWith("escape");
  });

  it("scrim click closes (closeOnOutsideClick — Figma scrim grammar)", () => {
    const { onClose } = renderShell();
    // The scrim is the dialog's parent .lumen-scrim element.
    const dialog = screen.getByRole("dialog");
    const scrim = dialog.parentElement as HTMLElement;
    fireEvent.click(scrim);
    expect(onClose).toHaveBeenCalledWith("outside");
  });

  it("the header ✕ closes", () => {
    const { onClose } = renderShell();
    fireEvent.click(screen.getByRole("button", { name: TOOLS_DIALOG_COPY.close }));
    expect(onClose).toHaveBeenCalledWith("escape");
  });
});

describe("ToolsCard — the collapsible card skeleton", () => {
  it("ships collapsed (frames) and expands on toggle click", () => {
    render(
      <ToolsCard>
        <ToolsRow label="Preamp">
          <ToolsValueChip>12.0 dB</ToolsValueChip>
        </ToolsRow>
      </ToolsCard>,
    );
    const toggle = screen.getByRole("button", {
      name: TOOLS_DIALOG_COPY.expand,
    });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    // Collapsed: the body is hidden but stays wired to aria-controls.
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: TOOLS_DIALOG_COPY.collapse })).toBe(toggle);
    expect(body).not.toHaveAttribute("hidden");
    expect(screen.getByText("Preamp")).toBeInTheDocument();
    expect(screen.getByText("12.0 dB")).toBeInTheDocument();
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<ToolsCard defaultOpen>rows</ToolsCard>);
    expect(
      screen.getByRole("button", { name: TOOLS_DIALOG_COPY.collapse }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("ToolsRow / ToolsValueChip — the row grammar", () => {
  it("renders label left slot and control right slot with an optional hint", () => {
    render(
      <ToolsRow label="Saturation" hint="0 is neutral" htmlFor="sat">
        <ToolsValueChip title="1.00">1.00</ToolsValueChip>
      </ToolsRow>,
    );
    expect(screen.getByText("Saturation")).toBeInTheDocument();
    expect(screen.getByText("0 is neutral")).toBeInTheDocument();
    const chip = screen.getByText("1.00");
    expect(chip).toHaveAttribute("title", "1.00");
  });
});
