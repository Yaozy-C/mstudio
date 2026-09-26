import { GenerationReferencePicker } from "./GenerationReferencePicker";
import type { MediaModel } from "../models/mediaRegistry";
import { mediaUrl } from "../bridge";
import { X } from "@phosphor-icons/react";
import type { Project } from "../model";
import type { ProductionTask, ProductionInput } from "./types";
export function GenerationReferences({
  project,
  model,
  draft,
  frames,
  tail,
  frame,
  patch,
}: {
  project: Project;
  model?: MediaModel;
  draft: ProductionTask;
  frames?: boolean;
  tail?: boolean;
  frame: (role: "first-frame" | "last-frame", id: string) => void;
  patch: (value: Partial<ProductionTask>) => void;
}) {
  return (
    <>
      <div className="generation-settings-references">
        <header>
          <span>参考素材</span>
          <GenerationReferencePicker {...{ project, draft, model, patch }} />
        </header>
        {draft.inputs
          .filter(
            (r) =>
              r.assetId &&
              !(["first-frame", "last-frame"].includes(r.role) && frames),
          )
          .map((r) => {
            const a = project.assets.find((a) => a.id === r.assetId);
            const roles: ProductionInput["role"][] =
              draft.kind === "image"
                ? ["edit", "reference"]
                : a?.kind === "video"
                  ? ["video-reference"]
                  : [
                      "reference",
                      ...(frames ? ["first-frame" as const] : []),
                      ...(tail ? ["last-frame" as const] : []),
                    ];
            return (
              <div key={r.key}>
                {a && (
                  <img
                    className="generation-reference-thumbnail"
                    src={mediaUrl(a.preview || a.path)}
                    alt=""
                  />
                )}
                <span title={a?.name}>{a?.name ?? "素材已移除"}</span>
                <select
                  aria-label={`${a?.name}的用途`}
                  value={r.role}
                  onChange={(e) => {
                    const role = e.target.value as ProductionInput["role"];
                    if (role === "first-frame" || role === "last-frame")
                      frame(role, r.assetId);
                    else
                      patch({
                        inputs: draft.inputs.map((v) =>
                          v.key === r.key ? { ...v, role } : v,
                        ),
                      });
                  }}
                >
                  {!roles.includes(r.role) && (
                    <option value={r.role} disabled>
                      {r.role === "first-frame"
                        ? "首帧"
                        : r.role === "last-frame"
                          ? "尾帧"
                          : "原用途"}
                      （需选择支持的模型）
                    </option>
                  )}
                  {roles.map((role) => (
                    <option value={role} key={role}>
                      {
                        {
                          edit: "要修改的图",
                          reference: "内容参考",
                          "video-reference": "动作参考",
                          "first-frame": "首帧",
                          "last-frame": "尾帧",
                          script: "脚本",
                          "video-edit": "原视频",
                        }[role]
                      }
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  aria-label={`移除参考 ${a?.name}`}
                  onClick={() =>
                    patch({
                      inputs: draft.inputs.filter((v) => v.key !== r.key),
                    })
                  }
                >
                  <X size={14} />
                </button>
              </div>
            );
          })}
      </div>
    </>
  );
}
