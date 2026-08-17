import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { checkForUpdate, RELEASES_PAGE_URL, type UpdateInfo } from "../lib/update";

type UpdateStatus = "idle" | "checking" | "current" | "available" | "error";

interface UpdateContextValue {
  status: UpdateStatus;
  info?: UpdateInfo;
  currentVersion?: string;
  error?: string;
  autoCheckEnabled: boolean;
  setAutoCheckEnabled: (enabled: boolean) => void;
  checkUpdate: () => Promise<void>;
  openRelease: () => Promise<void>;
}

const AUTO_CHECK_KEY = "metaclean.update.autoCheck";
const UpdateContext = createContext<UpdateContextValue | null>(null);

export function UpdateProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [info, setInfo] = useState<UpdateInfo>();
  const [currentVersion, setCurrentVersion] = useState<string>();
  const [error, setError] = useState<string>();
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
      } else {
        setInfo(undefined);
        setCurrentVersion(result.currentVersion);
        setStatus("current");
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

  useEffect(() => {
    if (!autoCheckEnabled) return;
    const timer = window.setTimeout(() => { void checkUpdate(); }, 1_500);
    return () => window.clearTimeout(timer);
  }, [autoCheckEnabled, checkUpdate]);

  const value = useMemo<UpdateContextValue>(() => ({
    status, info, currentVersion, error, autoCheckEnabled, setAutoCheckEnabled, checkUpdate, openRelease,
  }), [status, info, currentVersion, error, autoCheckEnabled, setAutoCheckEnabled, checkUpdate, openRelease]);

  return <UpdateContext.Provider value={value}>{children}</UpdateContext.Provider>;
}

export function useUpdate(): UpdateContextValue {
  const value = useContext(UpdateContext);
  if (!value) throw new Error("useUpdate must be used inside UpdateProvider");
  return value;
}
