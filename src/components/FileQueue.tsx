import { useMemo, useState } from "react";
import { ArrowDownWideNarrow, ArrowUpNarrowWide, ChevronRight, Copy, FileDown, FileImage, FileSearch, FileText, FileType2, FileVideo2, FolderOpen, Music2, Tag, Trash2, X } from "lucide-react";
import Button, { IconButton } from "./Button";
import Select from "./Select";
import ContextMenu, { useContextMenu, type MenuEntry } from "./ContextMenu";
import type { FileEntry, Finding } from "../types";
import { useI18n } from "../lib/i18n";
import { actionableFindingCount } from "../lib/files";
import { copyText } from "../lib/window";

interface FileQueueProps { entries: FileEntry[]; preserveColorProfile: boolean; removeExtendedAttributes: boolean; onRemove: (id: string) => void; onClear: () => void; onReveal: (path: string) => void; onNotify: (message: string) => void }

type SortKey = "name" | "type" | "sourceSize" | "outputSize" | "findings";

const icons = { image: FileImage, audio: Music2, video: FileVideo2, document: FileType2, pdf: FileText, text: FileText, unknown: FileText };

/** How the row's left edge reports what the engine has done with it. */
const RAILS: Record<FileEntry["status"], string> = {
  ready: "bg-transparent",
  scanning: "bg-brand/50",
  scanned: "bg-brand",
  clean: "bg-ok",
  error: "bg-danger",
};

function extension(name: string) {
  const separator = name.lastIndexOf(".");
  return separator > 0 ? name.slice(separator + 1).toLocaleLowerCase() : "";
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; value >= 1024 && index < units.length; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}

/** Whether a finding survives the current fidelity settings. */
function isKept(finding: Finding, preserveColorProfile: boolean, removeExtendedAttributes: boolean) {
  if (finding.category === "color_profile") return preserveColorProfile;
  if (finding.category === "macos_xattr") return !removeExtendedAttributes;
  return false;
}

