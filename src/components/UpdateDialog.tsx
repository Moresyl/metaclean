import { ArrowRight, Download, ExternalLink, Sparkles, X } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";
import Button, { IconButton } from "./Button";
import { useUpdate } from "../contexts/UpdateContext";
import { useI18n } from "../lib/i18n";
import { loopFocus } from "../lib/focus";

export default function UpdateDialog() {
  const update = useUpdate();
  const { text } = useI18n();
  const [opening, setOpening] = useState(false);
  const [openError, setOpenError] = useState<string>();
  const dialog = useRef<HTMLElement>(null);
  const primaryAction = useRef<HTMLButtonElement>(null);

  useLayoutEffect(() => {
    if (!update.promptOpen) return;
    const previous = document.activeElement;
    primaryAction.current?.focus();
    const keepFocusInside = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        update.dismissUpdatePrompt();
        return;
      }
      const current = dialog.current;
      if (!current) return;
      loopFocus(event, current);
    };
    window.addEventListener("keydown", keepFocusInside);
    return () => {
      window.removeEventListener("keydown", keepFocusInside);
      if (previous instanceof HTMLElement && document.contains(previous)) previous.focus();
    };
  }, [update.dismissUpdatePrompt, update.promptOpen]);

  if (!update.promptOpen || !update.info) return null;

  const publishedDate = update.info.publishedAt?.slice(0, 10);
  const updatePercent = update.progress?.total
    ? Math.min(100, Math.round((update.progress.downloaded / update.progress.total) * 100))
    : undefined;
  const runPrimaryAction = async () => {
    if (update.runtime.selfUpdateSupported) {
      await update.installUpdate();
      return;
    }
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
    <div
      className="animate-fade fixed inset-0 z-50 grid place-items-center bg-canvas-deep/60 p-6 backdrop-blur-[2px]"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) update.dismissUpdatePrompt();
      }}
    >
      <section
        className="animate-pop relative flex w-[min(430px,100%)] flex-col gap-4 rounded-panel border border-line-strong bg-surface p-5 shadow-lift"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-dialog-title"
        aria-describedby="update-dialog-notes"
        ref={dialog}
      >
        {/* Top-right rather than in the footer: the dialog is an offer, and the
            way out of an offer should not be one of the two decisions. */}
        <IconButton
          className="absolute top-2.5 right-2.5"
          size="sm"
          type="button"
          onClick={update.dismissUpdatePrompt}
          aria-label={text("稍后提醒", "Remind me later")}
        >
          <X size={14} strokeWidth={2} />
        </IconButton>

        <div className="grid gap-2.5">
          <div className="grid size-9 place-items-center rounded-panel bg-brand/12 text-brand" aria-hidden="true">
            <Sparkles size={17} strokeWidth={1.8} />
          </div>
          <div className="grid gap-0.5">
            <p className="caption">{text("发现 MetaClean 新版本", "A new MetaClean version is available")}</p>
            <h2 id="update-dialog-title" className="font-display text-2xl leading-tight font-semibold tabular-nums">
              v{update.info.availableVersion}
            </h2>
            <p className="text-sm text-muted tabular-nums">
              {text(`当前版本 v${update.info.currentVersion}`, `Currently installed: v${update.info.currentVersion}`)}
              {publishedDate ? ` · ${publishedDate}` : ""}
            </p>
          </div>
        </div>

        <div
          className="selectable grid max-h-[34vh] gap-1.5 overflow-y-auto rounded-control border border-line bg-canvas-deep p-3"
          aria-label={text("更新内容", "What's new")}
          id="update-dialog-notes"
        >
          <strong className="caption">{text("本次更新内容", "What's new")}</strong>
          <p className="text-base leading-relaxed whitespace-pre-line text-muted">
            {update.info.notes ?? text("查看 GitHub Release 获取本次版本的完整更新说明。", "Open the GitHub Release for the complete update notes.")}
          </p>
        </div>

        {update.status === "updating" ? (
          <progress className="progress" aria-label={text("更新进度", "Update progress")} max={100} value={updatePercent} />
        ) : null}

        {openError || (update.status === "error" && update.error) ? (
          <p className="rounded-control border border-danger/40 bg-danger/10 px-2.5 py-2 text-sm text-danger" role="alert">
            {openError
              ? text(`无法打开 GitHub：${openError}`, `Could not open GitHub: ${openError}`)
              : text(`安装失败：${update.error}`, `Update failed: ${update.error}`)}
          </p>
        ) : null}

        <div className="flex items-center justify-end gap-2 border-t border-line pt-3.5">
          <Button variant="ghost" onClick={update.dismissUpdatePrompt}>
            {text("稍后", "Later")}
          </Button>
          <Button ref={primaryAction} variant="primary" disabled={opening || update.status === "updating" || !update.runtimeReady} onClick={() => void runPrimaryAction()}>
            {update.runtime.selfUpdateSupported ? <Download size={14} strokeWidth={2} /> : <ExternalLink size={14} strokeWidth={2} />}
            {!update.runtimeReady
              ? text("正在确认安装方式…", "Checking install method…")
              : update.runtime.selfUpdateSupported
                ? update.status === "updating"
                  ? `${text("正在安装…", "Installing…")}${updatePercent === undefined ? "" : ` ${updatePercent}%`}`
                  : text("一键安装更新", "Install update")
                : opening
                  ? text("正在打开…", "Opening…")
                  : text("前往 GitHub 查看并下载", "View and download on GitHub")}
            {update.runtime.selfUpdateSupported ? null : <ArrowRight size={14} strokeWidth={2} />}
          </Button>
        </div>
      </section>
    </div>
  );
}
