import { FileImage, FileText, FileType2, Trash2, X } from "lucide-react";
import type { FileEntry } from "../types";

interface FileQueueProps { entries: FileEntry[]; onRemove: (id: string) => void; onClear: () => void }

const icons = { image: FileImage, document: FileType2, pdf: FileText, text: FileText, unknown: FileText };

export default function FileQueue({ entries, onRemove, onClear }: FileQueueProps) {
  return (
    <section className="queue-card">
      <header><div><h2>待处理文件</h2><span>{entries.length} 个文件</span></div><button type="button" onClick={onClear} disabled={!entries.length}><Trash2 size={14} />清空</button></header>
      {entries.length === 0 ? (
        <div className="empty-queue"><FileText size={22} /><span>添加文件后，将在这里展示扫描状态</span></div>
      ) : (
        <div className="file-list">
          {entries.map((entry) => {
            const Icon = icons[entry.kind];
            const findingCount = entry.report?.findings.reduce((total, finding) => total + finding.count, 0) ?? 0;
            const status = entry.report?.error
              ? entry.report.error
              : entry.status === "scanning" ? "正在扫描…"
              : entry.status === "clean" ? "清理完成"
              : entry.status === "scanned" ? (findingCount ? `发现 ${findingCount} 项痕迹` : "未发现隐私痕迹")
              : entry.kind === "unknown" ? "格式将在扫描时确认" : "等待扫描";
            return <div className={`file-row ${entry.status}`} key={entry.id}><div className="file-kind"><Icon size={17} /></div><div className="file-name"><strong>{entry.name}</strong><span>{status}</span>{entry.report?.findings.length ? <div className="finding-tags">{entry.report.findings.map((finding) => <em key={finding.category}>{finding.label} · {finding.count}</em>)}</div> : null}</div><button aria-label={`移除 ${entry.name}`} type="button" onClick={() => onRemove(entry.id)}><X size={15} /></button></div>;
          })}
        </div>
      )}
    </section>
  );
}