export default function FileQueue({ entries, preserveColorProfile, removeExtendedAttributes, onRemove, onClear, onReveal, onNotify }: FileQueueProps) {
  const { locale, text } = useI18n();
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [descending, setDescending] = useState(false);
  /** One row at a time: the window is short, and a list of open panels is a
   *  list nobody can scan. */
  const [expanded, setExpanded] = useState<string>();
  const [target, setTarget] = useState<FileEntry>();
  const menu = useContextMenu();
  const sortedEntries = useMemo(() => entries.map((entry, index) => ({ entry, index })).sort((left, right) => {
    const values: Record<SortKey, [string | number | undefined, string | number | undefined]> = {
      name: [left.entry.name, right.entry.name],
      type: [extension(left.entry.name), extension(right.entry.name)],
      sourceSize: [left.entry.report?.size ?? left.entry.result?.sourceSize ?? left.entry.size, right.entry.report?.size ?? right.entry.result?.sourceSize ?? right.entry.size],
      outputSize: [left.entry.result?.outputSize, right.entry.result?.outputSize],
      findings: [actionableFindingCount(left.entry.report, preserveColorProfile, removeExtendedAttributes), actionableFindingCount(right.entry.report, preserveColorProfile, removeExtendedAttributes)],
    };
    const [leftValue, rightValue] = values[sortKey];
    if (leftValue === undefined || rightValue === undefined) {
      if (leftValue === rightValue) return left.index - right.index;
      return leftValue === undefined ? 1 : -1;
    }
    const comparison = typeof leftValue === "string"
      ? leftValue.localeCompare(String(rightValue), locale, { numeric: true, sensitivity: "base" })
      : leftValue - Number(rightValue);
    return comparison === 0 ? left.index - right.index : descending ? -comparison : comparison;
  }).map(({ entry }) => entry), [descending, entries, locale, preserveColorProfile, removeExtendedAttributes, sortKey]);
  const findingLabel = (category: string, fallback: string) => ({
    unicode: text("不可见 Unicode 字符", "Invisible Unicode"),
    unicode_space: text("异常空白字符", "Unusual whitespace"),
    image_metadata: text("图片元数据", "Image metadata"),
    audio_metadata: text("音频元数据", "Audio metadata"),
    video_metadata: text("视频用户数据与位置", "Video user data and location"),
    provenance: text("来源标记", "Provenance marker"),
    office_metadata: text("Office 隐私痕迹", "Office privacy trace"),
    pdf_metadata: text("PDF 文档属性 / XMP", "PDF properties / XMP"),
    embedded_image_metadata: text("嵌入图片元数据 / C2PA", "Embedded image metadata / C2PA"),
    document_metadata: text("作者 / 生成器 / AI 元数据", "Author / generator / AI metadata"),
    color_profile: "ICC / sRGB",
    macos_xattr: "macOS · xattr",
  } as Record<string, string>)[category] ?? fallback;

  async function copy(value: string) {
    onNotify(await copyText(value)
      ? text("已复制到剪贴板", "Copied to clipboard")
      : text("无法访问剪贴板", "The clipboard is unavailable"));
  }

  async function exportReport() {
    const completed = entries.filter((entry) => entry.report || entry.result);
    if (!completed.length) return;
    try {
      const [{ save }, { invoke }, { getVersion }] = await Promise.all([
        import("@tauri-apps/plugin-dialog"),
        import("@tauri-apps/api/core"),
        import("@tauri-apps/api/app"),
      ]);
      const destination = await save({
        defaultPath: `MetaClean-audit-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!destination) return;
      const report = {
        schemaVersion: 1,
        product: "MetaClean",
        version: await getVersion(),
        exportedAt: new Date().toISOString(),
        summary: {
          files: completed.length,
          succeeded: completed.filter((entry) => entry.result?.success).length,
          failed: completed.filter((entry) => entry.status === "error").length,
          findings: completed.reduce((sum, entry) => sum + (entry.report?.findings.reduce((count, finding) => count + finding.count, 0) ?? 0), 0),
        },
        files: completed.map((entry) => ({
          path: entry.path,
          name: entry.name,
          format: entry.report?.format,
          sourceSize: entry.result?.sourceSize ?? entry.report?.size ?? entry.size,
          outputSize: entry.result?.outputSize,
          outputPath: entry.result?.outputPath,
          backupPath: entry.result?.backupPath,
          status: entry.status,
          findings: entry.report?.findings ?? [],
          removed: entry.result?.removed ?? [],
          error: entry.result?.error ?? entry.report?.error,
        })),
      };
      await invoke("export_audit_report", { path: destination, contents: JSON.stringify(report, null, 2) });
      onNotify(text(`审计报告已导出：${destination}`, `Audit report exported: ${destination}`));
    } catch (error) {
      onNotify(text(`无法导出审计报告：${String(error)}`, `Could not export audit report: ${String(error)}`));
    }
  }

  const menuEntries = (entry: FileEntry): MenuEntry[] => {
    const location = entry.result?.outputPath ?? entry.path;
    return [
      { id: "reveal", label: text("在文件夹中显示", "Show in folder"), icon: <FolderOpen size={14} />, disabled: !location, run: () => location && onReveal(location) },
      { id: "copy-path", label: text("复制路径", "Copy path"), icon: <Copy size={14} />, disabled: !location, run: () => location && void copy(location) },
      { id: "copy-name", label: text("复制文件名", "Copy file name"), icon: <Tag size={14} />, run: () => void copy(entry.name) },
      "separator",
      { id: "remove", label: text("从队列中移除", "Remove from queue"), icon: <X size={14} />, danger: true, run: () => onRemove(entry.id) },
    ];
  };

  const queueLabel = text("待处理文件", "File queue");

  return (
    <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-panel border border-line bg-surface shadow-panel">
      <header className="flex h-[42px] shrink-0 items-center gap-2 border-b border-line px-2.5">
        <h2 className="text-base font-semibold">{queueLabel}</h2>
        <span className="rounded-[3px] bg-surface-2 px-1.5 py-px text-xs text-muted tabular-nums">
          {entries.length} {text("个文件", "file(s)")}
        </span>
        <span className="flex-1" />

        {/* Two groups, not four controls in a row.
            What sorts the list and which way it sorts are one decision, so they
            are joined with no gap between them and read as a single control;
            what acts on the list is a second thing, and a hairline says so.
            Left as a plain row, a bordered dropdown followed by two bare icons
            followed by an icon with a word beside it gives the eye four
            unrelated objects and no way to tell which of them belong together. */}
        <div className="flex items-center gap-px">
          <Select
            aria-label={queueLabel}
            className="rounded-r-none"
            value={sortKey}
            onChange={(event) => setSortKey(event.target.value as SortKey)}
          >
            <option value="name">A–Z</option>
            <option value="type">.EXT</option>
            <option value="sourceSize">KB</option>
            <option value="outputSize">KB✓</option>
            <option value="findings">#</option>
          </Select>

          <IconButton
            variant="secondary"
            className="rounded-l-none"
            aria-label={descending ? text("改为升序", "Sort ascending") : text("改为降序", "Sort descending")}
            data-tip={text("排序方向", "Sort direction")}
            onClick={() => setDescending((current) => !current)}
          >
            {descending
              ? <ArrowDownWideNarrow size={14} strokeWidth={2} />
              : <ArrowUpNarrowWide size={14} strokeWidth={2} />}
          </IconButton>
        </div>

        <span className="mx-0.5 h-4 w-px shrink-0 bg-line-strong" aria-hidden="true" />

        <IconButton
          aria-label={text("导出审计报告", "Export audit report")}
          data-tip={text("导出审计报告", "Export audit report")}
          onClick={() => void exportReport()}
          disabled={!entries.some((entry) => entry.report || entry.result)}
        >
          <FileDown size={14} strokeWidth={2} />
        </IconButton>

        <Button variant="ghost" onClick={onClear} disabled={!entries.length}>
          <Trash2 size={14} strokeWidth={2} />
          {text("清空", "Clear")}
        </Button>
      </header>

      {entries.length === 0 ? (
        /* An empty panel is still a designed panel.
           A 22px glyph and a sentence, both in the dimmest ink the palette has,
           is what a disabled control looks like — and this one is not disabled,
           it is waiting. The glyph gets the same chip every other icon in this
           window sits in, and the sentence gets ink somebody can actually read
           at a glance across a 900px panel. */
        <div className="flex flex-1 flex-col items-center justify-center gap-2.5 px-6 py-10 text-center">
          <span className="grid size-11 place-items-center rounded-panel bg-surface-2 text-muted" aria-hidden="true">
            <FileText size={22} strokeWidth={1.8} />
          </span>
          <span className="text-sm text-muted">
            {text("添加文件后，将在这里展示扫描状态", "Add files to see scan status here")}
          </span>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
          {sortedEntries.map((entry) => {
            const Icon = icons[entry.kind];
            const findingCount = actionableFindingCount(entry.report, preserveColorProfile, removeExtendedAttributes);
            const sourceSize = entry.result?.sourceSize ?? entry.report?.size ?? entry.size;
            const outputSize = entry.result?.outputSize;
            const sizeDelta = sourceSize !== undefined && outputSize !== undefined ? sourceSize - outputSize : undefined;
            const outputPath = entry.result?.outputPath;
            const entryError = entry.result?.error ?? entry.report?.error;
            const status = entryError
              ? entryError
              : entry.status === "scanning" ? text("正在扫描…", "Scanning…")
              : entry.status === "clean" ? text("清理完成", "Cleaned")
              : entry.status === "scanned" ? (findingCount ? text(`发现 ${findingCount} 项痕迹`, `${findingCount} trace(s) found`) : text("未发现隐私痕迹", "No privacy traces found"))
              : entry.kind === "unknown" ? text("格式将在扫描时确认", "Format will be checked during scan") : text("等待扫描", "Waiting to scan");
            const findings = entry.report?.findings.length ? entry.report.findings : entry.result?.removed ?? [];
            const detailed = Boolean(entry.path || findings.length);
            const open = expanded === entry.id;
            const removedCategories = new Set((entry.result?.removed ?? []).map((finding) => finding.category));
            return (
              <div
                className={`file-item group relative overflow-hidden rounded-control transition-colors duration-100 ${open ? "bg-surface-2" : "hover:bg-surface-2/60"}`}
                key={entry.id}
                onContextMenu={(event) => { setTarget(entry); menu.open(event); }}
              >
                {/* The status is on the row's edge rather than in a badge: at a
                    glance the queue should read as a column of colours, and a
                    word per row is not a glance. */}
                <span
                  className={`absolute inset-y-1.5 left-0 w-[2px] rounded-full ${RAILS[entry.status]} ${entry.status === "scanning" ? "animate-pulse" : ""}`}
                  aria-hidden="true"
                />
                <div className="flex items-start gap-2.5 py-1.5 pr-1.5 pl-2.5">
                  <div className={`mt-px grid size-[26px] shrink-0 place-items-center rounded-control ${entry.status === "error" ? "bg-danger/12 text-danger" : entry.status === "clean" ? "bg-ok/12 text-ok" : "bg-canvas-deep text-muted"}`}>
                    <Icon size={14} strokeWidth={2} aria-hidden="true" />
                  </div>

                  <div className="file-name min-w-0 flex-1">
                    <strong className="block truncate text-base font-medium">{entry.name}</strong>
                    <span className={`block truncate text-xs ${entryError ? "text-danger" : entry.status === "clean" ? "text-ok" : "text-muted"}`}>
                      {status}
                    </span>
                    {sourceSize !== undefined ? (
                      /* `muted`, like the status line above it. The three lines
                         of a row are already ranked by size — 13, 12, 11 — and
                         ranking them a second time by ink only buys a bottom
                         line nobody can read: 11px digits at `faint` is the
                         least legible text in the window, and it is the line
                         that says how much of the file survived. */
                      <span className="block truncate text-2xs text-muted tabular-nums">
                        {formatBytes(sourceSize)}
                        {outputSize !== undefined && sizeDelta !== undefined
                          ? ` → ${formatBytes(outputSize)} (${sizeDelta === 0 ? "±0 B" : `${sizeDelta > 0 ? "−" : "+"}${formatBytes(Math.abs(sizeDelta))}`})`
                          : ""}
                      </span>
                    ) : null}
                    {findings.length ? (
                      <div className="mt-1 flex flex-wrap gap-1">
                        {findings.map((finding) => (
                          <em
                            key={finding.category}
                            className="rounded-[3px] bg-brand/12 px-1.5 py-px text-xs font-normal text-brand not-italic"
                          >
                            {findingLabel(finding.category, finding.label)} · {finding.count}
                          </em>
                        ))}
                      </div>
                    ) : null}
                  </div>

                  {/* Held back until the row is pointed at or one of them is
                      focused, so a queue of forty files is forty names rather
                      than a hundred and twenty buttons. */}
                  <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity duration-100 group-hover:opacity-100 group-focus-within:opacity-100">
                    {outputPath ? (
                      <IconButton size="sm" aria-label={outputPath} data-tip={outputPath} onClick={() => onReveal(outputPath)}>
                        <FileSearch size={14} strokeWidth={2} />
                      </IconButton>
                    ) : null}
                    {detailed ? (
                      <IconButton
                        size="sm"
                        aria-label={text("详细信息", "Details")}
                        aria-expanded={open}
                        onClick={() => setExpanded(open ? undefined : entry.id)}
                      >
                        <ChevronRight
                          size={14}
                          strokeWidth={2}
                          className={`transition-transform duration-150 ease-[var(--ease-out-soft)] ${open ? "rotate-90" : ""}`}
                        />
                      </IconButton>
                    ) : null}
                    <IconButton
                      size="sm"
                      aria-label={text(`移除 ${entry.name}`, `Remove ${entry.name}`)}
                      className="enabled:hover:bg-danger/12 enabled:hover:text-danger"
                      onClick={() => onRemove(entry.id)}
                    >
                      <X size={14} strokeWidth={2} />
                    </IconButton>
                  </div>
                </div>

                {open ? (
                  <div className="file-detail animate-rise grid gap-2.5 border-t border-line px-2.5 py-2.5 pl-[42px]">
                    <dl className="grid gap-1 text-xs">
                      {entry.report?.format ? <Field label={text("格式", "Format")}>{entry.report.format}</Field> : null}
                      {entry.path ? <Field label={text("位置", "Location")} selectable>{entry.path}</Field> : null}
                      {outputPath ? <Field label={text("输出", "Output")} selectable>{outputPath}</Field> : null}
                      {entry.result?.backupPath ? <Field label={text("备份", "Backup")} selectable>{entry.result.backupPath}</Field> : null}
                    </dl>
                    {findings.length ? (
                      <ul className="grid gap-px overflow-hidden rounded-control border border-line">
                        {findings.map((finding) => {
                          const kept = isKept(finding, preserveColorProfile, removeExtendedAttributes);
                          const gone = entry.status === "clean" && (removedCategories.size ? removedCategories.has(finding.category) : !kept);
                          return (
                            <li className="flex items-center gap-2 bg-surface px-2 py-1.5 text-xs" key={finding.category}>
                              <span className="min-w-0 flex-1 truncate">{findingLabel(finding.category, finding.label)}</span>
                              <span className="shrink-0 text-muted tabular-nums">{finding.count}</span>
                              <span
                                className={`shrink-0 rounded-[3px] px-1.5 py-px text-xs ${
                                  gone ? "bg-ok/14 text-ok" : kept ? "bg-surface-2 text-muted" : "bg-warn/14 text-warn"
                                }`}
                              >
                                {gone ? text("已移除", "Removed") : kept ? text("保留", "Kept") : text("将被移除", "Will be removed")}
                              </span>
                            </li>
                          );
                        })}
                      </ul>
                    ) : null}
                  </div>
                ) : null}
              </div>
            );
          })}
        </div>
      )}
      {menu.anchor && target ? <ContextMenu entries={menuEntries(target)} anchor={menu.anchor} label={target.name} onClose={menu.close} /> : null}
    </section>
  );
}

/* The value is the brighter of the two, which is the way round it was not.
 *
 * `格式` and `位置` are four characters the reader already knows are coming; the
 * path beside them is the thing they opened the row to see. Setting the label at
 * `faint` and the path at `muted` puts both of them below the window's own ink
 * and ranks them barely apart, so the row reads as a block of grey with a stripe
 * of slightly-less-grey in it. Label steps back, value steps up. */
function Field({ label, selectable, children }: { label: string; selectable?: boolean; children: React.ReactNode }) {
  return (
    <div className="flex items-baseline gap-2">
      <dt className="w-[3.5rem] shrink-0 text-muted">{label}</dt>
      <dd className={`min-w-0 flex-1 break-all text-text ${selectable ? "selectable" : ""}`}>{children}</dd>
    </div>
  );
}
