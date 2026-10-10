import { useState } from "react";
import {
  Plus,
  X,
  MagnifyingGlass,
  ImageSquare,
  Check,
} from "@phosphor-icons/react";
import { t } from "../i18n";
import { mediaUrl } from "../bridge";
import type { Asset, Project, Reference } from "../model";
import { isSelectableAsset } from "../workspace/assetLibrary";
import "./shot-references.css";

function Thumbnail({ asset }: { asset?: Asset }) {
  if (!asset || asset.missing)
    return <ImageSquare size={24} aria-label={t("素材不可用")} />;
  if (asset.kind === "video")
    return (
      <video src={mediaUrl(asset.path)} preload="metadata" muted playsInline />
    );
  return (
    <img
      src={mediaUrl(asset.preview || asset.path)}
      alt={asset.name}
      loading="lazy"
    />
  );
}

export function ShotReferences({
  project,
  references,
  onChange,
}: {
  project: Project;
  references: Reference[];
  onChange: (refs: Reference[]) => void;
}) {
  const [picking, setPicking] = useState(false);
  const [query, setQuery] = useState("");
  const assets = project.assets.filter(
    (a) => ["image", "video"].includes(a.kind) && isSelectableAsset(a),
  );
  const matches = assets.filter((a) =>
    a.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  function add(asset: Asset) {
    if (references.some((r) => r.assetId === asset.id)) return;
    onChange([
      ...references,
      {
        assetId: asset.id,
        purpose: "",
        ...(asset.kind === "video" ? { start: 0, end: asset.duration } : {}),
      },
    ]);
  }
  return (
    <section className="shot-references" aria-label={t("镜头参考素材")}>
      <div className="shot-references-heading">
        <div>
          <h3>{t("镜头参考素材")}</h3>
          <span className="shot-reference-count">{references.length}</span>
        </div>
        <button
          className="shot-reference-add"
          type="button"
          aria-expanded={picking}
          onClick={() => setPicking(!picking)}
        >
          {picking ? <X size={15} /> : <Plus size={15} />}{" "}
          {picking ? t("完成选择") : t("添加参考素材")}
        </button>
      </div>
      {picking && (
        <div className="shot-reference-picker">
          <div className="shot-reference-search">
            <MagnifyingGlass size={16} />
            <input
              autoFocus
              aria-label={t("搜索参考素材")}
              placeholder={t("搜索项目素材…")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className="shot-reference-options">
            {matches.map((asset) => {
              const selected = references.some((r) => r.assetId === asset.id);
              return (
                <button
                  type="button"
                  className="shot-reference-option"
                  key={asset.id}
                  title={asset.name}
                  aria-label={`${t("添加参考素材")}：${asset.name}`}
                  aria-pressed={selected}
                  disabled={selected}
                  onClick={() => add(asset)}
                >
                  <span className="shot-reference-option-image">
                    <Thumbnail asset={asset} />
                    {selected && (
                      <span className="shot-reference-selected">
                        <Check size={14} weight="bold" />
                      </span>
                    )}
                  </span>
                  <span className="shot-reference-filename">{asset.name}</span>
                </button>
              );
            })}
          </div>
          {!matches.length && (
            <p className="shot-reference-empty-search">{t("没有匹配的素材")}</p>
          )}
        </div>
      )}
      {!references.length && (
        <div className="shot-reference-empty">
          <ImageSquare size={28} weight="light" />
          <span>{t("尚未关联参考素材")}</span>
          <p>{t("添加商品、人物或场景，让这个镜头保持一致。")}</p>
        </div>
      )}
      <div className="shot-reference-grid">
        {references.map((ref, index) => {
          const asset = project.assets.find((a) => a.id === ref.assetId);
          return (
            <div className="shot-reference-card" key={ref.assetId}>
              <div className="shot-reference-thumbnail">
                <Thumbnail asset={asset} />
                <span>{String(index + 1).padStart(2, "0")}</span>
              </div>
              <div className="shot-reference-detail">
                <div className="shot-reference-meta">
                  <span className="shot-reference-filename" title={asset?.name}>
                    {asset?.name ?? t("素材不可用")}
                  </span>
                  <button
                    className="shot-reference-remove"
                    type="button"
                    title={t("解除引用")}
                    aria-label={`${t("解除引用")}：${asset?.name ?? ref.assetId}`}
                    onClick={() =>
                      onChange(
                        references.filter((r) => r.assetId !== ref.assetId),
                      )
                    }
                  >
                    <X size={14} />
                  </button>
                </div>
                <label>
                  <span>{t("引用用途")}</span>
                  <input
                    aria-label={`${t("引用用途")}：${asset?.name ?? ref.assetId}`}
                    value={ref.purpose}
                    placeholder={t("例如：保持商品外观一致")}
                    maxLength={400}
                    onChange={(e) =>
                      onChange(
                        references.map((r) =>
                          r.assetId === ref.assetId
                            ? { ...r, purpose: e.target.value }
                            : r,
                        ),
                      )
                    }
                  />
                </label>
              </div>
            </div>
          );
        })}
      </div>
      {!!references.length && (
        <p className="shot-reference-hint">
          {t("仅关联到当前镜头 · 原素材保留在素材库")}
        </p>
      )}
    </section>
  );
}
