import { useCallback, useEffect, useState } from "react";

export type ThemeMode = "light" | "dark";

const STORAGE_KEY = "mihani.theme";

// Mihani's brand default is the white/purple light theme regardless of the
// OS-level color scheme preference — dark mode is opt-in via the toggle,
// not something the app silently switches to on a dark-mode system.
const DEFAULT_THEME: ThemeMode = "light";

function readStored(): ThemeMode | null {
  const value = localStorage.getItem(STORAGE_KEY);
  return value === "light" || value === "dark" ? value : null;
}

export function useTheme() {
  const [theme, setTheme] = useState<ThemeMode>(() => readStored() ?? DEFAULT_THEME);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme((prev) => (prev === "light" ? "dark" : "light"));
  }, []);

  return { theme, setTheme, toggleTheme };
}
