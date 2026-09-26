import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";

interface TerminalOutputEvent {
  id: string;
  data: string;
}

interface TerminalExitEvent {
  id: string;
  code: number | null;
}

interface TerminalPaneProps {
  cwd?: string;
  theme: "light" | "dark";
  initialCommand?: string;
}

const XTERM_THEMES = {
  dark: {
    background: "#0f0b1a",
    foreground: "#ece7f9",
    cursor: "#9b7cff",
    selectionBackground: "#33285a",
  },
  light: {
    background: "#1a1526",
    foreground: "#ece7f9",
    cursor: "#7c5cff",
    selectionBackground: "#33285a",
  },
};

export function TerminalPane({ cwd, theme, initialCommand }: TerminalPaneProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const sessionIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      convertEol: true,
      cursorBlink: true,
      fontFamily: "'JetBrains Mono', 'Fira Code', Menlo, monospace",
      fontSize: 13,
      theme: XTERM_THEMES[theme],
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    fitAddon.fit();

    terminalRef.current = term;
    fitAddonRef.current = fitAddon;

    let unlistenOutput: UnlistenFn | undefined;
    let unlistenExit: UnlistenFn | undefined;
    let disposed = false;

    (async () => {
      const id = await invoke<string>("terminal_spawn", {
        cwd: cwd ?? null,
        cols: term.cols,
        rows: term.rows,
        initialCommand: initialCommand ?? null,
      });
      if (disposed) return;
      sessionIdRef.current = id;

      unlistenOutput = await listen<TerminalOutputEvent>(
        "terminal://output",
        (event) => {
          if (event.payload.id === id) {
            term.write(event.payload.data);
          }
        },
      );

      unlistenExit = await listen<TerminalExitEvent>(
        "terminal://exit",
        (event) => {
          if (event.payload.id === id) {
            term.write("\r\n\x1b[90m[process exited]\x1b[0m\r\n");
          }
        },
      );

      term.onData((data) => {
        invoke("terminal_write", { id, data }).catch(() => {});
      });
    })();

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      const id = sessionIdRef.current;
      if (id) {
        invoke("terminal_resize", {
          id,
          cols: term.cols,
          rows: term.rows,
        }).catch(() => {});
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      disposed = true;
      resizeObserver.disconnect();
      unlistenOutput?.();
      unlistenExit?.();
      const id = sessionIdRef.current;
      if (id) {
        invoke("terminal_kill", { id }).catch(() => {});
      }
      term.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    terminalRef.current?.options && (terminalRef.current.options.theme = XTERM_THEMES[theme]);
  }, [theme]);

  return <div ref={containerRef} className="terminal-pane" />;
}
