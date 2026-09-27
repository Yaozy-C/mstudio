import { initialPanels, toggleStudioPanel } from "./studioPanels";
import { useSidebarWidths } from "../ui/useSidebarWidths";
import { GenerationTaskSettings } from "../production/GenerationTaskSettings";
import type { WorkContext } from "../assistant/workContext";
import { useProduction } from "../production/useProduction";
import { useGeneratedJobs } from "./useGeneratedJobs";
import { useCreativeEvents } from "../creative/useCreativeEvents";
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
import { canvasAsset, canvasNote } from "./canvasAsset";
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
  useEffect(() => {
    const show = (event: Event) => {
      const id = (event as CustomEvent<string>).detail;
      setNodeId(id);
      setView("script");
      setPanels((p) => ({ ...p, preview: false }));
    };
    window.addEventListener("studio-show-script", show);
    return () => window.removeEventListener("studio-show-script", show);
  }, []);
  const togglePanel = (key: keyof typeof panels) => {
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
    () => setPanels((p) => ({ ...p, agent: true })),
    attachments,
  );
  useCreativeEvents({
    assist: () => {
      setPanels((p) => ({ ...p, agent: true, inspector: false }));
      setEditing(null);
      setDialog(null);
    },
    reading: () => {
      clock.pause();
      setClipId(null);
      setNodeId(null);
      setPanels((p) => ({ ...p, preview: false, timeline: false }));
    },
    arranged: () => setPanels((p) => ({ ...p, timeline: true, preview: true })),
  });
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
  const place = (asset: Asset) => {
    setView("storyboard");
    setPanels((p) => ({ ...p, preview: false }));
    change((p) => ({
      ...p,
      nodes: [...p.nodes, canvasAsset(asset, p.nodes.length)],
    }));
  };
  function addNote(kind: "text" | "shot", text = "") {
    setView("storyboard");
    setPanels((p) => ({ ...p, preview: false }));
    const node = canvasNote(project, kind, text);
    change((p) => ({ ...p, nodes: [...p.nodes, node] }));
    selectNode(node.id);
    if (!text) setEditing(node.id);
  }
  useClipEditing(m, clipId, clock, activeView === "film", setClipId);
  const split = () => playback.split(clipId);
  useShortcuts(m, setError, {
    clock,
    split,
    toggleTimeline: () => togglePanel("timeline"),
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
    agent: panels.agent || panels.inspector || !!creationTab,
  });
  const editNode = project.nodes.find((n) => n.id === editing);
  return (
    <>
      <div
        hidden={settingsOpen}
        style={sidebarWidths.style}
        data-media={panels.media}
        data-agent={panels.agent || panels.inspector || !!creationTab}
        className={`studio-shell ${panels.timeline && activeView === "film" ? "" : "timeline-hidden"}`}
      >
        <StudioChrome
          view={activeView}
          onView={(next) => {
            clock.pause();
            setView(next);
            setPanels((p) => ({ ...p, preview: false }));
          }}
          m={m}
          onBack={onBack}
          setError={setError}
          onSettings={() => setDialog("settings")}
          onModels={() => setDialog("models")}
          onExport={() => setDialog("export")}
          togglePanel={togglePanel}
          panels={panels}
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
          settings={() => setDialog("models")}
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
        {panels.inspector && !creationTab && (
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
          visible={panels.agent && !panels.inspector && !creationTab}
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
              setPanels((p) => ({ ...p, inspector: !!id || inspect }));
            }}
            reference={(id) => {
              attachments.add([{ kind: "clip", id }]);
              setPanels((p) => ({ ...p, agent: true }));
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
