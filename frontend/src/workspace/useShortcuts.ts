import { useEffect } from "react";
import type { PlaybackClock } from "../timeline/clock";
export function useShortcuts(
  m: { undo: () => void; redo: () => void; flush: () => Promise<void> },
  setError: (s: string) => void,
  actions: {
    film: boolean;
    clock: PlaybackClock;
    split: () => void;
    toggleTimeline: () => void;
    play: () => void;
    remove: () => void;
  },
) {
  useEffect(() => {
    const capturePlayback = (e: KeyboardEvent) => {
      if (
        !actions.film ||
        e.code !== "Space" ||
        e.isComposing ||
        e.metaKey ||
        e.ctrlKey ||
        e.altKey ||
        document.querySelector(
          '.modal-backdrop, .settings-page, [role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]',
        )
      )
        return;
      const target = e.target instanceof HTMLElement ? e.target : null;
      if (
        target?.closest('textarea, [contenteditable="true"], [role="textbox"]')
      )
        return;
      const input = target?.closest("input");
      if (
        input &&
        !["range", "number", "checkbox", "radio", "button", "submit"].includes(
          input.type,
        )
      )
        return;
      e.preventDefault();
      e.stopPropagation();
      if (!e.repeat) actions.play();
    };
    const handler = (e: KeyboardEvent) => {
      if (
        e.isComposing ||
        e.defaultPrevented ||
        document.querySelector(".modal-backdrop, .settings-page") ||
        document.querySelector(
          '.studio-menu[data-state="open"], [role="dialog"], [role="alertdialog"]',
        ) ||
        (e.target instanceof HTMLElement &&
          (["INPUT", "TEXTAREA", "SELECT"].includes(e.target.tagName) ||
            e.target.isContentEditable))
      )
        return;
      const cmd = e.metaKey || e.ctrlKey;
      const k = e.key.toLowerCase();
      let run: (() => void) | undefined;
      if (cmd && k === "z") run = e.shiftKey ? m.redo : m.undo;
      else if (cmd && k === "s")
        run = () => void m.flush().catch((e) => setError(String(e)));
      else if (cmd && k === "b") run = actions.split;
      else if (cmd && ["+", "=", "-"].includes(k))
        run = () =>
          window.dispatchEvent(
            new CustomEvent("timeline-zoom", {
              detail: k === "-" ? 1 / 1.5 : 1.5,
            }),
          );
      else if (!cmd && !e.altKey) {
        if (k === " ") run = actions.play;
        if (k === "arrowleft" || k === "arrowright")
          run = () =>
            actions.clock.step(
              (k === "arrowleft" ? -1 : 1) * (e.shiftKey ? 10 : 1),
            );
        if (k === "home" || k === "end")
          run = () => {
            actions.clock.pause();
            actions.clock.seek(k === "home" ? 0 : actions.clock.total);
          };
        if (k === "t") run = actions.toggleTimeline;
        if (k === "delete" || k === "backspace") run = actions.remove;
      }
      if (run) {
        e.preventDefault();
        if (!e.repeat || k.startsWith("arrow")) run();
      }
    };
    const hide = () => {
      if (document.hidden) actions.clock.pause();
    };
    window.addEventListener("keydown", capturePlayback, true);
    window.addEventListener("keydown", handler);
    document.addEventListener("visibilitychange", hide);
    return () => {
      window.removeEventListener("keydown", capturePlayback, true);
      window.removeEventListener("keydown", handler);
      document.removeEventListener("visibilitychange", hide);
    };
  });
}
