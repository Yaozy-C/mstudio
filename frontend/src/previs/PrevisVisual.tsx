import { Play, Pause, FloppyDisk } from "@phosphor-icons/react";
import { t } from "../i18n";
import { StudioSelect } from "../ui/StudioSelect";
import { ActionButton } from "../ui/ActionButton";
import { IconButton } from "../ui/IconButton";
import { PrevisTaskStatus } from "./PrevisTaskStatus";
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
    dirty,
    manualSave,
  } = state;
  const running = busy === "chat" || busy === "render";
  return (
    <section className="previs-visual">
      <div className="previs-stage" aria-busy={running}>
        {busy !== "render" && (file.code.trim() || !running) && (
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
        )}
        {running && (
          <div className="previs-stage-overlay">
            <PrevisTaskStatus state={state} stage />
          </div>
        )}
      </div>
      <div className="previs-transport">
        <IconButton
          icon={playing ? Pause : Play}
          disabled={!ready || !!busy}
          label={playing ? t("暂停预演") : t("播放预演")}
          onClick={() => {
            runtime.current?.play(!playing);
            setPlaying(!playing);
          }}
        />
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
      <PrevisExports state={state} />
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
              </label>
            ))
          ) : (
            <p>{t("暂无共用模型，生成镜头时可自动添加。")}</p>
          )}
        </section>
      )}
      <div className="previs-actions">
        <ActionButton
          icon={FloppyDisk}
          busy={busy === "save"}
          busyLabel={t("正在保存…")}
          disabled={!!busy || !dirty}
          onClick={() => void manualSave()}
        >
          {t("保存并预览")}
        </ActionButton>
        {dirty && (
          <button
            type="button"
            disabled={!!busy}
            onClick={() => {
              setDraft(file);
              setError("");
            }}
          >
            {t("放弃未保存修改")}
          </button>
        )}
      </div>
    </section>
  );
}
