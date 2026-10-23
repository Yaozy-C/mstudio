import { Dialog } from "@radix-ui/themes";
import {
  ArrowClockwise,
  ArrowRight,
  ArrowUpRight,
  CaretDown,
  FilmStrip,
  X,
} from "@phosphor-icons/react";
import { useRef, useState } from "react";
import { t, useLanguage } from "../i18n";
import type { BrowseWork } from "./browseWorks";
import "./browse-player.css";

export function BrowsePlayer({
  work,
  close,
  onAnalyze,
}: {
  work: BrowseWork;
  close: () => void;
  onAnalyze?: (work: BrowseWork) => void;
}) {
  useLanguage();
  const [attempt, setAttempt] = useState(0);
  const opener = useRef(
    typeof document === "undefined"
      ? null
      : (document.activeElement as HTMLElement | null),
  );
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <Dialog.Content
        className="browse-player"
        maxWidth="1120px"
        aria-describedby={undefined}
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          if (opener.current?.isConnected) opener.current.focus();
        }}
      >
        <header className="browse-player-heading">
          <div className="browse-player-title">
            <Dialog.Title>{work.title}</Dialog.Title>
            <span className="browse-category">{t(work.category)}</span>
          </div>
          <Dialog.Close>
            <button
              type="button"
              className="icon-button browse-player-close"
              aria-label={t("关闭播放器")}
            >
              <X aria-hidden="true" />
            </button>
          </Dialog.Close>
        </header>
        <div className="browse-player-stage">
          <iframe
            key={attempt}
            src={work.embed}
            title={t("完整作品 {title}", { title: work.title })}
            allow="autoplay; fullscreen; picture-in-picture"
            allowFullScreen
            referrerPolicy="strict-origin-when-cross-origin"
          />
        </div>
        <footer className="browse-player-info">
          <div className="browse-player-overview">
            <div className="browse-player-description">
              <p>{t(work.description)}</p>
              <div className="browse-attribution">
                <span>{work.credit}</span>
                <a href={work.source} target="_blank" rel="noreferrer">
                  {t("作品来源")}
                  <ArrowUpRight size={13} aria-hidden="true" />
                </a>
              </div>
            </div>
            {onAnalyze && (
              <button
                type="button"
                className="primary browse-player-analyze"
                onClick={() => {
                  close();
                  onAnalyze(work);
                }}
              >
                <FilmStrip size={18} aria-hidden="true" />
                {t("拆解这部作品")}
                <ArrowRight size={16} aria-hidden="true" />
              </button>
            )}
          </div>
          <div className="browse-player-utilities">
            <p className="browse-source-note">
              {t("AI 概念广告 · 版权归原作者")}
            </p>
            <details className="browse-player-help">
              <summary>
                {t("播放遇到问题？")}
                <CaretDown size={12} aria-hidden="true" />
              </summary>
              <div className="browse-player-help-content">
                <p>{t("若播放器未加载，请重试或查看作品来源。")}</p>
                <button type="button" onClick={() => setAttempt((v) => v + 1)}>
                  <ArrowClockwise size={14} aria-hidden="true" />
                  {t("重新播放")}
                </button>
              </div>
            </details>
          </div>
        </footer>
      </Dialog.Content>
    </Dialog.Root>
  );
}
