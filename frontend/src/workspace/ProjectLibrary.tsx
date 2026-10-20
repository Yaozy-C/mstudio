import { useState } from "react";
import { Plus, FilmSlate, MagnifyingGlass, X } from "@phosphor-icons/react";
import { AsyncButton, LoadingState } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { native, mediaUrl } from "../bridge";
import type { ProjectSummary } from "../model";
import { DeleteProjectButton } from "./DeleteProjectButton";

export function ProjectLibrary({
  entries,
  loading,
  error,
  refresh,
  openProject,
  onCreate,
  onReference,
  onDelete,
}: {
  entries: ProjectSummary[];
  loading: boolean;
  error: string;
  refresh: () => Promise<void>;
  openProject: (id: string) => Promise<void>;
  onCreate: () => void;
  onReference: () => void;
  onDelete: (id: string) => Promise<void>;
}) {
  const language = useLanguage();
  const [query, setQuery] = useState("");
  const [storageNotice, setStorageNotice] = useState(
    () => sessionStorage.getItem("mstudio-storage-notice") || "",
  );
  const filtered = entries.filter((p) =>
    p.name
      .toLocaleLowerCase(language)
      .includes(query.trim().toLocaleLowerCase(language)),
  );
  return (
    <main className="library-main project-library">
      <header className="library-heading">
        <h1>{t("项目空间")}</h1>
        <div className="library-heading-actions">
          <button onClick={onReference}>{t("从参考视频开始")}</button>
          <button className="primary" onClick={onCreate}>
            <Plus aria-hidden="true" />
            {t("新建项目")}
          </button>
        </div>
      </header>
      <section aria-labelledby="home-projects-title">
        <div className="library-tools">
          <h2 id="home-projects-title">
            {query.trim()
              ? t("搜索结果")
              : entries.length
                ? t("继续创作")
                : t("你的项目")}{" "}
            <span aria-live="polite">
              {filtered.length.toString().padStart(2, "0")}
            </span>
          </h2>
          <div className="project-search">
            <MagnifyingGlass aria-hidden="true" />
            <input
              type="search"
              aria-label={t("搜索项目")}
              placeholder={t("搜索项目…")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            {query && (
              <button
                type="button"
                aria-label={t("清空搜索")}
                onClick={() => setQuery("")}
              >
                <X />
              </button>
            )}
          </div>
        </div>
        {error && (
          <ErrorNotice error={error} fallback="OPERATION_FAILED">
            <AsyncButton busy={loading} onClick={() => void refresh()}>
              {t("重试")}
            </AsyncButton>
          </ErrorNotice>
        )}
        {loading && !entries.length && <LoadingState label={t("正在读取…")} />}
        {storageNotice && (
          <p role="status">
            {storageNotice}{" "}
            <button
              onClick={() => {
                setStorageNotice("");
                sessionStorage.removeItem("mstudio-storage-notice");
              }}
            >
              {t("知道了")}
            </button>
          </p>
        )}
        {!entries.length && !loading && !error && !query.trim() && (
          <p className="home-projects-empty">
            {t("你的下一部作品，从这里开始。创建项目后，随时回来继续。")}
          </p>
        )}
        <div className="project-grid" aria-busy={loading}>
          {filtered.map((p) => (
            <article className="project-card" key={p.id}>
              <button
                className="project-cover"
                onClick={() => void openProject(p.id)}
              >
                {p.preview ? (
                  <img src={mediaUrl(p.preview)} alt={p.name} loading="lazy" />
                ) : (
                  <FilmSlate className="project-empty-mark" />
                )}
                <span className="cover-ratio">
                  {p.width}:{p.height}
                </span>
              </button>
              <div className="project-meta">
                <button onClick={() => void openProject(p.id)}>
                  <h3>{p.name}</h3>
                  <p>
                    {p.clipCount} {t("个片段 ·")}{" "}
                    {new Date(p.updated * 1000).toLocaleDateString(language)}
                  </p>
                </button>
                <DeleteProjectButton
                  project={p}
                  onOpen={() => void openProject(p.id)}
                  onDelete={onDelete}
                />
              </div>
            </article>
          ))}
          {!query.trim() && (
            <button className="new-project-card" onClick={onCreate}>
              <Plus size={28} />
              <strong>{t("新建项目")}</strong>
            </button>
          )}
        </div>
        {!!query.trim() && !filtered.length && !loading && (
          <div className="project-search-empty" role="status">
            <MagnifyingGlass aria-hidden="true" />
            <p>{t("没有匹配的项目")}</p>
            <button type="button" onClick={() => setQuery("")}>
              {t("清空搜索")}
            </button>
          </div>
        )}
      </section>
      {!native && (
        <p className="subtle">
          {t("浏览器界面预览 · 素材导入、生成与导出请使用桌面应用")}
        </p>
      )}
    </main>
  );
}
