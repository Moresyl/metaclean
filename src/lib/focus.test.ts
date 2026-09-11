import { describe, expect, it } from "vitest";
import { focusableElements, loopFocus } from "./focus";

describe("focus containment helpers", () => {
  it("returns only enabled interactive descendants", () => {
    const container = document.createElement("div");
    container.innerHTML = '<button>one</button><button disabled>two</button><a href="#">three</a>';
    expect(focusableElements(container).map((element) => element.textContent)).toEqual(["one", "three"]);
  });

  it("wraps forward and backward traversal at the dialog edges", () => {
    const container = document.createElement("div");
    const first = document.createElement("button");
    const last = document.createElement("button");
    container.append(first, last);
    document.body.append(container);

    last.focus();
    const forward = new KeyboardEvent("keydown", { key: "Tab", cancelable: true });
    loopFocus(forward, container);
    expect(forward.defaultPrevented).toBe(true);
    expect(first).toHaveFocus();

    first.focus();
    const backward = new KeyboardEvent("keydown", { key: "Tab", shiftKey: true, cancelable: true });
    loopFocus(backward, container);
    expect(backward.defaultPrevented).toBe(true);
    expect(last).toHaveFocus();
    container.remove();
  });

  it("moves focus into the container when the browser reports an outside target", () => {
    const container = document.createElement("div");
    const button = document.createElement("button");
    container.append(button);
    document.body.append(container);
    const outside = document.createElement("button");
    document.body.append(outside);
    outside.focus();

    const event = new KeyboardEvent("keydown", { key: "Tab", cancelable: true });
    loopFocus(event, container);
    expect(event.defaultPrevented).toBe(true);
    expect(button).toHaveFocus();
    container.remove();
    outside.remove();
  });
});
