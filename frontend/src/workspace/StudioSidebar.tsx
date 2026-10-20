import { t, useLanguage } from "../i18n";
import {
  FilmSlate,
  GearSix,
  Graph,
  Stack,
  Cube,
  Wrench,
  SquaresFour,
  ArrowLeft,
  Images,
  BookOpen,
  Compass,
  FilmStrip,
} from "@phosphor-icons/react";
export function sidebarPages() {
  return [
    { id: "general", name: t("通用设置"), icon: GearSix },
    { id: "agents", name: "Agents", icon: Graph },
    { id: "skills", name: t("Skills 能力库"), icon: Stack },
    { id: "models", name: t("模型"), icon: Cube },
    { id: "tools", name: t("工具"), icon: Wrench },
  ] as const;
}
export type SettingsTab = ReturnType<typeof sidebarPages>[number]["id"];

export function StudioSidebar({
  activePage = "projects",
  onProjects,
  onBrowse,
  onVideoAnalysis,
  onSettings,
  onPublicAssets,
  onManual,
  count,
  project,
}: {
  activePage?:
    | "browse"
    | "video-analysis"
    | "projects"
    | "public-assets"
    | "manual"
    | SettingsTab;
  onProjects: () => void;
  onBrowse?: () => void;
  onVideoAnalysis?: () => void;
  onSettings: (page: SettingsTab) => void;
  onPublicAssets?: () => void;
  onManual?: () => void;
  count?: number;
  project?: { name: string };
}) {
  useLanguage();
  const pages = sidebarPages();
  const settingButton = ({ id, name, icon: Icon }: (typeof pages)[number]) => (
    <button
      key={id}
      className={`nav-item ${activePage === id ? "active" : ""}`}
      aria-current={activePage === id ? "page" : undefined}
      onClick={() => onSettings(id)}
    >
      <Icon size={21} />
      {name}
    </button>
  );
  return (
    <nav className="library-nav studio-sidebar" aria-label={t("Studio 导航")}>
      <div className="brand">
        <span className="brand-icon">
          <FilmSlate size={21} />
        </span>
        m <i>/</i> studio
      </div>
      <div className="sidebar-group" role="group" aria-label={t("工作空间")}>
        <h2 className="sidebar-group-title">{t("工作空间")}</h2>
        {onBrowse && !project && (
          <button
            className={`nav-item ${activePage === "browse" ? "active" : ""}`}
            aria-current={activePage === "browse" ? "page" : undefined}
            onClick={onBrowse}
          >
            <Compass size={21} />
            {t("浏览")}
          </button>
        )}
        {onVideoAnalysis && !project && (
          <button
            className={`nav-item ${activePage === "video-analysis" ? "active" : ""}`}
            aria-current={activePage === "video-analysis" ? "page" : undefined}
            onClick={onVideoAnalysis}
          >
            <FilmStrip size={21} />
            {t("视频拆解")}
          </button>
        )}
        <button
          className={`nav-item ${activePage === "projects" ? "active" : ""}`}
          aria-current={activePage === "projects" ? "page" : undefined}
          onClick={onProjects}
        >
          {project ? <ArrowLeft /> : <SquaresFour />}
          {project ? t("返回项目") : t("项目空间")}
          {count !== undefined && <span>{count}</span>}
        </button>
        {onPublicAssets && !project && (
          <button
            className={`nav-item ${activePage === "public-assets" ? "active" : ""}`}
            aria-current={activePage === "public-assets" ? "page" : undefined}
            onClick={onPublicAssets}
          >
            <Images size={21} />
            {t("公共素材")}
          </button>
        )}
        {project && (
          <div className="sidebar-project" title={project.name}>
            {project.name}
          </div>
        )}
      </div>
      <div
        className="sidebar-group sidebar-configuration"
        role="group"
        aria-label={t("创作配置")}
      >
        <h2 className="sidebar-group-title">{t("创作配置")}</h2>
        {pages.filter(({ id }) => id !== "general").map(settingButton)}
      </div>
      <div
        className="sidebar-group sidebar-utilities"
        role="group"
        aria-label={t("设置与帮助")}
      >
        {pages.filter(({ id }) => id === "general").map(settingButton)}
        {onManual && !project && (
          <button
            className={`nav-item ${activePage === "manual" ? "active" : ""}`}
            aria-current={activePage === "manual" ? "page" : undefined}
            onClick={onManual}
          >
            <BookOpen size={21} />
            {t("使用手册")}
          </button>
        )}
      </div>
    </nav>
  );
}
