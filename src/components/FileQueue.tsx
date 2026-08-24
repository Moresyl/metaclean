import { useMemo, useState } from "react";
import { ChevronRight, Copy, FileDown, FileImage, FileSearch, FileText, FileType2, FileVideo2, FolderOpen, Music2, Tag, Trash2, X } from "lucide-react";
import ContextMenu, { useContextMenu, type MenuEntry } from "./ContextMenu";
import type { FileEntry, Finding } from "../types";
import { useI18n } from "../lib/i18n";
import { actionableFindingCount } from "../lib/files";
import { copyText } from "../lib/window";

interface FileQueueProps { entries: FileEntry[]; preserveColorProfile: boolean; removeExtendedAttributes: boolean; onRemove: (id: string) => void; onClear: () => void; onReveal: (path: string) => void; onNotify: (message: string) => void }

type SortKey = "name" | "type" | "sourceSize" | "outputSize" | "findings";

const icons = { image: FileImage, audio: Music2, video: FileVideo2, document: FileType2, pdf: FileText, text: FileText, unknown: FileText };

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
      { id: "reveal", label: text("在文件夹中显示", "Show in folder"), icon: <FolderOpen size={15} />, disabled: !location, run: () => location && onReveal(location) },
      { id: "copy-path", label: text("复制路径", "Copy path"), icon: <Copy size={15} />, disabled: !location, run: () => location && void copy(location) },
      { id: "copy-name", label: text("复制文件名", "Copy file name"), icon: <Tag size={15} />, run: () => void copy(entry.name) },
      "separator",
      { id: "remove", label: text("从队列中移除", "Remove from queue"), icon: <X size={15} />, danger: true, run: () => onRemove(entry.id) },
    ];
  };

  return (
    <section className="queue-card">
      <header><div><h2>{text("待处理文件", "File queue")}</h2><span>{entries.length} {text("个文件", "file(s)")}</span></div><div className="queue-tools"><label><span className="visually-hidden">{text("待处理文件", "File queue")}</span><select aria-label={text("待处理文件", "File queue")} value={sortKey} onChange={(event) => setSortKey(event.target.value as SortKey)}><option value="name">A–Z</option><option value="type">.EXT</option><option value="sourceSize">KB</option><option value="outputSize">KB✓</option><option value="findings">#</option></select></label><button className="sort-direction" aria-label={`${text("待处理文件", "File queue")} ${descending ? "↓" : "↑"}`} type="button" onClick={() => setDescending((current) => !current)}>{descending ? "↓" : "↑"}</button><button type="button" aria-label={text("导出审计报告", "Export audit report")} data-tip={text("导出审计报告", "Export audit report")} onClick={() => void exportReport()} disabled={!entries.some((entry) => entry.report || entry.result)}><FileDown size={14} /></button><button type="button" onClick={onClear} disabled={!entries.length}><Trash2 size={14} />{text("清空", "Clear")}</button></div></header>
      {entries.length === 0 ? (
        <div className="empty-queue"><FileText size={22} /><span>{text("添加文件后，将在这里展示扫描状态", "Add files to see scan status here")}</span></div>
      ) : (
        <div className="file-list">
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
            return <div className={`file-item ${open ? "open" : ""}`} key={entry.id} onContextMenu={(event) => { setTarget(entry); menu.open(event); }}>
              <div className={`file-row ${entry.status}`}><div className="file-kind"><Icon size={17} /></div><div className="file-name"><strong>{entry.name}</strong><span>{status}</span>{sourceSize !== undefined ? <span className="file-size-delta">{formatBytes(sourceSize)}{outputSize !== undefined && sizeDelta !== undefined ? ` → ${formatBytes(outputSize)} (${sizeDelta === 0 ? "±0 B" : `${sizeDelta > 0 ? "−" : "+"}${formatBytes(Math.abs(sizeDelta))}`})` : ""}</span> : null}{findings.length ? <div className="finding-tags">{findings.map((finding) => <em key={finding.category}>{findingLabel(finding.category, finding.label)} · {finding.count}</em>)}</div> : null}</div><div className="file-actions">{outputPath ? <button aria-label={outputPath} title={outputPath} type="button" onClick={() => onReveal(outputPath)}><FileSearch size={15} /></button> : null}{detailed ? <button className={`file-disclosure ${open ? "open" : ""}`} aria-label={text("详细信息", "Details")} aria-expanded={open} type="button" onClick={() => setExpanded(open ? undefined : entry.id)}><ChevronRight size={15} /></button> : null}<button aria-label={text(`移除 ${entry.name}`, `Remove ${entry.name}`)} type="button" onClick={() => onRemove(entry.id)}><X size={15} /></button></div></div>
              {open ? (
                <div className="file-detail">
                  <dl>
                    {entry.report?.format ? <div><dt>{text("格式", "Format")}</dt><dd>{entry.report.format}</dd></div> : null}
                    {entry.path ? <div><dt>{text("位置", "Location")}</dt><dd className="selectable">{entry.path}</dd></div> : null}
                    {outputPath ? <div><dt>{text("输出", "Output")}</dt><dd className="selectable">{outputPath}</dd></div> : null}
                    {entry.result?.backupPath ? <div><dt>{text("备份", "Backup")}</dt><dd className="selectable">{entry.result.backupPath}</dd></div> : null}
                  </dl>
                  {findings.length ? (
                    <ul className="detail-findings">
                      {findings.map((finding) => {
                        const kept = isKept(finding, preserveColorProfile, removeExtendedAttributes);
                        const gone = entry.status === "clean" && (removedCategories.size ? removedCategories.has(finding.category) : !kept);
                        return (
                          <li key={finding.category}>
                            <span className="detail-finding-label">{findingLabel(finding.category, finding.label)}</span>
                            <span className="detail-finding-count">{finding.count}</span>
                            <span className={`detail-finding-state ${gone ? "removed" : kept ? "kept" : "pending"}`}>
                              {gone ? text("已移除", "Removed") : kept ? text("保留", "Kept") : text("将被移除", "Will be removed")}
                            </span>
                          </li>
                        );
                      })}
                    </ul>
                  ) : null}
                </div>
              ) : null}
            </div>;
          })}
        </div>
      )}
      {menu.anchor && target ? <ContextMenu entries={menuEntries(target)} anchor={menu.anchor} label={target.name} onClose={menu.close} /> : null}
    </section>
  );
}
