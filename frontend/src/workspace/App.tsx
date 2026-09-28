import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { StudioSidebar, type SettingsTab } from "./StudioSidebar";
import { useEffect, useState } from "react";
import { Plus, ArrowUpRight, FilmSlate } from "@phosphor-icons/react";
import { bridge, native, mediaUrl } from "../bridge";
import { newProject, type Project, type ProjectEntry } from "../model";
import { initPlugins } from "../plugins/runtime";
import { Studio } from "./Studio";
import { Settings } from "./Settings";
import { DeleteProjectButton } from "./DeleteProjectButton";
export function App() {
  const language = useLanguage();
  const [entries, setEntries] = useState<ProjectEntry[]>([]);
  const [project, setProject] = useState<Project | null>(null);
  const [settings, setSettings] = useState<SettingsTab | null>(null);
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [storageNotice, setStorageNotice] = useState(
    () => sessionStorage.getItem("mstudio-storage-notice") || "",
  );
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const refresh = () =>
    bridge<ProjectEntry[]>("list_projects")
      .then((items) => {
        setEntries(items);
        const reopen = sessionStorage.getItem("mstudio-reopen-project");
        if (reopen) {
          sessionStorage.removeItem("mstudio-reopen-project");
          setProject(items.find((p) => p.id === reopen)?.document ?? null);
        }
      })
      .catch((e) => setError(String(e)));
  useEffect(() => {
    void refresh();
    void initPlugins().catch((e) => setError(String(e)));
  }, []);
  async function create() {
    if (!name.trim()) return;
    const p = newProject(name.trim());
    await bridge("create_project", { document: p });
    setProject(p);
    setCreating(false);
    setName("");
  }
  if (settings)
    return (
      <Settings
        initialTab={settings}
        projectCount={entries.length}
        onClose={() => setSettings(null)}
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
        count={entries.length}
        onProjects={() => {}}
        onSettings={setSettings}
      />
      <main className="library-main">
        <header className="library-heading">
          <div>
            <h1>{t("项目空间")}</h1>
          </div>
          <button className="primary" onClick={() => setCreating(true)}>
            <Plus />
            {t("新建项目")}
          </button>
        </header>
        <div className="library-tools">
          <h3>
            {t("全部项目")}{" "}
            <span>{entries.length.toString().padStart(2, "0")}</span>
          </h3>
          <input
            placeholder={t("搜索项目…")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
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
        <section className="project-grid">
          {entries
            .filter((p) => p.name.toLowerCase().includes(query.toLowerCase()))
            .map((p) => (
              <article className="project-card" key={p.id}>
                <button
                  className="project-cover"
                  onClick={() => setProject(p.document)}
                >
                  {p.document.assets.find((a) => a.preview && !a.missing)
                    ?.preview ? (
                    <img
                      src={mediaUrl(
                        p.document.assets.find((a) => a.preview && !a.missing)!
                          .preview,
                      )}
                      alt={p.name}
                      loading="lazy"
                    />
                  ) : (
                    <FilmSlate className="project-empty-mark" />
                  )}
                  <span className="cover-ratio">
                    {p.document.width}:{p.document.height}
                  </span>
                  <ArrowUpRight size={20} />
                </button>
                <div className="project-meta">
                  <button onClick={() => setProject(p.document)}>
                    <h3>{p.name}</h3>
                    <p>
                      {p.document.clips.length} {t("个片段 ·")}{" "}
                      {new Date(p.updated * 1000).toLocaleDateString(language)}
                    </p>
                  </button>
                  <DeleteProjectButton
                    project={p}
                    onDelete={async (id) => {
                      await bridge("delete_project", { id });
                      localStorage.removeItem(`mstudio-chat-draft:${id}`);
                      setEntries((current) =>
                        current.filter((entry) => entry.id !== id),
                      );
                    }}
                  />
                </div>
              </article>
            ))}
          <button
            className="new-project-card"
            onClick={() => setCreating(true)}
          >
            <Plus size={28} />
            <strong>{t("新建项目")}</strong>
          </button>
        </section>
        {!native && (
          <p className="subtle">
            {t("浏览器界面预览 · 素材导入、生成与导出请使用桌面应用")}
          </p>
        )}
      </main>
      {creating && (
        <div className="modal-backdrop" onClick={() => setCreating(false)}>
          <form
            className="modal small"
            onClick={(e) => e.stopPropagation()}
            onSubmit={(e) => {
              e.preventDefault();
              void create().catch((e) => setError(String(e)));
            }}
          >
            <h2>{t("新建项目")}</h2>
            <input
              autoFocus
              placeholder={t("例如：夏日出行 · 商品短片")}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <p className="subtle">
              {t("默认竖屏 1080 × 1920，可在项目内调整。")}
            </p>
            <footer>
              <button type="button" onClick={() => setCreating(false)}>
                {t("取消")}
              </button>
              <button className="primary" disabled={!name.trim()}>
                {t("创建项目")} <ArrowUpRight />
              </button>
            </footer>
          </form>
        </div>
      )}
    </div>
  );
}
