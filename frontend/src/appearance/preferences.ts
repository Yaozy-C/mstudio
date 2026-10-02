import { useSyncExternalStore } from "react";
export type Appearance = {
  surface: "paper" | "white" | "slate";
  contrast: "standard" | "high";
};
export const APPEARANCE_KEY = "mstudio-appearance";
export const defaultAppearance: Appearance = {
  surface: "paper",
  contrast: "standard",
};
export function parseAppearance(raw: string | null): Appearance {
  try {
    const value = JSON.parse(raw || "null");
    return {
      surface:
        value?.surface === "white" || value?.surface === "slate"
          ? value.surface
          : "paper",
      contrast: value?.contrast === "high" ? "high" : "standard",
    };
  } catch {
    return { ...defaultAppearance };
  }
}
function read(): Appearance {
  try {
    return parseAppearance(localStorage.getItem(APPEARANCE_KEY));
  } catch {
    return { ...defaultAppearance };
  }
}
let current = read();
const listeners = new Set<() => void>();
function apply() {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.surface = current.surface;
    document.documentElement.dataset.contrast = current.contrast;
  }
  listeners.forEach((notify) => notify());
}
apply();
export function setAppearance(next: Appearance): boolean {
  current = next;
  let persisted = true;
  try {
    localStorage.setItem(APPEARANCE_KEY, JSON.stringify(next));
  } catch {
    persisted = false;
  }
  apply();
  return persisted;
}
if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === APPEARANCE_KEY || event.key === null) {
      current = read();
      apply();
    }
  });
}
function subscribe(notify: () => void) {
  listeners.add(notify);
  return () => {
    listeners.delete(notify);
  };
}
export function useAppearance() {
  return useSyncExternalStore(
    subscribe,
    () => current,
    () => defaultAppearance,
  );
}
