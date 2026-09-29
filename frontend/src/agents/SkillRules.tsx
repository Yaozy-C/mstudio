import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useRef, useState } from "react";
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
  const [path, setPath] = useState("SKILL.md");
  const [page, setPage] = useState<SkillPage | null>(null);
  const [draft, setDraft] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const sequence = useRef(0);
  async function read(selected = path) {
    const request = ++sequence.current;
    setBusy(true);
    setError("");
    try {
      let offset = 0;
      let text = "";
      let revision: number | undefined;
      while (true) {
        const part = await bridge<SkillPage>("read_creative_skill", {
          id,
          path: selected,
          offset,
        });
        if (revision !== undefined && part.revision !== revision)
          throw new Error(t("规则已更新，请重新加载"));
        revision = part.revision;
        text += part.text;
        if (part.nextOffset == null) {
          if (request === sequence.current) {
            setPath(selected);
            setPage({ ...part, text });
            setDraft(null);
          }
          break;
        }
        if (part.nextOffset <= offset || part.nextOffset > 200000)
          throw new Error(t("规则文档读取异常，请重试"));
        offset = part.nextOffset;
      }
    } catch (e) {
      if (request === sequence.current) setError(String(e));
    } finally {
      if (request === sequence.current) setBusy(false);
    }
  }
  async function save() {
    if (!page || draft === null) return;
    setBusy(true);
    setError("");
    try {
      const result = await bridge<{ revision: number }>("save_creative_skill", {
        id,
        path,
        text: draft,
        revision: page.revision,
      });
      setPage({ ...page, text: draft, revision: result.revision });
      setDraft(null);
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
          if (!open && page === null && !busy) void read();
        }}
      >
        {open ? t("收起规则") : t("查看规则")}
      </button>
      {open && (
        <div className="skill-document" id={`rules-${id}`} aria-busy={busy}>
          {page && (
            <>
              <div className="skill-document-actions">
                <select
                  aria-label={t("规则文档")}
                  value={path}
                  disabled={busy || draft !== null}
                  onChange={(e) => void read(e.target.value)}
                >
                  {["SKILL.md", ...(page.resources ?? [])].map((resource) => (
                    <option key={resource} value={resource}>
                      {resource === "CORE.md"
                        ? t("自动加载核心规则")
                        : resource}
                    </option>
                  ))}
                </select>
                {draft === null ? (
                  <button disabled={busy} onClick={() => setDraft(page.text)}>
                    {t("编辑规则")}
                  </button>
                ) : (
                  <>
                    <button
                      disabled={busy || !draft.trim()}
                      onClick={() => void save()}
                    >
                      {t("保存规则")}
                    </button>
                    <button
                      disabled={busy}
                      onClick={() => {
                        setDraft(null);
                        setError("");
                      }}
                    >
                      {t("放弃修改")}
                    </button>
                  </>
                )}
                <button
                  disabled={busy || draft !== null}
                  onClick={() => void read()}
                >
                  {t("重新加载")}
                </button>
              </div>
              {draft !== null ? (
                <textarea
                  className="skill-source"
                  aria-label={t("规则正文")}
                  value={draft}
                  disabled={busy}
                  rows={22}
                  onChange={(e) => setDraft(e.target.value)}
                />
              ) : (
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
                  {page.text.replace(
                    /^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/,
                    "",
                  )}
                </Markdown>
              )}
            </>
          )}
          {busy && <p role="status">{t("正在加载文档…")}</p>}
          {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
          {!busy && error && !page && (
            <button onClick={() => void read()}>{t("重试")}</button>
          )}
        </div>
      )}
    </>
  );
}
