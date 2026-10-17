import { PaperPlaneTilt, Stop } from "@phosphor-icons/react";
import { t } from "../i18n";
import { readyModel } from "../models/types";
import { StudioSelect } from "../ui/StudioSelect";
import { ActionButton } from "../ui/ActionButton";
import { AsyncButton, StatusMessage } from "../ui/AsyncState";
import { PrevisTaskStatus, previsTaskLabel } from "./PrevisTaskStatus";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { PrevisEditorState } from "./usePrevisEditor";

export function PrevisChat({ state }: { state: PrevisEditorState }) {
  const {
    file,
    messages,
    busy,
    pendingPrompt,
    stopping,
    phase,
    chatEnd,
    error,
    send,
    prompt,
    setPrompt,
    model,
    setModelId,
    catalog,
    stop,
  } = state;
  const running = busy === "chat" || busy === "render";
  return (
    <section className="previs-chat">
      <div className="previs-chat-heading">
        <h3>{file.kind === "model" ? t("模型 Chat") : t("镜头 Chat")}</h3>
        <p>{t("只修改当前文件，共用资产保留在项目中。")}</p>
      </div>
      <div className="previs-messages">
        {!messages.length && !running && (
          <p className="previs-chat-empty">
            {file.kind === "model"
              ? t("描述模型的形状、尺寸和可动部件。")
              : t("描述机位、动作和节奏，或直接生成第一版。")}
          </p>
        )}
        {messages.map((m, i) => (
          <div key={i} className={`previs-message ${m.role}`}>
            <small>{m.role === "user" ? t("你") : t("模型")}</small>
            <p>{m.text}</p>
          </div>
        ))}
        {pendingPrompt && (
          <div className="previs-message user">
            <small>{t("你")}</small>
            <p>{pendingPrompt}</p>
          </div>
        )}
        {running && <PrevisTaskStatus state={state} />}
        {!running && !error && phase && (
          <StatusMessage kind={phase === "cancelled" ? "paused" : "success"}>
            {previsTaskLabel(state)}
          </StatusMessage>
        )}
        <div ref={chatEnd} />
      </div>
      {error && (
        <div className="previs-error">
          <ErrorNotice error={error} fallback="PREVIS_CHAT_FAILED" />
        </div>
      )}
      <form
        className="previs-composer"
        aria-busy={running}
        onSubmit={(e) => {
          e.preventDefault();
          void send();
        }}
      >
        <textarea
          aria-label={t("预演修改要求")}
          value={prompt}
          disabled={!!busy}
          placeholder={
            running
              ? t("任务正在后台执行，完成后可继续修改。")
              : file.kind === "model"
                ? t("例如：做一个可以开盖的午餐包，标出盖子和提手。")
                : t("例如：复用午餐包，先取出餐盒放在桌上，再打开餐盒盖。")
          }
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              void send();
            }
          }}
        />
        <div>
          <StudioSelect
            label={t("预演对话模型")}
            disabled={!!busy}
            value={model?.id || ""}
            onValueChange={setModelId}
            placeholder={
              running
                ? t("任务正在后台执行，完成后可继续修改。")
                : t("选择对话模型")
            }
            options={catalog.profiles.map((m) => ({
              value: m.id,
              label: m.name,
              disabled: !readyModel(m),
            }))}
          />
          {running ? (
            <ActionButton
              className="previs-send"
              icon={Stop}
              busy={stopping}
              busyLabel={t("正在停止…")}
              onClick={() => void stop()}
            >
              {t("停止")}
            </ActionButton>
          ) : (
            <AsyncButton
              className="previs-send primary"
              type="submit"
              busy={busy === "save"}
              busyLabel={t("正在保存…")}
              disabled={
                !!busy || !prompt.trim() || !model || !readyModel(model)
              }
            >
              <PaperPlaneTilt />
              {t("发送")}
            </AsyncButton>
          )}
        </div>
      </form>
    </section>
  );
}
