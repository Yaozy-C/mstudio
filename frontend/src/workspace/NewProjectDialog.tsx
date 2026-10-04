import { useState } from "react";
import { ArrowUpRight } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { bridge } from "../bridge";
import { newProject, type Project } from "../model";
import { ErrorNotice } from "../errors/ErrorNotice";
import { AsyncButton } from "../ui/AsyncState";
import { StudioSelect } from "../ui/StudioSelect";
import "./new-project.css";

export function NewProjectDialog({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (project: Project) => void;
}) {
  useLanguage();
  const [name, setName] = useState("");
  const [preset, setPreset] = useState("1080x1920");
  const [width, setWidth] = useState("1080");
  const [height, setHeight] = useState("1920");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const valid = [width, height].every((value) => {
    const n = Number(value);
    return Number.isInteger(n) && n >= 64 && n <= 3840 && n % 2 === 0;
  });
  async function create() {
    if (saving || !name.trim() || !valid) return;
    setSaving(true);
    setError("");
    try {
      const project = {
        ...newProject(name.trim()),
        width: Number(width),
        height: Number(height),
      };
      await bridge("create_project", { document: project });
      onCreated(project);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <div
      className="modal-backdrop"
      onClick={() => {
        if (!saving) onClose();
      }}
    >
      <form
        className="modal small new-project-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-project-title"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape" && !saving) {
            e.stopPropagation();
            onClose();
          }
        }}
        onSubmit={(e) => {
          e.preventDefault();
          void create();
        }}
      >
        <h2 id="new-project-title">{t("新建项目")}</h2>
        <label>
          {t("项目名称")}
          <input
            autoFocus
            required
            disabled={saving}
            placeholder={t("例如：夏日出行 · 商品短片")}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <div className="new-project-format">
          <span>{t("项目画幅")}</span>
          <StudioSelect
            label={t("项目画幅")}
            value={preset}
            disabled={saving}
            onValueChange={(value) => {
              setPreset(value);
              if (value !== "custom") {
                const [w, h] = value.split("x");
                setWidth(w);
                setHeight(h);
              }
            }}
            options={[
              {
                value: "1080x1920",
                label: `${t("9:16 · 竖屏")} · 1080 × 1920`,
              },
              {
                value: "1920x1080",
                label: `${t("16:9 · 横屏")} · 1920 × 1080`,
              },
              { value: "1080x1080", label: `${t("1:1 · 方形")} · 1080 × 1080` },
              { value: "custom", label: t("自定义尺寸") },
            ]}
          />
        </div>
        {preset === "custom" && (
          <>
            <div className="new-project-dimensions">
              <label>
                {t("宽度（px）")}
                <input
                  type="number"
                  required
                  min={64}
                  max={3840}
                  step={2}
                  disabled={saving}
                  value={width}
                  onChange={(e) => setWidth(e.target.value)}
                />
              </label>
              <label>
                {t("高度（px）")}
                <input
                  type="number"
                  required
                  min={64}
                  max={3840}
                  step={2}
                  disabled={saving}
                  value={height}
                  onChange={(e) => setHeight(e.target.value)}
                />
              </label>
            </div>
            <p className="subtle">{t("宽高须为 64–3840 之间的偶数。")}</p>
          </>
        )}
        <p className="subtle">
          {t("用于成片预览与导出，并作为 Agent 创作时的画幅参考。")}
        </p>
        {error && <ErrorNotice error={error} />}
        <footer>
          <button disabled={saving} type="button" onClick={onClose}>
            {t("取消")}
          </button>
          <AsyncButton
            type="submit"
            busy={saving}
            className="primary"
            disabled={!name.trim() || !valid}
          >
            {t("创建项目")} <ArrowUpRight />
          </AsyncButton>
        </footer>
      </form>
    </div>
  );
}
