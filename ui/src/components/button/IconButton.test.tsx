import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { IconButton } from "./IconButton";

const Glyph = (): ReactElement => (
  <svg data-testid="glyph" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M8 5v14l11-7z" />
  </svg>
);

describe("IconButton", () => {
  it("exposes its label as the accessible name (doc 08 §4.5)", () => {
    render(
      <IconButton label="Play">
        <Glyph />
      </IconButton>,
    );
    expect(screen.getByRole("button", { name: "Play" })).toBeInTheDocument();
  });

  it("applies size modifiers and composes className", () => {
    render(
      <IconButton label="Stop" size="lg" className="extra">
        <Glyph />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Stop" });
    expect(button).toHaveClass("lumen-icon-button", "lumen-icon-button--lg", "extra");
  });

  it("renders the glyph and fires clicks", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Next" onClick={onClick}>
        <Glyph />
      </IconButton>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(onClick).toHaveBeenCalledOnce();
    expect(screen.getByTestId("glyph")).toBeInTheDocument();
  });

  it("disabled means disabled — no handlers fire", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Next" disabled onClick={onClick}>
        <Glyph />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Next" });
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("loading means inert: aria-busy + disabled", () => {
    const onClick = vi.fn();
    render(
      <IconButton label="Next" loading onClick={onClick}>
        <Glyph />
      </IconButton>,
    );
    const button = screen.getByRole("button", { name: "Next" });
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("DOM snapshot", () => {
    const { container } = render(
      <IconButton label="Snapshot">
        <Glyph />
      </IconButton>,
    );
    expect(container.firstChild).toMatchSnapshot();
  });
});
