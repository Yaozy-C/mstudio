import { useEffect } from "react";
import type { StudioView } from "./StudioStage";

export function useTaskNavigation(
  view: StudioView,
  navigate: (view: StudioView, nodeId?: string) => void,
) {
  useEffect(() => {
    const script = (event: Event) =>
      navigate("script", (event as CustomEvent<string>).detail);
    const canvas = () => navigate("drafts");
    const shot = (event: Event) =>
      navigate("storyboard", (event as CustomEvent<string>).detail);
    const assets = () => navigate("assets");
    window.addEventListener("studio-show-script", script);
    window.addEventListener("studio-show-canvas", canvas);
    window.addEventListener("studio-show-assets", assets);
    window.addEventListener("studio-show-shot", shot);
    return () => {
      window.removeEventListener("studio-show-script", script);
      window.removeEventListener("studio-show-canvas", canvas);
      window.removeEventListener("studio-show-assets", assets);
      window.removeEventListener("studio-show-shot", shot);
    };
  }, [navigate, view]);
}
