import { act, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import SegmentedControl from "./SegmentedControl";

const options = [
  { value: "first", label: "First" },
  { value: "disabled", label: "Unavailable", disabled: true },
  { value: "last", label: "Last", icon: <svg aria-label="Decorative icon" /> },
];

describe("SegmentedControl", () => {
  it("changes a controlled choice without deselecting or activating disabled options", () => {
    const change = vi.fn();
    function Harness() {
      const [value, setValue] = useState("first");
      return <SegmentedControl label="Choices" options={options} value={value} onChange={next => { change(next); setValue(next); }} />;
    }
    render(<Harness />);
    const first = screen.getByRole("button", { name: "First" });
    const last = screen.getByRole("button", { name: "Last" });
    expect(screen.getByRole("group", { name: "Choices" })).toContainElement(first);
    expect(first).toHaveAttribute("aria-pressed", "true");
    expect(first).toHaveAttribute("tabindex", "0");
    expect(last).toHaveAttribute("tabindex", "-1");
    fireEvent.click(first);
    fireEvent.click(screen.getByRole("button", { name: "Unavailable" }));
    expect(change).not.toHaveBeenCalled();
    fireEvent.click(last);
    expect(change).toHaveBeenCalledExactlyOnceWith("last");
    expect(first).toHaveAttribute("aria-pressed", "false");
    expect(last).toHaveAttribute("aria-pressed", "true");
    expect(last).toHaveAttribute("tabindex", "0");
    expect(screen.queryByRole("img", { name: "Decorative icon" })).not.toBeInTheDocument();
  });

  it("moves focus without selection, skips disabled choices and stops at either end", () => {
    const change = vi.fn();
    render(<SegmentedControl label="Choices" options={options} value="first" onChange={change} />);
    const first = screen.getByRole("button", { name: "First" });
    const last = screen.getByRole("button", { name: "Last" });
    first.focus();
    fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "ArrowRight" });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "Home" });
    expect(first).toHaveFocus();
    fireEvent.keyDown(first, { key: "ArrowLeft" });
    expect(first).toHaveFocus();
    fireEvent.keyDown(first, { key: "End" });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "ArrowLeft", ctrlKey: true });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "Tab" });
    fireEvent.keyDown(last, { key: "ArrowUp" });
    expect(first).toHaveFocus();
    fireEvent.keyDown(first, { key: "ArrowDown" });
    expect(last).toHaveFocus();
    expect(change).not.toHaveBeenCalled();
  });

  it("reverses horizontal keyboard movement in right-to-left groups", () => {
    render(<SegmentedControl className="rtl" label="Choices" options={options} value="first" onChange={vi.fn()} />);
    screen.getByRole("group").style.direction = "rtl";
    const first = screen.getByRole("button", { name: "First" });
    const last = screen.getByRole("button", { name: "Last" });
    first.focus();
    fireEvent.keyDown(first, { key: "ArrowLeft" });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "ArrowRight" });
    expect(first).toHaveFocus();
  });

  it("keeps an enabled keyboard entry when the selected value disappears or becomes disabled", () => {
    const { rerender } = render(<SegmentedControl label="Choices" options={options} value="missing" onChange={vi.fn()} />);
    expect(screen.getByRole("button", { name: "First" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("group").querySelector(".segment-thumb")).not.toBeVisible();
    rerender(<SegmentedControl label="Choices" options={options} value="disabled" onChange={vi.fn()} />);
    expect(screen.getByRole("button", { name: "First" })).toHaveAttribute("tabindex", "0");
    rerender(<SegmentedControl label="Choices" options={options.map(option => ({ ...option, disabled: true }))} value="first" onChange={vi.fn()} />);
    expect(screen.getAllByRole("button").every(button => button.tabIndex === -1)).toBe(true);
  });

  it.each([true, false])("realigns on resize and reveals a clipped selection with reduced motion %s", reduced => {
    let resize: (() => void) | undefined;
    const observe = vi.fn();
    const disconnect = vi.fn();
    vi.stubGlobal("ResizeObserver", class {
      constructor(callback: () => void) { resize = callback; }
      observe = observe;
      disconnect = disconnect;
    });
    vi.stubGlobal("matchMedia", () => ({ matches: reduced }));
    try {
      const { unmount } = render(<SegmentedControl label="Choices" options={options} value="last" onChange={vi.fn()} />);
      const group = screen.getByRole("group");
      const selected = screen.getByRole("button", { name: "Last" });
      const indicator = group.querySelector<HTMLElement>(".segment-thumb")!;
      Object.defineProperties(group, { clientWidth: { value: 100 }, scrollWidth: { value: 250 } });
      Object.defineProperties(selected, { offsetWidth: { value: 80, configurable: true }, offsetLeft: { value: 170 } });
      vi.spyOn(group, "getBoundingClientRect").mockReturnValue({ left: 0, right: 100, width: 100 } as DOMRect);
      vi.spyOn(selected, "getBoundingClientRect").mockReturnValue({ left: 170, right: 250, width: 80 } as DOMRect);
      const scroll = vi.fn();
      group.scrollBy = scroll;
      indicator.dataset.ready = "true";
      act(() => resize?.());
      expect(indicator.style.width).toBe("80px");
      expect(indicator.style.transform).toBe("translateX(170px)");
      expect(scroll).toHaveBeenCalledExactlyOnceWith({ left: 160, behavior: reduced ? "instant" : "smooth" });
      expect(observe).toHaveBeenCalledTimes(4);
      Object.defineProperty(selected, "offsetWidth", { value: 88 });
      act(() => resize?.());
      expect(indicator.style.width).toBe("88px");
      unmount();
      expect(disconnect).toHaveBeenCalledOnce();
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
