import { fireEvent, render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { SwatchGroup } from "./SwatchGroup";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");

describe("SwatchGroup (Settings - global.svg accent row, wave 26)", () => {
  it("renders the 9 design accents in frame order (orange first)", () => {
    render(<SwatchGroup label="Accent color" value="orange" onChange={() => {}} />);
    const swatches = screen.getAllByRole("radio");
    expect(swatches).toHaveLength(9);
    expect(swatches[0]).toHaveAttribute("data-accent", "orange");
    expect(swatches[1]).toHaveAttribute("data-accent", "red");
    expect(swatches[2]).toHaveAttribute("data-accent", "yellow");
    expect(swatches[8]).toHaveAttribute("data-accent", "gray");
  });

  it("exposes radiogroup semantics; the selected swatch is checked + tabbable", () => {
    render(<SwatchGroup label="Accent color" value="blue" onChange={() => {}} />);
    expect(screen.getByRole("radiogroup", { name: "Accent color" })).toBeInTheDocument();
    const blue = screen.getByRole("radio", { name: "blue" });
    expect(blue).toHaveAttribute("aria-checked", "true");
    expect(blue).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: "orange" })).toHaveAttribute("tabindex", "-1");
  });

  it("selects an accent through onChange on click; re-click is a no-op", () => {
    const onChange = vi.fn();
    render(<SwatchGroup label="Accent color" value="orange" onChange={onChange} />);
    fireEvent.click(screen.getByRole("radio", { name: "violet" }));
    expect(onChange).toHaveBeenCalledWith("violet");
    fireEvent.click(screen.getByRole("radio", { name: "orange" }));
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("arrow keys move the selection with wrap-around", () => {
    const onChange = vi.fn();
    render(<SwatchGroup label="Accent color" value="orange" onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole("radiogroup"), { key: "ArrowLeft" });
    expect(onChange).toHaveBeenCalledWith("gray");
  });

  it("custom options override the default set (localized labels)", () => {
    render(
      <SwatchGroup
        label="Accent color"
        value="orange"
        onChange={() => {}}
        options={[
          { value: "orange", label: "Orange" },
          { value: "red", label: "Red" },
        ]}
      />,
    );
    expect(screen.getAllByRole("radio")).toHaveLength(2);
    expect(screen.getByRole("radio", { name: "Red" })).toBeInTheDocument();
  });

  it("DOM snapshot", () => {
    const { container } = render(<SwatchGroup label="Accent color" value="orange" onChange={() => {}} />);
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("SwatchGroup skin consumes the Settings - global.svg language", () => {
  it("surface track + control radius; selected slot accent-tinted", () => {
    expect(formCss).toMatch(/\.lumen-swatchgroup\s*\{[^}]*--color-surface[^}]*--radius-control/);
    expect(formCss).toMatch(
      /\.lumen-swatchgroup__swatch\[aria-checked="true"\]\s*\{[^}]*--color-accent-tinted/,
    );
  });

  it("every accent id has a frame-measured dot color (frame-marked hex block)", () => {
    for (const id of ["orange", "red", "yellow", "green", "cyan", "blue", "violet", "pink", "gray"]) {
      expect(formCss).toMatch(new RegExp(`\\.lumen-swatchgroup__swatch\\[data-accent="${id}"\\] \\.lumen-swatchgroup__dot`));
    }
    // the raw-hex block is frame-marked + stylelint-scoped (contract §4)
    expect(formCss).toMatch(/stylelint-disable color-no-hex -- frame: Settings - global\.svg/);
  });

  it("frame geometry: 20x20 dot, 32x32 hit target", () => {
    expect(formCss).toMatch(/\.lumen-swatchgroup__dot\s*\{[^}]*width: 20px;[^}]*height: 20px;/s);
    expect(formCss).toMatch(/\.lumen-swatchgroup__swatch\s*\{[^}]*width: 32px;[^}]*height: 32px;/s);
  });
});
