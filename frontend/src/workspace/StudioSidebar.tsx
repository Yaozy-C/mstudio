import { t, useLanguage } from "../i18n";
import {
  FilmSlate,
  GearSix,
  Graph,
  Stack,
  Cube,
  Wrench,
  Notebook,
  SquaresFour,
  ArrowLeft,
} from "@phosphor-icons/react";
export function sidebarPages(hasProject = false) {
  return [
    { id: "general", name: t("通用设置"), icon: GearSix },
    { id: "agents", name: "Agents", icon: Graph },
    { id: "skills", name: t("Skills 能力库"), icon: Stack },
    { id: "models", name: t("模型"), icon: Cube },
    { id: "tools", name: t("工具"), icon: Wrench },
    ...(hasProject
      ? [{ id: "memory" as const, name: t("项目记忆"), icon: Notebook }]
      : []),
  ] as const;
}
export type SettingsTab = ReturnType<typeof sidebarPages>[number]["id"];

export function StudioSidebar({
  activePage = "projects",
  onProjects,
  onSettings,
  count,
  project,
}: {
  activePage?: "projects" | SettingsTab;
  onProjects: () => void;
  onSettings: (page: SettingsTab) => void;
  count?: number;
  project?: { name: string };
}) {
  useLanguage();
  return (
    <nav className="library-nav studio-sidebar" aria-label={t("Studio 导航")}>
      <div className="brand">
        <span className="brand-icon">
          <FilmSlate size={21} />
        </span>
        m <i>/</i> studio
      </div>
      <button
        className={`nav-item ${activePage === "projects" ? "active" : ""}`}
        aria-current={activePage === "projects" ? "page" : undefined}
        onClick={onProjects}
      >
        {project ? <ArrowLeft /> : <SquaresFour />}
        {project ? t("返回项目") : t("项目空间")}
        {count !== undefined && <span>{count}</span>}
      </button>
      {project && (
        <div className="sidebar-project" title={project.name}>
          {project.name}
        </div>
      )}
      {sidebarPages(!!project).map(({ id, name, icon: Icon }) => (
        <button
          key={id}
          className={`nav-item ${activePage === id ? "active" : ""}`}
          aria-current={activePage === id ? "page" : undefined}
          onClick={() => onSettings(id)}
        >
          <Icon size={21} />
          {name}
        </button>
      ))}
    </nav>
  );
}
