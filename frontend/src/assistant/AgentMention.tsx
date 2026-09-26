import { At, X, CurrencyDollar } from "@phosphor-icons/react";
import type { AgentProfile } from "../agents/catalog";
import { useEffect } from "react";
import type { MentionOption } from "./mentionOptions";
export function AgentMention({
  options,
  symbol,
  selected,
  query,
  index,
  choose,
  remove,
  close,
}: {
  options: MentionOption[];
  symbol: "@" | "$";
  selected?: AgentProfile;
  query: string | null;
  index: number;
  choose: (option: MentionOption) => void;
  remove: () => void;
  close: () => void;
}) {
  useEffect(() => {
    if (query !== null)
      document
        .getElementById(`agent-option-${index}`)
        ?.scrollIntoView({ block: "nearest" });
  }, [index, query]);
  return (
    <>
      {selected && (
        <div className="composer-mention-tag">
          <CurrencyDollar size={14} />
          <span>{selected.name}</span>
          <button
            type="button"
            aria-label={`取消选择 ${selected.name}`}
            onClick={remove}
          >
            <X size={12} />
          </button>
        </div>
      )}
      {query !== null && (
        <div
          className="agent-mention-menu"
          aria-label={symbol === "$" ? "切换 Agent" : "引用元素"}
        >
          <header>
            <strong>{symbol === "$" ? "切换 Agent" : "引用项目元素"}</strong>
            <button type="button" aria-label="关闭选择器" onClick={close}>
              <X />
            </button>
          </header>
          <div
            role="listbox"
            id="agent-mention-options"
            aria-label={symbol === "$" ? "Agent 候选" : "元素候选"}
          >
            {options.map((a, i) => (
              <button
                type="button"
                role="option"
                id={`agent-option-${i}`}
                key={a.key}
                aria-selected={i === index}
                disabled={a.disabled}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => choose(a)}
              >
                {symbol === "$" ? (
                  <CurrencyDollar size={18} />
                ) : (
                  <At size={18} />
                )}
                <span>
                  <strong>{a.title}</strong>
                  <small>{a.description}</small>
                </span>
              </button>
            ))}
            {!options.length && (
              <p>
                {symbol === "$"
                  ? "没有匹配的已启用 Agent。"
                  : "没有匹配元素，试试脚本、镜头或素材名称。"}
              </p>
            )}
          </div>
          <footer>输入名称筛选 · ↑↓ 选择 · Enter 确认 · Esc 关闭</footer>
        </div>
      )}
    </>
  );
}
