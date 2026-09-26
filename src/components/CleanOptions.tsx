import { Check as CheckIcon, Copy, FileWarning, ScanSearch } from "lucide-react";
import type { ReactNode } from "react";
import Button from "./Button";
import type { CleanMode } from "../types";
import { useI18n } from "../lib/i18n";

interface CleanOptionsProps { mode: CleanMode; onModeChange: (mode: CleanMode) => void; preserveTimestamps: boolean; onPreserveTimestampsChange: (value: boolean) => void; preserveOrientation: boolean; onPreserveOrientationChange: (value: boolean) => void; preserveColorProfile: boolean; onPreserveColorProfileChange: (value: boolean) => void; removeExtendedAttributes: boolean; onRemoveExtendedAttributesChange: (value: boolean) => void; disabled: boolean; scanned: boolean; hasFindings: boolean; busy: boolean; operation?: "scan" | "clean"; cancelable: boolean; cancelRequested: boolean; onCancel: () => void; onAction: () => void }

/**
 * The rail that says what is about to happen, and the one button that starts it.
 *
 * Its order is the order of the decision: where the output goes, what survives
 * the cleanup, what will be looked for, and only then the button. The safety
 * line under the button is the last thing read before the click, which is
 * exactly where it belongs.
 */
export default function CleanOptions({ mode, onModeChange, preserveTimestamps, onPreserveTimestampsChange, preserveOrientation, onPreserveOrientationChange, preserveColorProfile, onPreserveColorProfileChange, removeExtendedAttributes, onRemoveExtendedAttributesChange, disabled, scanned, hasFindings, busy, operation, cancelable, cancelRequested, onCancel, onAction }: CleanOptionsProps) {
  const { text } = useI18n();
  const scans = [
    "EXIF / GPS",
    text("音频标签与封面", "Audio tags and artwork"),
    text("视频用户数据与位置", "Video user data and location"),
    text("文档作者与修订", "Document authors and revisions"),
    `PDF ${text("属性与 XMP", "properties and XMP")}`,
    text("不可见 Unicode", "Invisible Unicode"),
  ];

  return (
    <aside className="clean-options flex min-h-0 flex-col overflow-hidden border-s border-line ps-5">
      {/* Only the settings scroll. The rail used to be one scrolling column with
          the button pushed to its end by `mt-auto`, which works right up until
          the content is taller than the rail — and then the one action the
          screen exists for is below the fold, on the first frame, at the
          default window size. A committing button belongs to the panel's frame,
          not to its contents. */}
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto pb-3">
        <div className="grid gap-1.5">
          <div className="caption">{text("清理方式", "Output mode")}</div>
          <ModeOption
            selected={mode === "copy"}
            icon={<Copy size={17} strokeWidth={1.8} />}
            title={text("保存为安全副本", "Save a safe copy")}
            detail={text("推荐，不修改原文件", "Recommended; keeps the original")}
            onClick={() => onModeChange("copy")}
          />
          <ModeOption
            selected={mode === "replace"}
            icon={<FileWarning size={17} strokeWidth={1.8} />}
            title={text("替换原文件", "Replace original")}
            detail={text("处理前自动创建备份", "Creates a backup before cleaning")}
            onClick={() => onModeChange("replace")}
          />
        </div>

        <div className="grid gap-1.5">
          <div className="caption">{text("保真选项", "Fidelity")}</div>
          <div className="grid gap-0.5">
            <Check checked={preserveOrientation} onChange={onPreserveOrientationChange}>
              {text("保留照片方向", "Preserve photo orientation")}
            </Check>
            <Check checked={preserveColorProfile} onChange={onPreserveColorProfileChange}>
              {text("图片", "Images")} · ICC / sRGB
            </Check>
            <Check checked={removeExtendedAttributes} onChange={onRemoveExtendedAttributesChange}>
              − macOS · xattr
            </Check>
            <Check checked={preserveTimestamps} onChange={onPreserveTimestampsChange}>
              {text("保留文件时间戳", "Preserve file timestamps")}
            </Check>
          </div>
        </div>

        {/* One flowing line, not a list.
         *
         * This is reference text — what the scan covers — and it was costing
         * 120px of a 720px window that cannot be resized: six rows, each with a
         * shield icon repeating what the caption above already said. A grid was
         * the next thing tried and it is the wrong shape for the same reason a
         * list was: fixed rows have to be tall enough for the longest phrase in
         * whichever language is loaded, so Chinese fits in two columns and
         * English immediately wraps every cell and overflows again. Set as
         * prose it simply takes the lines it needs, in any locale. */}
        <div className="grid gap-1.5">
          <div className="caption">{text("将检测", "Scans for")}</div>
          <p className="text-xs leading-relaxed text-muted">{scans.join(" · ")}</p>
        </div>
      </div>

      {/* Seated on the rail's frame, above a hairline, so it holds still while
          the list above it scrolls and while it changes length between locales. */}
      <div className="grid shrink-0 gap-2 border-t border-line pt-4">
        <Button
          variant="primary"
          size="lg"
          className="scan-button w-full"
          disabled={disabled || busy || (scanned && !hasFindings)}
          onClick={onAction}
        >
          <ScanSearch size={17} strokeWidth={1.8} />
          {busy
            ? text("处理中…", "Working…")
            : scanned
              ? hasFindings
                ? text("确认并开始清理", "Confirm and clean")
                : text("没有需要清理的痕迹", "No traces to clean")
              : text("扫描隐私痕迹", "Scan privacy traces")}
        </Button>
        {cancelable ? (
          <Button variant="ghost" disabled={cancelRequested} onClick={onCancel}>
            {cancelRequested ? text("正在取消…", "Cancelling…") : operation === "scan" ? text("取消扫描", "Cancel scan") : text("取消处理", "Cancel cleanup")}
          </Button>
        ) : null}
        {/* The last sentence read before the click, so it is set in ink somebody
            will actually read. `faint` on 12px CJK, centred under a filled mint
            button, is a line the eye skips — and it is the line that says the
            scan does not touch the file. */}
        <p className="text-center text-xs leading-snug text-muted">
          {scanned
            ? text("只清理报告中列出的痕迹。", "Only listed traces will be removed.")
            : text("扫描是只读操作，不会修改文件。", "Scanning is read-only and changes nothing.")}
        </p>
      </div>
    </aside>
  );
}

