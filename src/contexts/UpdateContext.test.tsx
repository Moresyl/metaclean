import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UpdateProvider, useUpdate } from "./UpdateContext";

const checkForUpdateMock = vi.hoisted(() => vi.fn());
const openUrlMock = vi.hoisted(() => vi.fn());
vi.mock("../lib/update", () => ({
  RELEASES_PAGE_URL: "https://github.com/Moresyl/metaclean/releases/latest",
  checkForUpdate: checkForUpdateMock,
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: openUrlMock }));

function Probe() {
  const update = useUpdate();
  return <div>
    <span>{update.status}</span>
    <span>{update.currentVersion}</span>
    <span>{update.error}</span>
    <button type="button" onClick={() => void update.checkUpdate()}>check</button>
    <button type="button" onClick={() => void update.openRelease()}>open</button>
    <button type="button" onClick={() => update.setAutoCheckEnabled(!update.autoCheckEnabled)}>toggle</button>
  </div>;
}

describe("UpdateProvider", () => {
  beforeEach(() => {
    checkForUpdateMock.mockReset();
    openUrlMock.mockReset();
  });

  it("reports the current version and persists the automatic-check switch", async () => {
    checkForUpdateMock.mockResolvedValue({ status: "current", currentVersion: "0.2.0" });
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    await screen.findByText("current");
    expect(screen.getByText("0.2.0")).toBeInTheDocument();
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

  it("deduplicates concurrent checks", async () => {
    let resolveCheck: ((value: { status: "current"; currentVersion: string }) => void) | undefined;
    checkForUpdateMock.mockReturnValue(new Promise((resolve) => { resolveCheck = resolve; }));
    render(<UpdateProvider><Probe /></UpdateProvider>);
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    fireEvent.click(screen.getByRole("button", { name: "check" }));
    expect(checkForUpdateMock).toHaveBeenCalledTimes(1);
    resolveCheck?.({ status: "current", currentVersion: "0.2.0" });
    await screen.findByText("current");
  });

  it("rejects consumers outside the provider", () => {
    expect(() => render(<Probe />)).toThrow("useUpdate must be used inside UpdateProvider");
  });
});
