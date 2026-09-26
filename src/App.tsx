import { useRef, useState } from "react";
import { useTheme } from "./hooks/useTheme";
import { useWorkspace } from "./hooks/useWorkspace";
import { useKeepAwake } from "./hooks/useKeepAwake";
import { TerminalTabs, type TerminalTabsHandle } from "./components/TerminalTabs";
import { GitStatusPanel } from "./components/GitStatusPanel";
import { AgentLauncher, type AgentDefinition } from "./components/AgentLauncher";
import { SettingsPanel } from "./components/SettingsPanel";
import "./App.css";

function App() {
  const { theme, toggleTheme } = useTheme();
  const { path, recents, loaded, openFolder, selectPath } = useWorkspace();
  const { enabled: keepAwake, toggle: toggleKeepAwake } = useKeepAwake();
  const terminalTabsRef = useRef<TerminalTabsHandle>(null);
  const [showSettings, setShowSettings] = useState(false);

  const launchAgent = (agent: AgentDefinition) => {
    terminalTabsRef.current?.openTab(agent.name, agent.command);
  };

  return (
    <div className="app-shell">
      <header className="app-titlebar">
        <div className="app-brand">
          <span className="app-brand-mark">M</span>
          <span className="app-brand-name">Mihani</span>
        </div>
        <div className="app-titlebar-actions">
          <button
            className={`keepawake-toggle ${keepAwake ? "keepawake-toggle-active" : ""}`}
            onClick={toggleKeepAwake}
            title={keepAwake ? "Keep awake: on" : "Keep awake: off"}
          >
            {keepAwake ? "◉ Awake" : "○ Awake"}
          </button>
          <button className="theme-toggle" onClick={() => setShowSettings(true)} title="Settings">
            ⚙
          </button>
          <button className="theme-toggle" onClick={toggleTheme} title="Toggle theme">
            {theme === "dark" ? "☾" : "☀"}
          </button>
        </div>
      </header>
      <div className="app-body">
        <aside className="app-sidebar">
          <div className="workspace-picker">
            <button className="workspace-open-btn" onClick={openFolder}>
              {path ? "Change Folder" : "Open Folder"}
            </button>
            {path && <div className="workspace-path" title={path}>{path}</div>}
            {!path && recents.length > 0 && (
              <ul className="workspace-recents">
                {recents.map((recent) => (
                  <li key={recent} onClick={() => selectPath(recent)} title={recent}>
                    {recent}
                  </li>
                ))}
              </ul>
            )}
          </div>
          <AgentLauncher onLaunch={launchAgent} onManage={() => setShowSettings(true)} />
          {path && (
            <>
              <div className="sidebar-section-title">Source Control</div>
              <GitStatusPanel path={path} />
            </>
          )}
        </aside>
        <main className="app-main">
          {loaded && <TerminalTabs ref={terminalTabsRef} cwd={path ?? undefined} theme={theme} />}
        </main>
      </div>
      {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
    </div>
  );
}

export default App;
