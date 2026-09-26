import { TaskReference } from "../production/TaskReference";
import { compositionGuard } from "./compositionGuard";
import type { WorkContext } from "./workContext";
import { useComposerFiles } from "./useComposerFiles";
import { MediaReferenceStrip } from "./MediaReferenceStrip";
import { ComposerGeneration } from "../production/ComposerGeneration";
import type { ProductionController } from "../production/useProduction";
import { ComposerContext } from "./ComposerContext";
import type { ModelConnection } from "../models/types";
import { unsupportedAttachments } from "./AttachmentSupport";
import { ConversationModel } from "./ConversationModel";
import { useState, type RefObject } from "react";
import { ComposerPrimitive } from "@assistant-ui/react";
import { ArrowUp, At, Square, Plus } from "@phosphor-icons/react";
import type { Project } from "../model";
import type { AgentProfile } from "../agents/catalog";
import { attachmentKey, MAX_ATTACHMENTS, type Attachment } from "./attachments";
import type { AttachmentDraft } from "./useAttachments";
import "../styles/agent-composer.css";
import "../styles/agent-mentions.css";
import { AgentMention } from "./AgentMention";
import { mentionQuery } from "./mentions";
import { mentionOptions, type MentionOption } from "./mentionOptions";
export function AgentComposer({
  canvas,
  input,
  project,
  work,
  draft,
  attachments,
  agents,
  agentId,
  chooseAgent,
  getText,
  setText,
  running,
  ready,
  model,
  settings,
}: {
  canvas?: ProductionController;
  input: RefObject<HTMLTextAreaElement | null>;
  project: Project;
  work?: WorkContext;
  draft: AttachmentDraft;
  attachments: Attachment[];
  agents: AgentProfile[];
  agentId: string | null;
  chooseAgent: (id: string | null) => void;
  getText: () => string;
  setText: (text: string) => void;
  running: boolean;
  ready: boolean;
  model?: ModelConnection;
  settings: () => void;
}) {
  const [ime] = useState(compositionGuard);
  const [query, setQuery] = useState<ReturnType<typeof mentionQuery>>(null);
  const [index, setIndex] = useState(0);
  const options = mentionOptions(
    project,
    agents,
    query?.query || "",
    query?.symbol ?? "@",
  ).map((o) => {
    const attached =
      o.ref &&
      attachments.some((a) => attachmentKey(a) === attachmentKey(o.ref!));
    return {
      ...o,
      disabled: !!o.ref && (attached || attachments.length >= MAX_ATTACHMENTS),
      description:
        o.description +
        (attached
          ? " · 已添加"
          : o.ref && attachments.length >= MAX_ATTACHMENTS
            ? " · 附件已满（12个）"
            : ""),
    };
  });
  function detect(text: string, caret: number) {
    setQuery(mentionQuery(text, caret));
    setIndex(0);
  }
  function choose(option: MentionOption) {
    if (option.disabled) return;
    const text = getText();
    if (query) setText(text.slice(0, query.start) + text.slice(query.end));
    if (option.agent) chooseAgent(option.agent.id);
    else if (option.ref) {
      if (canvas?.task) canvas.attach(option.ref);
      else draft.add([option.ref]);
    }
    setQuery(null);
    input.current?.focus();
  }
  function open(symbol: "@" | "$") {
    const text = getText();
    const caret = query?.start ?? input.current?.selectionStart ?? text.length;
    const end = query?.end ?? caret;
    const insert =
      (!query && caret > 0 && !/\s/.test(text[caret - 1]) ? " " : "") + symbol;
    const next = text.slice(0, caret) + insert + text.slice(end);
    setText(next);
    detect(next, caret + insert.length);
    requestAnimationFrame(() => {
      input.current?.focus();
      input.current?.setSelectionRange(
        caret + insert.length,
        caret + insert.length,
      );
    });
  }
  const { dragging, ...fileEvents } = useComposerFiles(
    draft,
    canvas?.task
      ? (refs) => refs.forEach((ref) => canvas.attach(ref))
      : undefined,
  );
  const blocked =
    attachments.length > MAX_ATTACHMENTS ||
    unsupportedAttachments(attachments, model).length > 0;
  return (
    <div className="agent-input-area">
      <ComposerPrimitive.Root
        className={`agent-composer${dragging ? " is-file-dragging" : ""}`}
        {...fileEvents}
      >
        {dragging && <div className="composer-drop-hint">松开以添加附件</div>}
        <AgentMention
          options={options}
          symbol={query?.symbol ?? "@"}
          selected={agents.find((a) => a.id === agentId)}
          query={query?.query ?? null}
          index={index}
          choose={choose}
          remove={() => chooseAgent(null)}
          close={() => setQuery(null)}
        />
        {canvas && <TaskReference canvas={canvas} />}
        {canvas?.task ? (
          <MediaReferenceStrip
            canvas={canvas}
            project={project}
            draft={draft}
          />
        ) : (
          <ComposerContext
            work={work}
            project={project}
            draft={draft}
            attachments={attachments}
            model={model}
            settings={settings}
          />
        )}
        {canvas?.task && blocked && (
          <p className="agent-attachment-hint">
            {attachments.length > MAX_ATTACHMENTS
              ? "本次最多引用 12 项，请移除部分引用。"
              : "当前对话模型无法读取所选资料，请切换支持这些素材的对话模型。"}
          </p>
        )}
        <ComposerPrimitive.Input
          {...ime}
          ref={input}
          addAttachmentOnPaste={false}
          minRows={2}
          maxRows={7}
          placeholder={
            attachments.length
              ? "想对这些内容做什么？"
              : "描述任务，@ 引用素材…"
          }
          aria-label="发送给 Agent"
          aria-autocomplete="list"
          aria-controls={query ? "agent-mention-options" : undefined}
          aria-activedescendant={
            query && options[index] ? `agent-option-${index}` : undefined
          }
          onChange={(e) => {
            detect(e.target.value, e.target.selectionStart);
          }}
          onSelect={(e) => {
            if (query)
              detect(e.currentTarget.value, e.currentTarget.selectionStart);
          }}
          onKeyDown={(e) => {
            if (e.nativeEvent.isComposing) return;
            if (
              !query &&
              (blocked || draft.busy) &&
              e.key === "Enter" &&
              !e.shiftKey
            ) {
              e.preventDefault();
              return;
            }
            if (
              query &&
              ["Enter", "Tab", "ArrowDown", "ArrowUp", "Escape"].includes(e.key)
            ) {
              e.preventDefault();
              e.stopPropagation();
              if (e.key === "Escape") setQuery(null);
              else if (["Enter", "Tab"].includes(e.key)) {
                if (options[index]) choose(options[index]);
              } else
                setIndex(
                  (i) =>
                    (i +
                      (e.key === "ArrowDown" ? 1 : -1) +
                      Math.max(1, options.length)) %
                    Math.max(1, options.length),
                );
            }
          }}
        />
        <div className="agent-compose-actions">
          <div className="composer-tools">
            <>
              <button
                type="button"
                className="composer-icon-button"
                aria-label="从电脑添加附件"
                title="从电脑添加附件"
                disabled={draft.busy || attachments.length >= MAX_ATTACHMENTS}
                onClick={() =>
                  void draft.importFiles(
                    canvas?.task
                      ? (refs) => refs.forEach((ref) => canvas.attach(ref))
                      : undefined,
                  )
                }
              >
                <Plus size={20} />
              </button>
              <button
                type="button"
                className="composer-mention-trigger composer-icon-button"
                aria-label="引用元素"
                title="引用项目元素（@）"
                disabled={running}
                onClick={() => open("@")}
              >
                <At size={19} />
              </button>
            </>
            {canvas ? (
              <ComposerGeneration
                canvas={canvas}
                project={project}
                settings={settings}
                running={running}
              />
            ) : (
              <ConversationModel
                projectId={project.id}
                disabled={running}
                settings={settings}
              />
            )}
          </div>
          <div className="composer-submit">
            {running ? (
              <ComposerPrimitive.Cancel aria-label="停止回答" title="停止回答">
                <Square size={13} weight="fill" />
              </ComposerPrimitive.Cancel>
            ) : (
              <ComposerPrimitive.Send
                aria-label="发送消息"
                title={ready ? "发送消息" : "请先连接并配置模型"}
                disabled={draft.busy || !ready || blocked}
              >
                <ArrowUp size={18} weight="bold" />
              </ComposerPrimitive.Send>
            )}
          </div>
        </div>
      </ComposerPrimitive.Root>
      <div className="composer-footer">
        ↵ 发送
        <span>⇧ ↵ 换行</span>
      </div>
    </div>
  );
}
