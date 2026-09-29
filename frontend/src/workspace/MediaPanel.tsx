import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import {
  FileText,
  Plus,
  Images,
  FilmStrip,
  MusicNotes,
  ImageSquare,
} from "@phosphor-icons/react";
import { ObjectMenu } from "../ui/ObjectMenu";
import { MediaPreview } from "./MediaPreview";
import { MediaTile } from "./MediaTile";
import { useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { isLibraryAsset } from "./assetLibrary";
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
  busy,
}: {
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  onImport: () => void;
  onAdd: (a: Asset) => void;
  onReference: (a: Asset) => void;
  onPlace: (a: Asset) => void;
  onRemove: (a: Asset) => void;
  busy: boolean;
}) {
  useLanguage();
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [filter, setFilter] = useState("all");
  const [scope, setScope] = useState<"project" | "global">("project");
  const [removing, setRemoving] = useState<Asset | null>(null);
  const library = useAssetLibrary(project.id, change);
  const disabled = busy || library.busy;
  const projectAssets = project.assets.filter(isLibraryAsset);
  const assets = scope === "project" ? projectAssets : library.assets;
  const upload = () =>
    scope === "project" ? onImport() : void library.upload();
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
  ];
  const active = groups.find((g) => g.id === filter)!;
  return (
    <aside className="media-panel">
      <header>
        <h3>
          {t("素材库")} <span>{assets.length}</span>
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
        {(["project", "global"] as const).map((value) => (
          <button
            key={value}
            aria-pressed={scope === value}
            disabled={disabled}
            onClick={() => {
              setScope(value);
              setSelected(null);
              setPreview(null);
            }}
          >
            {value === "project" ? t("项目素材") : t("全局素材")}
          </button>
        ))}
      </nav>
      <button className="import-box" onClick={upload} disabled={disabled}>
        <Plus size={18} />
        <strong>
          {disabled
            ? t("处理中…")
            : scope === "project"
              ? t("上传项目素材")
              : t("上传全局素材")}
        </strong>
        <small>{t("图片、文本、视频、音频、PDF")}</small>
      </button>
      {library.error && !removing && (
        <ErrorNotice error={library.error} fallback="ASSET_OPERATION_FAILED">
          <button disabled={disabled} onClick={() => void library.retry()}>
            {library.retryLabel}
          </button>
        </ErrorNotice>
      )}
      {library.message && (
        <p className="media-scope-hint" role="status">
          {library.message}
        </p>
      )}
      <div className="media-section-label">
        {active.label} <span>{active.assets.length}</span>
      </div>
      <div className="media-list">
        {active.assets.map((a) => (
          <ObjectMenu
            key={a.id}
            actions={[
              { label: t("预览素材"), run: () => setPreview(a.id) },
              ...(scope === "project"
                ? [
                    {
                      label: t("放到画布"),
                      run: () => onPlace(a),
                      disabled: a.missing,
                    },
                    {
                      label: t("加入时间线"),
                      run: () => onAdd(a),
                      disabled:
                        a.missing || a.kind === "text" || a.kind === "document",
                    },
                    {
                      label: t("引用到对话"),
                      run: () => onReference(a),
                      disabled: a.missing,
                    },
                    {
                      label: t("添加到全局素材库"),
                      run: () => void library.promote(a),
                      disabled:
                        disabled || library.assets.some((g) => g.id === a.id),
                    },
                  ]
                : [
                    {
                      label: t("加入项目素材"),
                      run: () => void library.use(a),
                      disabled:
                        disabled || projectAssets.some((p) => p.id === a.id),
                    },
                  ]),
              {
                label:
                  scope === "project"
                    ? t("从项目素材移除")
                    : t("从全局素材移除"),
                danger: true,
                disabled,
                run: () => setRemoving(a),
              },
            ]}
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
        {!active.assets.length && (
          <p className="media-empty">
            {scope === "global" && library.loading
              ? t("正在读取全局素材…")
              : t("暂无{v0}，可上传素材或切换分类。", { v0: active.label })}
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
        asset={assets.find((a) => a.id === preview)}
        close={() => setPreview(null)}
        place={scope === "project" ? onPlace : undefined}
        add={scope === "project" ? onAdd : undefined}
        reference={scope === "project" ? onReference : undefined}
      />
      <AlertDialog.Root
        open={!!removing}
        onOpenChange={(open) => {
          if (!open && !disabled) setRemoving(null);
        }}
      >
        <AlertDialog.Content className="modal small" aria-busy={disabled}>
          <AlertDialog.Title>
            {t("移除")}
            {scope === "project" ? t("项目") : t("全局")}
            {t("素材")}
          </AlertDialog.Title>
          <AlertDialog.Description>
            {t("将「")}
            {removing?.name}
            {t("」移出")}
            {scope === "project" ? t("项目") : t("全局")}
            {t("素材库？已用于分镜、时间线和对话的内容会保留")}
            {scope === "project"
              ? t("，全局素材不受影响")
              : t("，其他项目中已选用的素材不受影响")}
            。
          </AlertDialog.Description>
          {library.error && (
            <ErrorNotice error={library.error} fallback="OPERATION_FAILED" />
          )}
          <footer>
            <AlertDialog.Cancel>
              <button disabled={disabled}>{t("取消")}</button>
            </AlertDialog.Cancel>
            <button
              className="primary"
              disabled={disabled}
              onClick={async () => {
                if (!removing) return;
                if (scope === "global" && !(await library.remove(removing)))
                  return;
                if (scope === "project") onRemove(removing);
                setSelected(null);
                setPreview(null);
                setRemoving(null);
              }}
            >
              {disabled ? t("正在移除…") : t("确认移除")}
            </button>
          </footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </aside>
  );
}
