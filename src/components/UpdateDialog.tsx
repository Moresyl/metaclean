import { ArrowRight, ExternalLink, Sparkles, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useUpdate } from "../contexts/UpdateContext";
import { useI18n } from "../lib/i18n";

export default function UpdateDialog() {
  const update = useUpdate();
  const { text } = useI18n();
  const [opening, setOpening] = useState(false);
  const [openError, setOpenError] = useState<string>();

  useEffect(() => {
    if (!update.promptOpen) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") update.dismissUpdatePrompt();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [update]);

  if (!update.promptOpen || !update.info) return null;

  const publishedDate = update.info.publishedAt?.slice(0, 10);
  const openGitHub = async () => {
    setOpening(true);
    setOpenError(undefined);
    try {
      await update.openRelease();
      update.dismissUpdatePrompt();
    } catch (reason) {
      setOpenError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setOpening(false);
    }
  };

  return (
    <div className="update-dialog-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) update.dismissUpdatePrompt();
    }}>
      <section className="update-dialog" role="dialog" aria-modal="true" aria-labelledby="update-dialog-title">
        <button className="update-dialog-close" type="button" onClick={update.dismissUpdatePrompt} aria-label={text("稍后提醒", "Remind me later")}>
          <X size={18}/>
        </button>
        <div className="update-dialog-icon"><Sparkles size={22}/></div>
        <p className="update-dialog-eyebrow">{text("发现 MetaClean 新版本", "A new MetaClean version is available")}</p>
        <h2 id="update-dialog-title">v{update.info.availableVersion}</h2>
        <p className="update-dialog-version">{text(`当前版本 v${update.info.currentVersion}`, `Currently installed: v${update.info.currentVersion}`)}{publishedDate ? ` · ${publishedDate}` : ""}</p>
        <div className="update-dialog-notes" aria-label={text("更新内容", "What's new")}>
          <strong>{text("本次更新内容", "What's new")}</strong>
          <p>{update.info.notes ?? text("查看 GitHub Release 获取本次版本的完整更新说明。", "Open the GitHub Release for the complete update notes.")}</p>
        </div>
        {openError ? <p className="update-dialog-error" role="alert">{text(`无法打开 GitHub：${openError}`, `Could not open GitHub: ${openError}`)}</p> : null}
        <div className="update-dialog-actions">
          <button type="button" onClick={update.dismissUpdatePrompt}>{text("稍后", "Later")}</button>
          <button className="primary" type="button" disabled={opening} onClick={() => void openGitHub()}>
            <ExternalLink size={16}/>{opening ? text("正在打开…", "Opening…") : text("前往 GitHub 查看并下载", "View and download on GitHub")}<ArrowRight size={15}/>
          </button>
        </div>
      </section>
    </div>
  );
}
