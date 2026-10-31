import { StatusMessage } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { isShotAsset, canUseTaskReference } from "../workspace/resourceRoles";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { FrameRole } from "../production/frameInputs";
import { useEffect, useState } from "react";
import { Popover } from "@radix-ui/themes";
import {
  FolderSimple,
  Image,
  VideoCamera,
  Check,
  FileText,
} from "@phosphor-icons/react";
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
  useLanguage();
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
      ? t("首帧")
      : role === "last-frame"
        ? t("尾帧")
        : t("资产");
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
      (scope === "project"
        ? canUseTaskReference(project, a, canvas.task!)
        : isShotAsset(a)) &&
      (a.kind === "image" ||
        (!role &&
          (canvas.composerMode === "agent" ||
            (canvas.composerMode !== "reference" &&
              (a.kind === "text" || a.kind === "document")) ||
            (video && a.kind === "video")))) &&
      a.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  const nodes =
    !role && canvas.composerMode !== "reference" && scope === "project"
      ? project.nodes.filter(
          (n) =>
            ["note", "text", "shot", "screenplay"].includes(n.kind) &&
            n.title.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
        )
      : [];
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
          title={t("{v0}：选择已确认资产或制作结果", { v0: label })}
          aria-label={t("选择{v0}", { v0: label })}
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
            {role ? t("选择{v0}", { v0: label }) : t("资产与制作结果")}
          </strong>
          <button
            type="button"
            aria-label={t("管理镜头资产")}
            title={t("管理镜头资产")}
            disabled={draft.busy || busy}
            onClick={() => {
              setOpen(false);
              window.dispatchEvent(new Event("studio-show-assets"));
            }}
          >
            <FolderSimple size={18} />
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
              {value === "project" ? t("项目资产与结果") : t("公共资产")}
            </button>
          ))}
        </div>
        <input
          type="search"
          aria-label={t("搜索生成输入")}
          placeholder={t("搜索生成输入")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <div className="reference-picker-grid">
          {nodes.map((node) => {
            const added = canvas.task?.inputs.some((r) => r.nodeId === node.id);
            return (
              <button
                type="button"
                key={`node:${node.id}`}
                title={node.title}
                aria-label={`${added ? t("已引用") : t("引用")} ${node.title}`}
                disabled={busy || added}
                onClick={() => canvas.attach({ kind: "node", id: node.id })}
              >
                <div>
                  <FileText size={28} />
                  {added && (
                    <span className="reference-added">
                      <Check size={15} />
                    </span>
                  )}
                </div>
                <span>{node.title}</span>
              </button>
            );
          })}
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
                  aria-label={`${added ? t("已引用") : t("引用")} ${asset.name}`}
                  disabled={busy || added}
                  onClick={() => void reference(asset)}
                >
                  <div>
                    {asset.preview || asset.kind === "image" ? (
                      <img src={mediaUrl(asset.preview || asset.path)} alt="" />
                    ) : asset.kind === "video" ? (
                      <VideoCamera size={28} />
                    ) : (
                      <FileText size={28} />
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
          <StatusMessage className="reference-picker-empty">
            {t("正在加载素材…")}
          </StatusMessage>
        ) : (
          !assets.length &&
          !nodes.length && (
            <p className="reference-picker-empty">
              {query
                ? t("没有匹配的资产")
                : t("没有可用输入，请先在资产库建立并确认资产。")}
            </p>
          )
        )}
        <footer>
          <span>
            {selected.size
              ? t("已引用 {v0} 项", { v0: selected.size })
              : t("选择资产或制作结果")}
          </span>
          <Popover.Close>
            <button type="button">{t("完成")}</button>
          </Popover.Close>
        </footer>
      </Popover.Content>
    </Popover.Root>
  );
}
