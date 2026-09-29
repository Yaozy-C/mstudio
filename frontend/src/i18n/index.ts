import { useSyncExternalStore } from "react";
import english from "./en";

export type Language = "zh-CN" | "en";
export const LANGUAGE_KEY = "mstudio-language";
const messages: Record<string, string> = english;
const listeners = new Set<() => void>();

export function resolveLanguage(
  saved: string | null,
  browser = "zh-CN",
): Language {
  if (saved === "zh-CN" || saved === "en") return saved;
  return browser.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}
function initialLanguage(): Language {
  if (typeof window === "undefined") return "zh-CN";
  try {
    return resolveLanguage(
      localStorage.getItem(LANGUAGE_KEY),
      navigator.language,
    );
  } catch {
    return resolveLanguage(
      null,
      typeof navigator === "undefined" ? "zh-CN" : navigator.language,
    );
  }
}
let language = initialLanguage();
export function getLanguage(): Language {
  return language;
}
function syncDocument() {
  if (typeof document !== "undefined") document.documentElement.lang = language;
}
syncDocument();
export function setLanguage(next: Language) {
  if (next !== "zh-CN" && next !== "en") return;
  try {
    localStorage.setItem(LANGUAGE_KEY, next);
  } catch {
    /* Still works for this session. */
  }
  language = next;
  syncDocument();
  listeners.forEach((notify) => notify());
}
if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === LANGUAGE_KEY || event.key === null) {
      language = resolveLanguage(event.newValue, navigator.language);
      syncDocument();
      listeners.forEach((notify) => notify());
    }
  });
}
function subscribe(notify: () => void) {
  listeners.add(notify);
  return () => {
    listeners.delete(notify);
  };
}
/** Subscribe without remounting: drafts, playback and open dialogs stay intact. */
export function useLanguage() {
  return useSyncExternalStore(subscribe, getLanguage, getLanguage);
}
/** Only pass application-owned copy here, never user content or model output. */
export function t(
  source: string,
  values?: Record<string, string | number | undefined>,
): string {
  const translated =
    language === "en" && Object.hasOwn(messages, source)
      ? messages[source]
      : source;
  return translated.replace(/\{(\w+)\}/g, (match, key: string) =>
    values && Object.hasOwn(values, key) ? String(values[key] ?? "") : match,
  );
}
