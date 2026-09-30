import { t, useLanguage } from "../i18n";
import { StudioSidebar, sidebarPages, type SettingsTab } from "./StudioSidebar";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { GeneralSettings } from "./GeneralSettings";
import { ToolLibrary } from "../agents/ToolLibrary";
import { ModelCenter } from "../models/ModelCenter";
import { SkillLibrary } from "../agents/SkillLibrary";
import { ProjectMemory } from "../assistant/ProjectMemory";
import { AgentCenter } from "../assistant/AgentCenter";
export function Settings({
  onClose,
  initialTab = "general",
  project,
  projectCount,
  onPublicAssets,
}: {
  onClose: () => void;
  projectCount?: number;
  onPublicAssets?: () => void;
  project?: { id: string; name: string };
  initialTab?: SettingsTab;
}) {
  useLanguage();
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  const [tab, setTab] = useState(initialTab);
  const [backSlot, setBackSlot] = useState<HTMLDivElement | null>(null);
  const content = useRef<HTMLDivElement>(null);
  const resetScroll = useCallback(() => {
    content.current?.scrollTo({ top: 0 });
  }, []);
  useLayoutEffect(resetScroll, [tab, resetScroll]);
  const pages = sidebarPages(!!project);
  return (
    <main className="settings-page" aria-label={t("设置")}>
      <StudioSidebar
        activePage={tab}
        onProjects={onClose}
        onPublicAssets={onPublicAssets}
        onSettings={setTab}
        count={projectCount}
        project={project}
      />
      <div className="hub-main">
        <header className="settings-heading">
          <div className="settings-back-slot" ref={setBackSlot} />
          <div>
            <h1 ref={heading} tabIndex={-1}>
              {pages.find((p) => p.id === tab)?.name}
            </h1>
          </div>
        </header>
        <div className="hub-scroll" ref={content}>
          {tab === "skills" && (
            <SkillLibrary openAgents={() => setTab("agents")} />
          )}
          {tab === "memory" && project && (
            <ProjectMemory
              key={project.id}
              projectId={project.id}
              name={project.name}
            />
          )}
          {tab === "general" && <GeneralSettings projectId={project?.id} />}
          {tab === "models" && <ModelCenter />}
          <div hidden={tab !== "agents"}>
            <AgentCenter
              onNavigate={resetScroll}
              backTarget={tab === "agents" ? backSlot : null}
            />
          </div>
          {tab === "tools" && (
            <ToolLibrary openAgents={() => setTab("agents")} />
          )}
        </div>
      </div>
    </main>
  );
}
