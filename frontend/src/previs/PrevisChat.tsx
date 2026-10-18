import { useState } from "react";
import { compositionGuard } from "../assistant/compositionGuard";
import { usePrevisScroll } from "./usePrevisScroll";
import { ArrowUp, Stop } from "@phosphor-icons/react";
import { t } from "../i18n";
import { readyModel } from "../models/types";
import { StudioSelect } from "../ui/StudioSelect";
import { IconButton } from "../ui/IconButton";
import { PrevisAgentMessages } from "./PrevisAgentMessages";
import "../styles/agent-composer.css";
import { AsyncButton, StatusMessage } from "../ui/AsyncState";
import { PrevisTaskStatus, previsTaskLabel } from "./PrevisTaskStatus";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { PrevisEditorState } from "./usePrevisEditor";

export function PrevisChat({ state }: { state: PrevisEditorState }) {
  const {
    file,
    messages,
    busy,
    stopping,
    phase,
    error,
    send,
    prompt,
    setPrompt,
    model,
    setModelId,
    catalog,
    stop,
  } = state;
  const [ime] = useState(compositionGuard);
  const scroll = usePrevisScroll(state);
  const canSend = !busy && !!prompt.trim() && !!model && readyModel(model);
  const running = busy === "chat" || busy === "render";
  return (
    <section className="previs-chat">
      <div className="previs-chat-heading">
        <h3>{t("预演 Agent")}</h3>
        <p>{t("专门生成和修改预演，不带主对话上下文。")}</p>
      </div>
      <div className="previs-messages" {...scroll}>
        {!messages.length && !running && (
          <p className="previs-chat-empty">
            {file.kind === "model"
              ? t("描述模型的形状、尺寸和可动部件。")
              : t("描述机位、动作和节奏，或直接生成第一版。")}
          </p>
        )}
        <PrevisAgentMessages state={state} />
        {running && <PrevisTaskStatus state={state} />}
        {!running && !error && phase && (
          <StatusMessage kind={phase === "cancelled" ? "paused" : "success"}>
            {previsTaskLabel(state)}
          </StatusMessage>
        )}
      </div>
      {error && (
        <div className="previs-error">
          <ErrorNotice error={error} fallback="PREVIS_CHAT_FAILED" />
        </div>
      )}
      <form
        className="agent-composer previs-composer"
        aria-busy={running}
        onSubmit={(e) => {
          e.preventDefault();
          if (canSend) void send();
        }}
      >
        <textarea
          {...ime}
          rows={3}
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
            if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229)
              return;
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              if (canSend) void send();
            }
          }}
        />
        <div className="agent-compose-actions">
          <div className="composer-tools">
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
          </div>
          <div className="composer-submit">
            {running ? (
              <IconButton
                icon={Stop}
                label={stopping ? t("正在停止…") : t("停止")}
                busy={stopping}
                onClick={() => void stop()}
              />
            ) : (
              <AsyncButton
                type="submit"
                aria-label={t("发送")}
                title={t("发送")}
                busy={busy === "save"}
                busyLabel={t("正在保存…")}
                disabled={!canSend}
              >
                <ArrowUp aria-hidden="true" />
              </AsyncButton>
            )}
          </div>
        </div>
      </form>
    </section>
  );
}
