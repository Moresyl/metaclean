import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import CommandPalette, { score, type Command } from "./CommandPalette";
import ContextMenu, { useContextMenu, type MenuEntry } from "./ContextMenu";
import FileQueue from "./FileQueue";
import TitleBar from "./TitleBar";
import TooltipHost from "./TooltipHost";
import { I18nProvider } from "../lib/i18n";
import type { FileEntry } from "../types";

const minimizeMock = vi.hoisted(() => vi.fn());
const closeMock = vi.hoisted(() => vi.fn());
const startDraggingMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ minimize: minimizeMock, close: closeMock, startDragging: startDraggingMock }),
}));

const wrap = (node: ReactNode) => render(<I18nProvider>{node}</I18nProvider>);

/** jsdom lays nothing out, so every rect is zero until one is supplied. */
function stubRect(target: Element, rect: Partial<DOMRect>) {
  vi.spyOn(target, "getBoundingClientRect").mockReturnValue({
    x: 0, y: 0, top: 0, left: 0, right: 0, bottom: 0, width: 0, height: 0,
    toJSON: () => ({}), ...rect,
  } as DOMRect);
}

beforeEach(() => {
  minimizeMock.mockReset();
  closeMock.mockReset();
  startDraggingMock.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
  Reflect.deleteProperty(navigator, "clipboard");
});

describe("title bar", () => {
  it("drives the window commands the system caption used to own", async () => {
    wrap(<TitleBar closeToTray={false} onOpenCommands={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "最小化" }));
    await waitFor(() => expect(minimizeMock).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    await waitFor(() => expect(closeMock).toHaveBeenCalled());
  });

  it("stays draggable and opens the palette from the command centre", async () => {
    const onOpenCommands = vi.fn();
    const { container } = wrap(<TitleBar closeToTray={false} onOpenCommands={onOpenCommands} />);
    // Without this attribute an undecorated window cannot be moved at all.
    expect(container.querySelector(".titlebar")).toHaveAttribute("data-tauri-drag-region");
    fireEvent.mouseDown(container.querySelector(".titlebar-brand span:nth-child(2)")!, { button: 0 });
    await waitFor(() => expect(startDraggingMock).toHaveBeenCalledOnce());
    fireEvent.mouseDown(screen.getByRole("button", { name: "命令" }), { button: 0 });
    expect(startDraggingMock).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "命令" }));
    expect(onOpenCommands).toHaveBeenCalled();
  });

  it("offers the window menu on right-click", async () => {
    const { container } = wrap(<TitleBar closeToTray onOpenCommands={vi.fn()} />);
    fireEvent.contextMenu(container.querySelector(".titlebar")!, { clientX: 40, clientY: 10 });
    const menu = await screen.findByRole("menu", { name: "MetaClean" });
    fireEvent.click(within(menu).getByRole("menuitem", { name: /关闭/ }));
    await waitFor(() => expect(closeMock).toHaveBeenCalled());
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});

