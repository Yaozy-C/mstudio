import { ErrorNotice } from "../errors/ErrorNotice";
import type { FrameRole } from "../production/frameInputs";
import { useEffect, useState } from "react";
import { Popover } from "@radix-ui/themes";
import { UploadSimple, Image, VideoCamera, Check } from "@phosphor-icons/react";
import { bridge, mediaUrl } from "../bridge";
import type { Asset, Project } from "../model";
import type { ProductionController } from "../production/useProduction";
import type { AttachmentDraft } from "./useAttachments";
import "../styles/reference-picker.css";
export function ComposerReference({
  canvas,
  draft,
  project,
  role,
}: {
  role?: FrameRole;
  canvas: ProductionController;
  draft: AttachmentDraft;
  project: Project;
}) {
  const [open, setOpen] = useState(false);
  const [scope, setScope] = useState<"project" | "global">("project");
  const [globalAssets, setGlobalAssets] = useState<Asset[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const video = !role && canvas.composerMode !== "image";
  const label =
    role === "first-frame"
      ? "首帧"
      : role === "last-frame"
        ? "尾帧"
        : video
          ? "图片 / 视频"
          : "参考图";
  const current = project.assets.find(
    (a) => a.id === canvas.task?.inputs.find((r) => r.role === role)?.assetId,
  );
  useEffect(() => {
    if (!open || scope !== "global") return;
    let active = true;
    setLoading(true);
    setError("");
    bridge<Asset[]>("list_global_assets")
      .then(
        (items) => {
          if (active) setGlobalAssets(items);
        },
        (e) => {
          if (active) setError(String(e));
        },
      )
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [open, scope]);
  const assets = (scope === "project" ? project.assets : globalAssets).filter(
    (a) =>
      !a.missing &&
      (canvas.composerMode === "agent" ||
        a.kind === "image" ||
        (video && a.kind === "video")) &&
      a.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  const selected = new Set(
    canvas.task?.inputs.filter((r) => r.assetId).map((r) => r.assetId),
  );
  async function reference(asset: Asset) {
    setBusy(true);
    setError("");
    try {
      await canvas.referenceAsset(asset, scope === "global", role);
      if (role) setOpen(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger>
        <button
          type="button"
          className="composer-reference-tile"
          disabled={draft.busy}
          title={`${label}：引用素材或上传文件`}
          aria-label={`选择${label}`}
        >
          {current ? (
            <img
              src={mediaUrl(current.preview || current.path)}
              alt={current.name}
            />
          ) : (
            <Image size={16} />
          )}
          <span>{label}</span>
        </button>
      </Popover.Trigger>
      <Popover.Content side="top" align="start" className="reference-picker">
        <header>
          <strong>
            {role ? `选择${label}` : video ? "引用图片 / 视频" : "引用参考图"}
          </strong>
          <button
            type="button"
            aria-label="从电脑上传参考素材"
            title="从电脑上传"
            disabled={draft.busy || busy}
            onClick={() => {
              void draft.importFiles((refs) => {
                try {
                  if (role) {
                    if (refs.length !== 1)
                      throw new Error("每个帧槽位请选择一张图片");
                    canvas.attach(refs[0], role);
                  } else refs.forEach((ref) => canvas.attach(ref));
                  setOpen(false);
                } catch (e) {
                  setError(String(e).replace(/^Error: /, ""));
                }
              });
            }}
          >
            <UploadSimple size={18} />
          </button>
        </header>
        <div className="reference-picker-tabs">
          {(["project", "global"] as const).map((value) => (
            <button
              type="button"
              key={value}
              aria-pressed={scope === value}
              onClick={() => setScope(value)}
            >
              {value === "project" ? "项目素材" : "全局素材"}
            </button>
          ))}
        </div>
        <input
          type="search"
          aria-label="搜索参考素材"
          placeholder="搜索素材"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <div className="reference-picker-grid">
          {!loading &&
            assets.map((asset) => {
              const added = role
                ? current?.id === asset.id
                : selected.has(asset.id);
              return (
                <button
                  type="button"
                  key={asset.id}
                  title={asset.name}
                  aria-label={`${added ? "已引用" : "引用"} ${asset.name}`}
                  disabled={busy || added || (!role && selected.size >= 12)}
                  onClick={() => void reference(asset)}
                >
                  <div>
                    {asset.preview || asset.kind === "image" ? (
                      <img src={mediaUrl(asset.preview || asset.path)} alt="" />
                    ) : (
                      <VideoCamera size={28} />
                    )}
                    <span className="reference-kind">
                      {asset.kind === "video" ? (
                        <VideoCamera size={13} />
                      ) : (
                        <Image size={13} />
                      )}
                    </span>
                    {added && (
                      <span className="reference-added">
                        <Check size={15} />
                      </span>
                    )}
                  </div>
                  <span>{asset.name}</span>
                </button>
              );
            })}
        </div>
        {loading ? (
          <p className="reference-picker-empty">正在加载素材…</p>
        ) : (
          !assets.length && (
            <p className="reference-picker-empty">
              {query ? "没有匹配的素材" : "暂无可引用素材，可从电脑上传"}
            </p>
          )
        )}
        <footer>
          <span>
            {selected.size ? `已引用 ${selected.size} 项` : "点击素材即可引用"}
          </span>
          <Popover.Close>
            <button type="button">完成</button>
          </Popover.Close>
        </footer>
      </Popover.Content>
    </Popover.Root>
  );
}
