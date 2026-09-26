import { useCallback, useEffect, useState } from "react";
import { load, type Store } from "@tauri-apps/plugin-store";
import type { AgentDefinition } from "../components/AgentLauncher";

const STORE_FILE = "agents.json";
const CUSTOM_KEY = "customAgents";
const TRUSTED_KEY = "trustedAgentIds";

let storePromise: Promise<Store> | null = null;
function getStore() {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true });
  }
  return storePromise;
}

export function useCustomAgents() {
  const [customAgents, setCustomAgents] = useState<AgentDefinition[]>([]);
  const [trustedIds, setTrustedIds] = useState<Set<string>>(new Set());

  useEffect(() => {
    (async () => {
      const store = await getStore();
      const agents = (await store.get<AgentDefinition[]>(CUSTOM_KEY)) ?? [];
      const trusted = (await store.get<string[]>(TRUSTED_KEY)) ?? [];
      setCustomAgents(agents);
      setTrustedIds(new Set(trusted));
    })();
  }, []);

  const addAgent = useCallback(async (agent: Omit<AgentDefinition, "id">) => {
    const withId: AgentDefinition = { ...agent, id: `custom-${crypto.randomUUID()}` };
    const store = await getStore();
    setCustomAgents((prev) => {
      const next = [...prev, withId];
      store.set(CUSTOM_KEY, next);
      return next;
    });
  }, []);

  const removeAgent = useCallback(async (id: string) => {
    const store = await getStore();
    setCustomAgents((prev) => {
      const next = prev.filter((a) => a.id !== id);
      store.set(CUSTOM_KEY, next);
      return next;
    });
  }, []);

  const trustAgent = useCallback(async (id: string) => {
    const store = await getStore();
    setTrustedIds((prev) => {
      const next = new Set(prev).add(id);
      store.set(TRUSTED_KEY, Array.from(next));
      return next;
    });
  }, []);

  return { customAgents, trustedIds, addAgent, removeAgent, trustAgent };
}
