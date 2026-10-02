import { useRef, useState, useEffect } from "react";
import { ArrowsClockwise, ArrowUpRight } from "@phosphor-icons/react";
import { version } from "../../package.json";
import { bridge, native } from "../bridge";
import { t, useLanguage } from "../i18n";
import { ActionButton } from "../ui/ActionButton";
import { checkForUpdate, RELEASE_PAGE, type UpdateResult } from "./check";
import "./updates.css";
export function AboutSettings() {
  useLanguage();
  const [result, setResult] = useState<UpdateResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [openFailed, setOpenFailed] = useState(false);
  const active = useRef(true);
  const checking = useRef(false);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  async function check() {
    if (checking.current) return;
    checking.current = true;
    setBusy(true);
    setFailed(false);
    setResult(null);
    try {
      const next = await checkForUpdate();
      if (active.current) setResult(next);
    } catch {
      if (active.current) setFailed(true);
    } finally {
      checking.current = false;
      if (active.current) setBusy(false);
    }
  }
  return (
    <section className="general-info">
      <h2>{t("关于 Mstudio")}</h2>
      <div className="about-version-row">
        <span className="about-version">
          {t("版本 {version}", { version })}
          {!native && <small>{t("浏览器预览")}</small>}
        </span>
        <ActionButton
          icon={ArrowsClockwise}
          disabled={busy}
          onClick={() => void check()}
        >
          {busy ? t("正在检查更新…") : t("检查更新")}
        </ActionButton>
      </div>
      <div className="update-result" role="status" aria-live="polite">
        {failed
          ? t("检查失败，请重试")
          : result?.status === "available"
            ? t("新版本 {version} 可用", { version: result.version })
            : result?.status === "current"
              ? t("暂无新版本")
              : result?.status === "unpublished"
                ? t("暂无可查询的正式版本")
                : ""}
      </div>
      {(failed || (result && result.status !== "current")) &&
        (native ? (
          <ActionButton
            icon={ArrowUpRight}
            onClick={() => {
              setOpenFailed(false);
              void bridge("open_release_page").catch(() => {
                if (active.current) setOpenFailed(true);
              });
            }}
          >
            {t("查看发布页面")}
          </ActionButton>
        ) : (
          <a
            className="action-button"
            href={RELEASE_PAGE}
            target="_blank"
            rel="noopener noreferrer"
          >
            <ArrowUpRight aria-hidden="true" />
            <span>{t("查看发布页面")}</span>
          </a>
        ))}
      {openFailed && (
        <p className="general-description" role="alert">
          {t("无法打开浏览器，请手动访问：")}{" "}
          <span className="release-url">{RELEASE_PAGE}</span>
        </p>
      )}
    </section>
  );
}
