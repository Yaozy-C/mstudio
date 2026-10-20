import {
  ArrowRight,
  FilmStrip,
  Sparkle,
  VideoCamera,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import "./creative-home.css";

export function CreativeHomeStart({
  onReference,
  onCreate,
}: {
  onReference: () => void;
  onCreate: () => void;
}) {
  useLanguage();
  return (
    <>
      <header className="home-intro">
        <p className="home-eyebrow">{t("创作首页")}</p>
        <h1>{t("今天，你想创作什么？")}</h1>
        <p>{t("从一个灵感、一段参考视频，或自己的素材开始。")}</p>
      </header>
      <section className="home-starts" aria-label={t("开始创作")}>
        <button type="button" className="home-start" onClick={onReference}>
          <VideoCamera
            className="home-start-icon"
            size={26}
            aria-hidden="true"
          />
          <span className="home-start-copy">
            <strong>{t("探索参考作品")}</strong>
            <span>{t("理解喜欢的作品，创作自己的版本。")}</span>
            <span className="home-start-detail">
              {t("添加视频 · 分析声画与节奏")}
            </span>
          </span>
          <ArrowRight className="home-start-arrow" aria-hidden="true" />
        </button>
        <button type="button" className="home-start" onClick={onCreate}>
          <Sparkle className="home-start-icon" size={26} aria-hidden="true" />
          <span className="home-start-copy">
            <strong>{t("创作全新作品")}</strong>
            <span>{t("从素材或想法开始一部新作品。")}</span>
            <span className="home-start-detail">
              {t("构思脚本 · 制作镜头 · 剪辑成片")}
            </span>
          </span>
          <ArrowRight className="home-start-arrow" aria-hidden="true" />
        </button>
      </section>
      <section
        className="home-inspiration"
        aria-labelledby="home-inspiration-title"
      >
        <header className="home-section-heading">
          <h2 id="home-inspiration-title">{t("发现灵感")}</h2>
          <span>{t("看作品，也看创作方法")}</span>
        </header>
        <div className="home-inspiration-empty">
          <div className="home-inspiration-mark" aria-hidden="true">
            <FilmStrip size={32} />
          </div>
          <div className="home-inspiration-copy">
            <span className="home-eyebrow">{t("暂无精选视频")}</span>
            <h3>{t("好作品，值得从头看到尾")}</h3>
            <p>
              {t(
                "这里还没有完整的示例作品。带来一支你喜欢的视频，探索它的故事、声画与节奏。",
              )}
            </p>
          </div>
          <button type="button" onClick={onReference}>
            {t("添加自己的参考视频")}
            <ArrowRight aria-hidden="true" />
          </button>
        </div>
      </section>
    </>
  );
}
