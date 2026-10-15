import { Cube, Plus, PencilSimple, Trash } from "@phosphor-icons/react";
import { useState } from "react";
import { bridge, native } from "../bridge";
import { t, useLanguage } from "../i18n";
import { usePrevis } from "./context";
export function PrevisLibrary() {
  useLanguage();
  const { workspace, createModel, open, project, refresh, error, loading } =
    usePrevis();
  const [issue, setIssue] = useState("");
  const [deleting, setDeleting] = useState("");
  const [confirm, setConfirm] = useState("");
  const models = workspace.files.filter((f) => f.kind === "model");
  async function remove(id: string, revision: number) {
    setDeleting(id);
    setIssue("");
    try {
      await bridge("delete_previs_model", {
        projectId: project.id,
        id,
        expectedRevision: revision,
      });
      setConfirm("");
      await refresh();
    } catch (e) {
      setIssue(String(e));
    } finally {
      setDeleting("");
    }
  }
  return (
    <section className="previs-library" aria-label={t("项目模型素材")}>
      <header>
        <h3>
          <Cube size={18} />
          {t("模型素材")} <small>{models.length}</small>
        </h3>
        <button
          type="button"
          disabled={!native || loading}
          aria-label={t("添加共用模型")}
          onClick={createModel}
        >
          <Plus />
          {t("添加")}
        </button>
      </header>
      <p className="previs-library-note">
        {t("项目镜头共用 · 有名称的模型资产")}
      </p>
      {(issue || error) && (
        <p role="alert" className="previs-error">
          {issue || error}
        </p>
      )}
      {models.map((m) => {
        const usage = workspace.files.filter(
          (f) =>
            f.kind === "shot" &&
            f.assetIds.includes(m.id) &&
            project.nodes.some((n) => n.id === f.id && n.shot),
        );
        return (
          <article key={m.id}>
            <button
              type="button"
              className="previs-model-open"
              onClick={() => open(m.id)}
            >
              <Cube size={30} />
              <span>
                <strong>{m.name}</strong>
                <small>{m.description || t("用途待补充")}</small>
                <small>
                  {t("{count} 个镜头使用", { count: usage.length })} · v
                  {m.revision}
                </small>
              </span>
              <PencilSimple size={14} />
            </button>
            {confirm === m.id ? (
              <div className="previs-model-delete">
                <span>{t("删除这个模型？")}</span>
                <button
                  disabled={!!deleting}
                  onClick={() => void remove(m.id, m.revision)}
                >
                  {t("删除")}
                </button>
                <button disabled={!!deleting} onClick={() => setConfirm("")}>
                  {t("取消")}
                </button>
              </div>
            ) : (
              <button
                className="previs-model-remove"
                type="button"
                disabled={!!deleting || usage.length > 0}
                title={usage.length ? t("先解除镜头引用") : t("删除模型")}
                aria-label={`${t("删除模型")}：${m.name}`}
                onClick={() => setConfirm(m.id)}
              >
                <Trash size={14} />
              </button>
            )}
          </article>
        );
      })}
      {!models.length && (
        <div className="previs-library-empty">
          <Cube size={36} />
          <p>
            {loading
              ? t("正在读取模型…")
              : t("添加模型，供不同镜头组装和复用。")}
          </p>
          {!native && <small>{t("模型预演需要桌面应用。")}</small>}
        </div>
      )}
    </section>
  );
}
