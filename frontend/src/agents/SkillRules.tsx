import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { bridge } from "../bridge";
import type { SkillPage } from "../plugins/skills";
import "../styles/skill-document.css";
export function SkillRules({
  id,
  available,
}: {
  id: string;
  available: boolean;
}) {
  useLanguage();
  const [open, setOpen] = useState(false);
  const [text, setText] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function read() {
    setBusy(true);
    setError("");
    try {
      let offset = 0;
      let document = "";
      while (true) {
        const page = await bridge<SkillPage>("read_creative_skill", {
          id,
          path: "SKILL.md",
          offset,
        });
        document += page.text;
        if (page.nextOffset == null) break;
        if (page.nextOffset <= offset || page.nextOffset > 200000)
          throw new Error(t("规则文档读取异常，请重试"));
        offset = page.nextOffset;
      }
      setText(document.replace(/^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/, ""));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <button
        disabled={!available}
        aria-expanded={open}
        aria-controls={`rules-${id}`}
        onClick={() => {
          setOpen(!open);
          if (!open && text === null && !busy) void read();
        }}
      >
        {open ? t("收起规则") : t("查看规则")}
      </button>
      {open && (
        <div className="skill-document" id={`rules-${id}`} aria-busy={busy}>
          {text !== null && (
            <Markdown
              remarkPlugins={[remarkGfm]}
              components={{
                a: ({ href, children }) =>
                  href && /^https?:\/\//.test(href) ? (
                    <a href={href} target="_blank" rel="noreferrer">
                      {children}
                    </a>
                  ) : (
                    <span className="skill-document-reference" title={href}>
                      {children}
                    </span>
                  ),
              }}
            >
              {text}
            </Markdown>
          )}
          {busy && <p role="status">{t("正在加载文档…")}</p>}
          {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
          {!busy && error && (
            <button onClick={() => void read()}>{t("重试")}</button>
          )}
        </div>
      )}
    </>
  );
}
