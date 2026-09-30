import { requestCreativeTask } from "../creative/aiTasks";
import { effectAgentTask } from "./effectAgentTask";
import { TaskPanel } from "../production/TaskPanel";
import { useTaskNavigation } from "./useTaskNavigation";
import { initialPanels, toggleStudioPanel } from "./studioPanels";
import { useSidebarWidths } from "../ui/useSidebarWidths";
import { GenerationTaskSettings } from "../production/GenerationTaskSettings";
import type { WorkContext } from "../assistant/workContext";
import { useProduction } from "../production/useProduction";
import { useGeneratedJobs } from "./useGeneratedJobs";
import { useStudioCreativeEvents } from "./useStudioCreativeEvents";
import { useStudioPlayback } from "./useStudioPlayback";
import { CreationPanel } from "../creation/CreationPanel";
import { StudioNodeEditor } from "./StudioNodeEditor";
import { useAttachments } from "../assistant/useAttachments";
import { StudioAgent } from "./StudioAgent";
import { removeNodes } from "../canvas/removeNodes";
import { uncollectAsset } from "./assetLibrary";
import { useMissingAssets } from "./useMissingAssets";
import { useAgentTools } from "../assistant/useAgentTools";
import { useClipEditing } from "./useClipEditing";
import { useShortcuts } from "./useShortcuts";
import { useEffect, useCallback, useState } from "react";
import { StudioError } from "./StudioError";
import { useCanvasInsertion } from "./useCanvasInsertion";
import { StudioChrome } from "./StudioChrome";
import { type Project, type Asset } from "../model";
import { useImportMedia } from "./useImportMedia";
import { useProject } from "./useProject";
import { StudioStage, type StudioView } from "./StudioStage";
import { StudioMedia } from "./StudioMedia";
import { StudioTimeline } from "./StudioTimeline";
import { StudioInspector } from "./StudioInspector";
import { StudioDialogs } from "./StudioDialogs";
type StudioProps = { initial: Project; onBack: () => void };
export function Studio({ initial, onBack }: StudioProps) {
  const m = useProject(initial);
  useAgentTools(m);
  const { project, change } = m;
  useMissingAssets(project, change);
  const attachments = useAttachments(initial.id, change);
  const [nodeId, setNodeId] = useState<string | null>(null);
  const [clipId, setClipId] = useState<string | null>(null);
  const [panels, setPanels] = useState(() =>
    initialPanels(initial.clips.length > 0),
  );
  const [scriptContext, setScriptContext] = useState<WorkContext>({
    view: "script",
  });
  const [view, setView] = useState<StudioView>("script");
  const activeView = panels.preview ? "film" : view;
  useTaskNavigation(activeView, (next, id) => {
    if (id) setNodeId(id);
    setView(next);
    setPanels((p) => ({ ...p, preview: false }));
  });
  const canInspect = project.clips.some((clip) => clip.id === clipId);
  const inspectorOpen = panels.inspector && canInspect;
  const togglePanel = (key: keyof typeof panels) => {
    if (key === "inspector" && !canInspect && activeView === "film") return;
    if (key === "agent" || key === "inspector") setCreationTab(null);
    if (key === "inspector" && nodeId && !clipId) setEditing(nodeId);
    else setPanels((p) => toggleStudioPanel(p, key));
  };
  const [editing, setEditing] = useState<string | null>(null);
  const [creationTab, setCreationTab] = useState<string | null>(null);
  const playback = useStudioPlayback(
    project,
    change,
    () => setPanels((p) => ({ ...p, preview: true })),
    () => {
      setClipId(null);
      setNodeId(null);
    },
  );
  const { clock, play } = playback;
  const add = (asset: Asset) => {
    playback.add(asset);
    setPanels((p) => ({ ...p, timeline: true }));
  };
  const [error, setError] = useState("");
  useGeneratedJobs(project.id, m.get, change, m.flush, setError);
  const { busy, importMedia } = useImportMedia(initial.id, change, setError);
  const [dialog, setDialog] = useState<
    "settings" | "models" | "agents" | "export" | null
  >(null);
  const canvas = useProduction(
    project,
    change,
    m.get,
    m.flush,
    () => setPanels((p) => ({ ...p, agent: true, inspector: false })),
    attachments,
  );
  useStudioCreativeEvents(
    clock,
    setPanels,
    setEditing,
    setDialog,
    setClipId,
    setNodeId,
  );
  const selectNode = useCallback((id: string | null) => {
    setNodeId(id);
    setClipId(null);
    if (id) setPanels((p) => ({ ...p, inspector: false }));
  }, []);
  const referenceNode = (id: string) => {
    selectNode(id);
    attachments.add([{ kind: "node", id }]);
    setPanels((p) => ({ ...p, agent: true }));
  };
  const { place, addNote } = useCanvasInsertion(
    project,
    change,
    setView,
    setPanels,
    selectNode,
    setEditing,
  );
  useClipEditing(m, clipId, clock, activeView === "film", setClipId);
  const split = () => playback.split(clipId);
  useShortcuts(m, setError, {
    film: activeView === "film",
    clock,
    split,
    toggleTimeline: () => {
      if (activeView === "film") togglePanel("timeline");
    },
    play,
    remove: () =>
      activeView === "storyboard"
        ? canvas.items
            .filter((n) => canvas.selected.includes(n.key))
            .forEach(canvas.remove)
        : change((p) => ({
            ...removeNodes(p, nodeId ? [nodeId] : []),
            clips: p.clips.filter((c) => c.id !== clipId),
          })),
  });
  const settingsOpen = ["settings", "models", "agents"].includes(dialog ?? "");
  useEffect(() => {
    if (settingsOpen) clock.pause();
  }, [settingsOpen, clock]);
  const sidebarWidths = useSidebarWidths({
    ...panels,
    agent: panels.agent || inspectorOpen || !!creationTab,
  });
  const editNode = project.nodes.find((n) => n.id === editing);
  return (
    <>
      <div
        hidden={settingsOpen}
        style={sidebarWidths.style}
        data-media={panels.media}
        data-agent={panels.agent || inspectorOpen || !!creationTab}
        data-view={activeView}
        className={`studio-shell ${panels.timeline && activeView === "film" ? "" : "timeline-hidden"}`}
      >
        <StudioChrome
          canInspect={canInspect}
          tasks={
            <TaskPanel
              project={project}
              canvas={canvas}
              settings={() => {
                canvas.setTaskPanelOpen(false);
                setDialog("models");
              }}
            />
          }
          view={activeView}
          onView={(next) => {
            clock.pause();
            setView(next);
            setPanels((p) => ({ ...p, preview: false }));
          }}
          m={m}
          onBack={onBack}
          setError={setError}
          onModels={() => setDialog("models")}
          onExport={() => setDialog("export")}
          togglePanel={togglePanel}
          panels={{ ...panels, inspector: inspectorOpen }}
          addNote={addNote}
          onCreation={setCreationTab}
        />
        {creationTab && (
          <CreationPanel
            key={creationTab}
            project={project}
            clock={clock}
            change={change}
            initialTab={creationTab}
            onClose={() => setCreationTab(null)}
          />
        )}
        <StudioError message={error} close={() => setError("")} />
        <StudioStage
          onAdd={add}
          onContext={setScriptContext}
          canvas={canvas}
          view={activeView}
          navigate={setView}
          clock={clock}
          project={project}
          saved={m.saved}
          selected={nodeId}
          onSelect={(id) => {
            selectNode(id);
            if (id) canvas.focusShot(id);
          }}
          onChange={change}
          onReference={referenceNode}
        />
        {panels.media && (
          <StudioMedia
            close={() => togglePanel("media")}
            project={project}
            change={change}
            onImport={() => void importMedia()}
            onAdd={add}
            onPlace={place}
            onEffect={(effect, asset) => {
              clock.pause();
              setCreationTab(null);
              canvas.setComposerMode("agent");
              const ref = asset
                ? { kind: "asset" as const, id: asset.id }
                : clipId
                  ? { kind: "clip" as const, id: clipId }
                  : nodeId
                    ? { kind: "node" as const, id: nodeId }
                    : undefined;
              requestCreativeTask(effectAgentTask(effect, ref));
            }}
            onRemove={(a) => change((p) => uncollectAsset(p, a.id))}
            onReference={(a) => {
              if (canvas.task && activeView === "storyboard")
                canvas.attach({ kind: "asset", id: a.id });
              else attachments.add([{ kind: "asset", id: a.id }]);
              setPanels((p) => ({ ...p, agent: true }));
            }}
            busy={busy}
          />
        )}
        {inspectorOpen && !creationTab && (
          <StudioInspector
            project={project}
            clipId={clipId}
            onChange={change}
            close={() => togglePanel("inspector")}
          />
        )}
        <StudioAgent
          onInspect={() => togglePanel("inspector")}
          canvas={canvas}
          project={project}
          nodeId={nodeId}
          clipId={clipId}
          clock={clock}
          draft={attachments}
          visible={panels.agent && !inspectorOpen && !creationTab}
          flush={m.flush}
          work={activeView === "script" ? scriptContext : { view: activeView }}
          onSettings={() => setDialog("models")}
          onClose={() => togglePanel("agent")}
        />
        {sidebarWidths.handles}
        <GenerationTaskSettings project={project} canvas={canvas} />
        {editNode && (
          <StudioNodeEditor
            key={editNode.id}
            node={editNode}
            project={project}
            change={change}
            close={() => setEditing(null)}
            reference={referenceNode}
            add={add}
          />
        )}
        {activeView === "film" && (
          <StudioTimeline
            project={project}
            clock={clock}
            visible={panels.timeline}
            selected={clipId}
            change={change}
            split={split}
            play={play}
            toggle={() => togglePanel("timeline")}
            captions={() => setCreationTab("captions")}
            select={(id, inspect) => {
              if (id) setCreationTab(null);
              clock.pause();
              setClipId(id);
              setNodeId(null);
              if (inspect) setPanels((p) => ({ ...p, inspector: !!id }));
            }}
            reference={(id) => {
              attachments.add([{ kind: "clip", id }]);
              setPanels((p) => ({ ...p, agent: true, inspector: false }));
            }}
          />
        )}
      </div>
      <StudioDialogs
        kind={dialog}
        project={project}
        close={() => setDialog(null)}
      />
    </>
  );
}
