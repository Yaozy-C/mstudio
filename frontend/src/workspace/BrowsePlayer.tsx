import { Dialog } from "@radix-ui/themes";
import { X } from "@phosphor-icons/react";
import { useRef, useState } from "react";
import { t, useLanguage } from "../i18n";
import type { BrowseWork } from "./browseWorks";

export function BrowsePlayer({
  work,
  close,
}: {
  work: BrowseWork;
  close: () => void;
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
        <header>
          <Dialog.Title>{work.title}</Dialog.Title>
          <Dialog.Close>
            <button
              type="button"
              className="icon-button"
              aria-label={t("关闭播放器")}
            >
              <X aria-hidden="true" />
            </button>
          </Dialog.Close>
        </header>
        <iframe
          key={attempt}
          src={work.embed}
          title={t("完整作品 {title}", { title: work.title })}
          allow="autoplay; fullscreen; picture-in-picture"
          allowFullScreen
          referrerPolicy="strict-origin-when-cross-origin"
        />
        <div className="browse-player-help">
          <span>{t("若播放器未加载，请重试或查看作品来源。")}</span>
          <button onClick={() => setAttempt((v) => v + 1)}>
            {t("重新播放")}
          </button>
        </div>
        <footer>
          <p>{t(work.description)}</p>
          <div className="browse-attribution">
            <span>{work.credit}</span>
            <a href={work.source} target="_blank" rel="noreferrer">
              {t("作品来源")}
            </a>
          </div>
          <p className="browse-source-note">
            {t("AI 概念广告 · 通过发布方播放器观看，版权归原作者。")}
          </p>
        </footer>
      </Dialog.Content>
    </Dialog.Root>
  );
}
