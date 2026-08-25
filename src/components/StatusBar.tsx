import { Files, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import { useUpdate } from "../contexts/UpdateContext";
import { useI18n } from "../lib/i18n";

interface StatusBarProps {
  busy: boolean;
  fileCount: number;
}

/**
 * The strip along the bottom of the window.
 *
 * Everything on it is a readout that stays true for as long as the app is open:
 * whether the engine is working, that it never leaves this machine, how much is
 * queued, and which build is running.
 */
export default function StatusBar({ busy, fileCount }: StatusBarProps) {
  const { text } = useI18n();
  const update = useUpdate();

  return (
    <footer className="chrome relative z-20 flex h-[26px] items-stretch border-t border-line text-xs text-muted select-none">
      <Segment>
        <span className="relative grid size-[7px] shrink-0 place-items-center" aria-hidden="true">
          {/* A ring that breathes rather than a spinner: the scan reports its
              own progress in the queue, and a second animation racing it there
              would only be a worse copy. */}
          {busy ? (
            <span className="absolute inset-0 animate-ping rounded-full bg-brand/70" />
          ) : null}
          <span className={`relative size-[7px] rounded-full ${busy ? "bg-brand" : "bg-ok"}`} />
        </span>
        <span>{busy ? text("正在处理", "Working") : text("就绪", "Ready")}</span>
      </Segment>

      <Segment tip={text("扫描和清理均在本机完成", "Scanning and cleaning stay on this device")}>
        <ShieldCheck size={12} aria-hidden="true" className="shrink-0 text-brand" />
        <span>{text("纯本地处理", "Local only")}</span>
      </Segment>

      <span className="flex-1" />

      {/* Nothing on this strip sets its own ink. It is one rank of information
          at one size, and the two segments that had opted down to `faint` were
          the two carrying facts — how many files are queued, and which build is
          running — rendered dimmer than the word `就绪` beside them. */}
      <Segment>
        <Files size={12} aria-hidden="true" className="shrink-0" />
        <span className="tabular-nums">{fileCount} {text("个文件", "files")}</span>
      </Segment>

      <Segment>
        <span className="tabular-nums">MetaClean v{update.currentVersion ?? "…"}</span>
      </Segment>
    </footer>
  );
}

function Segment({ tip, children }: { tip?: string; children: ReactNode }) {
  return (
    <div className="flex items-center gap-1.5 px-2.5 whitespace-nowrap" data-tip={tip}>
      {children}
    </div>
  );
}
