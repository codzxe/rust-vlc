// ColorsTool tests (w26.c4) — pins the Colors card against its frame
// ("Video - Colors.svg"): collapsed-by-default card + toggle, the eleven
// frame rows (two hex+swatch+toggle rows, two label-only sliders, five
// toggle lines, the Gradient sub-dropdown, the Sepia slider), separators,
// local-state interactions, and the honest staged markers.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ColorsTool } from "./ColorsTool";

function expand(): HTMLElement {
  const toggle = screen.getByRole("button", { name: "Expand Settings" });
  fireEvent.click(toggle);
  return toggle;
}

describe("ColorsTool — card shell", () => {
  it("ships collapsed (frame state) with the expand toggle", () => {
    render(<ColorsTool />);
    const toggle = screen.getByRole("button", { name: "Expand Settings" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).toHaveAttribute("hidden");
    expect(screen.queryByRole("slider", { name: "Saturation" })).not.toBeInTheDocument();
  });

  it("expands on toggle click and collapses back", () => {
    render(<ColorsTool />);
    const toggle = expand();
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toBe(toggle);
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toHaveAttribute("hidden");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(body).toHaveAttribute("hidden");
  });

  it("defaultOpen mounts the card expanded", () => {
    render(<ColorsTool defaultOpen />);
    expect(
      screen.getByRole("button", { name: "Collapse Settings" }),
    ).toHaveAttribute("aria-expanded", "true");
  });
});

describe("ColorsTool — frame rows", () => {
  it("renders the two hex rows: label, #FF0000 input, swatch, OFF switch", () => {
    render(<ColorsTool defaultOpen />);
    for (const label of ["Color Extraction", "Color Threshold"]) {
      const input = screen.getByRole("textbox", { name: label });
      expect(input).toHaveValue("#FF0000");
      expect(screen.getByRole("switch", { name: label })).toHaveAttribute(
        "aria-checked",
        "false",
      );
    }
    const swatches = document.querySelectorAll(".video-hex__swatch");
    expect(swatches).toHaveLength(2);
    expect(swatches[0]).toHaveStyle({ backgroundColor: "#FF0000" });
  });

  it("renders the three sliders with staged ranges and valuetext", () => {
    render(<ColorsTool defaultOpen />);
    const saturation = screen.getByRole("slider", { name: "Saturation" });
    expect(saturation).toHaveAttribute("aria-valuemin", "0");
    expect(saturation).toHaveAttribute("aria-valuemax", "100");
    // w26.i08: staged initial calibrated through the shared flush travel so
    // the PAINTED thumb lands on the frame mark (20.73%) — the value itself
    // reads 21.6 (aria-valuetext rounds to "22 %").
    expect(saturation).toHaveAttribute("aria-valuenow", "21.6");
    expect(saturation).toHaveAttribute("aria-valuetext", "22 %");
    expect(screen.getByRole("slider", { name: "Similarity" })).toHaveAttribute(
      "aria-valuetext",
      "22 %",
    );
    expect(
      screen.getByRole("slider", { name: "Sepia Intensity" }),
    ).toHaveAttribute("aria-valuetext", "35 %");
  });

  it("only the Sepia slider carries degree ticks (frame)", () => {
    const { container } = render(<ColorsTool defaultOpen />);
    expect(
      container.querySelector('[data-row="sepia"] .dynamics-slider__ticks'),
    ).not.toBeNull();
    expect(
      container.querySelectorAll('[data-row="sepia"] .dynamics-slider__tick'),
    ).toHaveLength(2);
    expect(container.querySelector('[data-row="saturation"] .dynamics-slider__ticks')).toBeNull();
    expect(container.querySelector('[data-row="similarity"] .dynamics-slider__ticks')).toBeNull();
  });

  it("renders the five toggle lines, all OFF (frame)", () => {
    render(<ColorsTool defaultOpen />);
    for (const label of [
      "Negate Colors",
      "Posterize",
      "Gradient",
      "Color",
      "Cartoon",
    ]) {
      expect(screen.getByRole("switch", { name: label })).toHaveAttribute(
        "aria-checked",
        "false",
      );
    }
  });

  it("sections the rows with five full-width separators (frame)", () => {
    const { container } = render(<ColorsTool defaultOpen />);
    const seps = [...container.querySelectorAll(".video-sep")];
    expect(seps).toHaveLength(5);
    // w26.i08: per-separator modifiers carry the frame's varied cadence
    // (video.css pins each gap — h15-P2/g17-P2-1).
    expect(seps.map((sep) => sep.className)).toEqual([
      "video-sep video-sep--1",
      "video-sep video-sep--2",
      "video-sep video-sep--3",
      "video-sep video-sep--4",
      "video-sep video-sep--5",
    ]);
  });
});

describe("ColorsTool — local state", () => {
  it("hex typing updates the field and the swatch fill", () => {
    render(<ColorsTool defaultOpen />);
    const input = screen.getByRole("textbox", { name: "Color Extraction" });
    fireEvent.change(input, { target: { value: "#00AA33" } });
    expect(input).toHaveValue("#00AA33");
    expect(document.querySelector(".video-hex__swatch")).toHaveStyle({
      backgroundColor: "#00AA33",
    });
  });

  it("an invalid hex keeps the swatch honest (no fill) until valid again", () => {
    render(<ColorsTool defaultOpen />);
    const input = screen.getByRole("textbox", { name: "Color Threshold" });
    fireEvent.change(input, { target: { value: "#00" } });
    expect(input).toHaveValue("#00");
    // Invalid partial hex → no swatch fill (no background in the inline style).
    const swatches = document.querySelectorAll(".video-hex__swatch");
    const thresholdSwatch = swatches[1] ?? swatches[0];
    expect(thresholdSwatch?.getAttribute("style") ?? "").not.toMatch(/background/i);
    fireEvent.change(input, { target: { value: "#0000FF" } });
    expect(thresholdSwatch).toHaveStyle({ backgroundColor: "#0000FF" });
  });

  it("toggling a line flips its switch", () => {
    render(<ColorsTool defaultOpen />);
    const negate = screen.getByRole("switch", { name: "Negate Colors" });
    fireEvent.click(negate);
    expect(negate).toHaveAttribute("aria-checked", "true");
  });

  it("keyboard commit on a slider updates its valuetext", () => {
    render(<ColorsTool defaultOpen />);
    const similarity = screen.getByRole("slider", { name: "Similarity" });
    fireEvent.keyDown(similarity, { key: "ArrowRight" });
    expect(similarity).toHaveAttribute("aria-valuetext", "23 %");
  });

  it("the Gradient sub-dropdown opens and selects a mode", () => {
    render(<ColorsTool defaultOpen />);
    const trigger = screen.getByRole("button", { name: "Gradient mode" });
    expect(trigger).toHaveTextContent("Gradient"); // frame: selected mode
    fireEvent.click(trigger);
    const list = screen.getByRole("listbox", { name: "Gradient mode" });
    expect(list).toBeInTheDocument();
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(3);
    fireEvent.click(screen.getByRole("option", { name: "Edge" }));
    expect(trigger).toHaveTextContent("Edge");
  });

  it("adds no escape markers in jsdom — the shared placement holds (w26.i08)", async () => {
    // Zero-geometry jsdom degrades to the shared Dropdown's in-flow default
    // (no data-list-escape / data-list-direction on the wrapper — the
    // live clip-box decision is unit-pinned in
    // useDropdownScrollportFlip.test.ts + Playwright-probed).
    render(<ColorsTool defaultOpen />);
    const wrapper = document.querySelector(".video-subdropdown");
    expect(wrapper).not.toBeNull();
    expect(wrapper?.getAttribute("data-list-direction")).toBeNull();
    expect(wrapper?.getAttribute("data-list-escape")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Gradient mode" }));
    await waitFor(() =>
      expect(screen.getByRole("listbox", { name: "Gradient mode" })).toBeInTheDocument(),
    );
    await waitFor(() => {
      expect(wrapper?.getAttribute("data-list-direction")).toBeNull();
      expect(wrapper?.getAttribute("data-list-escape")).toBeNull();
    });
  });
});

describe("ColorsTool — honest staging (effects-area pattern)", () => {
  it("marks the card staged with the pending command marker", () => {
    render(<ColorsTool defaultOpen />);
    const card = screen.getByLabelText("Video colors");
    expect(card).toHaveAttribute("data-staged-tool", "colors");
    expect(card).toHaveAttribute("data-pending-command", "video.set-colors");
    const note = screen.getByRole("note");
    expect(note.textContent).toMatch(/staged/i);
  });
});
