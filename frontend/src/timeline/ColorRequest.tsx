import { useEffect, useState, type ReactNode } from "react";
import { CircleHalf, Sparkle, ArrowRight } from "@phosphor-icons/react";
import type { Clip } from "../model";
import { requestCreativeTask } from "../creative/aiTasks";
import "../styles/color-workflow.css";
export function ColorRequest({
  clip,
  children,
}: {
  clip: Clip;
  children?: ReactNode;
}) {
  const [text, setText] = useState("");
  const [compare, setCompare] = useState(false);
  const comparing = compare && !!clip.visual;
  useEffect(() => {
    if (!clip.visual) setCompare(false);
  }, [clip.visual]);
  useEffect(() => {
    window.dispatchEvent(
      new CustomEvent("studio-color-compare", {
        detail: comparing ? clip.id : null,
      }),
    );
    return () => {
      window.dispatchEvent(
        new CustomEvent("studio-color-compare", { detail: null }),
      );
    };
  }, [comparing, clip.id]);
  return (
    <>
      <div className="inspector-comparison">
        <button
          className="inspector-action"
          aria-pressed={comparing}
          disabled={!clip.visual}
          onClick={() => setCompare((v) => !v)}
        >
          <CircleHalf size={22} />
          {comparing ? "返回调色效果" : "原片对比"}
        </button>
      </div>
      {children}
      <section className="inspector-section inspector-agent">
        <h3>
          <Sparkle size={20} />
          Agent 调色
        </h3>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!text.trim()) return;
            requestCreativeTask({
              agentId: "colorist",
              refs: [{ kind: "clip", id: clip.id }],
              text: `请为本轮引用的片段调色：${text.trim()}\n保留其他片段、剪辑、声音和字幕。`,
            });
          }}
        >
          <textarea
            aria-label="调色要求"
            rows={2}
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="整体偏冷，保留产品原色"
          />
          <button
            className="inspector-action inspector-send"
            disabled={!text.trim()}
            type="submit"
          >
            <ArrowRight size={20} />
            发起调色
          </button>
        </form>
      </section>
    </>
  );
}
