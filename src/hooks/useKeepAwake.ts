import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useKeepAwake() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    invoke<boolean>("keepawake_status").then(setEnabled).catch(() => {});
  }, []);

  const setKeepAwake = useCallback(async (next: boolean, reason = "Mihani agent session") => {
    if (next) {
      await invoke("keepawake_enable", { reason });
    } else {
      await invoke("keepawake_disable");
    }
    setEnabled(next);
  }, []);

  const toggle = useCallback(() => setKeepAwake(!enabled), [enabled, setKeepAwake]);

  return { enabled, setKeepAwake, toggle };
}
