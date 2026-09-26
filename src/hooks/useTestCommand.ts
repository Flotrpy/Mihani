import { useCallback, useEffect, useState } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";

const STORE_FILE = "remote.json";
const KEY = "testCommand";

let storePromise: Promise<Store> | null = null;
function getStore() {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true });
  }
  return storePromise;
}

export function useTestCommand() {
  const [testCommand, setTestCommandState] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const store = await getStore();
      const value = await store.get<string>(KEY);
      setTestCommandState(value ?? null);
    })();
  }, []);

  const setTestCommand = useCallback(async (value: string) => {
    const store = await getStore();
    await store.set(KEY, value);
    setTestCommandState(value);
  }, []);

  return { testCommand, setTestCommand };
}
