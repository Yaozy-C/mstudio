import { t, useLanguage } from "../i18n";
import { useRef, type ComponentProps } from "react";
import { useContentMotion } from "../ui/useContentMotion";
import { ScriptWorkspace } from "../creative/ScriptWorkspace";
import { ProductionCanvas } from "../production/ProductionCanvas";
import type { ProductionController } from "../production/useProduction";
import { Preview } from "../timeline/Preview";
import type { PlaybackClock } from "../timeline/clock";

export type StudioView = "script" | "storyboard" | "film";
export function StudioStage({
  view,
  clock,
  navigate,
  canvas,
  onAdd,
  ...props
}: Omit<ComponentProps<typeof ScriptWorkspace>, "navigate"> & {
  canvas: ProductionController;
  onAdd: ComponentProps<typeof ProductionCanvas>["onAdd"];
  view: StudioView;
  clock: PlaybackClock;
  navigate: (view: StudioView) => void;
}) {
  useLanguage();
  const stage = useRef<HTMLElement>(null);
  // Native video and canvas coordinates must not pass through a filtered layer.
  useContentMotion(stage, view, view === "script");
  return (
    <main
      ref={stage}
      className={`studio-stage view-${view}`}
      aria-label={
        view === "script"
          ? t("脚本工作区")
          : view === "storyboard"
            ? t("分镜工作区")
            : t("成片工作区")
      }
    >
      {view === "script" ? (
        <ScriptWorkspace {...props} navigate={navigate} />
      ) : view === "storyboard" ? (
        <ProductionCanvas
          onAdd={onAdd}
          project={props.project}
          change={props.onChange}
          canvas={canvas}
        />
      ) : (
        <Preview project={props.project} clock={clock} />
      )}
    </main>
  );
}
