import { agentLabel } from "../agents/display";
import { t, useLanguage } from "../i18n";
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
  useLanguage();
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
          <span>{agentLabel(selected)}</span>
          <button
            type="button"
            aria-label={t("取消选择 {v0}", { v0: agentLabel(selected) })}
            onClick={remove}
          >
            <X size={12} />
          </button>
        </div>
      )}
      {query !== null && (
        <div
          className="agent-mention-menu"
          aria-label={symbol === "$" ? t("切换 Agent") : t("引用元素")}
        >
          <header>
            <strong>
              {symbol === "$" ? t("切换 Agent") : t("引用项目元素")}
            </strong>
            <button type="button" aria-label={t("关闭选择器")} onClick={close}>
              <X />
            </button>
          </header>
          <div
            role="listbox"
            id="agent-mention-options"
            aria-label={symbol === "$" ? t("Agent 候选") : t("元素候选")}
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
                  <strong>{a.agent ? agentLabel(a.agent) : a.title}</strong>
                  <small>
                    {a.agent
                      ? agentLabel(a.agent, "description")
                      : t(a.description)}
                  </small>
                </span>
              </button>
            ))}
            {!options.length && (
              <p>
                {symbol === "$"
                  ? t("没有匹配的已启用 Agent。")
                  : t("没有匹配元素，试试脚本、镜头或素材名称。")}
              </p>
            )}
          </div>
          <footer>{t("输入名称筛选 · ↑↓ 选择 · Enter 确认 · Esc 关闭")}</footer>
        </div>
      )}
    </>
  );
}
