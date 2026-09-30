import { useEffect, useState } from "react";
const eventName = "mstudio:transition-dock";
export function openTransitionDock(leftId: string, rightId: string) {
  window.dispatchEvent(
    new CustomEvent(eventName, {
      detail: `transition:${JSON.stringify([leftId, rightId])}`,
    }),
  );
}
export function useCreationTab() {
  const [tab, setTab] = useState<string | null>(null);
  useEffect(() => {
    const open = (event: Event) =>
      setTab((event as CustomEvent<string>).detail);
    window.addEventListener(eventName, open);
    return () => window.removeEventListener(eventName, open);
  }, []);
  return [tab, setTab] as const;
}
