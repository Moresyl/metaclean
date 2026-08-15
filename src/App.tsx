import { useCallback, useEffect, useState } from "react";
import { Minus, Sparkles } from "lucide-react";
import Sidebar from "./components/Sidebar";
import DropZone from "./components/DropZone";
import FileQueue from "./components/FileQueue";
import CleanOptions from "./components/CleanOptions";
import { entryFromPath, mergeEntries } from "./lib/files";
import type { CleanMode, FileEntry } from "./types";

export default function App() {
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [mode, setMode] = useState<CleanMode>("copy");
  const addEntries = useCallback((incoming: FileEntry[]) => setEntries((current) => mergeEntries(current, incoming)), []);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") addEntries(event.payload.paths.map(entryFromPath));
    })).then((unlisten) => { dispose = unlisten; }).catch(() => undefined);
    return () => dispose?.();
  }, [addEntries]);

  return (
    <div className="app-shell">
      <Sidebar />
      <main className="workspace">
        <header className="topbar"><div><h1>文件净化</h1><p>清除文件里的隐私痕迹，分享前更安心</p></div><div className="window-actions"><button aria-label="最小化" type="button"><Minus size={15} /></button></div></header>
        <div className="content-grid">
          <div className="main-column">
            <div className="notice"><Sparkles size={15} /><span><strong>所有处理均在本机完成。</strong>MetaClean 不上传、不保存、也不分析你的文件内容。</span></div>
            <DropZone onAdd={addEntries} />
            <FileQueue entries={entries} onClear={() => setEntries([])} onRemove={(id) => setEntries((current) => current.filter((entry) => entry.id !== id))} />
          </div>
          <CleanOptions mode={mode} onModeChange={setMode} disabled={!entries.length} />
        </div>
      </main>
    </div>
  );
}
