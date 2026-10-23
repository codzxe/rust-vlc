import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { TextField } from "./TextField";

describe("TextField", () => {
  it("wires label, input and typing", () => {
    const onChange = vi.fn();
    render(<TextField label="Network URL" onChange={onChange} />);
    const input = screen.getByLabelText("Network URL");
    expect(input).toHaveAttribute("type", "text");
    fireEvent.change(input, { target: { value: "https://example.org/stream" } });
    expect(onChange).toHaveBeenCalledOnce();
  });

  it("error state: aria-invalid + aria-describedby point at the message", () => {
    render(<TextField label="Name" error="Enter a folder name" />);
    const input = screen.getByLabelText("Name");
    expect(input).toHaveAttribute("aria-invalid", "true");
    const describedBy = input.getAttribute("aria-describedby");
    expect(describedBy).toBeTruthy();
    expect(screen.getByText("Enter a folder name")).toHaveAttribute("id", describedBy);
  });

  it("no error state by default", () => {
    render(<TextField label="Name" />);
    expect(screen.getByLabelText("Name")).not.toHaveAttribute("aria-invalid");
    expect(screen.queryByText(/error/i)).toBeNull();
  });

  it("disabled means disabled — native disabled blocks user input (browser-gated)", () => {
    render(<TextField label="Name" disabled onChange={() => {}} />);
    // The disabled attribute IS the contract: browsers never deliver user
    // events to disabled controls. (fireEvent bypasses that gate, so we
    // assert the attribute rather than fake a dispatch.)
    expect(screen.getByLabelText("Name")).toBeDisabled();
  });

  it("hideLabel keeps the accessible name", () => {
    render(<TextField label="Search" hideLabel />);
    expect(screen.getByLabelText("Search")).toBeInTheDocument();
  });

  it("forwards the ref + DOM snapshot", () => {
    const ref = createRef<HTMLInputElement>();
    const { container } = render(<TextField ref={ref} label="Snapshot" error="boom" />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
    expect(container.firstChild).toMatchSnapshot();
  });
});
