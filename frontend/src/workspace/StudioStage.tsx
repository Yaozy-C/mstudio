import { t, useLanguage } from "../i18n";
import { TaskPanel } from "../production/TaskPanel";
import type { ComponentProps } from "react";
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
  settings,
  onAdd,
  ...props
}: Omit<ComponentProps<typeof ScriptWorkspace>, "navigate"> & {
  canvas: ProductionController;
  onAdd: ComponentProps<typeof ProductionCanvas>["onAdd"];
  settings: () => void;
  view: StudioView;
  clock: PlaybackClock;
  navigate: (view: StudioView) => void;
}) {
  useLanguage();
  return (
    <main
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
      <TaskPanel project={props.project} canvas={canvas} settings={settings} />
    </main>
  );
}
