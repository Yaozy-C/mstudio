import type { ReactNode } from "react";
import {
  FilmSlate,
  GearSix,
  SquaresFour,
  ArrowLeft,
} from "@phosphor-icons/react";
export function StudioSidebar({
  settings,
  onProjects,
  onSettings,
  count,
  project,
  children,
}: {
  settings: boolean;
  onProjects: () => void;
  onSettings: () => void;
  count?: number;
  project?: { name: string };
  children?: ReactNode;
}) {
  return (
    <nav className="library-nav studio-sidebar" aria-label="Studio 导航">
      <div className="brand">
        <span className="brand-icon">
          <FilmSlate size={21} />
        </span>
        m <i>/</i> studio
      </div>
      <div className="nav-caption">WORKSPACE</div>
      <button
        className={`nav-item ${!settings ? "active" : ""}`}
        aria-current={!settings ? "page" : undefined}
        onClick={onProjects}
      >
        {project ? <ArrowLeft /> : <SquaresFour />}
        {project ? "返回项目" : "项目空间"}
        {count !== undefined && <span>{count}</span>}
      </button>
      {project && (
        <div className="sidebar-project" title={project.name}>
          {project.name}
        </div>
      )}
      <button
        className={`nav-item ${settings ? "active" : ""}`}
        aria-current={settings ? "page" : undefined}
        onClick={onSettings}
      >
        <GearSix />
        设置
      </button>
      {children}
      <div className="nav-bottom">
        <span className="online-dot" />
        本地工作室 <small>v0.1</small>
      </div>
    </nav>
  );
}
