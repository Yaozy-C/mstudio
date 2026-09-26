import { Trash } from "@phosphor-icons/react";
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
  planId,
  shots,
  change,
  openShots,
}: {
  select?: () => void;
  paragraph: ScriptParagraph;
  index: number;
  planId: string;
  shots: BoardNode[];
  change: (f: (p: Project) => Project) => void;
  openShots: (id?: string) => void;
}) {
  const [count, setCount] = useState(1);
  const [splitting, setSplitting] = useState(false);
  const context = `段落 ${index + 1}`;
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
          aria-label={`${context} · 标题`}
          value={s.title}
          placeholder="段落标题"
          onChange={(e) =>
            change((p) =>
              updateParagraph(p, planId, s.id, { title: e.target.value }),
            )
          }
        />
        <small>
          {shots.length ? `${shots.length} 个关联镜头` : "尚未拆分"}
        </small>
      </header>
      <div className="script-paragraph-time">
        <DurationInput
          label={`${context} · 时长`}
          value={paragraphDuration(s)}
          commit={(duration) =>
            change((p) => updateParagraph(p, planId, s.id, { duration }))
          }
        />
      </div>
      {(
        [
          ["action", "画面与动作", "观众看见什么事件与变化？"],
          ["onScreenText", "画面文字", "画面上出现的准确文字；没有则留空"],
          ["dialogue", "台词 / 旁白", "人物说什么，或旁白如何讲述？"],
          ["sound", "声音与节奏", "环境声、音乐或停顿…"],
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
            change((p) => updateParagraph(p, planId, s.id, { [key]: value }))
          }
        />
      ))}
      <footer>
        <button
          aria-label={`${context} · 删除段落`}
          title="删除段落，保留关联镜头；可撤销"
          onClick={() => {
            flushPendingEdits();
            change((p) => ({
              ...p,
              nodes: p.nodes.map((n) =>
                n.id !== planId
                  ? n
                  : {
                      ...n,
                      plan: {
                        ...n.plan!,
                        script: writeScript(n.plan?.script, {
                          removeParagraphIds: [s.id],
                        }),
                      },
                    },
              ),
            }));
          }}
        >
          <Trash aria-hidden="true" /> 删除段落
        </button>
        {shots.length > 0 && (
          <button onClick={() => openShots(shots[0].id)}>
            查看这段的 {shots.length} 个镜头
          </button>
        )}
        <button
          disabled={!s.action.trim() && !s.dialogue.trim()}
          onClick={() =>
            requestCreativeTask(
              creativeTask(
                "split",
                planId,
                `第 ${index + 1} 段「${s.title || "未命名"}」`,
              ),
            )
          }
        >
          {shots.length ? "用 AI 调整这段镜头" : "用 AI 设计这段镜头"}
        </button>
      </footer>
      <details onToggle={(e) => setSplitting(e.currentTarget.open)}>
        <summary>手动创建镜头草稿</summary>
        {splitting && (
          <div className="script-split">
            <label>
              镜头数量{" "}
              <select
                aria-label={`${context} · 镜头数量`}
                value={count}
                onChange={(e) => setCount(Number(e.target.value))}
              >
                {[1, 2, 3, 4, 5, 6, 7, 8].map((v) => (
                  <option key={v}>{v}</option>
                ))}
              </select>
            </label>
            <p>
              创建文字镜头草稿，保留段落关联。多个镜头需逐个细化动作、分配台词，再制作画面。
            </p>
            <button
              className="primary"
              onClick={() => {
                change((p) => splitParagraph(p, planId, s.id, count));
                setSplitting(false);
              }}
            >
              创建 {count} 个镜头草稿
            </button>
          </div>
        )}
      </details>
    </article>
  );
}
