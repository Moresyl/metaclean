import { useId, useLayoutEffect, useRef } from "react";
import Button, { IconButton } from "./Button";
import { loopFocus } from "../lib/focus";
import { useI18n } from "../lib/i18n";
import { X } from "lucide-react";

interface ConfirmDialogProps {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/** A small, modal guard for actions that discard local application state. */
export default function ConfirmDialog({ title, description, confirmLabel, onConfirm, onCancel }: ConfirmDialogProps) {
  const { text } = useI18n();
  const titleId = useId();
  const descriptionId = useId();
  const dialog = useRef<HTMLElement>(null);
  const confirm = useRef<HTMLButtonElement>(null);
  const onCancelRef = useRef(onCancel);
  onCancelRef.current = onCancel;

  useLayoutEffect(() => {
    const previous = document.activeElement;
    confirm.current?.focus();
    const keepFocusInside = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onCancelRef.current();
        return;
      }
      if (dialog.current) loopFocus(event, dialog.current);
    };
    window.addEventListener("keydown", keepFocusInside);
    return () => {
      window.removeEventListener("keydown", keepFocusInside);
      if (previous instanceof HTMLElement && previous !== document.body && previous !== document.documentElement && document.contains(previous) && !previous.matches(":disabled,[aria-disabled='true']")) {
        previous.focus();
      } else {
        document.querySelector<HTMLElement>("main[tabindex='-1']")?.focus();
      }
    };
  }, []);

  return (
    <div
      className="animate-fade fixed inset-0 z-50 grid place-items-center bg-canvas-deep/60 p-6 backdrop-blur-[2px]"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <section
        ref={dialog}
        className="animate-pop relative flex w-[min(390px,100%)] flex-col gap-3 rounded-panel border border-line-strong bg-surface p-5 shadow-lift"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <IconButton
          className="absolute top-2.5 right-2.5"
          size="sm"
          aria-label={text("取消", "Cancel")}
          onClick={onCancel}
        >
          <X size={14} strokeWidth={2} />
        </IconButton>
        <div className="grid gap-1 pr-7">
          <h2 id={titleId} className="font-display text-lg font-semibold">{title}</h2>
          <p id={descriptionId} className="text-sm leading-relaxed text-muted">{description}</p>
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-line pt-3">
          <Button variant="ghost" onClick={onCancel}>{text("取消", "Cancel")}</Button>
          <Button ref={confirm} variant="danger" onClick={onConfirm}>{confirmLabel}</Button>
        </div>
      </section>
    </div>
  );
}
