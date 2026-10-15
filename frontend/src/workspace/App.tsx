import { AsyncButton, LoadingState } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { StudioSidebar, type SettingsTab } from "./StudioSidebar";
import { useEffect, useRef, useState } from "react";
import { Plus, FilmSlate, MagnifyingGlass, X } from "@phosphor-icons/react";
import { bridge, native, mediaUrl } from "../bridge";
import { type Project, type ProjectSummary } from "../model";
import { NewProjectDialog } from "./NewProjectDialog";
import { initPlugins } from "../plugins/runtime";
import { Studio } from "./Studio";
import { Settings } from "./Settings";
import { DeleteProjectButton } from "./DeleteProjectButton";
import { PublicAssets } from "./PublicAssets";
import { UserManual } from "../manual/UserManual";
import { forgetViewport } from "../canvas/viewportMemory";
export function App() {
  const language = useLanguage();
  const [entries, setEntries] = useState<ProjectSummary[]>([]);
  const opening = useRef(0);
  const [project, setProject] = useState<Project | null>(null);
  const [settings, setSettings] = useState<SettingsTab | null>(null);
  const [page, setPage] = useState<"projects" | "public-assets" | "manual">(
    "projects",
  );
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [storageNotice, setStorageNotice] = useState(
    () => sessionStorage.getItem("mstudio-storage-notice") || "",
  );
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const filtered = entries.filter((p) =>
    p.name
      .toLocaleLowerCase(language)
      .includes(query.trim().toLocaleLowerCase(language)),
  );
  const navigate = (action: () => void) => {
    opening.current++;
    action();
  };
  async function openProject(id: string) {
    const request = ++opening.current;
    try {
      const document = await bridge<Project>("get_project", { projectId: id });
      if (opening.current === request) setProject(document);
    } catch (e) {
      if (opening.current === request) setError(String(e));
    }
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
      />
    );
  if (project)
    return (
      <Studio
        key={project.id}
        initial={project}
        onBack={() => {
          setProject(null);
          void refresh();
        }}
      />
    );
  return (
    <div className="library-shell">
      <StudioSidebar
        activePage={page}
        count={entries.length}
        onProjects={() => navigate(() => setPage("projects"))}
        onPublicAssets={() => navigate(() => setPage("public-assets"))}
        onManual={() => navigate(() => setPage("manual"))}
        onSettings={(tab) => navigate(() => setSettings(tab))}
      />
      {page === "manual" ? (
        <UserManual onBack={() => setPage("projects")} />
      ) : page === "public-assets" ? (
        <PublicAssets />
      ) : (
        <>
          <main className="library-main">
            <header className="library-heading">
              <div>
                <h1>{t("项目空间")}</h1>
              </div>
              <button
                className="primary"
                onClick={() => navigate(() => setCreating(true))}
              >
                <Plus />
                {t("新建项目")}
              </button>
            </header>
            <div className="library-tools">
              <h3>
                {query.trim() ? t("搜索结果") : t("全部项目")}{" "}
                <span aria-live="polite">
                  {filtered.length.toString().padStart(2, "0")}
                </span>
              </h3>
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
            {loading && !entries.length && (
              <LoadingState label={t("正在读取…")} />
            )}
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
            <section className="project-grid" aria-busy={loading}>
              {filtered.map((p) => (
                <article className="project-card" key={p.id}>
                  <button
                    className="project-cover"
                    onClick={() => void openProject(p.id)}
                  >
                    {p.preview ? (
                      <img
                        src={mediaUrl(p.preview)}
                        alt={p.name}
                        loading="lazy"
                      />
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
                        {new Date(p.updated * 1000).toLocaleDateString(
                          language,
                        )}
                      </p>
                    </button>
                    <DeleteProjectButton
                      project={p}
                      onOpen={() => void openProject(p.id)}
                      onDelete={async (id) => {
                        await bridge("delete_project", { id });
                        opening.current++;
                        forgetViewport(id);
                        localStorage.removeItem(`mstudio-chat-draft:${id}`);
                        setEntries((current) =>
                          current.filter((entry) => entry.id !== id),
                        );
                      }}
                    />
                  </div>
                </article>
              ))}
              {!query.trim() && (
                <button
                  className="new-project-card"
                  onClick={() => navigate(() => setCreating(true))}
                >
                  <Plus size={28} />
                  <strong>{t("新建项目")}</strong>
                </button>
              )}
            </section>
            {!!query.trim() && !filtered.length && !loading && (
              <div className="project-search-empty" role="status">
                <MagnifyingGlass aria-hidden="true" />
                <p>{t("没有匹配的项目")}</p>
                <button type="button" onClick={() => setQuery("")}>
                  {t("清空搜索")}
                </button>
              </div>
            )}
            {!native && (
              <p className="subtle">
                {t("浏览器界面预览 · 素材导入、生成与导出请使用桌面应用")}
              </p>
            )}
          </main>
        </>
      )}
      {creating && (
        <NewProjectDialog
          onClose={() => setCreating(false)}
          onCreated={(p) => {
            setProject(p);
            setCreating(false);
          }}
        />
      )}
    </div>
  );
}
