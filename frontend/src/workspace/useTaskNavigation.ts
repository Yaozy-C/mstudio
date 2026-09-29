import { useEffect } from "react";
import type { StudioView } from "./StudioStage";

export function useTaskNavigation(
  view: StudioView,
  navigate: (view: StudioView, nodeId?: string) => void,
) {
  useEffect(() => {
    const script = (event: Event) =>
      navigate("script", (event as CustomEvent<string>).detail);
    const canvas = () => navigate("storyboard");
    window.addEventListener("studio-show-script", script);
    window.addEventListener("studio-show-canvas", canvas);
    return () => {
      window.removeEventListener("studio-show-script", script);
      window.removeEventListener("studio-show-canvas", canvas);
    };
  }, [navigate, view]);
}
