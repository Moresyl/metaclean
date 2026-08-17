import { useMemo, useState } from "react";
import { FileImage, FileSearch, FileText, FileType2, FileVideo2, Music2, Trash2, X } from "lucide-react";
import type { FileEntry } from "../types";
import { useI18n } from "../lib/i18n";
import { actionableFindingCount } from "../lib/files";

interface FileQueueProps { entries: FileEntry[]; preserveColorProfile: boolean; removeExtendedAttributes: boolean; onRemove: (id: string) => void; onClear: () => void; onReveal: (path: string) => void }

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

export default function FileQueue({ entries, preserveColorProfile, removeExtendedAttributes, onRemove, onClear, onReveal }: FileQueueProps) {
  const { locale, text } = useI18n();
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [descending, setDescending] = useState(false);
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
    document_metadata: text("作者 / 生成器 / AI 元数据", "Author / generator / AI metadata"),
    color_profile: "ICC / sRGB",
    macos_xattr: "macOS · xattr",
  } as Record<string, string>)[category] ?? fallback;
  return (
    <section className="queue-card">
      <header><div><h2>{text("待处理文件", "File queue")}</h2><span>{entries.length} {text("个文件", "file(s)")}</span></div><div className="queue-tools"><label><span className="visually-hidden">{text("待处理文件", "File queue")}</span><select aria-label={text("待处理文件", "File queue")} value={sortKey} onChange={(event) => setSortKey(event.target.value as SortKey)}><option value="name">A–Z</option><option value="type">.EXT</option><option value="sourceSize">KB</option><option value="outputSize">KB✓</option><option value="findings">#</option></select></label><button className="sort-direction" aria-label={`${text("待处理文件", "File queue")} ${descending ? "↓" : "↑"}`} type="button" onClick={() => setDescending((current) => !current)}>{descending ? "↓" : "↑"}</button><button type="button" onClick={onClear} disabled={!entries.length}><Trash2 size={14} />{text("清空", "Clear")}</button></div></header>
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
            return <div className={`file-row ${entry.status}`} key={entry.id}><div className="file-kind"><Icon size={17} /></div><div className="file-name"><strong>{entry.name}</strong><span>{status}</span>{sourceSize !== undefined ? <span className="file-size-delta">{formatBytes(sourceSize)}{outputSize !== undefined && sizeDelta !== undefined ? ` → ${formatBytes(outputSize)} (${sizeDelta === 0 ? "±0 B" : `${sizeDelta > 0 ? "−" : "+"}${formatBytes(Math.abs(sizeDelta))}`})` : ""}</span> : null}{entry.report?.findings.length ? <div className="finding-tags">{entry.report.findings.map((finding) => <em key={finding.category}>{findingLabel(finding.category, finding.label)} · {finding.count}</em>)}</div> : null}</div><div className="file-actions">{outputPath ? <button aria-label={outputPath} title={outputPath} type="button" onClick={() => onReveal(outputPath)}><FileSearch size={15} /></button> : null}<button aria-label={text(`移除 ${entry.name}`, `Remove ${entry.name}`)} type="button" onClick={() => onRemove(entry.id)}><X size={15} /></button></div></div>;
          })}
        </div>
      )}
    </section>
  );
}
