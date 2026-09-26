import { ErrorNotice } from "../errors/ErrorNotice";
import { useState } from "react";
import { Popover } from "@radix-ui/themes";
import { Plus, Check, VideoCamera } from "@phosphor-icons/react";
import { mediaUrl } from "../bridge";
import type { Project } from "../model";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";
import { addTaskReference, supportsReference } from "./referenceSelection";
import "../styles/reference-picker.css";

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
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const selected = new Set(
    draft.inputs.filter((r) => r.assetId).map((r) => r.assetId),
  );
  const assets = project.assets.filter(
    (a) =>
      supportsReference(a, model) &&
      a.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
  );
  return (
    <Popover.Root>
      <Popover.Trigger>
        <button type="button" className="generation-add-reference">
          <Plus size={14} />
          添加参考素材
        </button>
      </Popover.Trigger>
      <Popover.Content className="reference-picker" side="top" align="start">
        <header>
          <strong>添加项目参考素材</strong>
        </header>
        <input
          type="search"
          aria-label="搜索任务参考素材"
          placeholder="搜索图片或视频"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="reference-picker-grid">
          {assets.map((a) => (
            <button
              type="button"
              key={a.id}
              title={a.name}
              aria-label={`${selected.has(a.id) ? "已添加" : "添加"} ${a.name}`}
              disabled={selected.has(a.id) || selected.size >= 12}
              onClick={() => {
                try {
                  patch(addTaskReference(draft, a, model));
                  setError("");
                } catch (e) {
                  setError(String(e).replace(/^Error: /, ""));
                }
              }}
            >
              <div>
                {a.preview || a.kind === "image" ? (
                  <img src={mediaUrl(a.preview || a.path)} alt="" />
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
              ? "没有匹配的素材"
              : "没有当前模型可用的参考素材；首尾帧请在对应槽位选择。"}
          </p>
        )}
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
        <footer>
          <span>已引用 {selected.size} 项</span>
          <Popover.Close>
            <button type="button">完成</button>
          </Popover.Close>
        </footer>
      </Popover.Content>
    </Popover.Root>
  );
}
