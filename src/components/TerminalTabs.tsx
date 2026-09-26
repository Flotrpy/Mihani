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
  hidden: boolean;
}

let tabCounter = 0;

export const TerminalTabs = forwardRef<TerminalTabsHandle, TerminalTabsProps>(
  function TerminalTabs({ cwd, theme }, ref) {
    const [tabs, setTabs] = useState<TabState[]>(() => [
      { id: crypto.randomUUID(), title: `Terminal ${++tabCounter}`, exited: false, hidden: false },
    ]);
    const [activeId, setActiveId] = useState(() => tabs[0].id);
    const [showBackground, setShowBackground] = useState(false);
    const notify = useNotify();

    const addTab = (title?: string, initialCommand?: string) => {
      const tab = {
        id: crypto.randomUUID(),
        title: title ?? `Terminal ${++tabCounter}`,
        initialCommand,
        exited: false,
        hidden: false,
      };
      setTabs((prev) => [...prev, tab]);
      setActiveId(tab.id);
    };

    useImperativeHandle(ref, () => ({
      openTab: (title, initialCommand) => addTab(title, initialCommand),
    }));

    const visibleTabs = tabs.filter((t) => !t.hidden);
    const backgroundTabs = tabs.filter((t) => t.hidden);

    const closeTab = (id: string) => {
      setTabs((prev) => {
        const next = prev.filter((t) => t.id !== id);
        if (activeId === id) {
          const stillVisible = next.filter((t) => !t.hidden);
          if (stillVisible.length > 0) setActiveId(stillVisible[stillVisible.length - 1].id);
        }
        return next;
      });
    };

    const hideTab = (id: string) => {
      setTabs((prev) => {
        const next = prev.map((t) => (t.id === id ? { ...t, hidden: true } : t));
        if (activeId === id) {
          const stillVisible = next.filter((t) => !t.hidden);
          if (stillVisible.length > 0) setActiveId(stillVisible[stillVisible.length - 1].id);
        }
        return next;
      });
    };

    const restoreTab = (id: string) => {
      setTabs((prev) => prev.map((t) => (t.id === id ? { ...t, hidden: false } : t)));
      setActiveId(id);
      setShowBackground(false);
    };

    const markExited = (id: string) => {
      setTabs((prev) => {
        const tab = prev.find((t) => t.id === id);
        if (tab && (tab.hidden || id !== activeId)) {
          notify("Mihani", `${tab.title} finished`);
        }
        return prev.map((t) => (t.id === id ? { ...t, exited: true } : t));
      });
    };

    return (
      <div className="terminal-tabs">
        <div className="terminal-tabbar">
          {visibleTabs.map((tab) => (
            <div
              key={tab.id}
              className={`terminal-tab ${tab.id === activeId ? "terminal-tab-active" : ""}`}
              onClick={() => setActiveId(tab.id)}
            >
              {tab.exited && <span className="terminal-tab-exited-dot" title="Process exited" />}
              <span>{tab.title}</span>
              <button
                className="terminal-tab-hide"
                title="Run in background"
                onClick={(e) => {
                  e.stopPropagation();
                  hideTab(tab.id);
                }}
              >
                ⌄
              </button>
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
          <div className="terminal-tabbar-spacer" />
          {backgroundTabs.length > 0 && (
            <div className="background-tasks">
              <button
                className="background-tasks-btn"
                onClick={() => setShowBackground((v) => !v)}
              >
                Background ({backgroundTabs.length})
              </button>
              {showBackground && (
                <ul className="background-tasks-list">
                  {backgroundTabs.map((tab) => (
                    <li key={tab.id} onClick={() => restoreTab(tab.id)}>
                      {tab.exited && <span className="terminal-tab-exited-dot" />}
                      {tab.title}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
        <div className="terminal-tab-content">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className="terminal-tab-pane"
              style={{ display: !tab.hidden && tab.id === activeId ? "flex" : "none" }}
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
