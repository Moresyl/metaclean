import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  checkForUpdate,
  getInstalledVersion,
  getUpdateRuntime,
  installAvailableUpdate,
  RELEASES_PAGE_URL,
  type UpdateInfo,
  type UpdateProgress,
  type UpdateRuntime,
} from "../lib/update";

type UpdateStatus = "idle" | "checking" | "current" | "available" | "updating" | "error";

interface UpdateContextValue {
  status: UpdateStatus;
  info?: UpdateInfo;
  currentVersion?: string;
  error?: string;
  progress?: UpdateProgress;
  runtime: UpdateRuntime;
  promptOpen: boolean;
  autoCheckEnabled: boolean;
  setAutoCheckEnabled: (enabled: boolean) => void;
  checkUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;
  openRelease: () => Promise<void>;
  showUpdatePrompt: () => void;
  dismissUpdatePrompt: () => void;
}

const AUTO_CHECK_KEY = "metaclean.update.autoCheck";
const DISMISSED_VERSION_KEY = "metaclean.update.dismissedVersion";
const DEFAULT_RUNTIME: UpdateRuntime = { selfUpdateSupported: false, portable: false };
const UpdateContext = createContext<UpdateContextValue | null>(null);

export function UpdateProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [info, setInfo] = useState<UpdateInfo>();
  const [currentVersion, setCurrentVersion] = useState<string>();
  const [error, setError] = useState<string>();
  const [progress, setProgress] = useState<UpdateProgress>();
  const [runtime, setRuntime] = useState<UpdateRuntime>(DEFAULT_RUNTIME);
  const [promptOpen, setPromptOpen] = useState(false);
  const [autoCheckEnabled, setAutoCheckState] = useState(() => localStorage.getItem(AUTO_CHECK_KEY) !== "false");
  const checking = useRef(false);

  const setAutoCheckEnabled = useCallback((enabled: boolean) => {
    localStorage.setItem(AUTO_CHECK_KEY, String(enabled));
    setAutoCheckState(enabled);
  }, []);

  const checkUpdate = useCallback(async () => {
    if (checking.current) return;
    checking.current = true;
    setStatus("checking");
    setError(undefined);
    try {
      const result = await checkForUpdate();
      if (result.status === "available") {
        setInfo(result.info);
        setCurrentVersion(result.info.currentVersion);
        setStatus("available");
        setPromptOpen(localStorage.getItem(DISMISSED_VERSION_KEY) !== result.info.availableVersion);
      } else {
        setInfo(undefined);
        setCurrentVersion(result.currentVersion);
        setStatus("current");
        setPromptOpen(false);
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setStatus("error");
    } finally {
      checking.current = false;
    }
  }, []);

  const openRelease = useCallback(async () => {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(info?.releaseUrl ?? RELEASES_PAGE_URL);
  }, [info?.releaseUrl]);

  const showUpdatePrompt = useCallback(() => {
    if (!info) return;
    localStorage.removeItem(DISMISSED_VERSION_KEY);
    setPromptOpen(true);
  }, [info]);

  const dismissUpdatePrompt = useCallback(() => {
    if (info?.availableVersion) {
      localStorage.setItem(DISMISSED_VERSION_KEY, info.availableVersion);
    }
    setPromptOpen(false);
  }, [info?.availableVersion]);

  const installUpdate = useCallback(async () => {
    if (!runtime.selfUpdateSupported) {
      await openRelease();
      return;
    }
    setStatus("updating");
    setError(undefined);
    setProgress({ stage: "downloading", downloaded: 0 });
    try {
      const installed = await installAvailableUpdate({ onProgress: setProgress });
      if (!installed) {
        setInfo(undefined);
        setProgress(undefined);
        setStatus("current");
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setProgress(undefined);
      setStatus("error");
    }
  }, [openRelease, runtime.selfUpdateSupported]);

  useEffect(() => {
    let active = true;
    void getInstalledVersion()
      .then((version) => { if (active) setCurrentVersion(version); })
      .catch(() => undefined);
    void getUpdateRuntime()
      .then((value) => { if (active) setRuntime(value); })
      .catch(() => undefined);
    return () => { active = false; };
  }, []);

  useEffect(() => {
    if (!autoCheckEnabled) return;
    const timer = window.setTimeout(() => { void checkUpdate(); }, 1_500);
    return () => window.clearTimeout(timer);
  }, [autoCheckEnabled, checkUpdate]);

  const value = useMemo<UpdateContextValue>(() => ({
    status,
    info,
    currentVersion,
    error,
    progress,
    runtime,
    promptOpen,
    autoCheckEnabled,
    setAutoCheckEnabled,
    checkUpdate,
    installUpdate,
    openRelease,
    showUpdatePrompt,
    dismissUpdatePrompt,
  }), [status, info, currentVersion, error, progress, runtime, promptOpen, autoCheckEnabled, setAutoCheckEnabled, checkUpdate, installUpdate, openRelease, showUpdatePrompt, dismissUpdatePrompt]);

  return <UpdateContext.Provider value={value}>{children}</UpdateContext.Provider>;
}

export function useUpdate(): UpdateContextValue {
  const value = useContext(UpdateContext);
  if (!value) throw new Error("useUpdate must be used inside UpdateProvider");
  return value;
}
