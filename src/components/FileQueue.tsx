import { FileImage, FileText, FileType2, FileVideo2, Music2, Trash2, X } from "lucide-react";
import type { FileEntry } from "../types";
import { useI18n } from "../lib/i18n";
import { actionableFindingCount } from "../lib/files";

interface FileQueueProps { entries: FileEntry[]; preserveColorProfile: boolean; onRemove: (id: string) => void; onClear: () => void }

const icons = { image: FileImage, audio: Music2, video: FileVideo2, document: FileType2, pdf: FileText, text: FileText, unknown: FileText };

export default function FileQueue({ entries, preserveColorProfile, onRemove, onClear }: FileQueueProps) {
  const { text } = useI18n();
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
  } as Record<string, string>)[category] ?? fallback;
  return (
    <section className="queue-card">
      <header><div><h2>{text("待处理文件", "File queue")}</h2><span>{entries.length} {text("个文件", "file(s)")}</span></div><button type="button" onClick={onClear} disabled={!entries.length}><Trash2 size={14} />{text("清空", "Clear")}</button></header>
      {entries.length === 0 ? (
        <div className="empty-queue"><FileText size={22} /><span>{text("添加文件后，将在这里展示扫描状态", "Add files to see scan status here")}</span></div>
      ) : (
        <div className="file-list">
          {entries.map((entry) => {
            const Icon = icons[entry.kind];
            const findingCount = actionableFindingCount(entry.report, preserveColorProfile);
            const status = entry.report?.error
              ? entry.report.error
              : entry.status === "scanning" ? text("正在扫描…", "Scanning…")
              : entry.status === "clean" ? text("清理完成", "Cleaned")
              : entry.status === "scanned" ? (findingCount ? text(`发现 ${findingCount} 项痕迹`, `${findingCount} trace(s) found`) : text("未发现隐私痕迹", "No privacy traces found"))
              : entry.kind === "unknown" ? text("格式将在扫描时确认", "Format will be checked during scan") : text("等待扫描", "Waiting to scan");
            return <div className={`file-row ${entry.status}`} key={entry.id}><div className="file-kind"><Icon size={17} /></div><div className="file-name"><strong>{entry.name}</strong><span>{status}</span>{entry.report?.findings.length ? <div className="finding-tags">{entry.report.findings.map((finding) => <em key={finding.category}>{findingLabel(finding.category, finding.label)} · {finding.count}</em>)}</div> : null}</div><button aria-label={text(`移除 ${entry.name}`, `Remove ${entry.name}`)} type="button" onClick={() => onRemove(entry.id)}><X size={15} /></button></div>;
          })}
        </div>
      )}
    </section>
  );
}
