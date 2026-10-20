import { Play, FilmStrip } from "@phosphor-icons/react";
import { useState } from "react";
import { t, useLanguage } from "../i18n";
import { browseWorks, type BrowseWork } from "./browseWorks";
import { BrowsePlayer } from "./BrowsePlayer";
import "./browse.css";

export function Browse({ works = browseWorks }: { works?: BrowseWork[] }) {
  useLanguage();
  const [watching, setWatching] = useState<BrowseWork | null>(null);
  return (
    <main className="library-main browse-page">
      <header className="library-heading">
        <div>
          <h1>{t("浏览")}</h1>
          <p className="browse-intro">
            {t("看一部好作品，发现下一次创作的灵感。")}
          </p>
        </div>
      </header>
      <section className="browse-gallery" aria-label={t("精选作品")}>
        {works.map((work) => (
          <article className="browse-work" key={work.id}>
            <button
              type="button"
              className="browse-cover"
              aria-label={t("观看完整作品 {title}", { title: work.title })}
              onClick={() => setWatching(work)}
            >
              <img src={work.poster} alt="" loading="lazy" />
              <span className="browse-play" aria-hidden="true">
                <Play weight="fill" size={24} />
              </span>
              <span className="browse-duration">
                {Math.floor(work.duration / 60)}:
                {String(work.duration % 60).padStart(2, "0")}
              </span>
            </button>
            <div className="browse-work-meta">
              <span className="browse-category">{t(work.category)}</span>
              <h2>
                <button onClick={() => setWatching(work)}>{work.title}</button>
              </h2>
              <p>{t(work.description)}</p>
              <span className="browse-credit">
                Blender Studio · {t("外部参考作品")}
              </span>
            </div>
          </article>
        ))}
      </section>
      {!works.length && (
        <div className="browse-empty" role="status">
          <FilmStrip size={32} aria-hidden="true" />
          <h2>{t("作品准备中")}</h2>
          <p>{t("这里会展示可以完整观看的视频作品。")}</p>
        </div>
      )}
      {!!works.length && (
        <p className="browse-source-note">
          {t("精选外部短片，非 mstudio 制作案例。在线播放需要网络连接。")}
        </p>
      )}
      {watching && (
        <BrowsePlayer
          key={watching.id}
          work={watching}
          close={() => setWatching(null)}
        />
      )}
    </main>
  );
}
