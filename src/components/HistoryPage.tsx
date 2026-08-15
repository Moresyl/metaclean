import { CheckCircle2, Clock3, Trash2, XCircle } from "lucide-react";
import type { HistoryEntry } from "../types";

export default function HistoryPage({ entries, onClear }: { entries: HistoryEntry[]; onClear: () => void }) {
  return (
    <section className="secondary-page">
      <div className="page-heading"><div><h1>处理记录</h1><p>记录仅保存在此设备的应用数据中，不包含文件内容。</p></div><button type="button" onClick={onClear} disabled={!entries.length}><Trash2 size={14} />清空记录</button></div>
      {!entries.length ? <div className="large-empty"><Clock3 size={27} /><strong>还没有处理记录</strong><span>完成一次文件净化后，结果会显示在这里。</span></div> : <div className="history-list">{entries.map((entry) => {
        const successes = entry.results.filter((result) => result.success).length;
        return <article className="history-card" key={entry.id}><header><div><strong>{new Date(entry.createdAt).toLocaleString("zh-CN")}</strong><span>{entry.mode === "copy" ? "安全副本" : "替换并备份"}</span></div><em>{successes}/{entry.results.length} 成功</em></header>{entry.results.map((result) => <div className="history-result" key={result.sourcePath}>{result.success ? <CheckCircle2 size={15} /> : <XCircle size={15} />}<span><strong>{result.sourcePath.split(/[\\/]/).pop()}</strong><small>{result.success ? result.outputPath : result.error}</small></span></div>)}</article>;
      })}</div>}
    </section>
  );
}
