import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useTheme } from "./hooks/useTheme";
import { useWorkspace } from "./hooks/useWorkspace";
import { useKeepAwake } from "./hooks/useKeepAwake";
import { useTestCommand } from "./hooks/useTestCommand";
import { TerminalTabs, type TerminalTabsHandle } from "./components/TerminalTabs";
import { GitStatusPanel } from "./components/GitStatusPanel";
import { AgentLauncher, type AgentDefinition } from "./components/AgentLauncher";
import { SettingsPanel } from "./components/SettingsPanel";
import { RemoteCommitRequest } from "./components/RemoteCommitRequest";
import "./App.css";

interface SessionStartedEvent {
  id: string;
  title: string;
}

interface CommitRequestEvent {
  path: string;
}

function App() {
  const { theme, toggleTheme } = useTheme();
  const { path, recents, loaded, openFolder, selectPath } = useWorkspace();
  const { enabled: keepAwake, toggle: toggleKeepAwake } = useKeepAwake();
  const { testCommand } = useTestCommand();
  const terminalTabsRef = useRef<TerminalTabsHandle>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [commitRequest, setCommitRequest] = useState<string | null>(null);

  const launchAgent = (agent: AgentDefinition) => {
    terminalTabsRef.current?.openTab(agent.name, agent.command);
  };

  // Keep the remote server's fixed-action context in sync with the local
  // workspace/test command — the remote client never supplies these itself.
  useEffect(() => {
    invoke("remote_set_context", {
      workspacePath: path ?? null,
      testCommand: testCommand ?? null,
    }).catch(() => {});
  }, [path, testCommand]);

  useEffect(() => {
    const unlistenPromise = listen<SessionStartedEvent>("remote://session-started", (event) => {
      terminalTabsRef.current?.attachTab(event.payload.id, event.payload.title);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const unlistenPromise = listen<CommitRequestEvent>("remote://commit-request", (event) => {
      setCommitRequest(event.payload.path);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

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
            aria-label={keepAwake ? "Keep awake: on" : "Keep awake: off"}
            aria-pressed={keepAwake}
          >
            {keepAwake ? "◉ Awake" : "○ Awake"}
          </button>
          <button
            className="theme-toggle"
            onClick={() => setShowSettings(true)}
            title="Settings"
            aria-label="Open settings"
          >
            ⚙
          </button>
          <button
            className="theme-toggle"
            onClick={toggleTheme}
            title="Toggle theme"
            aria-label={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
          >
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
            {path && (
              <div className="workspace-path-row">
                <div className="workspace-path" title={path}>{path}</div>
                <button
                  className="workspace-reveal-btn"
                  title="Reveal in file manager"
                  aria-label="Reveal workspace folder in file manager"
                  onClick={() => revealItemInDir(path)}
                >
                  ⤢
                </button>
              </div>
            )}
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
      {commitRequest && (
        <RemoteCommitRequest path={commitRequest} onClose={() => setCommitRequest(null)} />
      )}
    </div>
  );
}

export default App;
