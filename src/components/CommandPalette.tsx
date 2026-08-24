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
    <div className="palette-layer" onPointerDown={onClose}>
      <div
        className="command-palette"
        role="dialog"
        aria-modal="true"
        aria-label={text("命令", "Commands")}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <div className="palette-search">
          <Search size={15} strokeWidth={1.9} aria-hidden="true" />
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
        </div>
        <div className="palette-results" id="palette-results" role="listbox" aria-label={text("命令", "Commands")} ref={list}>
          {matches.length === 0 ? (
            <p className="palette-empty">{text("没有匹配的命令", "No matching command")}</p>
          ) : (
            matches.map((command, index) => {
              // Headings only survive the unfiltered list; once results are
              // ranked, grouping them again would fight the ranking.
              const label = query.trim() || command.group === heading ? undefined : command.group;
              if (label) heading = label;
              return (
                <div key={command.id}>
                  {label ? <div className="palette-group">{label}</div> : null}
                  <button
                    type="button"
                    role="option"
                    aria-selected={index === active}
                    className={`palette-item ${index === active ? "active" : ""}`}
                    disabled={command.disabled}
                    onPointerEnter={() => setActive(index)}
                    onClick={() => choose(command)}
                  >
                    <span className="palette-icon" aria-hidden="true">{command.icon}</span>
                    <span className="palette-label">{command.label}</span>
                    {command.accelerator ? <kbd>{command.accelerator}</kbd> : null}
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
