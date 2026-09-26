# Mihani

Mihani is a desktop console for running and coordinating multiple AI coding agents
against real projects. It combines an integrated terminal, live Git status, and
agent orchestration into a single native app for Windows and macOS.

## Stack

- **Shell / UI:** Tauri 2, React, TypeScript, Vite
- **Backend:** Rust
- **Terminal:** `portable-pty` on the backend, `xterm.js` on the frontend
- **Git integration:** `git2` (libgit2 bindings)

## Development

Prerequisites: Node.js 20+, Rust (stable), and the platform prerequisites for
Tauri (see https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev
```

## Project layout

```
src/            React + TypeScript frontend
  components/   UI components (terminal pane, git status panel, ...)
  hooks/        Shared React hooks (theme, ...)
src-tauri/      Rust backend
  src/terminal.rs   PTY-backed terminal sessions, exposed as Tauri commands
  src/git.rs        Git status/branch info via libgit2
```

## Status

This repository is under active development. See the project roadmap for the
planned sequence of pull requests.
