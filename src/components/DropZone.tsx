import { FilePlus2, FolderOpen } from "lucide-react";
import { useRef, useState } from "react";
import Button from "./Button";
import type { FileEntry } from "../types";
import { entryFromFile } from "../lib/files";
import { pickPaths } from "../lib/pick";
import { useI18n } from "../lib/i18n";

interface DropZoneProps {
  onAdd: (entries: FileEntry[]) => void;
  onAddNativePaths: (paths: string[]) => Promise<void>;
  /** Set while a native (Tauri) drag hovers the window. */
  dragActive?: boolean;
  /** Collapse to a slim intake bar once the queue has files to show. */
  compact?: boolean;
}

const FORMATS = ["Images", "Audio", "Video", "Office", "PDF", "Text"];

export default function DropZone({ onAdd, onAddNativePaths, dragActive = false, compact = false }: DropZoneProps) {
  const { text } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const [browserDrag, setBrowserDrag] = useState(false);
  const hovering = dragActive || browserDrag;
  const labels = [text("图片", "Images"), text("音频", "Audio"), text("视频", "Video"), "Office", "PDF", text("文本", "Text")];

  async function choose(directory: boolean) {
    try {
      const paths = await pickPaths(directory);
      if (paths) await onAddNativePaths(paths);
    } catch {
      if (!directory) inputRef.current?.click();
    }
  }

  return (
    <section
      className={[
        // A dashed edge, because the whole shape is an invitation rather than a
        // container — and it is the one place in the window where a border is
        // doing more than separating two grounds.
        "relative flex shrink-0 flex-col items-center rounded-panel border border-dashed",
        "text-center transition-colors duration-150 ease-[var(--ease-out-soft)]",
        compact ? "gap-2 px-4 py-3.5" : "gap-2.5 px-5 py-7",
        hovering
          ? "border-brand bg-brand/8"
          : "border-line-strong bg-surface hover:border-faint",
      ].join(" ")}
      onDragEnter={() => setBrowserDrag(true)}
      onDragOver={(event) => {
        event.preventDefault();
        setBrowserDrag(true);
      }}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setBrowserDrag(false);
      }}
      onDrop={(event) => {
        event.preventDefault();
        setBrowserDrag(false);
        onAdd(Array.from(event.dataTransfer.files, entryFromFile));
      }}
    >
      {!compact ? (
        <div
          className={`grid size-12 place-items-center rounded-panel transition-colors duration-150 ${hovering ? "bg-brand text-on-brand" : "bg-brand/12 text-brand"}`}
          aria-hidden="true"
        >
          <FilePlus2 size={24} strokeWidth={1.8} />
        </div>
      ) : null}

      <div className="grid gap-0.5">
        <h2 className={compact ? "text-md font-semibold" : "text-lg font-semibold"}>
          {text("拖入要净化的文件", "Drop files to clean")}
        </h2>
        <p className="text-sm text-muted">
          {text("先扫描隐私痕迹，再由你确认是否清理", "Scan privacy traces first, then confirm cleanup")}
        </p>
      </div>

      <div className="flex flex-wrap justify-center gap-2">
        <Button variant="primary" onClick={() => void choose(false)}>
          <FilePlus2 size={14} strokeWidth={2} />
          {text("选择文件", "Choose files")}
        </Button>
        <Button onClick={() => void choose(true)}>
          <FolderOpen size={14} strokeWidth={2} />
          {text("选择文件夹", "Choose folder")}
        </Button>
      </div>

      {/* The desktop build never reaches this: it is the fallback for a plain
          browser, where there is no system picker to fail over from. */}
      <input
        ref={inputRef}
        className="sr-only"
        type="file"
        multiple
        onChange={(event) => {
          onAdd(Array.from(event.target.files ?? [], entryFromFile));
          event.target.value = "";
        }}
      />

      {!compact ? (
        <div className="mt-0.5 flex flex-wrap justify-center gap-1">
          {FORMATS.map((format, index) => (
            <span
              key={format}
              className="rounded-[3px] bg-surface-2 px-1.5 py-px text-xs text-muted"
            >
              {labels[index]}
            </span>
          ))}
        </div>
      ) : null}
    </section>
  );
}
