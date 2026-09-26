import { StudioSidebar } from "./StudioSidebar";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import {
  Wrench,
  Cube,
  Graph,
  Stack,
  Notebook,
  FolderOpen,
} from "@phosphor-icons/react";
import { StorageSettings } from "./StorageSettings";
import { ToolLibrary } from "../agents/ToolLibrary";
import { ModelCenter } from "../models/ModelCenter";
import { SkillLibrary } from "../agents/SkillLibrary";
import { ProjectMemory } from "../assistant/ProjectMemory";
import { AgentCenter } from "../assistant/AgentCenter";
export function Settings({
  onClose,
  initialTab = "models",
  project,
  projectCount,
}: {
  onClose: () => void;
  projectCount?: number;
  project?: { id: string; name: string };
  initialTab?: "models" | "tools" | "agents" | "skills" | "memory" | "storage";
}) {
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
  const pages = [
    { id: "agents", name: "Agents", icon: Graph, hint: "职责、工具与 Skills" },
    { id: "skills", name: "Skills 能力库", icon: Stack, hint: "可装配的能力" },
    { id: "models", name: "模型", icon: Cube, hint: "连接与输入输出能力" },
    ...(project
      ? [
          {
            id: "memory" as const,
            name: "项目记忆",
            icon: Notebook,
            hint: project.name,
          },
        ]
      : []),
    { id: "tools", name: "工具", icon: Wrench, hint: "Agent 操作权限" },
    { id: "storage", name: "存储", icon: FolderOpen, hint: "文件位置与迁移" },
  ] as const;
  return (
    <main className="settings-page" aria-label="设置">
      <StudioSidebar
        settings
        onProjects={onClose}
        onSettings={() => {}}
        count={projectCount}
        project={project}
      >
        <div className="settings-nav" aria-label="设置分类">
          {pages.map(({ id, name, icon: Icon, hint }) => (
            <button
              key={id}
              aria-pressed={tab === id}
              onClick={() => setTab(id)}
            >
              <Icon size={21} />
              <span>
                {name}
                <small>{hint}</small>
              </span>
            </button>
          ))}
        </div>
      </StudioSidebar>
      <div className="hub-main">
        <header className="settings-heading">
          <div className="settings-back-slot" ref={setBackSlot} />
          <div>
            <div className="settings-breadcrumb">
              设置 / {pages.find((p) => p.id === tab)?.name}
            </div>
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
          {tab === "models" && <ModelCenter />}
          {tab === "storage" && <StorageSettings projectId={project?.id} />}
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
