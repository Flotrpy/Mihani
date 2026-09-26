import { useState } from "react";
import { useTheme } from "./hooks/useTheme";
import { TerminalPane } from "./components/TerminalPane";
import { GitStatusPanel } from "./components/GitStatusPanel";
import "./App.css";

function App() {
  const { theme, toggleTheme } = useTheme();
  const [workspacePath] = useState<string>(".");

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
          <div className="sidebar-section-title">Source Control</div>
          <GitStatusPanel path={workspacePath} />
        </aside>
        <main className="app-main">
          <TerminalPane cwd={workspacePath} theme={theme} />
        </main>
      </div>
    </div>
  );
}

export default App;
