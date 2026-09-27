import { useEffect, useState } from "react";
import type { Clip } from "../model";
import { requestCreativeTask } from "../creative/aiTasks";
import "../styles/color-workflow.css";
export function ColorRequest({ clip }: { clip: Clip }) {
  const [text, setText] = useState("");
  const [compare, setCompare] = useState(false);
  useEffect(() => {
    window.dispatchEvent(
      new CustomEvent("studio-color-compare", {
        detail: compare ? clip.id : null,
      }),
    );
    return () => {
      window.dispatchEvent(
        new CustomEvent("studio-color-compare", { detail: null }),
      );
    };
  }, [compare, clip.id]);
  return (
    <section className="color-request">
      <h3>让 Agent 调色</h3>
      <p>描述想要的色彩，Agent 读取引用片段后调整。原片保留，修改可撤销。</p>
      <textarea
        aria-label="调色要求"
        rows={3}
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="例如：整体偏冷，降低饱和度，保留产品本来的颜色"
      />
      <button
        className="color-request-submit"
        disabled={!text.trim()}
        onClick={() =>
          requestCreativeTask({
            agentId: "colorist",
            refs: [{ kind: "clip", id: clip.id }],
            text: `请为本轮引用的片段调色：${text.trim()}\n保留其他片段、剪辑、声音和字幕。`,
          })
        }
      >
        交给 Agent
      </button>
      <button
        aria-pressed={compare}
        disabled={!clip.visual}
        onClick={() => setCompare((v) => !v)}
      >
        {compare ? "返回调色效果" : "对比原片"}
      </button>
      <small>
        {compare
          ? "正在预览原片色彩，项目与导出设置未改变"
          : "对比仅切换预览，保留字幕、位置和剪辑"}
      </small>
    </section>
  );
}
