import { Play, Pause, FloppyDisk, UploadSimple } from "@phosphor-icons/react";
import { t } from "../i18n";
import { StudioSelect } from "../ui/StudioSelect";
import { RuntimeFrame } from "./RuntimeFrame";
import { PrevisExports } from "./PrevisExports";
import type { PrevisEditorState } from "./usePrevisEditor";

export function PrevisVisual({ state }: { state: PrevisEditorState }) {
  const {
    file,
    project,
    workspace,
    draft,
    setDraft,
    busy,
    ready,
    playing,
    setPlaying,
    time,
    setTime,
    runtime,
    onReady,
    setError,
    models,
    dependency,
    showCode,
    setShowCode,
    dirty,
    manualSave,
    progress,
  } = state;
  return (
    <section className="previs-visual">
      <div className="previs-stage">
        <RuntimeFrame
          ref={runtime}
          projectId={project.id}
          file={file}
          files={workspace.files}
          onReady={onReady}
          onTime={setTime}
          onError={(e) => {
            setError(e);
            onReady(false);
          }}
        />
      </div>
      <div className="previs-transport">
        <button
          type="button"
          disabled={!ready || !!busy}
          aria-label={playing ? t("暂停预演") : t("播放预演")}
          onClick={() => {
            runtime.current?.play(!playing);
            setPlaying(!playing);
          }}
        >
          {playing ? <Pause /> : <Play />}
        </button>
        <input
          type="range"
          min="0"
          max={file.duration}
          step={1 / file.fps}
          value={Math.min(time, file.duration)}
          aria-label={t("预演进度")}
          disabled={!ready || !!busy}
          onChange={(e) => {
            const t = Number(e.target.value);
            runtime.current?.seek(t);
            setTime(t);
            setPlaying(false);
          }}
        />
        <span>
          {time.toFixed(2)} / {file.duration.toFixed(2)}s
        </span>
      </div>
      <div
        className={`previs-properties${file.kind === "model" ? " previs-model-properties" : ""}`}
      >
        <label>
          {t("名称")}
          <input
            value={draft.name}
            disabled={!!busy}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          />
        </label>
        <label className="previs-description">
          {file.kind === "model" ? t("结构与用途说明") : t("镜头画面与动作")}
          <textarea
            value={draft.description}
            rows={3}
            disabled={!!busy}
            onChange={(e) =>
              setDraft({ ...draft, description: e.target.value })
            }
          />
        </label>
        {file.kind === "shot" && (
          <>
            <label>
              {t("预演时长（秒）")}
              <input
                type="number"
                min={1 / draft.fps}
                step="0.1"
                value={draft.duration}
                disabled={!!busy}
                onChange={(e) =>
                  setDraft({ ...draft, duration: Number(e.target.value) })
                }
              />
            </label>
            <label>
              {t("画幅")}
              <StudioSelect
                label={t("画幅")}
                value={`${draft.width}x${draft.height}`}
                disabled={!!busy}
                onValueChange={(value) => {
                  const [width, height] = value.split("x").map(Number);
                  setDraft({ ...draft, width, height });
                }}
                options={[
                  { value: "720x1280", label: "9:16 · 720 × 1280" },
                  { value: "1280x720", label: "16:9 · 1280 × 720" },
                  { value: "720x720", label: "1:1 · 720 × 720" },
                ]}
              />
            </label>
          </>
        )}
      </div>
      {file.kind === "shot" && (
        <section className="previs-model-picker">
          <h3>
            {t("共用模型")} <small>{draft.assetIds.length}</small>
          </h3>
          {models.length ? (
            models.map((m) => (
              <label key={m.id} title={m.description}>
                <input
                  type="checkbox"
                  checked={draft.assetIds.includes(m.id)}
                  disabled={!!busy}
                  onChange={(e) => dependency(m.id, e.target.checked)}
                />
                <span>
                  {m.name}
                  <small>{m.description || t("用途待补充")}</small>
                </span>
                <span>v{m.revision}</span>
              </label>
            ))
          ) : (
            <p>{t("暂无共用模型，生成镜头时可自动添加。")}</p>
          )}
        </section>
      )}
      <details
        open={showCode}
        onToggle={(e) => setShowCode(e.currentTarget.open)}
        className="previs-code"
      >
        <summary>
          {file.kind === "model" ? "model.js" : "scene.js"} ·{" "}
          {t("查看或编辑代码")}
        </summary>
        <textarea
          spellCheck={false}
          aria-label={t("预演代码")}
          value={draft.code}
          disabled={!!busy}
          onChange={(e) => setDraft({ ...draft, code: e.target.value })}
        />
        <label className="previs-code-import" aria-disabled={!!busy}>
          <UploadSimple size={17} />
          {t("导入 JS 文件")}
          <input
            type="file"
            accept=".js,.mjs"
            disabled={!!busy}
            onChange={async (e) => {
              const f = e.target.files?.[0];
              if (!f) return;
              const code = await f.text();
              setDraft((d) => ({ ...d, code }));
              e.target.value = "";
            }}
          />
        </label>
      </details>
      <div className="previs-actions">
        <button
          type="button"
          disabled={!!busy || !dirty}
          onClick={() => void manualSave()}
        >
          <FloppyDisk />
          {t("保存并预览")}
        </button>
        {dirty && (
          <button
            type="button"
            disabled={!!busy}
            onClick={() => {
              setDraft(file);
              setError("");
            }}
          >
            {t("恢复已保存版本")}
          </button>
        )}
      </div>
      {progress && (
        <p className="previs-progress" role="status">
          {progress}
        </p>
      )}
      <PrevisExports state={state} />
    </section>
  );
}
