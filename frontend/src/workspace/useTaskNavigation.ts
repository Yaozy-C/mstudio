import { useEffect } from "react";
import type { StudioView } from "./StudioStage";

export function useTaskNavigation(
  view: StudioView,
  navigate: (view: StudioView, nodeId?: string) => void,
) {
  useEffect(() => {
    const script = (event: Event) =>
      navigate("script", (event as CustomEvent<string>).detail);
    const task = () => {
      if (view === "film") navigate("storyboard");
    };
    window.addEventListener("studio-show-script", script);
    window.addEventListener("studio-show-generation", task);
    return () => {
      window.removeEventListener("studio-show-script", script);
      window.removeEventListener("studio-show-generation", task);
    };
  }, [navigate, view]);
}
