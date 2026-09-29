import type { ReactNode } from "react";
import { t, useLanguage } from "../i18n";
import { DropdownMenu } from "@radix-ui/themes";
import {
  ArrowLeft,
  CheckCircle,
  CircleDashed,
  WarningCircle,
  ArrowCounterClockwise,
  ArrowClockwise,
  Export,
  Plus,
  NotePencil,
  FilmSlate,
  Images,
  Graph,
  CaretDown,
  SlidersHorizontal,
  Subtitles,
  Cube,
} from "@phosphor-icons/react";
import { SAVED_LABEL, SAVE_DESCRIPTION } from "./projectAutosave";
import type { StudioView } from "./StudioStage";
import type { useProject } from "./useProject";

type Panel = "media" | "preview" | "inspector" | "agent";

export function StudioChrome({
  view,
  canInspect,
  onView,
  m,
  onBack,
  setError,
  onModels,
  onExport,
  togglePanel,
  panels,
  addNote,
  onCreation,
  tasks,
}: {
  canInspect: boolean;
  tasks: ReactNode;
  view: StudioView;
  onView: (view: StudioView) => void;
  m: ReturnType<typeof useProject>;
  onBack: () => void;
  setError: (s: string) => void;
  onModels: () => void;
  onExport: () => void;
  togglePanel: (key: Panel) => void;
  panels: Record<Panel, boolean>;
  addNote: (kind: "text" | "shot") => void;
  onCreation: (tab: string) => void;
}) {
  useLanguage();
  const { project } = m;
  const saveFailed = m.saved.startsWith("保存失败");
  const SaveIcon = saveFailed
    ? WarningCircle
    : m.saved === SAVED_LABEL
      ? CheckCircle
      : CircleDashed;
  return (
    <header className="studio-top" aria-label={t("工作栏")}>
      <button
        className="icon-button"
        title={t("返回项目库")}
        onClick={() =>
          void m
            .flush()
            .then(onBack)
            .catch((e) => setError(String(e)))
        }
      >
        <ArrowLeft />
      </button>
      <span className="studio-wordmark">m / studio</span>
      <div className="studio-project">
        <strong className="project-title" title={project.name}>
          {project.name}
        </strong>
        <span
          className={`project-save-status${saveFailed ? " is-error" : ""}`}
          role="status"
          aria-label={t(m.saved)}
          title={`${t(m.saved)} · ${t(SAVE_DESCRIPTION)}`}
        >
          <SaveIcon size={16} aria-hidden="true" />
          {saveFailed && <small>{t("保存失败")}</small>}
        </span>
        {saveFailed && (
          <button
            type="button"
            onClick={() => void m.flush().catch((e) => setError(String(e)))}
          >
            {t("重试保存")}
          </button>
        )}
      </div>
      <nav className="studio-modes" aria-label={t("工作区模式")}>
        {(
          [
            ["script", t("脚本")],
            ["storyboard", t("制作画布")],
            ["film", t("成片")],
          ] as const
        ).map(([key, label]) => (
          <button
            key={key}
            aria-pressed={view === key}
            onClick={() => onView(key)}
          >
            {label}
          </button>
        ))}
      </nav>
      <span className="top-divider" />
      <button
        className="icon-button"
        title={t("撤销 ⌘Z")}
        disabled={!m.canUndo}
        onClick={m.undo}
      >
        <ArrowCounterClockwise />
      </button>
      <button
        className="icon-button"
        title={t("重做 ⇧⌘Z")}
        disabled={!m.canRedo}
        onClick={m.redo}
      >
        <ArrowClockwise />
      </button>
      <span className="top-divider" />
      <button
        className="chrome-action"
        aria-pressed={panels.media}
        title={t("素材库 · 导入与管理素材")}
        onClick={() => togglePanel("media")}
      >
        <Images />
        {t("素材")}
      </button>
      {tasks}
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <button className="chrome-action">
            <Plus />
            {t("创作")}
            <CaretDown className="chrome-caret" />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content className="studio-menu" sideOffset={12}>
          <DropdownMenu.Item onSelect={() => addNote("text")}>
            <NotePencil />
            {t("文字笔记")}
          </DropdownMenu.Item>
          {
            <DropdownMenu.Item onSelect={() => addNote("shot")}>
              <FilmSlate />
              {t("独立镜头")}
            </DropdownMenu.Item>
          }
          <DropdownMenu.Separator />
          <DropdownMenu.Item onSelect={onModels}>
            <Cube />
            {t("模型中心")}
          </DropdownMenu.Item>
          <DropdownMenu.Item onSelect={() => onCreation("captions")}>
            <Subtitles />
            {t("字幕 / 配音")}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <button
        className="chrome-action"
        aria-pressed={panels.agent}

        onClick={() => togglePanel("agent")}
      >
        <Graph />
        Agent
      </button>
      {view === "film" && canInspect && (
        <button
          className="chrome-action"
          aria-pressed={panels.inspector}
          onClick={() => togglePanel("inspector")}
        >
          <SlidersHorizontal />
          {t("编辑属性")}
        </button>
      )}
      <span className="top-divider" />
      <button
        className="primary chrome-export"
        disabled={!project.clips.length}
        onClick={onExport}
      >
        <Export />
        {t("导出成片")}
      </button>
    </header>
  );
}
