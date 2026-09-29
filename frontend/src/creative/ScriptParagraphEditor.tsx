import { ActionButton } from "../ui/ActionButton";
import { t, useLanguage } from "../i18n";
import { Trash, FilmStrip, Sparkle } from "@phosphor-icons/react";
import { flushPendingEdits } from "../workspace/pendingEdits";
import { writeScript } from "./scriptWrite";
import { DurationInput } from "./ScriptTiming";
import { paragraphDuration } from "./timing";
import { creativeTask, requestCreativeTask } from "./aiTasks";
import { useState } from "react";
import type { BoardNode, Project } from "../model";
import type { ScriptParagraph } from "./types";
import { InlineText } from "./InlineText";
import { splitParagraph, updateParagraph } from "./script";

export function ScriptParagraphEditor({
  paragraph: s,
  select,
  index,
  screenplayId,
  shots,
  change,
  openShots,
}: {
  select?: () => void;
  paragraph: ScriptParagraph;
  index: number;
  screenplayId: string;
  shots: BoardNode[];
  change: (f: (p: Project) => Project) => void;
  openShots: (id?: string) => void;
}) {
  useLanguage();
  const [count, setCount] = useState(1);
  const [splitting, setSplitting] = useState(false);
  const context = t("段落 {v0}", { v0: index + 1 });
  return (
    <article
      className="script-paragraph"
      id={`script-${s.id}`}
      onFocusCapture={select}
      onClick={select}
    >
      <header>
        <span>{String(index + 1).padStart(2, "0")}</span>
        <input
          aria-label={t("{v0} · 标题", { v0: context })}
          value={s.title}
          placeholder={t("段落标题")}
          onChange={(e) =>
            change((p) =>
              updateParagraph(p, screenplayId, s.id, { title: e.target.value }),
            )
          }
        />
        <small>
          {shots.length
            ? t("{v0} 个关联镜头", { v0: shots.length })
            : t("尚未拆分")}
        </small>
      </header>
      <div className="script-paragraph-time">
        <DurationInput
          label={t("{v0} · 时长", { v0: context })}
          value={paragraphDuration(s)}
          commit={(duration) =>
            change((p) => updateParagraph(p, screenplayId, s.id, { duration }))
          }
        />
      </div>
      {(
        [
          ["action", t("画面与动作"), t("观众看见什么事件与变化？")],
          [
            "onScreenText",
            t("画面文字"),
            t("画面上出现的准确文字；没有则留空"),
          ],
          ["dialogue", t("台词 / 旁白"), t("人物说什么，或旁白如何讲述？")],
          ["sound", t("声音与节奏"), t("环境声、音乐或停顿…")],
        ] as const
      ).map(([key, label, placeholder]) => (
        <InlineText
          key={key}
          context={context}
          label={label}
          value={s[key] ?? ""}
          placeholder={placeholder}
          limit={6000}
          commit={(value) =>
            change((p) =>
              updateParagraph(p, screenplayId, s.id, { [key]: value }),
            )
          }
        />
      ))}
      <footer>
        <ActionButton
          icon={Trash}
          aria-label={t("{v0} · 删除段落", { v0: context })}
          title={t("删除段落，保留关联镜头；可撤销")}
          onClick={() => {
            flushPendingEdits();
            change((p) => ({
              ...p,
              nodes: p.nodes.map((n) =>
                n.id !== screenplayId
                  ? n
                  : {
                      ...n,
                      screenplay: {
                        ...n.screenplay!,
                        script: writeScript(n.screenplay?.script, {
                          removeParagraphIds: [s.id],
                        }),
                      },
                    },
              ),
            }));
          }}
        >
          {t("删除")}
        </ActionButton>
        {shots.length > 0 && (
          <ActionButton icon={FilmStrip} onClick={() => openShots(shots[0].id)}>
            {t("镜头")} {shots.length}
          </ActionButton>
        )}
        <ActionButton
          icon={Sparkle}
          disabled={!s.action.trim() && !s.dialogue.trim()}
          onClick={() =>
            requestCreativeTask(
              creativeTask(
                "split",
                screenplayId,
                t("第 {v0} 段「{v1}」", {
                  v0: index + 1,
                  v1: s.title || t("未命名"),
                }),
              ),
            )
          }
        >
          {shots.length ? t("调整镜头") : t("设计镜头")}
        </ActionButton>
      </footer>
      <details onToggle={(e) => setSplitting(e.currentTarget.open)}>
        <summary>{t("手动创建镜头草稿")}</summary>
        {splitting && (
          <div className="script-split">
            <label>
              {t("镜头数量")}{" "}
              <select
                aria-label={t("{v0} · 镜头数量", { v0: context })}
                value={count}
                onChange={(e) => setCount(Number(e.target.value))}
              >
                {[1, 2, 3, 4, 5, 6, 7, 8].map((v) => (
                  <option key={v}>{v}</option>
                ))}
              </select>
            </label>
            <p>
              {t(
                "创建文字镜头草稿，保留段落关联。多个镜头需逐个细化动作、分配台词，再制作画面。",
              )}
            </p>
            <button
              className="primary"
              onClick={() => {
                change((p) => splitParagraph(p, screenplayId, s.id, count));
                setSplitting(false);
              }}
            >
              {t("创建")} {count} {t("个镜头草稿")}
            </button>
          </div>
        )}
      </details>
    </article>
  );
}
