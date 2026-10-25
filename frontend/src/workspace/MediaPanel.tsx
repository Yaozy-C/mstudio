import { mediaAssetActions } from "./mediaAssetActions";
import { SuccessToast } from "../ui/SuccessToast";
import { PrevisLibrary } from "../previs/PrevisLibrary";
import { useOptionalPrevis } from "../previs/context";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import {
  FileText,
  FolderSimple,
  Globe,
  SquaresFour,
  UploadSimple,
  Plus,
  Images,
  FilmStrip,
  MusicNotes,
  ImageSquare,
  Cube,
} from "@phosphor-icons/react";
import { ObjectMenu } from "../ui/ObjectMenu";
import { MediaPreview } from "./MediaPreview";
import { MediaTile } from "./MediaTile";
import { EffectLibrary } from "./EffectLibrary";
import type { EffectPreset } from "./effects";
import { useState } from "react";
import { RemoveLibraryAssetDialog } from "./RemoveLibraryAssetDialog";
import { isLibraryAsset, uncollectAsset, setAssetUsage } from "./assetLibrary";
import { useAssetLibrary } from "./useAssetLibrary";
import { type Project, type Asset } from "../model";
export function MediaPanel({
  project,
  change,
  onImport,
  onAdd,
  onReference,
  onPlace,
  onRemove,
  onEffect,
  busy,
}: {
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  onImport: () => void;
  onAdd: (a: Asset) => void;
  onReference: (a: Asset) => void;
  onPlace: (a: Asset) => void;
  onRemove: (a: Asset) => void;
  onEffect: (effect: EffectPreset, asset?: Asset) => void;
  busy: boolean;
}) {
  useLanguage();
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [filter, setFilter] = useState("all");
  const [scope, setScope] = useState<"project" | "global" | "effects">(
    "project",
  );
  const [removing, setRemoving] = useState<Asset | null>(null);
  const library = useAssetLibrary(project.id, change);
  const previs = useOptionalPrevis();
  const modelCategory = scope === "project" && filter === "models";
  const disabled = busy || library.busy;
  const projectAssets = project.assets.filter(isLibraryAsset);
  const assets = scope === "project" ? projectAssets : library.assets;
  const upload = () =>
    modelCategory
      ? previs?.createModel()
      : scope === "project"
        ? onImport()
        : void library.upload();
  const groups = [
    {
      id: "reference",
      label: t("图片"),
      icon: ImageSquare,
      assets: assets.filter((a) => a.kind === "image"),
    },
    {
      id: "shots",
      label: t("视频"),
      icon: FilmStrip,
      assets: assets.filter((a) => a.kind === "video"),
    },
    {
      id: "audio",
      label: t("音频"),
      icon: MusicNotes,
      assets: assets.filter((a) => a.kind === "audio"),
    },
    {
      id: "documents",
      label: t("文档"),
      icon: FileText,
      assets: assets.filter((a) => a.kind === "text" || a.kind === "document"),
    },
    { id: "all", label: t("全部素材"), icon: Images, assets },
    ...(scope === "project"
      ? [{ id: "models", label: t("模型素材"), icon: Cube, assets: [] }]
      : []),
  ];
  const active =
    groups.find((g) => g.id === filter) ?? groups.find((g) => g.id === "all")!;
  return (
    <aside className="media-panel">
      <header hidden={scope === "effects"}>
        <h3>
          {t("素材库")}{" "}
          <span>
            {modelCategory
              ? (previs?.workspace.files.filter((f) => f.kind === "model")
                  .length ?? 0)
              : assets.length}
          </span>
        </h3>
        <button
          className="icon-button"
          onClick={upload}
          title={t("导入素材")}
          disabled={disabled}
        >
          <Plus />
        </button>
      </header>
      <nav className="media-scopes" aria-label={t("素材范围")}>
        {(["project", "global", "effects"] as const).map((value) => (
          <button
            key={value}
            title={t(
              value === "project"
                ? "项目素材"
                : value === "global"
                  ? "公共素材"
                  : "特效",
            )}
            aria-label={t(
              value === "project"
                ? "项目素材"
                : value === "global"
                  ? "公共素材"
                  : "特效",
            )}
            aria-pressed={scope === value}
            disabled={disabled}
            onClick={() => {
              setScope(value);
              if (value !== "effects") setSelected(null);
              setPreview(null);
            }}
          >
            {value === "project" ? (
              <FolderSimple size={20} />
            ) : value === "global" ? (
              <Globe size={20} />
            ) : (
              <SquaresFour size={20} />
            )}
          </button>
        ))}
      </nav>
      {scope === "effects" ? (
        <EffectLibrary
          onUse={(effect) =>
            onEffect(
              effect,
              projectAssets.find((a) => a.id === selected && !a.missing),
            )
          }
        />
      ) : (
        <>
          {!modelCategory && (
            <div className="media-toolbar">
              <button
                className="import-box"
                onClick={upload}
                disabled={disabled}
                title={
                  disabled
                    ? t("处理中…")
                    : scope === "project"
                      ? t("上传项目素材")
                      : t("上传公共素材")
                }
                aria-label={
                  scope === "project" ? t("上传项目素材") : t("上传公共素材")
                }
              >
                <UploadSimple size={20} />
                {t("导入")}
              </button>
            </div>
          )}
          {library.error && !removing && (
            <ErrorNotice
              error={library.error}
              fallback="ASSET_OPERATION_FAILED"
            >
              <button disabled={disabled} onClick={() => void library.retry()}>
                {library.retryLabel}
              </button>
            </ErrorNotice>
          )}
          <SuccessToast
            message={library.message}
            onDismiss={library.dismissMessage}
          />
          {!modelCategory && (
            <div className="media-section-label">
              {active.label} <span>{active.assets.length}</span>
            </div>
          )}
          <div
            className={`media-list${modelCategory ? " media-list-models" : ""}`}
          >
            {modelCategory && <PrevisLibrary />}
            {active.assets.map((a) => (
              <ObjectMenu
                key={a.id}
                actions={mediaAssetActions({
                  a,
                  scope,
                  disabled,
                  library,
                  projectAssets,
                  setPreview,
                  setRemoving,
                  onPlace,
                  onAdd,
                  onReference,
                  onUsage: (asset, usage) =>
                    change((p) => setAssetUsage(p, asset.id, usage)),
                  onUncollect: (asset) =>
                    change((p) => uncollectAsset(p, asset.id)),
                })}
              >
                <MediaTile
                  asset={a}
                  scope={scope}
                  selected={selected === a.id}
                  select={setSelected}
                  preview={setPreview}
                />
              </ObjectMenu>
            ))}
            {!modelCategory && !active.assets.length && (
              <p className="media-empty">
                {scope === "global" && library.loading
                  ? t("正在读取公共素材…")
                  : t("暂无{v0}", { v0: active.label })}
              </p>
            )}
          </div>
          <nav className="media-categories" aria-label={t("素材分类")}>
            {groups.map((g) => (
              <button
                key={g.id}
                aria-pressed={filter === g.id}
                onClick={() => setFilter(g.id)}
              >
                <g.icon />
                <span>{g.label}</span>
                <small>
                  {g.id === "models"
                    ? (previs?.workspace.files.filter((f) => f.kind === "model")
                        .length ?? 0)
                    : g.assets.length}
                </small>
              </button>
            ))}
          </nav>
          <MediaPreview
            asset={assets.find((a) => a.id === preview)}
            close={() => setPreview(null)}
            place={scope === "project" ? onPlace : undefined}
            add={scope === "project" ? onAdd : undefined}
            reference={scope === "project" ? onReference : undefined}
          />
          <RemoveLibraryAssetDialog
            removing={removing}
            scope={scope}
            disabled={disabled}
            error={library.error}
            onClose={() => setRemoving(null)}
            onConfirm={async () => {
              if (!removing) return;
              if (scope === "global" && !(await library.remove(removing)))
                return;
              if (scope === "project") onRemove(removing);
              setSelected(null);
              setPreview(null);
              setRemoving(null);
            }}
          />
        </>
      )}
    </aside>
  );
}