function ModeOption({ selected, icon, title, detail, onClick }: { selected: boolean; icon: ReactNode; title: string; detail: string; onClick: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onClick}
      className={[
        "flex items-start gap-2.5 rounded-panel border p-2.5 text-left transition-colors duration-100",
        selected
          ? "border-line bg-surface text-text"
          : "border-transparent text-muted hover:border-line hover:bg-surface-2/60",
      ].join(" ")}
    >
      <span className={`mt-px shrink-0 ${selected ? "text-brand" : "text-muted"}`} aria-hidden="true">{icon}</span>
      <span className="min-w-0 flex-1 grid gap-1">
        <strong className="text-base font-semibold text-text">{title}</strong>
        <small className="text-xs leading-snug text-muted">{detail}</small>
      </span>
      {selected ? <CheckIcon size={14} className="mt-1 shrink-0 text-focus" aria-hidden="true" /> : null}
    </button>
  );
}

/* The label is the row, and it is set in the window's own ink.
 *
 * It used to be `muted` beside a box filled with the accent, which put the
 * brightest thing in the rail on a 15px square and the dimmest on the words
 * that say what the square does — the hierarchy exactly inverted. The box
 * carries the state; the text carries the meaning, and reads first. */
function Check({ checked, onChange, children }: { checked: boolean; onChange: (value: boolean) => void; children: ReactNode }) {
  return (
    <label className="flex min-h-7 items-center gap-2.5 rounded-control px-1.5 py-0.5 text-base text-text transition-colors duration-100 hover:bg-surface-2">
      <input className="check" type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span>{children}</span>
    </label>
  );
}
