import { render, screen } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ValueChip } from "./ValueChip";

const { dirname } = import.meta as ImportMeta & { dirname?: string };
const formCss = readFileSync(`${dirname}/form.css`, "utf8");

describe("ValueChip (EQ value readout, wave 26)", () => {
  it("renders the value as its accessible content", () => {
    render(<ValueChip value="12.0 dB" />);
    expect(screen.getByText("12.0 dB")).toBeInTheDocument();
  });

  it("accepts a composition class", () => {
    const { container } = render(<ValueChip value="200%" className="eq-row__value" />);
    expect(container.querySelector(".lumen-valuechip.eq-row__value")).not.toBeNull();
  });

  it("DOM snapshot", () => {
    const { container } = render(<ValueChip value="0.0 dB" />);
    expect(container.firstChild).toMatchSnapshot();
  });
});

describe("ValueChip skin consumes the audio-value language", () => {
  it("recessed deep well, control radius, bright tabular value text", () => {
    expect(formCss).toMatch(
      /\.lumen-valuechip\s*\{[^}]*--color-content-bright[^}]*--color-surface-deep[^}]*--radius-control/,
    );
    expect(formCss).toMatch(/\.lumen-valuechip\s*\{[^}]*font-variant-numeric: tabular-nums/);
  });
});
