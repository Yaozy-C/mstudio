import { useLanguage } from "../i18n";
import { StudioSidebar, type SettingsTab } from "./StudioSidebar";
import { useEffect, useRef, useState } from "react";
import { bridge } from "../bridge";
import { type Project, type ProjectSummary } from "../model";
import { NewProjectDialog } from "./NewProjectDialog";
import { initPlugins } from "../plugins/runtime";
import { Studio } from "./Studio";
import { Settings } from "./Settings";
import { PublicAssets } from "./PublicAssets";
import { UserManual } from "../manual/UserManual";
import { forgetViewport } from "../canvas/viewportMemory";
import { Browse } from "./Browse";
import "./workspace-home.css";
import { ProjectLibrary } from "./ProjectLibrary";
import { VideoAnalysis } from "./videoAnalysis/VideoAnalysis";
import { useVideoAnalysis } from "./videoAnalysis/useVideoAnalysis";
import type { CreativeTask } from "../creative/aiTasks";
import { createAnalysisLauncher } from "./videoAnalysis/creationHandoff";
import type { BrowseWork } from "./browseWorks";
export function App() {
  useLanguage();
  const analysis = useVideoAnalysis();
  const [launcher] = useState(() =>
    createAnalysisLauncher(bridge, localStorage),
  );
  const [initialTask, setInitialTask] = useState<CreativeTask>();
  const [entries, setEntries] = useState<ProjectSummary[]>([]);
  const opening = useRef(0);
  const [project, setProject] = useState<Project | null>(null);
  const [settings, setSettings] = useState<SettingsTab | null>(null);
  const [page, setPage] = useState<
    "browse" | "video-analysis" | "projects" | "public-assets" | "manual"
  >("browse");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const analyzeWork = (work: BrowseWork) =>
    navigate(() => {
      analysis.controller.selectWork(work);
      setPage("video-analysis");
    });
  const navigate = (action: () => void) => {
    opening.current++;
    action();
  };
  async function openProject(id: string) {
    const request = ++opening.current;
    try {
      const document = await bridge<Project>("get_project", { projectId: id });
      if (opening.current === request) {
        setInitialTask(undefined);
        setProject(document);
      }
    } catch (e) {
      if (opening.current === request) setError(String(e));
    }
  }
  async function createFromAnalysis() {
    const request = ++opening.current;
    const handoff = await launcher.create(analysis.controller.getSnapshot());
    if (opening.current !== request) return;
    setInitialTask(handoff.task);
    setProject(handoff.project);
  }
  const refresh = () => {
    const request = opening.current;
    setLoading(true);
    return bridge<ProjectSummary[]>("list_projects")
      .then(async (items) => {
        setEntries(items);
        setError("");
        const reopen = sessionStorage.getItem("mstudio-reopen-project");
        if (reopen) {
          sessionStorage.removeItem("mstudio-reopen-project");
          if (opening.current === request && items.some((p) => p.id === reopen))
            await openProject(reopen);
        }
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };
  useEffect(() => {
    void refresh();
    void initPlugins().catch((e) => setError(String(e)));
  }, []);
  if (settings)
    return (
      <Settings
        initialTab={settings}
        projectCount={entries.length}
        onClose={() => {
          setSettings(null);
          setPage("projects");
        }}
        onManual={() => {
          setSettings(null);
          setPage("manual");
        }}
        onPublicAssets={() => {
          setSettings(null);
          setPage("public-assets");
        }}
        onVideoAnalysis={() => {
          setSettings(null);
          setPage("video-analysis");
        }}
        onBrowse={() => {
          setSettings(null);
          setPage("browse");
        }}
      />
    );
  if (project)
    return (
      <Studio
        key={project.id}
        initial={project}
        initialTask={initialTask}
        onBack={() => {
          setProject(null);
          setInitialTask(undefined);
          void refresh();
        }}
      />
    );
  return (
    <div className="library-shell workspace-home-shell">
      <StudioSidebar
        activePage={page}
        count={entries.length}
        onProjects={() => navigate(() => setPage("projects"))}
        onBrowse={() => navigate(() => setPage("browse"))}
        onVideoAnalysis={() => navigate(() => setPage("video-analysis"))}
        onPublicAssets={() => navigate(() => setPage("public-assets"))}
        onManual={() => navigate(() => setPage("manual"))}
        onSettings={(tab) => navigate(() => setSettings(tab))}
      />
      {page === "browse" ? (
        <Browse onAnalyze={analyzeWork} />
      ) : page === "video-analysis" ? (
        <VideoAnalysis
          {...analysis}
          onCreate={createFromAnalysis}
          continuing={launcher.hasProject(analysis.state)}
          onBrowse={() => setPage("browse")}
          onSettings={() => setSettings("models")}
        />
      ) : page === "manual" ? (
        <UserManual onBack={() => setPage("projects")} />
      ) : page === "public-assets" ? (
        <PublicAssets />
      ) : (
        <ProjectLibrary
          entries={entries}
          loading={loading}
          error={error}
          refresh={refresh}
          openProject={openProject}
          onCreate={() => navigate(() => setCreating(true))}
          onReference={() => navigate(() => setPage("video-analysis"))}
          onDelete={async (id) => {
            await bridge("delete_project", { id });
            opening.current++;
            forgetViewport(id);
            localStorage.removeItem(`mstudio-chat-draft:${id}`);
            setEntries((current) => current.filter((entry) => entry.id !== id));
          }}
        />
      )}
      {creating && (
        <NewProjectDialog
          onClose={() => setCreating(false)}
          onCreated={(p) => {
            setInitialTask(undefined);
            setProject(p);
            setCreating(false);
          }}
        />
      )}
    </div>
  );
}
