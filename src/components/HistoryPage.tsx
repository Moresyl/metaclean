import { CheckCircle2, Clock3, Trash2, XCircle } from "lucide-react";
import type { HistoryEntry } from "../types";
import { useI18n } from "../lib/i18n";

export default function HistoryPage({ entries, onClear }: { entries: HistoryEntry[]; onClear: () => void }) {
  const { locale, text } = useI18n();
  return (
    <section className="secondary-page">
      <div className="page-heading"><div><h1>{text("处理记录", "History")}</h1><p>{text("记录仅保存在此设备的应用数据中，不包含文件内容。", "History stays on this device and never stores file content.")}</p></div><button type="button" onClick={onClear} disabled={!entries.length}><Trash2 size={14} />{text("清空记录", "Clear history")}</button></div>
      {!entries.length ? <div className="large-empty"><Clock3 size={27} /><strong>{text("还没有处理记录", "No history yet")}</strong><span>{text("完成一次文件净化后，结果会显示在这里。", "Completed cleanup jobs will appear here.")}</span></div> : <div className="history-list">{entries.map((entry) => {
        const successes = entry.results.filter((result) => result.success).length;
        return <article className="history-card" key={entry.id}><header><div><strong>{new Date(entry.createdAt).toLocaleString(locale === "zh" ? "zh-CN" : "en-US")}</strong><span>{entry.mode === "copy" ? text("安全副本", "Safe copy") : text("替换并备份", "Replace with backup")}</span></div><em>{successes}/{entry.results.length} {text("成功", "succeeded")}</em></header>{entry.results.map((result) => <div className="history-result" key={result.sourcePath}>{result.success ? <CheckCircle2 size={15} /> : <XCircle size={15} />}<span><strong>{result.sourcePath.split(/[\\/]/).pop()}</strong><small>{result.success ? result.outputPath : result.error}</small></span></div>)}</article>;
      })}</div>}
    </section>
  );
}
