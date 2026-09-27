import { useEffect, useRef } from "react";
import type { Clip } from "../model";
import type { PlaybackClock } from "../timeline/clock";
import {
  editKey,
  pasteClip,
  trimAtPlayhead,
  type ClipEdit,
} from "../timeline/clipEdits";
import type { useProject } from "./useProject";
export function useClipEditing(
  m: ReturnType<typeof useProject>,
  selected: string | null,
  clock: PlaybackClock,
  enabled: boolean,
  select: (id: string | null) => void,
) {
  const clipboard = useRef<{ projectId: string; clip: Clip } | null>(null);
  useEffect(() => {
    function edit(command: ClipEdit, id = selected): boolean {
      const p = m.get();
      const clip = p.clips.find((c) => c.id === id);
      if (command === "paste") {
        const saved = clipboard.current;
        if (!saved || saved.projectId !== p.id) return false;
        const result = pasteClip(p, saved.clip, clock.getSnapshot().time);
        if (!result.id) return false;
        clock.pause();
        m.change(() => result.project);
        select(result.id);
        return true;
      }
      if (!clip) return false;
      if (command === "copy" || command === "cut") {
        clipboard.current = { projectId: p.id, clip: structuredClone(clip) };
        if (command === "cut") {
          clock.pause();
          m.change((p) => ({
            ...p,
            clips: p.clips.filter((c) => c.id !== id),
          }));
          select(null);
        }
      } else {
        clock.pause();
        m.change((p) =>
          trimAtPlayhead(p, clip.id, clock.getSnapshot().time, command),
        );
      }
      return true;
    }
    const key = (e: KeyboardEvent) => {
      if (
        !enabled ||
        e.defaultPrevented ||
        e.isComposing ||
        document.querySelector(
          '.modal-backdrop, .settings-page, [role="dialog"], [role="alertdialog"], [role="menu"]',
        )
      )
        return;
      if (
        e.target instanceof HTMLElement &&
        e.target.closest(
          'input, textarea, select, [contenteditable="true"], [role="textbox"]',
        )
      )
        return;
      // macOS Option+[ / ] may produce a Unicode character; use the physical bracket key.
      const command = editKey({
        ...e,
        key:
          e.altKey && e.code === "BracketLeft"
            ? "["
            : e.altKey && e.code === "BracketRight"
              ? "]"
              : e.key,
        metaKey: e.metaKey,
        ctrlKey: e.ctrlKey,
        altKey: e.altKey,
        shiftKey: e.shiftKey,
      });
      if (command && !e.repeat && edit(command)) e.preventDefault();
    };
    const request = (e: Event) => {
      const { command, id } = (
        e as CustomEvent<{ command: ClipEdit; id: string }>
      ).detail;
      if (enabled) edit(command, id);
    };
    window.addEventListener("keydown", key);
    window.addEventListener("studio-clip-edit", request);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("studio-clip-edit", request);
    };
  });
}
