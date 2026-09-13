import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UpdateProvider, useUpdate } from "./UpdateContext";

const checkForUpdateMock = vi.hoisted(() => vi.fn());
const getInstalledVersionMock = vi.hoisted(() => vi.fn());
const getUpdateRuntimeMock = vi.hoisted(() => vi.fn());
const installAvailableUpdateMock = vi.hoisted(() => vi.fn());
const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("../lib/update", () => ({
  RELEASES_PAGE_URL: "https://github.com/Moresyl/metaclean/releases/latest",
  checkForUpdate: checkForUpdateMock,
  getInstalledVersion: getInstalledVersionMock,
  getUpdateRuntime: getUpdateRuntimeMock,
  installAvailableUpdate: installAvailableUpdateMock,
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

function Probe() {
  const update = useUpdate();
  return <div>
    <span>{update.status}</span>
    <span>{update.currentVersion}</span>
    <span data-testid="available-version">{update.info?.availableVersion}</span>
    <span>{update.error}</span>
    <span data-testid="self-update-supported">{String(update.runtime.selfUpdateSupported)}</span>
    <span data-testid="runtime-ready">{String(update.runtimeReady)}</span>
    <span>{update.progress?.downloaded}</span>
    <span data-testid="prompt-open">{String(update.promptOpen)}</span>
    <button type="button" onClick={() => void update.checkUpdate()}>check</button>
    <button type="button" onClick={() => void update.installUpdate()}>install</button>
    <button type="button" onClick={() => void update.openRelease()}>open</button>
    <button type="button" onClick={() => update.setAutoCheckEnabled(!update.autoCheckEnabled)}>toggle</button>
    <button type="button" onClick={update.showUpdatePrompt}>show prompt</button>
    <button type="button" onClick={update.dismissUpdatePrompt}>dismiss prompt</button>
  </div>;
}

describe("UpdateProvider", () => {
  beforeEach(() => {
    localStorage.clear();
    checkForUpdateMock.mockReset();
    getInstalledVersionMock.mockReset();
    getInstalledVersionMock.mockResolvedValue("0.4.1");
    getUpdateRuntimeMock.mockReset();
    installAvailableUpdateMock.mockReset();
    openUrlMock.mockReset();
    getUpdateRuntimeMock.mockResolvedValue({ selfUpdateSupported: true, portable: false });
  });

  it("reports the current version and persists the automatic-check switch", async () => {
    checkForUpdateMock.mockResolvedValue({ status: "current", currentVersion: "0.3.0" });
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("current");
    expect(screen.getByText("0.3.0")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "toggle" }));
    expect(localStorage.getItem("metaclean.update.autoCheck")).toBe("false");
  });

  it("captures non-Error failures and opens the default release page", async () => {
    localStorage.setItem("metaclean.update.autoCheck", "false");
    checkForUpdateMock.mockRejectedValue("offline");
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("error");
    expect(screen.getByText("offline")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "open" }));
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledWith("https://github.com/Moresyl/metaclean/releases/latest"));
  });

  it("clears a stale reviewed version after a failed re-check", async () => {
    localStorage.setItem("metaclean.update.autoCheck", "false");
    checkForUpdateMock.mockResolvedValueOnce({
      status: "available",
      info: { currentVersion: "0.3.0", availableVersion: "0.4.0", name: "MetaClean v0.4.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.4.0" },
    }).mockRejectedValueOnce(new Error("offline"));
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("available");
    expect(screen.getByTestId("available-version")).toHaveTextContent("0.4.0");
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("error");
    expect(screen.getByTestId("available-version")).toBeEmptyDOMElement();
    fireEvent.click(screen.getByRole("button", { name: "install" }));
    expect(installAvailableUpdateMock).not.toHaveBeenCalled();
  });

  it("falls back safely when the native runtime response is malformed", async () => {
    getUpdateRuntimeMock.mockResolvedValue(undefined);
    render(<UpdateProvider><Probe /></UpdateProvider>);
    expect(await screen.findByTestId("self-update-supported")).toHaveTextContent("false");
    expect(screen.getByRole("button", { name: "check" })).toBeEnabled();
  });

  it("deduplicates concurrent checks", async () => {
    let resolveCheck: ((value: { status: "current"; currentVersion: string }) => void) | undefined;
    checkForUpdateMock.mockReturnValue(new Promise((resolve) => { resolveCheck = resolve; }));
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    expect(checkForUpdateMock).toHaveBeenCalledTimes(1);
    resolveCheck?.({ status: "current", currentVersion: "0.3.0" });
    await screen.findByText("current");
  });

  it("does not let an unmounted update check overwrite a newer provider", async () => {
    let resolveFirst: ((result: { status: "available"; info: { currentVersion: string; availableVersion: string; name: string; releaseUrl: string } }) => void) | undefined;
    checkForUpdateMock
      .mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve; }))
      .mockResolvedValue({ status: "current", currentVersion: "0.5.0" });
    const first = render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    first.unmount();
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("current");
    resolveFirst?.({ status: "available", info: { currentVersion: "0.4.1", availableVersion: "0.6.0", name: "MetaClean v0.6.0", releaseUrl: "https://example.test/v0.6.0" } });
    await Promise.resolve();
    expect(screen.getByTestId("available-version")).toBeEmptyDOMElement();
  });

  it("installs a signed update and forwards native download progress", async () => {
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "0.3.0", availableVersion: "0.4.0", name: "MetaClean v0.4.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.4.0" },
    });
    installAvailableUpdateMock.mockImplementation(async ({ onProgress }) => {
      onProgress({ stage: "downloading", downloaded: 50, total: 100 });
      return true;
    });
    render(<UpdateProvider><Probe /></UpdateProvider>);
    await screen.findByTestId("runtime-ready");
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("available");
    fireEvent.click(screen.getByRole("button", { name: "install" }));
    await screen.findByText("updating");
    expect(screen.getByText("50")).toBeInTheDocument();
    expect(installAvailableUpdateMock).toHaveBeenCalledWith(expect.objectContaining({ expectedVersion: "0.4.0" }));
  });

  it("deduplicates installs and update checks while an install is active", async () => {
    let finishInstall: ((installed: boolean) => void) | undefined;
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "0.3.0", availableVersion: "0.4.0", name: "MetaClean v0.4.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.4.0" },
    });
    installAvailableUpdateMock.mockReturnValue(new Promise((resolve) => { finishInstall = resolve; }));
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("available");

    fireEvent.click(screen.getByRole("button", { name: "install" }));
    fireEvent.click(screen.getByRole("button", { name: "install" }));
    fireEvent.click(screen.getByRole("button", { name: "check" }));

    expect(installAvailableUpdateMock).toHaveBeenCalledTimes(1);
    expect(checkForUpdateMock).toHaveBeenCalledTimes(1);
    finishInstall?.(false);
    await screen.findByText("current");
    expect(screen.getByTestId("prompt-open")).toHaveTextContent("false");
  });

  it("refuses installation when no reviewed version is available", async () => {
    localStorage.setItem("metaclean.update.autoCheck", "false");
    render(<UpdateProvider><Probe /></UpdateProvider>);
    await screen.findByTestId("self-update-supported");
    fireEvent.click(screen.getByRole("button", { name: "install" }));
    await screen.findByText("error");
    expect(screen.getByText(/No reviewed update is available/u)).toBeInTheDocument();
    expect(installAvailableUpdateMock).not.toHaveBeenCalled();
  });

  it("opens the release page instead of self-updating a portable runtime", async () => {
    getUpdateRuntimeMock.mockResolvedValue({ selfUpdateSupported: false, portable: true });
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "0.3.0", availableVersion: "0.4.0", name: "MetaClean v0.4.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.4.0" },
    });
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("available");
    fireEvent.click(screen.getByRole("button", { name: "install" }));
    await waitFor(() => expect(openUrlMock).toHaveBeenCalledWith("https://github.com/Moresyl/metaclean/releases/tag/v0.4.0"));
    expect(installAvailableUpdateMock).not.toHaveBeenCalled();
  });

  it("surfaces release-page failures for portable runtimes", async () => {
    getUpdateRuntimeMock.mockResolvedValue({ selfUpdateSupported: false, portable: true });
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "0.3.0", availableVersion: "0.4.0", name: "MetaClean v0.4.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.4.0" },
    });
    openUrlMock.mockRejectedValue(new Error("blocked by policy"));
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("available");
    fireEvent.click(screen.getByRole("button", { name: "install" }));
    await screen.findByText("error");
    expect(screen.getByText("blocked by policy")).toBeInTheDocument();
    expect(installAvailableUpdateMock).not.toHaveBeenCalled();
  });

  it("remembers a dismissed version and allows the prompt to be reopened", async () => {
    checkForUpdateMock.mockResolvedValue({
      status: "available",
      info: { currentVersion: "0.4.1", availableVersion: "0.5.0", name: "MetaClean v0.5.0", releaseUrl: "https://github.com/Moresyl/metaclean/releases/tag/v0.5.0" },
    });
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await waitFor(() => expect(screen.getByTestId("prompt-open")).toHaveTextContent("true"));
    fireEvent.click(screen.getByRole("button", { name: "dismiss prompt" }));
    expect(screen.getByTestId("prompt-open")).toHaveTextContent("false");
    expect(localStorage.getItem("metaclean.update.dismissedVersion")).toBe("0.5.0");
    fireEvent.click(screen.getByRole("button", { name: "show prompt" }));
    expect(screen.getByTestId("prompt-open")).toHaveTextContent("true");
    expect(localStorage.getItem("metaclean.update.dismissedVersion")).toBeNull();
  });

  it("rejects consumers outside the provider", () => {
    expect(() => render(<Probe />)).toThrow("useUpdate must be used inside UpdateProvider");
  });
});
