import { t, useLanguage } from "../i18n";
import { useRef, type ComponentProps } from "react";
import { useContentMotion } from "../ui/useContentMotion";
import { ScriptWorkspace } from "../creative/ScriptWorkspace";
import { DraftCanvas } from "../production/DraftCanvas";
import type { ProductionController } from "../production/useProduction";
import { Preview } from "../timeline/Preview";
import type { PlaybackClock } from "../timeline/clock";
import { ConceptBoard } from "./ConceptBoard";
import { requestCreativeTask } from "../creative/aiTasks";
import { conceptScriptTask } from "../creative/conceptTasks";
import { StoryAssets } from "./StoryAssets";
import type { Asset } from "../model";
import { MaterialsWorkspace } from "./MaterialsWorkspace";
import { ProductionWorkspace } from "../production/ProductionWorkspace";

export type StudioView =
  | "drafts"
  | "concepts"
  | "script"
  | "materials"
  | "assets"
  | "storyboard"
  | "film";
export function StudioStage({
  view,
  clock,
  navigate,
  canvas,
  onAdd,
  onPlace,
  onAssetReference,
  ...props
}: Omit<ComponentProps<typeof ScriptWorkspace>, "navigate"> & {
  canvas: ProductionController;
  onAdd: (asset: Asset) => void;
  onPlace: (asset: Asset) => void;
  onAssetReference: (asset: Asset) => void;
  view: StudioView;
  clock: PlaybackClock;
  navigate: (view: StudioView) => void;
}) {
  useLanguage();
  const stage = useRef<HTMLElement>(null);
  // Native video and canvas coordinates must not pass through a filtered layer.
  useContentMotion(stage, view, view === "script" || view === "concepts");
  return (
    <main
      ref={stage}
      className={`studio-stage view-${view}`}
      aria-label={
        view === "drafts"
          ? t("草稿画布")
          : view === "concepts"
            ? t("创意板")
            : view === "assets"
              ? t("资产工作区")
              : view === "materials"
                ? t("素材资料")
                : view === "script"
                  ? t("脚本工作区")
                  : view === "storyboard"
                    ? t("分镜工作区")
                    : t("成片工作区")
      }
    >
      {view === "drafts" ? (
        <DraftCanvas
          project={props.project}
          change={props.onChange}
          canvas={canvas}
        />
      ) : view === "concepts" ? (
        <ConceptBoard
          project={props.project}
          change={props.onChange}
          write={(id) => {
            navigate("script");
            requestCreativeTask(
              conceptScriptTask(
                id,
                props.project.nodes.find((n) => n.kind === "screenplay")?.id,
              ),
            );
          }}
        />
      ) : view === "materials" ? (
        <MaterialsWorkspace
          project={props.project}
          change={props.onChange}
          reference={onAssetReference}
          place={onPlace}
        />
      ) : view === "assets" ? (
        <StoryAssets
          project={props.project}
          change={props.onChange}
          reference={onAssetReference}
          place={onPlace}
        />
      ) : view === "script" ? (
        <ScriptWorkspace {...props} navigate={navigate} />
      ) : view === "storyboard" ? (
        <ProductionWorkspace
          onAdd={onAdd}
          project={props.project}
          change={props.onChange}
          canvas={canvas}
          selected={props.selected}
          select={props.onSelect}
          navigate={navigate}
        />
      ) : (
        <Preview project={props.project} clock={clock} />
      )}
    </main>
  );
}
