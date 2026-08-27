import { useEffect, useRef } from "react";

export interface TerminalLine {
  id: string;
  text: string;
  variant?: "stdout" | "stderr" | "info";
}

interface TerminalViewProps {
  lines: TerminalLine[];
  emptyMessage?: string;
}

/**
 * Lightweight scrollback pane. Deliberately not xterm.js: Forgeyard only
 * needs to render plain, already-line-split text with per-line coloring,
 * not a full terminal emulator (cursor control, ANSI SGR sequences, PTY
 * resize) — a real terminal emulator earns its place only once a feature
 * actually needs one (e.g. an interactive shell), see AutomationPanel/
 * ProcessPanel for where that line currently sits.
 */
export function TerminalView({ lines, emptyMessage = "No output yet." }: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [lines.length]);

  return (
    <div className="terminal" ref={containerRef}>
      {lines.length === 0 ? (
        <div className="terminal__empty">{emptyMessage}</div>
      ) : (
        lines.map((line) => (
          <div key={line.id} className={`terminal__line terminal__line--${line.variant ?? "stdout"}`}>
            {line.text}
          </div>
        ))
      )}
    </div>
  );
}