describe("context menu", () => {
  const build = (run: () => void): MenuEntry[] => [
    { id: "one", label: "第一项", run },
    "separator",
    { id: "two", label: "第二项", disabled: true, run: vi.fn() },
    { id: "three", label: "第三项", accelerator: "Del", run: vi.fn() },
  ];

  function Harness() {
    const menu = useContextMenu();
    return (
      <div>
        <button type="button" onContextMenu={menu.open}>宿主</button>
        {menu.anchor ? <ContextMenu entries={build(vi.fn())} anchor={menu.anchor} label="测试菜单" onClose={menu.close} /> : null}
      </div>
    );
  }

  it("steps over the disabled item and runs the one that is highlighted", () => {
    const run = vi.fn();
    wrap(<ContextMenu entries={build(run)} anchor={{ x: 10, y: 10 }} label="测试菜单" onClose={vi.fn()} />);
    const menu = screen.getByRole("menu", { name: "测试菜单" });
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    // Two steps land on the third item, because the disabled second is skipped.
    expect(screen.getByRole("menuitem", { name: /第三项/ })).toHaveClass("active");
    fireEvent.keyDown(menu, { key: "ArrowUp" });
    fireEvent.keyDown(menu, { key: "Enter" });
    expect(run).toHaveBeenCalled();
  });

  it("closes on Escape and on a press outside", () => {
    const onClose = vi.fn();
    const { container } = wrap(<ContextMenu entries={build(vi.fn())} anchor={{ x: 10, y: 10 }} label="测试菜单" onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.pointerDown(container.querySelector(".menu-layer")!);
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("flips a flyout that would fall off the window instead of clipping it", () => {
    stubRect(Element.prototype, { width: 200, height: 120 });
    wrap(<ContextMenu entries={build(vi.fn())} anchor={{ x: 1000, y: 700 }} label="测试菜单" onClose={vi.fn()} />);
    const menu = screen.getByRole("menu");
    expect(menu.style.left).toBe("800px");
    expect(menu.style.top).toBe("580px");
  });

  it("anchors on the focused control when the keyboard opens it", async () => {
    wrap(<Harness />);
    const host = screen.getByRole("button", { name: "宿主" });
    stubRect(host, { left: 30, bottom: 48, width: 60, height: 24 });
    // Shift+F10 and the Menu key report no pointer coordinates.
    fireEvent.contextMenu(host, { clientX: 0, clientY: 0 });
    const menu = await screen.findByRole("menu", { name: "测试菜单" });
    expect(menu.style.left).toBe("42px");
    expect(menu.style.top).toBe("44px");
  });
});

describe("command palette", () => {
  const commands: Command[] = [
    { id: "go-clean", group: "前往", label: "文件净化", accelerator: "Ctrl+1", run: vi.fn() },
    { id: "go-history", group: "前往", label: "处理记录", accelerator: "Ctrl+2", run: vi.fn() },
    { id: "pick", group: "操作", label: "选择文件", run: vi.fn() },
    { id: "clean", group: "操作", label: "确认并开始清理", disabled: true, run: vi.fn() },
  ];

  it("ranks the literal spelling above a scattered subsequence", () => {
    expect(score("cf", "Choose files")).toBeGreaterThan(0);
    expect(score("choose", "Choose files")!).toBeGreaterThan(score("cf", "Choose files")!);
    expect(score("zzz", "Choose files")).toBeNull();
    // Between two equally good matches the shorter label is the one meant.
    expect(score("clean", "Clean files")!).toBeGreaterThan(score("clean", "Clean files and folders")!);
  });

  it("groups the unfiltered list and drops the headings once ranked", () => {
    const { container } = wrap(<CommandPalette commands={commands} onClose={vi.fn()} />);
    expect([...container.querySelectorAll(".palette-group")].map((node) => node.textContent)).toEqual(["前往", "操作"]);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "记录" } });
    expect(container.querySelectorAll(".palette-group")).toHaveLength(0);
    expect(screen.getAllByRole("option")).toHaveLength(1);
  });

  it("runs the highlighted command and refuses the disabled one", () => {
    const run = vi.fn();
    const onClose = vi.fn();
    wrap(<CommandPalette commands={[{ ...commands[2], run }, commands[3]]} onClose={onClose} />);
    const field = screen.getByRole("combobox");
    fireEvent.keyDown(field, { key: "ArrowDown" });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(run).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(field, { key: "Home" });
    fireEvent.keyDown(field, { key: "Enter" });
    expect(run).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("says so when nothing matches, and closes on Escape", () => {
    const onClose = vi.fn();
    wrap(<CommandPalette commands={commands} onClose={onClose} />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "zzzz" } });
    expect(screen.getByText("没有匹配的命令")).toBeInTheDocument();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("takes the keyboard on open and hands it back on close", () => {
    const opener = render(<button type="button">开启</button>).getByRole("button");
    opener.focus();
    const { unmount } = wrap(<CommandPalette commands={commands} onClose={vi.fn()} />);
    expect(screen.getByRole("combobox")).toHaveFocus();
    unmount();
    // Dismissing the palette must never strand focus on the document body.
    expect(opener).toHaveFocus();
  });
});

describe("tooltip host", () => {
  it("shows one tip after the hover delay and takes it down on a press", () => {
    vi.useFakeTimers();
    render(<><button type="button" data-tip="提示文本">宿主</button><TooltipHost /></>);
    fireEvent.pointerOver(screen.getByRole("button"));
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    act(() => void vi.advanceTimersByTime(500));
    expect(screen.getByRole("tooltip")).toHaveTextContent("提示文本");
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("stays out of the way of controls that carry no tip", () => {
    vi.useFakeTimers();
    render(<><button type="button">宿主</button><TooltipHost /></>);
    fireEvent.pointerOver(screen.getByRole("button"));
    act(() => void vi.advanceTimersByTime(500));
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });
});

describe("queue row details", () => {
  const entry: FileEntry = {
    id: "1",
    name: "photo.jpg",
    path: "C:\\work\\photo.jpg",
    kind: "image",
    status: "scanned",
    report: {
      path: "C:\\work\\photo.jpg", name: "photo.jpg", format: "JPEG", size: 2048, supported: true,
      findings: [
        { category: "image_metadata", label: "EXIF", count: 3, severity: "privacy" },
        { category: "color_profile", label: "ICC", count: 1, severity: "informational" },
      ],
    },
  };
  const queue = (entries: FileEntry[], overrides: Partial<Parameters<typeof FileQueue>[0]> = {}) => wrap(
    <FileQueue
      entries={entries}
      preserveColorProfile
      removeExtendedAttributes={false}
      onRemove={vi.fn()}
      onClear={vi.fn()}
      onReveal={vi.fn()}
      onNotify={vi.fn()}
      {...overrides}
    />,
  );

  it("expands one row at a time and states the fate of every finding", () => {
    const { container } = queue([entry, { ...entry, id: "2", name: "zoo.jpg" }]);
    const disclosures = screen.getAllByRole("button", { name: "详细信息" });
    fireEvent.click(disclosures[0]);
    expect(container.querySelectorAll(".file-detail")).toHaveLength(1);
    expect(screen.getByText("C:\\work\\photo.jpg")).toBeInTheDocument();
    expect(screen.getByText("JPEG")).toBeInTheDocument();
    // The preserved profile is called out as kept, not queued for removal.
    expect(screen.getByText("将被移除")).toBeInTheDocument();
    expect(screen.getByText("保留")).toBeInTheDocument();
    fireEvent.click(disclosures[1]);
    expect(container.querySelectorAll(".file-detail")).toHaveLength(1);
    fireEvent.click(disclosures[1]);
    expect(container.querySelectorAll(".file-detail")).toHaveLength(0);
  });

  it("marks what a finished cleanup actually took out", () => {
    queue([{
      ...entry,
      status: "clean",
      result: { sourcePath: entry.path!, outputPath: "C:\\work\\photo.cleaned.jpg", removed: [entry.report!.findings[0]], success: true },
    }]);
    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));
    expect(screen.getByText("已移除")).toBeInTheDocument();
    expect(screen.getByText("保留")).toBeInTheDocument();
    expect(screen.getByText("C:\\work\\photo.cleaned.jpg")).toBeInTheDocument();
  });

  it("copies a path from the row's own menu and reports the outcome", async () => {
    const onNotify = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const { container } = queue([entry], { onNotify });
    fireEvent.contextMenu(container.querySelector(".file-item")!, { clientX: 20, clientY: 20 });
    const menu = await screen.findByRole("menu", { name: "photo.jpg" });
    fireEvent.click(within(menu).getByRole("menuitem", { name: "复制路径" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("C:\\work\\photo.jpg"));
    await waitFor(() => expect(onNotify).toHaveBeenCalledWith("已复制到剪贴板"));
  });

  it("removes a row from that same menu", async () => {
    const onRemove = vi.fn();
    const { container } = queue([entry], { onRemove });
    fireEvent.contextMenu(container.querySelector(".file-item")!, { clientX: 20, clientY: 20 });
    const menu = await screen.findByRole("menu", { name: "photo.jpg" });
    fireEvent.click(within(menu).getByRole("menuitem", { name: "从队列中移除" }));
    expect(onRemove).toHaveBeenCalledWith("1");
  });
});
