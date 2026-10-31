import { StudioProviders } from "./StudioProviders";
import { StudioTaskPanel } from "./StudioTaskPanel";
import { useTaskNavigation } from "./useTaskNavigation";
import { initialPanels, toggleStudioPanel } from "./studioPanels";
import { useSidebarWidths } from "../ui/useSidebarWidths";
import { GenerationTaskSettings } from "../production/GenerationTaskSettings";
import type { WorkContext } from "../assistant/workContext";
import { useProduction } from "../production/useProduction";
import { useStudioCreativeEvents } from "./useStudioCreativeEvents";
import { useStudioPlayback, useCreationTab } from "./useStudioPlayback";
import { CreationPanel } from "../creation/CreationPanel";
import { StudioNodeEditor } from "./StudioNodeEditor";
import { useAttachments } from "../assistant/useAttachments";
import { StudioAgent } from "./StudioAgent";
import { useMissingAssets } from "./useMissingAssets";
import { useClipEditing } from "./useClipEditing";
import { useShortcuts } from "./useShortcuts";
import { useCallback, useState } from "react";
import { StudioError } from "./StudioError";
import { useCanvasInsertion } from "./useCanvasInsertion";
import { StudioChrome } from "./StudioChrome";
import type { Asset } from "../model";
import { useProject } from "./useProject";
import { StudioStage, type StudioView } from "./StudioStage";
import { StudioTimeline } from "./StudioTimeline";
import { StudioInspector } from "./StudioInspector";
import { StudioDialogs, type StudioDialogKind } from "./StudioDialogs";
import type { StudioProps } from "./StudioProps";
import { initialStudioView } from "./initialStudioView";
export function Studio({ initial, initialTask, onBack }: StudioProps) {
  const m = useProject(initial);
  const { project, change } = m;
  useMissingAssets(project, change);
  const attachments = useAttachments(initial.id, change, initialTask);
  const [nodeId, setNodeId] = useState<string | null>(null);
  const [clipId, setClipId] = useState<string | null>(null);
  const [panels, setPanels] = useState(() =>
    initialPanels(initial.clips.length > 0),
  );
  const [scriptContext, setScriptContext] = useState<WorkContext>({
    view: "script",
  });
  const [view, setView] = useState<StudioView>(() =>
    initialStudioView(initial, initialTask),
  );
  const activeView = panels.preview ? "film" : view;
  useTaskNavigation(activeView, (next, id) => {
    if (id) setNodeId(id);
    setView(next);
    if (next !== "storyboard" && next !== "drafts")
      canvas.setComposerMode("agent");
    setCreationTab(null);
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
  const [creationTab, setCreationTab] = useCreationTab();
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
  const [dialog, setDialog] = useState<StudioDialogKind>(null);
  const canvas = useProduction(
    project,
    change,
    m.get,
    m.flush,
    () => setPanels((p) => ({ ...p, agent: true, inspector: false })),
    attachments,
    activeView === "storyboard" ? (nodeId ?? undefined) : undefined,
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
  const split = () => {
    if (activeView === "film") playback.split(clipId);
  };
  useShortcuts(m, setError, {
    film: activeView === "film",
    clock,
    split,
    toggleTimeline: () => {
      if (activeView === "film") togglePanel("timeline");
    },
    play,
    remove: () => {
      if (activeView !== "film" && !(activeView === "drafts")) return;
      return activeView === "drafts"
        ? canvas.items
            .filter((n) => canvas.selected.includes(n.key))
            .forEach(canvas.remove)
        : change((p) => ({
            ...p,
            clips: p.clips.filter((c) => c.id !== clipId),
          }));
    },
  });
  const sidebarWidths = useSidebarWidths({
    ...panels,
    agent: panels.agent || inspectorOpen || !!creationTab,
  });
  const editNode = project.nodes.find((n) => n.id === editing);
  const showModels = () => setDialog("models");
  return (
    <StudioProviders workspace={m} draft={attachments}>
      <div
        hidden={["settings", "models", "agents"].includes(dialog ?? "")}
        style={sidebarWidths.style}
        data-agent={panels.agent || inspectorOpen || !!creationTab}
        data-view={activeView}
        className={`studio-shell ${panels.timeline && activeView === "film" ? "" : "timeline-hidden"}`}
      >
        <StudioChrome
          canInspect={canInspect}
          tasks={
            <StudioTaskPanel
              project={project}
              canvas={canvas}
              onModels={showModels}
            />
          }
          view={activeView}
          onView={(next) => {
            clock.pause();
            setView(next);
            if (next !== "storyboard" && next !== "drafts")
              canvas.setComposerMode("agent");
            setPanels((p) => ({
              ...p,
              preview: false,
              ...(next !== "film" ? { inspector: false } : {}),
            }));
            setCreationTab(null);
          }}
          m={m}
          onBack={onBack}
          setError={setError}
          onModels={showModels}
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
          onPlace={place}
          onAssetReference={(a) => {
            canvas.setComposerMode("agent");
            if (activeView === "materials") setView("script");
            attachments.add([{ kind: "asset", id: a.id }]);
            setPanels((p) => ({ ...p, agent: true, inspector: false }));
            setCreationTab(null);
          }}
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
          canvas={
            activeView === "storyboard" || activeView === "drafts"
              ? canvas
              : undefined
          }
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
        clock={clock}
        kind={dialog}
        project={project}
        close={() => setDialog(null)}
      />
    </StudioProviders>
  );
}
