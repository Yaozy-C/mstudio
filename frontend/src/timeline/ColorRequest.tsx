import { ActionButton } from "../ui/ActionButton";
import { t, useLanguage } from "../i18n";
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
  useLanguage();
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
        <ActionButton
          icon={CircleHalf}
          aria-pressed={comparing}
          disabled={!clip.visual}
          onClick={() => setCompare((v) => !v)}
        >
          {comparing ? t("返回调色效果") : t("原片对比")}
        </ActionButton>
      </div>
      {children}
      <section className="inspector-section inspector-agent">
        <h3>
          <Sparkle size={20} />
          {t("Agent 调色")}
        </h3>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!text.trim()) return;
            requestCreativeTask({
              agentId: "colorist",
              refs: [{ kind: "clip", id: clip.id }],
              text: t(
                "请为本轮引用的片段调色：{v0}\n保留其他片段、剪辑、声音和字幕。",
                { v0: text.trim() },
              ),
            });
          }}
        >
          <textarea
            aria-label={t("调色要求")}
            rows={2}
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={t("整体偏冷，保留产品原色")}
          />
          <ActionButton
            icon={ArrowRight}
            className="inspector-send"
            disabled={!text.trim()}
            type="submit"
          >
            {t("发起调色")}
          </ActionButton>
        </form>
      </section>
    </>
  );
}
