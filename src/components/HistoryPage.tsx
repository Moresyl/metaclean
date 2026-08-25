import { CheckCircle2, Clock3, Trash2, XCircle } from "lucide-react";
import Button from "./Button";
import type { HistoryEntry } from "../types";
import { useI18n } from "../lib/i18n";

export default function HistoryPage({ entries, onClear }: { entries: HistoryEntry[]; onClear: () => void }) {
  const { locale, text } = useI18n();
  return (
    <section className="flex h-full max-w-[900px] flex-col gap-3">
      <div className="flex shrink-0 justify-end">
        <Button size="sm" onClick={onClear} disabled={!entries.length}>
          <Trash2 size={14} strokeWidth={2} />
          {text("清空记录", "Clear history")}
        </Button>
      </div>

      {!entries.length ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 rounded-panel border border-dashed border-line-strong px-6 text-center">
          {/* The same chip the queue's empty state wears, because they are the
              same moment in two rooms: nothing here yet, and nothing wrong. */}
          <span className="mb-0.5 grid size-11 place-items-center rounded-panel bg-surface-2 text-muted" aria-hidden="true">
            <Clock3 size={22} strokeWidth={1.8} />
          </span>
          <strong className="text-md font-semibold">{text("还没有处理记录", "No history yet")}</strong>
          <span className="max-w-[36ch] text-sm leading-relaxed text-muted">
            {text("完成一次文件净化后，结果会显示在这里。", "Completed cleanup jobs will appear here.")}
          </span>
        </div>
      ) : (
        <div className="grid min-h-0 flex-1 auto-rows-max gap-2.5 overflow-y-auto pr-0.5">
          {entries.map((entry) => {
            const successes = entry.results.filter((result) => result.success).length;
            const complete = successes === entry.results.length;
            return (
              <article className="overflow-hidden rounded-panel border border-line bg-surface shadow-panel" key={entry.id}>
                <header className="flex items-center gap-3 border-b border-line px-3 py-2">
                  <div className="min-w-0 flex-1 grid gap-0.5">
                    <strong className="truncate text-base font-medium tabular-nums">
                      {new Date(entry.createdAt).toLocaleString(locale)}
                    </strong>
                    <span className="truncate text-xs text-muted">
                      {entry.mode === "copy" ? text("安全副本", "Safe copy") : text("替换并备份", "Replace with backup")}
                    </span>
                  </div>
                  <em
                    className={`shrink-0 rounded-[3px] px-1.5 py-px text-xs font-normal not-italic tabular-nums ${
                      complete ? "bg-ok/14 text-ok" : "bg-warn/14 text-warn"
                    }`}
                  >
                    {successes}/{entry.results.length} {text("成功", "succeeded")}
                  </em>
                </header>
                <div className="grid">
                  {entry.results.map((result) => (
                    <div className="flex items-start gap-2 px-3 py-1.5 text-sm" key={result.sourcePath}>
                      <span className={`mt-px shrink-0 ${result.success ? "text-ok" : "text-danger"}`} aria-hidden="true">
                        {result.success ? <CheckCircle2 size={14} strokeWidth={2} /> : <XCircle size={14} strokeWidth={2} />}
                      </span>
                      <span className="min-w-0 flex-1 grid gap-0.5">
                        <strong className="truncate font-medium">{result.sourcePath.split(/[\\/]/).pop()}</strong>
                        {/* `xs`, not `2xs`: on the success path this line is a
                            Windows path and 11px would have been fine, but on
                            the failure path it is the error message, which is
                            Chinese — and 11px CJK is where the strokes merge. A
                            line cannot be sized for the case that reads best. */}
                        <small className={`selectable truncate text-xs ${result.success ? "text-muted" : "text-danger"}`}>
                          {result.success ? result.outputPath : result.error}
                        </small>
                      </span>
                    </div>
                  ))}
                </div>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
