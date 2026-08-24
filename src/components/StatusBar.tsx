import { Files, ShieldCheck } from "lucide-react";
import { useUpdate } from "../contexts/UpdateContext";
import { useI18n } from "../lib/i18n";

interface StatusBarProps {
  busy: boolean;
  fileCount: number;
}

export default function StatusBar({ busy, fileCount }: StatusBarProps) {
  const { text } = useI18n();
  const update = useUpdate();

  return (
    <footer className="statusbar">
      <div className="statusbar-segment statusbar-state">
        <span className={`status-dot ${busy ? "busy" : ""}`} aria-hidden="true" />
        <span>{busy ? text("正在处理", "Working") : text("就绪", "Ready")}</span>
      </div>
      <div className="statusbar-segment" data-tip={text("扫描和清理均在本机完成", "Scanning and cleaning stay on this device")}>
        <ShieldCheck size={11} aria-hidden="true" />
        <span>{text("纯本地处理", "Local only")}</span>
      </div>
      <span className="statusbar-spacer" />
      <div className="statusbar-segment">
        <Files size={11} aria-hidden="true" />
        <span>{fileCount} {text("个文件", "files")}</span>
      </div>
      <div className="statusbar-segment statusbar-version">MetaClean v{update.currentVersion ?? "…"}</div>
    </footer>
  );
}
