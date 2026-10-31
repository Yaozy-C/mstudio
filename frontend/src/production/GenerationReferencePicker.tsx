import { canUseTaskReference } from "../workspace/resourceRoles";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { Plus, Check, VideoCamera, MusicNote, X } from "@phosphor-icons/react";
import { ActionButton } from "../ui/ActionButton";
import { mediaUrl } from "../bridge";
import type { Project } from "../model";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";
import { toggleTaskReference, supportsReference } from "./referenceSelection";
import "../styles/reference-picker.css";
import "./generation-reference-dialog.css";

export function GenerationReferencePicker({
  project,
  draft,
  model,
  patch,
}: {
  project: Project;
  draft: ProductionTask;
  model?: MediaModel;
  patch: (value: Partial<ProductionTask>) => void;
}) {
  useLanguage();
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const selected = new Set(
    draft.inputs.filter((r) => r.assetId).map((r) => r.assetId),
  );
  const assets = project.assets.filter(
    (a) =>
      (selected.has(a.id) ||
        (canUseTaskReference(project, a, draft) &&
          supportsReference(a, model))) &&
      a.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  return (
    <Dialog.Root>
      <Dialog.Trigger>
        <ActionButton icon={Plus} className="generation-add-reference">
          {draft.generationPurpose === "asset"
            ? t("选择资产来源")
            : t("添加资产或制作结果")}
        </ActionButton>
      </Dialog.Trigger>
      <Dialog.Content
        className="reference-picker generation-reference-dialog"
        aria-describedby={undefined}
      >
        <header>
          <Dialog.Title>
            {draft.generationPurpose === "asset"
              ? t("选择资产来源")
              : t("资产与制作结果")}
          </Dialog.Title>
          <Dialog.Close>
            <ActionButton icon={X} aria-label={t("关闭")} />
          </Dialog.Close>
        </header>
        <input
          type="search"
          aria-label={t("搜索生成输入")}
          placeholder={t("搜索生成输入")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="reference-picker-grid">
          {assets.map((a) => (
            <button
              type="button"
              key={a.id}
              title={a.name}
              aria-pressed={selected.has(a.id)}
              aria-label={`${selected.has(a.id) ? t("已添加") : t("添加")} ${a.name}`}
              disabled={
                !selected.has(a.id) &&
                model?.capabilities?.referenceLimit !== undefined &&
                selected.size >= model.capabilities.referenceLimit
              }
              onClick={() => {
                try {
                  patch(toggleTaskReference(draft, a, model));
                  setError("");
                } catch (e) {
                  setError(String(e).replace(/^Error: /, ""));
                }
              }}
            >
              <div>
                {a.preview || a.kind === "image" ? (
                  <img src={mediaUrl(a.preview || a.path)} alt="" />
                ) : a.kind === "audio" ? (
                  <MusicNote size={28} />
                ) : (
                  <VideoCamera size={28} />
                )}
                {selected.has(a.id) && (
                  <span className="reference-added">
                    <Check size={15} />
                  </span>
                )}
              </div>
              <span>{a.name}</span>
            </button>
          ))}
        </div>
        {!assets.length && (
          <p className="reference-picker-empty">
            {query
              ? t("没有匹配的素材")
              : t(
                  "没有可用输入。镜头制作使用已确认资产或已有分镜图、视频片段。",
                )}
          </p>
        )}
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <footer>
          <span>
            {t("已引用")} {selected.size} {t("项")}
          </span>
          <Dialog.Close>
            <ActionButton icon={Check}>{t("完成")}</ActionButton>
          </Dialog.Close>
        </footer>
      </Dialog.Content>
    </Dialog.Root>
  );
}
