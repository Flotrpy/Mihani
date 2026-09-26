import { useCallback, useEffect, useRef } from "react";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export function useNotify() {
  const grantedRef = useRef(false);

  useEffect(() => {
    (async () => {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      grantedRef.current = granted;
    })();
  }, []);

  return useCallback((title: string, body: string) => {
    if (grantedRef.current) {
      sendNotification({ title, body });
    }
  }, []);
}
