import { useEffect, useSyncExternalStore } from "react";
import type { PlaybackClock } from "../timeline/clock";
/** Transient selection shared by the timeline and inspector, scoped to one workspace. */
export class CaptionSelection {
  private id: string | null = null;
  private listeners = new Set<() => void>();
  get = () => this.id;
  set = (id: string | null) => {
    if (id === this.id) return;
    this.id = id;
    this.listeners.forEach((fn) => fn());
  };
  subscribe = (fn: () => void) => {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  };
}
const selections = new WeakMap<PlaybackClock, CaptionSelection>();
export function useCaptionSelection(clock: PlaybackClock) {
  let selection = selections.get(clock);
  if (!selection) {
    selection = new CaptionSelection();
    selections.set(clock, selection);
  }
  const id = useSyncExternalStore(selection.subscribe, selection.get);
  return [id, selection.set] as const;
}

export function useTimelineCaptionSelection(
  clock: PlaybackClock,
  selected: string | null,
  selectClip: (id: string | null) => void,
) {
  const [id, setId] = useCaptionSelection(clock);
  useEffect(() => {
    if (selected) setId(null);
  }, [selected]);
  useEffect(() => {
    if (id) selectClip(null);
  }, [id]);
  return [id, setId] as const;
}
