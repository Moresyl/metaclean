import { FilePlus2, FolderOpen, Image, Music2, Video, FileType2, FileText, Type } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import Button from "./Button";
import type { FileEntry } from "../types";
import { normalizeBrowserFiles } from "../lib/files";
import { PickerUnavailableError, pickPaths } from "../lib/pick";
import { useI18n } from "../lib/i18n";

interface DropZoneProps {
  onAdd: (entries: FileEntry[]) => void;
  onAddNativePaths: (paths: string[]) => Promise<void>;
  onError?: (error: unknown) => void;
  onOpenPicker?: (directory: boolean) => void;
  /** Set while a native (Tauri) drag hovers the window. */
  dragActive?: boolean;
  /** Collapse to a slim intake bar once the queue has files to show. */
  compact?: boolean;
}

const FORMATS = ["Images", "Audio", "Video", "Office", "PDF", "Text"];
const FORMAT_ICONS = [Image, Music2, Video, FileType2, FileText, Type];

export default function DropZone({ onAdd, onAddNativePaths, onError, onOpenPicker, dragActive = false, compact = false }: DropZoneProps) {
  const { text } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const pickerBusyRef = useRef(false);
  const [browserDrag, setBrowserDrag] = useState(false);
  const hovering = dragActive || browserDrag;
  const labels = [text("图片", "Images"), text("音频", "Audio"), text("视频", "Video"), "Office", "PDF", text("文本", "Text")];

  const addBrowserFiles = useCallback((files: FileList | null) => {
    if (!files) return;
    let length: number;
    try {
      length = files.length;
    } catch {
      onError?.(new Error("浏览器选择无效 / The browser selection is invalid"));
      return;
    }
    const entries = normalizeBrowserFiles(files);
    if (length > 0 && entries.length === 0) {
      onError?.(new Error("浏览器选择无效或超过 10,000 个文件 / The browser selection is invalid or exceeds 10,000 files"));
      return;
    }
    onAdd(entries);
  }, [onAdd, onError]);

  const choose = useCallback(async (directory: boolean) => {
    if (pickerBusyRef.current) return;
    pickerBusyRef.current = true;
    try {
      let paths: string[] | null;
      try {
        paths = await pickPaths(directory);
      } catch (error) {
        if (!(error instanceof PickerUnavailableError)) {
          onError?.(error);
          return;
        }
        // A plain browser has no Tauri dialog. Mirror the desktop folder action
        // with the native directory input so relative paths survive intake.
        inputRef.current?.toggleAttribute("webkitdirectory", directory);
        inputRef.current?.click();
        return;
      }
      if (paths) {
        try {
          await onAddNativePaths(paths);
        } catch {
          // The parent owns the visible native-intake error. Never turn that
          // failure into an unhandled rejection from a button event.
        }
      }
    } finally {
      pickerBusyRef.current = false;
    }
  }, [onAddNativePaths, onError]);

  return (
    <section
      className={[
        // A dashed edge, because the whole shape is an invitation rather than a
        // container — and it is the one place in the window where a border is
        // doing more than separating two grounds.
        "drop-zone relative flex items-center border",
        "text-center transition-colors duration-150 ease-[var(--ease-out-soft)]",
        compact ? "shrink-0 flex-wrap justify-between gap-3 rounded-panel px-4 py-3" : "min-h-0 flex-1 flex-col justify-center gap-6 rounded-[24px] px-6 py-8",
        hovering
          ? "border-brand bg-brand/8"
          : compact ? "border-line bg-surface/40 hover:border-line-strong" : "border-line bg-surface/30 hover:border-line-strong",
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
        addBrowserFiles(event.dataTransfer.files);
      }}
    >
      {!compact ? (
        <div
          className={`grid size-14 place-items-center rounded-2xl border border-line transition-colors duration-150 ${hovering ? "bg-brand text-on-brand" : "bg-canvas text-text"}`}
          aria-hidden="true"
        >
          <FilePlus2 size={26} strokeWidth={1.4} />
        </div>
      ) : null}

      <div className={`grid gap-2 ${compact ? "text-left" : ""}`}>
        <h2 className={compact ? "text-base font-medium" : "text-[26px] leading-snug font-semibold"}>
          {text("拖入要净化的文件", "Drop files to clean")}
        </h2>
        <p className="text-sm text-muted">
          {text("先扫描隐私痕迹，再由你确认是否清理", "Scan privacy traces first, then confirm cleanup")}
        </p>
      </div>

      <div className="flex flex-wrap justify-center gap-2">
        <Button variant="primary" size={compact ? "sm" : "lg"} onClick={() => onOpenPicker ? onOpenPicker(false) : void choose(false)}>
          <FilePlus2 size={14} strokeWidth={2} />
          {text("选择文件", "Choose files")}
        </Button>
        <Button size={compact ? "sm" : "lg"} onClick={() => onOpenPicker ? onOpenPicker(true) : void choose(true)}>
          <FolderOpen size={14} strokeWidth={2} />
          {text("选择文件夹", "Choose folder")}
        </Button>
      </div>

      {/* The desktop build never reaches this: it is the fallback for a plain
          browser, where there is no system picker to fail over from. The App
          root owns one persistent input when commands share this zone. */}
      {!onOpenPicker ? (
        <input
          ref={inputRef}
          className="sr-only"
          type="file"
          multiple
          onChange={(event) => {
            addBrowserFiles(event.target.files);
            event.currentTarget.removeAttribute("webkitdirectory");
            event.target.value = "";
          }}
        />
      ) : null}

      {!compact ? (
        <div className="mt-3 flex flex-wrap justify-center gap-x-5 gap-y-3 border-t border-line pt-5">
          {FORMATS.map((format, index) => {
            const Icon = FORMAT_ICONS[index];
            return (
            <span
              key={format}
              className="grid justify-items-center gap-2 text-xs text-muted"
            >
              <Icon size={17} strokeWidth={1.5} aria-hidden="true" />
              {labels[index]}
            </span>
          ); })}
        </div>
      ) : null}
    </section>
  );
}
