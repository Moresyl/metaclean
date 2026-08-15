import { FilePlus2, FolderOpen } from "lucide-react";
import { useRef } from "react";
import type { FileEntry } from "../types";
import { entryFromFile, entryFromPath } from "../lib/files";

interface DropZoneProps { onAdd: (entries: FileEntry[]) => void }

export default function DropZone({ onAdd }: DropZoneProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  async function chooseFiles() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const paths = await open({ multiple: true, directory: false });
      if (paths) {
        onAdd(paths.map(entryFromPath));
      }
    } catch {
      inputRef.current?.click();
    }
  }
  return (
    <section className="drop-zone" onDragOver={(event) => event.preventDefault()} onDrop={(event) => {
      event.preventDefault();
      onAdd(Array.from(event.dataTransfer.files, entryFromFile));
    }}>
      <div className="drop-icon"><FilePlus2 size={27} /></div>
      <h2>拖入要净化的文件</h2>
      <p>先扫描隐私痕迹，再由你确认是否清理</p>
      <button className="primary-button" type="button" onClick={() => void chooseFiles()}>
        <FolderOpen size={16} />选择文件
      </button>
      <input ref={inputRef} className="sr-only" type="file" multiple onChange={(event) => {
        onAdd(Array.from(event.target.files ?? [], entryFromFile));
        event.target.value = "";
      }} />
      <div className="format-list"><span>图片</span><span>Office</span><span>PDF</span><span>文本</span></div>
    </section>
  );
}
