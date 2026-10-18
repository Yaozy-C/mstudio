import {
  ArrowUpRight,
  ArrowRight,
  FilmStrip,
  PencilLine,
  Plus,
} from "@phosphor-icons/react";
import { t } from "../i18n";
import type { ManualSection } from "./sections";
import { manualShortcuts } from "./shortcuts";

function Shortcuts() {
  return (
    <>
      <p>
        {t(
          "剪辑前先选中片段，在时间尺上点一下，把播放头放到想操作的位置。正在输入文字时，这些按键仍用于文字编辑。",
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
            {manualShortcuts.map(([label, mac, windows]) => (
              <tr key={label}>
                <th scope="row">{t(label)}</th>
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
    </>
  );
}

export function ManualArticle({ section }: { section: ManualSection }) {
  return (
    <div className="manual-body">
      {section.id === "start" && (
        <nav className="manual-routes" aria-label={t("选择你的起点")}>
          <a href="#manual-start-steps">
            <FilmStrip size={20} aria-hidden="true" />
            <span>
              <strong>{t("我有现成素材")}</strong>
              <small>{t("把图片和视频剪成作品")}</small>
            </span>
            <ArrowUpRight size={17} aria-hidden="true" />
          </a>
          <a href="#manual-models">
            <PencilLine size={20} aria-hidden="true" />
            <span>
              <strong>{t("我有一个想法")}</strong>
              <small>{t("和 AI 一起写脚本、准备画面")}</small>
            </span>
            <ArrowUpRight size={17} aria-hidden="true" />
          </a>
        </nav>
      )}
      {section.id === "shortcuts" ? (
        <Shortcuts />
      ) : section.kind === "faq" ? (
        <div className="manual-faq">
          {section.steps.map((step, index) => (
            <details key={step.title}>
              <summary>
                <span className="manual-faq-index" aria-hidden="true">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span>{t(step.title)}</span>
                <Plus size={17} aria-hidden="true" />
              </summary>
              <p>{t(step.body)}</p>
            </details>
          ))}
        </div>
      ) : section.kind === "reference" ? (
        <dl className="manual-definitions">
          {section.steps.map((step) => (
            <div key={step.title}>
              <dt>{t(step.title)}</dt>
              <dd>{t(step.body)}</dd>
            </div>
          ))}
        </dl>
      ) : (
        <ol
          role="list"
          className="manual-steps"
          id={section.id === "start" ? "manual-start-steps" : undefined}
        >
          {section.steps.map((step) => (
            <li key={step.title}>
              <h3>{t(step.title)}</h3>
              <p>{t(step.body)}</p>
            </li>
          ))}
        </ol>
      )}
      {section.example && (
        <section className="manual-example">
          <h3>{t("可以这样说")}</h3>
          <blockquote>{t(section.example)}</blockquote>
        </section>
      )}
      {section.outcome && (
        <section className="manual-outcome">
          <ArrowRight size={18} aria-hidden="true" />
          <div>
            <h3>{t("完成后")}</h3>
            <p>{t(section.outcome)}</p>
          </div>
        </section>
      )}
      {section.tip && <aside className="manual-note">{t(section.tip)}</aside>}
    </div>
  );
}
