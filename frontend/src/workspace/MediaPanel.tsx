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
import { mediaUrl } from "../bridge";
import { useState } from "react";
import { AlertDialog } from "@radix-ui/themes";
import { isLibraryAsset } from "./assetLibrary";
import { useAssetLibrary } from "./useAssetLibrary";
import { MissingAsset } from "./MissingAsset";
import { formatTime, type Project, type Asset } from "../model";
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
      label: "图片",
      icon: ImageSquare,
      assets: assets.filter((a) => a.kind === "image"),
    },
    {
      id: "shots",
      label: "视频",
      icon: FilmStrip,
      assets: assets.filter((a) => a.kind === "video"),
    },
    {
      id: "audio",
      label: "音频",
      icon: MusicNotes,
      assets: assets.filter((a) => a.kind === "audio"),
    },
    {
      id: "documents",
      label: "文档",
      icon: FileText,
      assets: assets.filter((a) => a.kind === "text" || a.kind === "document"),
    },
    { id: "all", label: "全部素材", icon: Images, assets },
  ];
  const active = groups.find((g) => g.id === filter)!;
  return (
    <aside className="media-panel">
      <header>
        <h3>
          素材库 <span>{assets.length}</span>
        </h3>
        <button
          className="icon-button"
          onClick={upload}
          title="导入素材"
          disabled={disabled}
        >
          <Plus />
        </button>
      </header>
      <nav className="media-scopes" aria-label="素材范围">
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
            {value === "project" ? "项目素材" : "全局素材"}
          </button>
        ))}
      </nav>
      <button className="import-box" onClick={upload} disabled={disabled}>
        <Plus size={18} />
        <strong>
          {disabled
            ? "处理中…"
            : scope === "project"
              ? "上传项目素材"
              : "上传全局素材"}
        </strong>
        <small>图片、文本、视频、音频、PDF</small>
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
              { label: "预览素材", run: () => setPreview(a.id) },
              ...(scope === "project"
                ? [
                    {
                      label: "放到画布",
                      run: () => onPlace(a),
                      disabled: a.missing,
                    },
                    {
                      label: "加入时间线",
                      run: () => onAdd(a),
                      disabled:
                        a.missing || a.kind === "text" || a.kind === "document",
                    },
                    {
                      label: "引用到对话",
                      run: () => onReference(a),
                      disabled: a.missing,
                    },
                    {
                      label: "添加到全局素材库",
                      run: () => void library.promote(a),
                      disabled:
                        disabled || library.assets.some((g) => g.id === a.id),
                    },
                  ]
                : [
                    {
                      label: "加入项目素材",
                      run: () => void library.use(a),
                      disabled:
                        disabled || projectAssets.some((p) => p.id === a.id),
                    },
                  ]),
              {
                label:
                  scope === "project" ? "从项目素材移除" : "从全局素材移除",
                danger: true,
                disabled,
                run: () => setRemoving(a),
              },
            ]}
          >
            <article
              className="media-item"
              onContextMenu={() => setSelected(a.id)}
            >
              <button
                className="media-thumbnail"
                draggable={scope === "project" && !a.missing}
                onDragStart={(e) => {
                  if (scope !== "project" || a.missing) {
                    e.preventDefault();
                    return;
                  }
                  e.dataTransfer.setData(
                    "application/x-mstudio-reference",
                    JSON.stringify({ kind: "asset", id: a.id }),
                  );
                  e.dataTransfer.effectAllowed = "copy";
                }}
                onClick={() => setSelected(a.id)}
                onDoubleClick={() => setPreview(a.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    setPreview(a.id);
                  }
                }}
                aria-pressed={selected === a.id}
                title="双击预览 · 右键更多操作"
              >
                {a.missing ? (
                  <MissingAsset asset={a} compact />
                ) : a.preview ? (
                  <img loading="lazy" src={mediaUrl(a.preview)} alt={a.name} />
                ) : a.kind === "audio" ? (
                  <MusicNotes size={28} />
                ) : (
                  <FileText size={28} />
                )}
                {!a.missing && (
                  <span>
                    {a.kind === "image"
                      ? "图片"
                      : a.kind === "text"
                        ? "文本"
                        : a.kind === "document"
                          ? "PDF"
                          : formatTime(a.duration)}
                  </span>
                )}
              </button>
              <div className="media-caption">
                {a.kind === "video" ? (
                  <FilmStrip />
                ) : a.kind === "image" ? (
                  <ImageSquare />
                ) : a.kind === "audio" ? (
                  <MusicNotes />
                ) : (
                  <FileText />
                )}
                <span title={a.name}>{a.name}</span>
              </div>
            </article>
          </ObjectMenu>
        ))}
        {!active.assets.length && (
          <p className="media-empty">
            {scope === "global" && library.loading
              ? "正在读取全局素材…"
              : `暂无${active.label}，可上传素材或切换分类。`}
          </p>
        )}
      </div>
      <nav className="media-categories" aria-label="素材分类">
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
            移除{scope === "project" ? "项目" : "全局"}素材
          </AlertDialog.Title>
          <AlertDialog.Description>
            将「{removing?.name}」移出{scope === "project" ? "项目" : "全局"}
            素材库？已用于分镜、时间线和对话的内容会保留
            {scope === "project"
              ? "，全局素材不受影响"
              : "，其他项目中已选用的素材不受影响"}
            。
          </AlertDialog.Description>
          {library.error && (
            <ErrorNotice error={library.error} fallback="OPERATION_FAILED" />
          )}
          <footer>
            <AlertDialog.Cancel>
              <button disabled={disabled}>取消</button>
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
              {disabled ? "正在移除…" : "确认移除"}
            </button>
          </footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </aside>
  );
}
