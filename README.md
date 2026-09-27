# Mihani

Mihani is a desktop console for running and coordinating multiple AI coding
agents against real projects. It combines an integrated terminal, Git
workflow, agent orchestration, and optional remote control into a single
native app for **Windows and macOS**.

Mihani is not code-signed or notarized yet. Windows SmartScreen will show an
"unknown publisher" warning on first run ("More info" → "Run anyway"), and
an unnotarized macOS build requires right-click → Open (or clearing the
quarantine flag) the first time. Both are expected for this release and not
a sign of a problem — see [Distribution](#distribution) below.

## Features

- **Multi-tab terminal** — real PTY sessions (`portable-pty`), tab rename,
  Ctrl/Cmd+F search, copy-on-select, and per-tab exit indicators.
- **Hidden background terminals** — tuck a running session into the
  background (it keeps running) and bring it back later; combined with a
  tray icon and hide-to-tray window close, a session survives even with the
  window shut.
- **Agents** — launch built-in adapters (Claude Code, Codex CLI, Gemini CLI,
  Aider) or your own custom agent command, with install detection, a
  trust-before-first-run gate for custom commands, and optional
  user-defined auth checks.
- **Git** — status, per-file diff, selective staging, commit, discard,
  branch switch/create, push/pull, and commit history, all via `git2`.
- **GitHub** — personal access token stored in the OS keychain (never on
  disk), and one-click "push + create pull request" attributed to your own
  account.
- **Remote control** — optionally view a session's output and trigger a
  fixed set of actions (start a built-in agent, cancel, restart, stop, run
  your configured test command, view git status/diff, request a commit +
  push) from another device. The remote client can never send freeform text
  or shell commands into a session — the action set is a closed enum with
  no such variant — and anything effectful (commit/push) still requires a
  local approval. Off by default; binds to this device only unless you
  explicitly allow LAN access; every route needs a regenerable token.
- **Keep-awake**, **desktop notifications** for background completions,
  **window-state persistence**, a **native app menu** and **system tray**,
  and light/dark purple theming.

## Stack

- **Shell / UI:** Tauri 2, React, TypeScript, Vite
- **Backend:** Rust
- **Terminal:** `portable-pty` on the backend, `xterm.js` on the frontend
- **Git integration:** `git2` (libgit2 bindings)
- **Remote control:** `axum` + `tokio`, served from an in-process HTTP/WS
  server (see `src-tauri/src/remote.rs`)
- **Secrets:** OS keychain via the `keyring` crate (never plaintext config)

## Development

Prerequisites: Node.js 20+, Rust (stable), and the platform prerequisites for
Tauri (see https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev
```

Run the Rust test suite:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

CI builds real installers on every push: a Windows NSIS `.exe`/`.msi` (via
`windows-installer`) and a macOS `.dmg`/`.app` (via `macos-installer`), both
uploaded as workflow artifacts, so a working installer for each platform is
always available without needing local Windows/macOS hardware.

## Distribution

Mihani's initial release ships through **GitHub Releases**, not an app
store: each release includes a Windows installer and a macOS disk image,
built by CI. Signing (Windows Authenticode, Apple Developer ID + notarization)
is planned but not a blocker for this release — the build is already
structured so it can be added later without changing the packaging targets.

## Project layout

```
src/                        React + TypeScript frontend
  components/                UI components (terminal, git panel, settings, ...)
  hooks/                      Shared React hooks (theme, workspace, agents, ...)
src-tauri/                  Rust backend
  src/terminal.rs             PTY-backed terminal sessions
  src/git.rs                  Git operations via libgit2
  src/github.rs                Keychain-backed GitHub auth + PR creation
  src/agents.rs                Agent registry, install/auth checks
  src/remote.rs                Remote control server (fixed actions only)
  src/keepawake.rs             Cross-platform keep-awake
  src/app_menu.rs / tray.rs    Native menu and tray icon
```

## Security notes

- GitHub tokens live in the OS keychain, not app config.
- Remote control never accepts freeform input; every action is a named,
  fixed operation, and destructive ones (commit/push) still require local
  human approval in the app itself.
- Custom agents (arbitrary shell commands the user defines) require an
  explicit "Trust & Run" confirmation before their first launch, and are
  never exposed to the remote-control action set — only built-in agents can
  be started remotely.

## Status

This repository is under active development.
