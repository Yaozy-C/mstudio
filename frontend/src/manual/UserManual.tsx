import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  MagnifyingGlass,
  X,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { ActionButton } from "../ui/ActionButton";
import { ManualArticle } from "./ManualArticle";
import { manualSections, type ManualSection } from "./sections";
import { manualShortcuts } from "./shortcuts";
import "./manual.css";

const chapters: ManualSection[] = [
  ...manualSections,
  {
    id: "shortcuts",
    group: "help",
    title: "常用操作速查",
    summary: "熟悉这些按键，剪辑时就能少点几次鼠标。",
    steps: manualShortcuts.map(([title, mac, windows]) => ({
      title,
      body: `${mac} ${windows}`,
    })),
  },
];
const groups = [
  { id: "start", title: "入门" },
  { id: "create", title: "创作" },
  { id: "help", title: "管理与帮助" },
];
const number = (index: number) => String(index + 1).padStart(2, "0");
function currentChapter() {
  const hash = typeof window === "undefined" ? "" : window.location.hash;
  const id = hash.startsWith("#manual-") ? hash.slice(8) : "";
  return chapters.find((chapter) => chapter.id === id)?.id ?? "start";
}
function searchText(chapter: ManualSection) {
  return [
    chapter.title,
    chapter.summary,
    chapter.tip,
    chapter.example,
    chapter.outcome,
    ...chapter.steps.flatMap((step) => [step.title, step.body]),
  ]
    .filter((text): text is string => !!text)
    .map((text) => t(text))
    .join(" ");
}

export function UserManual({ onBack }: { onBack: () => void }) {
  useLanguage();
  const [active, setActive] = useState(currentChapter);
  const [query, setQuery] = useState("");
  const main = useRef<HTMLElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const index = chapters.findIndex((chapter) => chapter.id === active);
  const section = chapters[index];
  const term = query.trim().toLocaleLowerCase();
  const matches = chapters.filter((chapter) =>
    searchText(chapter).toLocaleLowerCase().includes(term),
  );

  useEffect(() => {
    const sync = () => setActive(currentChapter());
    window.addEventListener("hashchange", sync);
    return () => window.removeEventListener("hashchange", sync);
  }, []);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
    main.current?.scrollIntoView({ block: "start" });
  }, [active]);

  return (
    <main ref={main} className="library-main manual-main">
      <header className="library-heading manual-heading">
        <h1>{t("使用手册")}</h1>
        <ActionButton icon={ArrowLeft} onClick={onBack}>
          {t("返回项目空间")}
        </ActionButton>
      </header>
      <div className="manual-layout">
        <aside className="manual-sidebar">
          <div className="manual-search">
            <MagnifyingGlass size={17} aria-hidden="true" />
            <input
              ref={search}
              type="search"
              aria-label={t("搜索手册")}
              placeholder={t("搜索操作或问题…")}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            {query && (
              <button
                type="button"
                aria-label={t("清除搜索")}
                onClick={() => {
                  setQuery("");
                  search.current?.focus();
                }}
              >
                <X size={16} />
              </button>
            )}
          </div>
          <nav className="manual-contents" aria-label={t("手册目录")}>
            {groups.map((group) => {
              const items = matches.filter(
                (chapter) => chapter.group === group.id,
              );
              if (!items.length) return null;
              return (
                <div className="manual-nav-group" key={group.id}>
                  <h2>{t(group.title)}</h2>
                  {items.map((chapter) => (
                    <a
                      key={chapter.id}
                      href={`#manual-${chapter.id}`}
                      aria-current={chapter.id === active ? "page" : undefined}
                    >
                      <span>{number(chapters.indexOf(chapter))}</span>
                      <span>{t(chapter.title)}</span>
                    </a>
                  ))}
                </div>
              );
            })}
            {!matches.length && (
              <div className="manual-no-results" role="status">
                <strong>{t("还没找到相关内容")}</strong>
                <p>{t("试试「字幕」「导出」这样的词，或清除搜索看看目录。")}</p>
              </div>
            )}
          </nav>
        </aside>
        <div className="manual-reading">
          <article
            className="manual-paper"
            id={`manual-${section.id}`}
            aria-labelledby="manual-chapter-title"
          >
            <header className="manual-chapter-heading">
              <div className="manual-chapter-copy">
                <h2 id="manual-chapter-title" ref={heading} tabIndex={-1}>
                  {t(section.title)}
                </h2>
                <p>{t(section.summary)}</p>
              </div>
              <div className="manual-chapter-mark" aria-hidden="true">
                <svg viewBox="0 0 160 144" fill="none">
                  <path
                    className="manual-frame-guide"
                    d="M24 24H136V120H24Z M8 44H0 M152 100H160 M80 0V8 M80 136V144"
                  />
                  <path
                    className="manual-frame-corners"
                    d="M8 32V8H32 M128 8H152V32 M152 112V136H128 M32 136H8V112"
                  />
                </svg>
                <span className="manual-chapter-index">{number(index)}</span>
                <span className="manual-chapter-total">
                  / {number(chapters.length - 1)}
                </span>
              </div>
            </header>
            <ManualArticle key={section.id} section={section} />
          </article>
          <nav className="manual-pagination" aria-label={t("章节导航")}>
            {index > 0 ? (
              <a href={`#manual-${chapters[index - 1].id}`}>
                <ArrowLeft size={17} aria-hidden="true" />
                <span>
                  <small>{t("上一节")}</small>
                  {t(chapters[index - 1].title)}
                </span>
              </a>
            ) : (
              <span />
            )}
            {index < chapters.length - 1 && (
              <a href={`#manual-${chapters[index + 1].id}`}>
                <span>
                  <small>{t("下一节")}</small>
                  {t(chapters[index + 1].title)}
                </span>
                <ArrowRight size={17} aria-hidden="true" />
              </a>
            )}
          </nav>
        </div>
      </div>
    </main>
  );
}
