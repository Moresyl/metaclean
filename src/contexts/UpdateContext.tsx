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
import { readStorage, removeStorage, writeStorage } from "../lib/storage";
import { boundedErrorMessage } from "../lib/errors";

type UpdateStatus = "idle" | "checking" | "current" | "available" | "updating" | "error";

interface UpdateContextValue {
  status: UpdateStatus;
  info?: UpdateInfo;
  currentVersion?: string;
  error?: string;
  progress?: UpdateProgress;
  runtime: UpdateRuntime;
  runtimeReady: boolean;
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

function normalizeRuntime(value: unknown): UpdateRuntime {
  if (!value || typeof value !== "object") return DEFAULT_RUNTIME;
  const candidate = value as Partial<UpdateRuntime>;
  if (typeof candidate.selfUpdateSupported !== "boolean" || typeof candidate.portable !== "boolean") {
    return DEFAULT_RUNTIME;
  }
  return { selfUpdateSupported: candidate.selfUpdateSupported, portable: candidate.portable };
}

export function UpdateProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [info, setInfo] = useState<UpdateInfo>();
  const [currentVersion, setCurrentVersion] = useState<string>();
  const [error, setError] = useState<string>();
  const [progress, setProgress] = useState<UpdateProgress>();
  const [runtime, setRuntime] = useState<UpdateRuntime>(DEFAULT_RUNTIME);
  const [runtimeReady, setRuntimeReady] = useState(false);
  const [promptOpen, setPromptOpen] = useState(false);
  const [autoCheckEnabled, setAutoCheckState] = useState(() => readStorage(AUTO_CHECK_KEY) !== "false");
  const checking = useRef(false);
  const installing = useRef(false);
  const mountedRef = useRef(true);

  const setAutoCheckEnabled = useCallback((enabled: boolean) => {
    writeStorage(AUTO_CHECK_KEY, String(enabled));
    setAutoCheckState(enabled);
  }, []);

  const checkUpdate = useCallback(async () => {
    if (!mountedRef.current || checking.current || installing.current) return;
    checking.current = true;
    setStatus("checking");
    setError(undefined);
    try {
      const result = await checkForUpdate();
      if (!mountedRef.current) return;
      if (result.status === "available") {
        setInfo(result.info);
        setCurrentVersion(result.info.currentVersion);
        setStatus("available");
        setPromptOpen(readStorage(DISMISSED_VERSION_KEY) !== result.info.availableVersion);
      } else {
        setInfo(undefined);
        setCurrentVersion(result.currentVersion);
        setStatus("current");
        setPromptOpen(false);
      }
    } catch (reason) {
      if (!mountedRef.current) return;
      setInfo(undefined);
      setPromptOpen(false);
      setError(boundedErrorMessage(reason));
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
    removeStorage(DISMISSED_VERSION_KEY);
    setPromptOpen(true);
  }, [info]);

  const dismissUpdatePrompt = useCallback(() => {
    if (info?.availableVersion) {
      writeStorage(DISMISSED_VERSION_KEY, info.availableVersion);
    }
    setPromptOpen(false);
  }, [info?.availableVersion]);

  const installUpdate = useCallback(async () => {
    if (!mountedRef.current || installing.current || checking.current) return;
    if (!runtime.selfUpdateSupported) {
      installing.current = true;
      if (mountedRef.current) setError(undefined);
      try {
        await openRelease();
      } catch (reason) {
        if (!mountedRef.current) return;
        setError(boundedErrorMessage(reason));
        setProgress(undefined);
        setStatus("error");
      } finally {
        installing.current = false;
      }
      return;
    }
    const expectedVersion = info?.availableVersion;
    if (!expectedVersion) {
      if (!mountedRef.current) return;
      setError("没有经过确认的更新版本，请重新检查更新。 / No reviewed update is available. Check for updates again.");
      setProgress(undefined);
      setStatus("error");
      return;
    }
    installing.current = true;
    if (!mountedRef.current) {
      installing.current = false;
      return;
    }
    setStatus("updating");
    setError(undefined);
    setProgress({ stage: "downloading", downloaded: 0 });
    try {
      const installed = await installAvailableUpdate({
        expectedVersion,
        onProgress: (value) => { if (mountedRef.current) setProgress(value); },
      });
      if (!mountedRef.current) return;
      if (!installed) {
        setInfo(undefined);
        setPromptOpen(false);
        setProgress(undefined);
        setStatus("current");
      }
    } catch (reason) {
      if (!mountedRef.current) return;
      setError(boundedErrorMessage(reason));
      setProgress(undefined);
      setStatus("error");
    } finally {
      installing.current = false;
    }
  }, [info?.availableVersion, openRelease, runtime.selfUpdateSupported]);

  useEffect(() => {
    mountedRef.current = true;
    let active = true;
    void getInstalledVersion()
      .then((version) => { if (active) setCurrentVersion(version); })
      .catch(() => undefined);
    void getUpdateRuntime()
      .then((value) => { if (active) setRuntime(normalizeRuntime(value)); })
      .catch(() => undefined)
      .finally(() => { if (active) setRuntimeReady(true); });
    return () => { active = false; mountedRef.current = false; };
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
    runtimeReady,
    promptOpen,
    autoCheckEnabled,
    setAutoCheckEnabled,
    checkUpdate,
    installUpdate,
    openRelease,
    showUpdatePrompt,
    dismissUpdatePrompt,
  }), [status, info, currentVersion, error, progress, runtime, runtimeReady, promptOpen, autoCheckEnabled, setAutoCheckEnabled, checkUpdate, installUpdate, openRelease, showUpdatePrompt, dismissUpdatePrompt]);

  return <UpdateContext.Provider value={value}>{children}</UpdateContext.Provider>;
}

export function useUpdate(): UpdateContextValue {
  const value = useContext(UpdateContext);
  if (!value) throw new Error("useUpdate must be used inside UpdateProvider");
  return value;
}
