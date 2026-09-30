import { useEffect, useRef, useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { ArrowUpRight, X } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { effects, effectCover, type EffectPreset } from "./effects";
import "./resource-library.css";

function EffectCard({
  effect,
  open,
}: {
  effect: EffectPreset;
  open: () => void;
}) {
  const video = useRef<HTMLVideoElement>(null);
  useEffect(() => {
    const element = video.current;
    if (!element) return;
    let visible = false;
    const syncPlayback = () => {
      if (visible && !document.hidden) {
        void element.play().catch(() => {});
      } else {
        element.pause();
      }
    };
    const observer = new IntersectionObserver(
      ([entry]) => {
        visible = entry.isIntersecting;
        syncPlayback();
      },
      { threshold: 0.1 },
    );
    observer.observe(element);
    document.addEventListener("visibilitychange", syncPlayback);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", syncPlayback);
      element.pause();
    };
  }, [effect.previewVideo]);
  return (
    <button
      className="effect-card"
      onClick={open}
      aria-label={t("查看特效：{name}", { name: t(effect.name) })}
    >
      {effect.previewVideo ? (
        <video
          ref={video}
          src={effect.previewVideo}
          poster={effectCover(effect)}
          muted
          loop
          playsInline
          preload="none"
        />
      ) : (
        <img
          src={effectCover(effect)}
          alt={t(effect.description)}
          loading="lazy"
        />
      )}
      <span className="effect-card-caption">
        <strong>{t(effect.name)}</strong>
        <ArrowUpRight size={16} />
      </span>
    </button>
  );
}

export function EffectLibrary({
  onUse,
}: {
  onUse: (effect: EffectPreset) => void;
}) {
  useLanguage();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<EffectPreset | null>(null);
  const [prompt, setPrompt] = useState("");
  const filtered = effects.filter((effect) =>
    `${t(effect.name)} ${t(effect.description)} ${t(effect.category)} ${effect.source.name}`
      .toLocaleLowerCase()
      .includes(query.trim().toLocaleLowerCase()),
  );
  return (
    <section className="effect-library" aria-label={t("特效库")}>
      <input
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t("搜索特效")}
        aria-label={t("搜索特效")}
      />
      <div className="effect-library-heading">
        <span>{t("全部特效")}</span>
        <span>{filtered.length}</span>
      </div>
      <div className="effect-grid">
        {filtered.map((effect) => (
          <EffectCard
            key={effect.id}
            effect={effect}
            open={() => {
              setSelected(effect);
              setPrompt(effect.prompt);
            }}
          />
        ))}
      </div>
      {!filtered.length && <p className="media-empty">{t("没有匹配的特效")}</p>}
      <Dialog.Root
        open={!!selected}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
      >
        <Dialog.Content
          className="effect-detail"
          maxWidth="820px"
          aria-describedby={undefined}
        >
          {selected && (
            <>
              <Dialog.Close>
                <button
                  className="effect-detail-close"
                  aria-label={t("关闭特效预览")}
                >
                  <X size={20} />
                </button>
              </Dialog.Close>
              <div className="effect-detail-visual">
                {selected.previewVideo ? (
                  <video
                    src={selected.previewVideo}
                    poster={effectCover(selected)}
                    controls
                    autoPlay
                    loop
                    muted
                    playsInline
                  />
                ) : (
                  <img
                    src={effectCover(selected)}
                    alt={t(selected.description)}
                  />
                )}
                {!selected.previewVideo && <span>{t("静态效果图")}</span>}
              </div>
              <div className="effect-detail-body">
                <span className="effect-detail-eyebrow">{t("特效")}</span>
                <Dialog.Title>{t(selected.name)}</Dialog.Title>
                <p>{t(selected.description)}</p>
                <label className="effect-prompt-label">
                  {t("效果 Prompt")}
                  <textarea
                    value={prompt}
                    onChange={(e) => setPrompt(e.target.value)}
                    aria-label={t("效果 Prompt")}
                  />
                </label>
                <div className="effect-detail-action">
                  <button
                    className="primary"
                    disabled={!prompt.trim()}
                    onClick={() => {
                      onUse({ ...selected, prompt: prompt.trim() });
                      setSelected(null);
                    }}
                  >
                    {t("交给 Agent 使用")}
                    <ArrowUpRight size={17} />
                  </button>
                </div>
              </div>
            </>
          )}
        </Dialog.Content>
      </Dialog.Root>
    </section>
  );
}
