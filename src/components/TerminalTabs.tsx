import { useState } from "react";
import { TerminalPane } from "./TerminalPane";

interface TerminalTabsProps {
  cwd?: string;
  theme: "light" | "dark";
}

interface TabState {
  id: string;
  title: string;
}

let tabCounter = 0;

export function TerminalTabs({ cwd, theme }: TerminalTabsProps) {
  const [tabs, setTabs] = useState<TabState[]>(() => [
    { id: crypto.randomUUID(), title: `Terminal ${++tabCounter}` },
  ]);
  const [activeId, setActiveId] = useState(() => tabs[0].id);

  const addTab = () => {
    const tab = { id: crypto.randomUUID(), title: `Terminal ${++tabCounter}` };
    setTabs((prev) => [...prev, tab]);
    setActiveId(tab.id);
  };

  const closeTab = (id: string) => {
    setTabs((prev) => {
      const next = prev.filter((t) => t.id !== id);
      if (activeId === id && next.length > 0) {
        setActiveId(next[next.length - 1].id);
      }
      return next;
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
        <button className="terminal-tab-add" onClick={addTab} title="New terminal">
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
            <TerminalPane cwd={cwd} theme={theme} />
          </div>
        ))}
      </div>
    </div>
  );
}
