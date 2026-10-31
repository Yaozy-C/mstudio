import { referenceAsset } from "../previs/media";
import { useState } from "react";
import {
  Plus,
  X,
  MagnifyingGlass,
  ImageSquare,
  Check,
  MusicNote,
} from "@phosphor-icons/react";
import { t } from "../i18n";
import { mediaUrl } from "../bridge";
import type { Asset, Project, Reference } from "../model";
import { isShotAsset } from "../workspace/resourceRoles";
import "./shot-references.css";

function Thumbnail({ asset }: { asset?: Asset }) {
  if (!asset || asset.missing)
    return <ImageSquare size={24} aria-label={t("素材不可用")} />;
  if (asset.kind === "audio")
    return <MusicNote size={24} aria-label={asset.name} />;
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
  const assets = project.assets.filter((a) => isShotAsset(a));
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
    <section className="shot-references" aria-label={t("镜头使用的资产")}>
      <div className="shot-references-heading">
        <div>
          <h3>{t("镜头使用的资产")}</h3>
          <span className="shot-reference-count">{references.length}</span>
        </div>
        <button
          className="shot-reference-add"
          type="button"
          aria-expanded={picking}
          onClick={() => setPicking(!picking)}
        >
          {picking ? <X size={15} /> : <Plus size={15} />}{" "}
          {picking ? t("完成选择") : t("关联资产")}
        </button>
      </div>
      {picking && (
        <div className="shot-reference-picker">
          <div className="shot-reference-search">
            <MagnifyingGlass size={16} />
            <input
              autoFocus
              aria-label={t("搜索资产")}
              placeholder={t("搜索已确认资产")}
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
                  aria-label={`${t("关联资产")}：${asset.name}`}
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
            <p className="shot-reference-empty-search">
              {t("没有可用资产，请先在资产库建立并确认。")}
            </p>
          )}
        </div>
      )}
      {!references.length && (
        <div className="shot-reference-empty">
          <ImageSquare size={28} weight="light" />
          <span>{t("尚未关联资产")}</span>
          <p>{t("添加商品、人物或场景，让这个镜头保持一致。")}</p>
        </div>
      )}
      <div className="shot-reference-grid">
        {references.map((ref, index) => {
          const asset = referenceAsset(project, ref.assetId);
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
          {t("资产在镜头间复用；素材资料不直接参与镜头制作。")}
        </p>
      )}
    </section>
  );
}
