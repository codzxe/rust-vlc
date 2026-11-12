// GraphicEQ tests (w26.f05) — pins the A2 vertical-fader EQUALIZER panel
// per EQ.svg / the Settings - audio-1.svg embed: the morphing bar↔panel
// toggle ("Open/Close Equalizer"), the 11 fader columns (Preamp + 10 bands,
// VERTICAL axis, monochrome — no accent dots, w26.e07 F8), the 0 dB tick
// class on band faders only, chips, and the controlled state seam the
// settings preset row drives.
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { keys as laneKeys } from "../../../../i18n/keys/w26.f05";
import { GraphicEQ } from "./GraphicEQ";
import { DEFAULT_EQUALIZER_STATE } from "./presets";

const OPEN = laneKeys["screen.eq.open-panel"];
const CLOSE = laneKeys["screen.eq.close-panel"];

describe("GraphicEQ — the morphing bar↔panel toggle (frame: EQ.svg)", () => {
  it("ships collapsed: the 36px 'Open Equalizer' bar, faders hidden but wired", () => {
    render(<GraphicEQ />);
    const toggle = screen.getByRole("button", { name: OPEN });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(body).toHaveAttribute("role", "group");
    // Collapsed bar exposes no faders to the a11y tree.
    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("the panel is a named region (settings area mount)", () => {
    render(<GraphicEQ />);
    expect(screen.getByRole("region", { name: "Equalizer" })).toBeInTheDocument();
  });

  it("expand reveals the faders; collapse hides them again", () => {
    render(<GraphicEQ />);
    const toggle = screen.getByRole("button", { name: OPEN });
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("button", { name: CLOSE })).toBe(toggle);
    expect(screen.getAllByRole("slider")).toHaveLength(11);
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("slider")).toBeNull();
  });

  it("defaultOpen mounts the panel expanded ('Close Equalizer')", () => {
    render(<GraphicEQ defaultOpen />);
    expect(screen.getByRole("button", { name: CLOSE })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("controlled open/onOpenChange drives the toggle (tools-dialog mount)", () => {
    const onOpenChange = vi.fn();
    const { rerender } = render(
      <GraphicEQ open={false} onOpenChange={onOpenChange} defaultOpen />,
    );
    // Controlled wins over defaultOpen: still collapsed.
    expect(screen.getByRole("button", { name: OPEN })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    fireEvent.click(screen.getByRole("button", { name: OPEN }));
    expect(onOpenChange).toHaveBeenCalledWith(true);
    rerender(<GraphicEQ open onOpenChange={onOpenChange} defaultOpen />);
    expect(screen.getByRole("button", { name: CLOSE })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });
});

describe("GraphicEQ — fader columns (frame: 11 columns, pitch 68)", () => {
  it("renders Preamp + the 10 frame bands as VERTICAL faders (EQ.svg)", () => {
    render(<GraphicEQ defaultOpen />);
    expect(screen.getByRole("slider", { name: "Preamp" })).toHaveAttribute(
      "aria-orientation",
      "vertical",
    );
    for (const label of [
      "60 Hz",
      "170 Hz",
      "310 Hz",
      "600 Hz",
      "1 KHz",
      "3 KHz",
      "6 KHz",
      "12 KHz",
      "14 KHz",
      "16 KHz",
    ]) {
      expect(screen.getByRole("slider", { name: label })).toHaveAttribute(
        "aria-orientation",
        "vertical",
      );
    }
    expect(screen.getAllByRole("slider")).toHaveLength(11);
  });

  it("renders the frame chips: preamp 12.0 dB, bands 0.0 dB", () => {
    render(<GraphicEQ defaultOpen />);
    expect(screen.getAllByText("12.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("0.0 dB")).toHaveLength(10);
  });

  it("carries the ±20 dB range and tabular aria-valuetext (doc 08 §4.5)", () => {
    render(<GraphicEQ defaultOpen />);
    const preamp = screen.getByRole("slider", { name: "Preamp" });
    expect(preamp).toHaveAttribute("aria-valuemin", "-20");
    expect(preamp).toHaveAttribute("aria-valuemax", "20");
    expect(preamp).toHaveAttribute("aria-valuetext", "12.0 dB");
    expect(screen.getByRole("slider", { name: "60 Hz" })).toHaveAttribute(
      "aria-valuetext",
      "0.0 dB",
    );
  });

  it("the 0 dB tick rides the 10 band faders, not Preamp (frame Vector 60)", () => {
    const { container } = render(<GraphicEQ defaultOpen />);
    const zero = container.querySelectorAll(".eq-panel__fader--zero");
    expect(zero).toHaveLength(10);
    // The preamp column fader carries no tick class.
    const preampCol = screen.getByRole("slider", { name: "Preamp" }).closest(
      ".eq-panel__col",
    );
    expect(preampCol?.querySelector(".eq-panel__fader--zero")).toBeNull();
  });

  it("monochrome columns — NO band accent dots anywhere (w26.e07 F8)", () => {
    const { container } = render(<GraphicEQ defaultOpen />);
    expect(container.querySelector(".eq-tool__accent")).toBeNull();
    expect(container.querySelector("[data-accent]")).toBeNull();
  });
});

describe("GraphicEQ — the state seam (host-owned mounts)", () => {
  it("controlled state drives the faders (the preset row's shape)", () => {
    render(
      <GraphicEQ
        defaultOpen
        state={{ preamp: 6, gains: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0], preset: "none" }}
      />,
    );
    expect(screen.getAllByText("6.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("3.0 dB")).toHaveLength(1);
  });

  it("defaultState seeds an uncontrolled panel (hydration seam)", () => {
    render(
      <GraphicEQ
        defaultOpen
        defaultState={{ preamp: 6, gains: [3, 0, 0, 0, 0, 0, 0, 0, 0, 0] }}
      />,
    );
    expect(screen.getAllByText("6.0 dB")).toHaveLength(1);
    expect(screen.getAllByText("3.0 dB")).toHaveLength(1);
  });

  it("a fader commit fires onChange with the full state (ArrowUp = up)", () => {
    const onChange = vi.fn();
    render(<GraphicEQ defaultOpen onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "60 Hz" }), {
      key: "ArrowUp",
    });
    expect(screen.getAllByText("1.0 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      ...DEFAULT_EQUALIZER_STATE,
      gains: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
  });

  it("a preamp commit updates its chip and the seam payload", () => {
    const onChange = vi.fn();
    render(<GraphicEQ defaultOpen onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("slider", { name: "Preamp" }), {
      key: "ArrowDown",
    });
    expect(screen.getAllByText("11.0 dB")).toHaveLength(1);
    expect(onChange).toHaveBeenLastCalledWith({
      ...DEFAULT_EQUALIZER_STATE,
      preamp: 11,
    });
  });
});
