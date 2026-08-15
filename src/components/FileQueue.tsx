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
            return <div className="file-row" key={entry.id}><div className="file-kind"><Icon size={17} /></div><div className="file-name"><strong>{entry.name}</strong><span>{entry.kind === "unknown" ? "暂不支持" : "等待扫描"}</span></div><button aria-label={`移除 ${entry.name}`} type="button" onClick={() => onRemove(entry.id)}><X size={15} /></button></div>;
          })}
        </div>
      )}
    </section>
  );
}
