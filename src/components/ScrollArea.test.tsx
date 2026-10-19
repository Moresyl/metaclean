import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import ScrollArea from "./ScrollArea";

afterEach(() => vi.unstubAllGlobals());

it("tracks hidden content after scrolling, content resizing and viewport resizing", () => {
  let resized!: ResizeObserverCallback;
  const disconnect = vi.fn();
  vi.stubGlobal("ResizeObserver", class {
    constructor(callback: ResizeObserverCallback) { resized = callback; }
    observe = vi.fn();
    disconnect = disconnect;
  });
  const view = render(<ScrollArea label="Properties"><p>Selectable contents</p></ScrollArea>);
  const area = screen.getByRole("region", { name: "Properties" });
  expect(area).toHaveAttribute("tabindex", "0");
  expect(view.container.querySelector("[data-scroll-edge]")).toBeNull();
  Object.defineProperties(area, { scrollHeight: { value: 400, configurable: true }, clientHeight: { value: 100, configurable: true } });
  act(() => resized([], {} as ResizeObserver));
  expect(view.container.querySelector('[data-scroll-edge="bottom"]')).toHaveAttribute("aria-hidden", "true");
  expect(view.container.querySelector('[data-scroll-edge="top"]')).toBeNull();
  fireEvent.scroll(area, { target: { scrollTop: 150 } });
  expect(view.container.querySelectorAll("[data-scroll-edge]")).toHaveLength(2);
  fireEvent.scroll(area, { target: { scrollTop: 299.5 } });
  expect(view.container.querySelector('[data-scroll-edge="bottom"]')).toBeNull();
  fireEvent.scroll(area, { target: { scrollTop: -20 } });
  expect(view.container.querySelector('[data-scroll-edge="top"]')).toBeNull();
  Object.defineProperty(area, "clientHeight", { value: 500 });
  fireEvent.resize(window);
  expect(view.container.querySelector("[data-scroll-edge]")).toBeNull();
  view.unmount();
  expect(disconnect).toHaveBeenCalled();
});

it("remeasures changed content when ResizeObserver is unavailable", () => {
  vi.stubGlobal("ResizeObserver", undefined);
  const view = render(<ScrollArea label="Properties">Short</ScrollArea>);
  const area = screen.getByRole("region");
  Object.defineProperties(area, { scrollHeight: { value: 200 }, clientHeight: { value: 50 } });
  view.rerender(<ScrollArea label="Properties">Longer contents</ScrollArea>);
  expect(view.container.querySelector('[data-scroll-edge="bottom"]')).not.toBeNull();
  expect(area).toHaveTextContent("Longer contents");
});
