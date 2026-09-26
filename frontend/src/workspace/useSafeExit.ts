import { useEffect, useState, useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { createExitGuard } from "./exitGuard";
let saveBeforeExit: () => Promise<unknown> = async () => {};
export const flushBeforeStorageChange = () => saveBeforeExit();
export function registerExitSave(save: () => Promise<unknown>) {
  saveBeforeExit = save;
  return () => {
    saveBeforeExit = async () => {};
  };
}
export function useSafeExit() {
  const [guard] = useState(() =>
    createExitGuard(
      () => saveBeforeExit(),
      () => bridge("finish_exit"),
    ),
  );
  const state = useSyncExternalStore(guard.subscribe, guard.getSnapshot);
  useEffect(() => {
    if (!native) return;
    const off = listen("before-exit", () => {
      void guard.request();
    });
    return () => {
      void off.then((f) => f());
    };
  }, [guard]);
  return { ...state, retry: guard.request, dismiss: guard.dismiss };
}
