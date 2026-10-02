import { useEffect, useRef } from "react";
import { ArrowLeft } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { manualSections } from "./sections";
import "./manual.css";

export function UserManual({ onBack }: { onBack: () => void }) {
  useLanguage();
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }, []);
  return (
    <main className="library-main manual-main">
      <header className="library-heading manual-heading">
        <div>
          <h1 ref={heading} tabIndex={-1}>
            {t("使用手册")}
          </h1>
          <p>{t("从想法到成片，一步步完成你的创作。")}</p>
        </div>
        <button onClick={onBack}>
          <ArrowLeft />
          {t("返回项目空间")}
        </button>
      </header>
      <div className="manual-layout">
        <nav className="manual-contents" aria-label={t("手册目录")}>
          <h2>{t("手册目录")}</h2>
          {manualSections.map((section, index) => (
            <a key={section.id} href={`#manual-${section.id}`}>
              <span>{String(index + 1).padStart(2, "0")}</span>
              {t(section.title)}
            </a>
          ))}
          <a href="#manual-shortcuts">
            <span>09</span>
            {t("常用操作速查")}
          </a>
        </nav>
        <div className="manual-articles">
          {manualSections.map((section, index) => (
            <section
              className="manual-section"
              key={section.id}
              id={`manual-${section.id}`}
              aria-labelledby={`manual-title-${section.id}`}
              tabIndex={-1}
            >
              <span className="manual-number" aria-hidden="true">
                {String(index + 1).padStart(2, "0")}
              </span>
              <h2 id={`manual-title-${section.id}`}>{t(section.title)}</h2>
              <p className="manual-summary">{t(section.summary)}</p>
              <ol>
                {section.steps.map((step) => (
                  <li key={step.title}>
                    <h3>{t(step.title)}</h3>
                    <p>{t(step.body)}</p>
                  </li>
                ))}
              </ol>
              <aside className="manual-note">{t(section.tip)}</aside>
            </section>
          ))}
          <section
            className="manual-section"
            id="manual-shortcuts"
            aria-labelledby="manual-shortcuts-title"
            tabIndex={-1}
          >
            <span className="manual-number" aria-hidden="true">
              09
            </span>
            <h2 id="manual-shortcuts-title">{t("常用操作速查")}</h2>
            <p className="manual-summary">
              {t(
                "在时间线选中片段后使用快捷键；输入文字时保留文字编辑快捷键。",
              )}
            </p>
            <div className="manual-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th scope="col">{t("操作")}</th>
                    <th scope="col">macOS</th>
                    <th scope="col">Windows</th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    [t("播放 / 暂停"), "Space", "Space"],
                    [t("分割片段"), "⌘B", "Ctrl+B"],
                    [t("复制片段"), "⌘C", "Ctrl+C"],
                    [t("剪切片段"), "⌘X", "Ctrl+X"],
                    [t("粘贴到播放头"), "⌘V", "Ctrl+V"],
                    [t("裁掉播放头之前"), "⌥[", "Alt+["],
                    [t("裁掉播放头之后"), "⌥]", "Alt+]"],
                    [t("撤销 / 重做"), "⌘Z / ⇧⌘Z", "Ctrl+Z / Ctrl+Shift+Z"],
                    [t("逐帧移动"), "← / →", "← / →"],
                  ].map(([label, mac, windows]) => (
                    <tr key={label}>
                      <th scope="row">{label}</th>
                      <td>
                        <kbd>{mac}</kbd>
                      </td>
                      <td>
                        <kbd>{windows}</kbd>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </div>
      </div>
    </main>
  );
}
