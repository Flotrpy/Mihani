import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { load, type Store } from "@tauri-apps/plugin-store";

const STORE_FILE = "workspace.json";
const RECENT_KEY = "recentPaths";
const ACTIVE_KEY = "activePath";
const MAX_RECENTS = 8;

let storePromise: Promise<Store> | null = null;
function getStore() {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true });
  }
  return storePromise;
}

export function useWorkspace() {
  const [path, setPath] = useState<string | null>(null);
  const [recents, setRecents] = useState<string[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    (async () => {
      const store = await getStore();
      const active = await store.get<string>(ACTIVE_KEY);
      const recent = (await store.get<string[]>(RECENT_KEY)) ?? [];
      setPath(active ?? null);
      setRecents(recent);
      setLoaded(true);
    })();
  }, []);

  const selectPath = useCallback(async (newPath: string) => {
    setPath(newPath);
    const store = await getStore();
    await store.set(ACTIVE_KEY, newPath);
    setRecents((prev) => {
      const next = [newPath, ...prev.filter((p) => p !== newPath)].slice(0, MAX_RECENTS);
      store.set(RECENT_KEY, next);
      return next;
    });
  }, []);

  const openFolder = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await selectPath(selected);
    }
  }, [selectPath]);

  const removeRecent = useCallback(async (target: string) => {
    const store = await getStore();
    setRecents((prev) => {
      const next = prev.filter((p) => p !== target);
      store.set(RECENT_KEY, next);
      return next;
    });
  }, []);

  return { path, recents, loaded, openFolder, selectPath, removeRecent };
}
