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
  const [failed, setFailed] = useState(false);
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
        <video
          key={attempt}
          src={work.video}
          poster={work.poster}
          controls
          autoPlay
          playsInline
          preload="metadata"
          aria-label={t("完整作品 {title}", { title: work.title })}
          onError={() => setFailed(true)}
        />
        {failed && (
          <div className="browse-playback-error" role="alert">
            <p>{t("视频暂时无法播放，请检查网络连接后重试。")}</p>
            <button
              onClick={() => {
                setFailed(false);
                setAttempt((v) => v + 1);
              }}
            >
              {t("重新播放")}
            </button>
          </div>
        )}
        <footer>
          <p>{t(work.description)}</p>
          <div className="browse-attribution">
            <span>{work.credit}</span>
            <a href={work.source} target="_blank" rel="noreferrer">
              {t("作品来源")}
            </a>
            <a href={work.licenseSource} target="_blank" rel="noreferrer">
              {t("授权说明")}
            </a>
            <a
              href="https://creativecommons.org/licenses/by/4.0/"
              target="_blank"
              rel="noreferrer"
            >
              CC BY 4.0
            </a>
          </div>
          <p className="browse-source-note">
            {t("外部参考作品 · 完整原片，未经剪辑。")}
          </p>
        </footer>
      </Dialog.Content>
    </Dialog.Root>
  );
}
