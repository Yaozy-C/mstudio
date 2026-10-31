import { useState } from "react";
import {
  FileText,
  FolderSimple,
  Globe,
  UploadSimple,
  Plus,
  Images,
  FilmStrip,
  MusicNotes,
  ImageSquare,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import type { Project, Asset } from "../model";
import { ObjectMenu } from "../ui/ObjectMenu";
import { ErrorNotice } from "../errors/ErrorNotice";
import { SuccessToast } from "../ui/SuccessToast";
import { MediaPreview } from "./MediaPreview";
import { MediaTile } from "./MediaTile";
import { RemoveLibraryAssetDialog } from "./RemoveLibraryAssetDialog";
import { uncollectAsset, setAssetUsage } from "./assetLibrary";
import { isPlanningMaterial } from "./resourceRoles";
import { useAssetLibrary } from "./useAssetLibrary";
import { mediaAssetActions } from "./mediaAssetActions";
import { native } from "../bridge";

export function MediaPanel({
  project,
  change,
  onImport,
  onReference,
  onPlace,
  onRemove,
  busy,
}: {
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  onImport: () => void;
  onReference: (a: Asset) => void;
  onPlace?: (a: Asset) => void;
  onRemove: (a: Asset) => void;
  busy: boolean;
}) {
  useLanguage();
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [filter, setFilter] = useState("all");
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<"project" | "global">("project");
  const [removing, setRemoving] = useState<Asset | null>(null);
  const library = useAssetLibrary(project.id, change);
  const disabled = busy || library.busy;
  const projectAssets = project.assets.filter((a) =>
    isPlanningMaterial(a, project),
  );
  const assets =
    scope === "project"
      ? projectAssets
      : library.assets.filter((a) => isPlanningMaterial(a));
  const upload = () =>
    scope === "project" ? onImport() : void library.upload();
  const groups = [
    { id: "all", label: t("全部素材"), icon: Images, assets },
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
  ];
  const active = groups.find((g) => g.id === filter) ?? groups[0];
  const matches = active.assets.filter((a) =>
    a.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  return (
    <aside className="media-panel">
      <header>
        <h3>
          {t("素材资料")} <span>{assets.length}</span>
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
      <p className="materials-purpose">
        {t("素材供创意和脚本使用；镜头制作请使用资产库。")}
      </p>
      <nav className="media-scopes" aria-label={t("素材范围")}>
        {(["project", "global"] as const).map((value) => (
          <button
            key={value}
            title={value === "project" ? t("项目素材") : t("公共素材")}
            aria-label={value === "project" ? t("项目素材") : t("公共素材")}
            aria-pressed={scope === value}
            disabled={disabled}
            onClick={() => {
              setScope(value);
              setSelected(null);
              setPreview(null);
            }}
          >
            {value === "project" ? (
              <>
                <FolderSimple size={20} />
                {t("项目资料")}
              </>
            ) : (
              <>
                <Globe size={20} />
                {t("公共资料")}
              </>
            )}
          </button>
        ))}
      </nav>
      <div className="media-toolbar">
        <button
          className="import-box"
          onClick={upload}
          disabled={disabled || !native}
          title={t("导入素材")}
        >
          <UploadSimple size={20} />
          {t("导入")}
        </button>
        <input
          type="search"
          aria-label={t("搜索素材资料")}
          placeholder={t("搜索素材资料")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>
      {library.error && !removing && (
        <ErrorNotice error={library.error} fallback="ASSET_OPERATION_FAILED">
          <button disabled={disabled} onClick={() => void library.retry()}>
            {t(library.retryLabel)}
          </button>
        </ErrorNotice>
      )}
      <SuccessToast
        message={library.message}
        onDismiss={library.dismissMessage}
      />
      <div className="media-section-label">
        {active.label} <span>{active.assets.length}</span>
      </div>
      <div className="media-list">
        {matches.map((a) => (
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
              onReference,
              onPlace,
              onUsage: (asset, usage) => {
                change((p) => setAssetUsage(p, asset.id, usage));
                if (usage === "asset")
                  window.dispatchEvent(new Event("studio-show-assets"));
              },
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
              referenceDrag={false}
            />
          </ObjectMenu>
        ))}
        {!matches.length && (
          <p className="media-empty">
            {scope === "global" && library.loading
              ? t("正在读取公共素材…")
              : query.trim()
                ? t("没有匹配的素材")
                : t("还没有素材资料，导入后用于创意和脚本。")}
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
            <small>{g.assets.length}</small>
          </button>
        ))}
      </nav>
      <MediaPreview
        asset={(scope === "project" ? project.assets : assets).find(
          (a) => a.id === preview,
        )}
        close={() => setPreview(null)}
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
          if (scope === "global" && !(await library.remove(removing))) return;
          if (scope === "project") onRemove(removing);
          setSelected(null);
          setPreview(null);
          setRemoving(null);
        }}
      />
    </aside>
  );
}
