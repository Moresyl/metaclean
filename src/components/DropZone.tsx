import { FilePlus2, FolderOpen } from "lucide-react";
import { useRef } from "react";
import type { FileEntry } from "../types";
import { entryFromFile } from "../lib/files";
import { useI18n } from "../lib/i18n";

interface DropZoneProps {
  onAdd: (entries: FileEntry[]) => void;
  onAddNativePaths: (paths: string[]) => Promise<void>;
}

export default function DropZone({ onAdd, onAddNativePaths }: DropZoneProps) {
  const { text } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  async function choose(directory: boolean) {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const paths = await open({ multiple: !directory, directory });
      if (paths) {
        await onAddNativePaths(Array.isArray(paths) ? paths : [paths]);
      }
    } catch {
      if (!directory) inputRef.current?.click();
    }
  }
  return (
    <section className="drop-zone" onDragOver={(event) => event.preventDefault()} onDrop={(event) => {
      event.preventDefault();
      onAdd(Array.from(event.dataTransfer.files, entryFromFile));
    }}>
      <div className="drop-icon"><FilePlus2 size={27} /></div>
      <h2>{text("拖入要净化的文件", "Drop files to clean")}</h2>
      <p>{text("先扫描隐私痕迹，再由你确认是否清理", "Scan privacy traces first, then confirm cleanup")}</p>
      <div className="intake-actions"><button className="primary-button" type="button" onClick={() => void choose(false)}><FilePlus2 size={16} />{text("选择文件", "Choose files")}</button><button className="secondary-button" type="button" onClick={() => void choose(true)}><FolderOpen size={16} />{text("选择文件夹", "Choose folder")}</button></div>
      <input ref={inputRef} className="sr-only" type="file" multiple onChange={(event) => {
        onAdd(Array.from(event.target.files ?? [], entryFromFile));
        event.target.value = "";
      }} />
      <div className="format-list"><span>{text("图片", "Images")}</span><span>{text("音频", "Audio")}</span><span>{text("视频", "Video")}</span><span>Office</span><span>PDF</span><span>{text("文本", "Text")}</span></div>
    </section>
  );
}
