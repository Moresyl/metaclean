import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Search } from "lucide-react";
import { useI18n } from "../lib/i18n";

/**
 * Every command the window can run, one keystroke away.
 *
 * A desktop app is judged partly on whether the keyboard can reach the whole
 * of it. The menu bar used to be that guarantee; in a chromeless window this
 * is. It also does what a menu bar never could — find a command by any part of
 * its name, in whichever of the thirty-two languages the interface is running.
 */
export interface Command {
  id: string;
  /** The heading this command is listed under. */
  group: string;
  label: string;
  icon?: ReactNode;
  accelerator?: string;
  disabled?: boolean;
  run: () => void;
}

/**
 * Ranks a command against what has been typed so far.
 *
 * The match is a subsequence rather than a substring, so "cf" finds "Choose
 * files" — but runs of adjacent characters and matches at the start of a word
 * score far higher, which is what keeps the literal spelling on top. Returns
 * null when the characters are not all there, in order.
 */
export function score(query: string, candidate: string): number | null {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return 0;
  const hay = candidate.toLocaleLowerCase();
  let from = 0;
  let total = 0;
  let previous = -1;
  for (const character of needle) {
    if (character === " ") continue;
    const at = hay.indexOf(character, from);
    if (at < 0) return null;
    total += at === previous + 1 ? 3 : 1;
    if (at === 0 || " -·/、，".includes(hay[at - 1] ?? "")) total += 2;
    previous = at;
    from = at + character.length;
  }
  // Among equally good matches the shorter label is the one meant.
  return total * 1000 - hay.length;
}

export default function CommandPalette({ commands, onClose }: { commands: Command[]; onClose: () => void }) {
  const { text } = useI18n();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const restoreFocus = useRef<Element | null>(null);

  const matches = useMemo(() => {
    if (!query.trim()) return commands;
    return commands
      .flatMap((command) => {
        const ranked = score(query, command.label);
        const grouped = score(query, `${command.group} ${command.label}`);
        const best = ranked ?? (grouped === null ? null : grouped - 4000);
        return best === null ? [] : [{ command, rank: best }];
      })
      .sort((left, right) => right.rank - left.rank)
      .map(({ command }) => command);
  }, [commands, query]);

  // A new query invalidates the highlight; keep it on the best match instead of
  // wherever the previous list happened to leave it.
  useEffect(() => setActive(0), [query]);

  // The palette takes the keyboard on open and gives it back on close, so
  // dismissing it never strands focus on the document body.
  useLayoutEffect(() => {
    restoreFocus.current = document.activeElement;
    field.current?.focus();
    return () => {
      const previous = restoreFocus.current;
      if (previous instanceof HTMLElement && document.contains(previous)) previous.focus();
    };
  }, []);

  useEffect(() => {
    list.current?.querySelector<HTMLElement>("[aria-selected='true']")?.scrollIntoView({ block: "nearest" });
  }, [active, matches]);

  const step = (delta: number) => {
    if (!matches.length) return;
    setActive((current) => (current + delta + matches.length) % matches.length);
  };
  const choose = (command: Command | undefined) => {
    if (!command || command.disabled) return;
    onClose();
    command.run();
  };

  let heading: string | undefined;
  return (
    <div
      // Sits high rather than centred: the palette is answered by typing, and a
      // box in the middle of the window pushes the answer below the eye line.
      className="palette-layer animate-fade fixed inset-0 z-50 flex justify-center bg-canvas-deep/55 pt-[12vh] backdrop-blur-[2px]"
      onPointerDown={onClose}
    >
      <div
        className="animate-pop flex h-fit max-h-[62vh] w-[min(560px,88vw)] flex-col overflow-hidden rounded-panel border border-line-strong bg-surface shadow-lift"
        role="dialog"
        aria-modal="true"
        aria-label={text("命令", "Commands")}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <div className="flex shrink-0 items-center gap-2.5 border-b border-line px-3">
          <Search size={14} strokeWidth={2} aria-hidden="true" className="shrink-0 text-muted" />
          <input
            ref={field}
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-results"
            aria-label={text("搜索命令…", "Search commands…")}
            placeholder={text("搜索命令…", "Search commands…")}
            spellCheck={false}
            autoComplete="off"
            value={query}
            className="h-[42px] min-w-0 flex-1 bg-transparent text-md text-text outline-none placeholder:text-faint"
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") { event.preventDefault(); onClose(); return; }
              if (event.key === "ArrowDown") { event.preventDefault(); step(1); return; }
              if (event.key === "ArrowUp") { event.preventDefault(); step(-1); return; }
              if (event.key === "Home") { event.preventDefault(); setActive(0); return; }
              if (event.key === "End") { event.preventDefault(); setActive(Math.max(0, matches.length - 1)); return; }
              if (event.key === "Enter") { event.preventDefault(); choose(matches[active]); }
            }}
          />
          <kbd className="kbd hidden sm:block">Esc</kbd>
        </div>
        <div
          className="min-h-0 flex-1 overflow-y-auto p-1.5"
          id="palette-results"
          role="listbox"
          aria-label={text("命令", "Commands")}
          ref={list}
        >
          {matches.length === 0 ? (
            <p className="px-3 py-8 text-center text-base text-muted">{text("没有匹配的命令", "No matching command")}</p>
          ) : (
            matches.map((command, index) => {
              // Headings only survive the unfiltered list; once results are
              // ranked, grouping them again would fight the ranking.
              const label = query.trim() || command.group === heading ? undefined : command.group;
              if (label) heading = label;
              return (
                <div key={command.id}>
                  {label ? <div className="palette-group caption px-2 pt-2.5 pb-1">{label}</div> : null}
                  <button
                    type="button"
                    role="option"
                    aria-selected={index === active}
                    // Every row is set in the window's own ink, the selected one
                    // included. The list used to grey fourteen commands so that
                    // one could be legible, which is backwards: these are the
                    // options somebody is choosing *between*, and they all have
                    // to be readable at once for the choice to be made at all.
                    // The cursor does not need the rest of the list to step out
                    // of its way — it is a mint ground and a mint glyph, and on
                    // a near-black panel that is not a subtle thing.
                    //
                    // `bg-surface-2`, which is what it was, is eight units of
                    // lightness away from the panel behind it. On a surface
                    // driven by arrow keys, where the highlight is the only
                    // answer to "what does Enter do", eight units is not a
                    // highlight. The tint is the same one every selected control
                    // in this window wears, so it needs no learning.
                    className={[
                      "flex w-full items-center gap-2.5 rounded-control px-2 py-[7px] text-left text-base text-text",
                      "transition-colors duration-75 disabled:pointer-events-none disabled:opacity-40",
                      index === active ? "active bg-brand/12" : "",
                    ].join(" ")}
                    disabled={command.disabled}
                    onPointerEnter={() => setActive(index)}
                    onClick={() => choose(command)}
                  >
                    <span
                      className={`grid size-[15px] shrink-0 place-items-center ${index === active ? "text-brand" : "text-muted"}`}
                      aria-hidden="true"
                    >
                      {command.icon}
                    </span>
                    <span className="min-w-0 flex-1 truncate">{command.label}</span>
                    {command.accelerator ? <kbd className="kbd">{command.accelerator}</kbd> : null}
                  </button>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
