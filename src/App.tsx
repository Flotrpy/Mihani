import { useTheme } from "./hooks/useTheme";
import { useWorkspace } from "./hooks/useWorkspace";
import { TerminalTabs } from "./components/TerminalTabs";
import { GitStatusPanel } from "./components/GitStatusPanel";
import "./App.css";

function App() {
  const { theme, toggleTheme } = useTheme();
  const { path, recents, loaded, openFolder, selectPath } = useWorkspace();

  return (
    <div className="app-shell">
      <header className="app-titlebar">
        <div className="app-brand">
          <span className="app-brand-mark">M</span>
          <span className="app-brand-name">Mihani</span>
        </div>
        <button className="theme-toggle" onClick={toggleTheme} title="Toggle theme">
          {theme === "dark" ? "☾" : "☀"}
        </button>
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
          {path && (
            <>
              <div className="sidebar-section-title">Source Control</div>
              <GitStatusPanel path={path} />
            </>
          )}
        </aside>
        <main className="app-main">
          {loaded && <TerminalTabs cwd={path ?? undefined} theme={theme} />}
        </main>
      </div>
    </div>
  );
}

export default App;
