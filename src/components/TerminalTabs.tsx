import { forwardRef, useImperativeHandle, useState } from "react";
import { TerminalPane } from "./TerminalPane";
import { useNotify } from "../hooks/useNotify";

interface TerminalTabsProps {
  cwd?: string;
  theme: "light" | "dark";
}

export interface TerminalTabsHandle {
  openTab: (title: string, initialCommand?: string) => void;
}

interface TabState {
  id: string;
  title: string;
  initialCommand?: string;
  exited: boolean;
}

let tabCounter = 0;

export const TerminalTabs = forwardRef<TerminalTabsHandle, TerminalTabsProps>(
  function TerminalTabs({ cwd, theme }, ref) {
    const [tabs, setTabs] = useState<TabState[]>(() => [
      { id: crypto.randomUUID(), title: `Terminal ${++tabCounter}`, exited: false },
    ]);
    const [activeId, setActiveId] = useState(() => tabs[0].id);
    const notify = useNotify();

    const addTab = (title?: string, initialCommand?: string) => {
      const tab = {
        id: crypto.randomUUID(),
        title: title ?? `Terminal ${++tabCounter}`,
        initialCommand,
        exited: false,
      };
      setTabs((prev) => [...prev, tab]);
      setActiveId(tab.id);
    };

    useImperativeHandle(ref, () => ({
      openTab: (title, initialCommand) => addTab(title, initialCommand),
    }));

    const closeTab = (id: string) => {
      setTabs((prev) => {
        const next = prev.filter((t) => t.id !== id);
        if (activeId === id && next.length > 0) {
          setActiveId(next[next.length - 1].id);
        }
        return next;
      });
    };

    const markExited = (id: string) => {
      setTabs((prev) => {
        const tab = prev.find((t) => t.id === id);
        if (tab && id !== activeId) {
          notify("Mihani", `${tab.title} finished`);
        }
        return prev.map((t) => (t.id === id ? { ...t, exited: true } : t));
      });
    };

    return (
      <div className="terminal-tabs">
        <div className="terminal-tabbar">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`terminal-tab ${tab.id === activeId ? "terminal-tab-active" : ""}`}
              onClick={() => setActiveId(tab.id)}
            >
              {tab.exited && <span className="terminal-tab-exited-dot" title="Process exited" />}
              <span>{tab.title}</span>
              {tabs.length > 1 && (
                <button
                  className="terminal-tab-close"
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTab(tab.id);
                  }}
                >
                  ×
                </button>
              )}
            </div>
          ))}
          <button className="terminal-tab-add" onClick={() => addTab()} title="New terminal">
            +
          </button>
        </div>
        <div className="terminal-tab-content">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className="terminal-tab-pane"
              style={{ display: tab.id === activeId ? "flex" : "none" }}
            >
              <TerminalPane
                cwd={cwd}
                theme={theme}
                initialCommand={tab.initialCommand}
                onExit={() => markExited(tab.id)}
              />
            </div>
          ))}
        </div>
      </div>
    );
  },
);
