import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import { t } from "../i18n";
import { contextPolicy } from "./contextLimits";
import type { ModelConnection, TextModelLimits } from "./types";

export function ModelContextFields({
  profile,
  update,
}: {
  profile: ModelConnection;
  update: (value: Partial<ModelConnection>) => void;
}) {
  const [refresh, setRefresh] = useState(0);
  const [loading, setLoading] = useState(false);
  const [notice, setNotice] = useState("");
  const policy = contextPolicy(profile);
  useEffect(() => {
    setLoading(false);
    if (
      !native ||
      !profile.connectionId ||
      !profile.model.trim() ||
      profile.adapter === "codex"
    )
      return;
    let active = true;
    const timer = window.setTimeout(() => {
      setLoading(true);
      setNotice("");
      void bridge<TextModelLimits | null>("resolve_text_model_limits", {
        id: profile.connectionId,
        model: profile.model.trim(),
      })
        .then((limits) => {
          if (active) update({ modelLimits: limits });
        })
        .catch(() => {
          if (active)
            setNotice(t("服务元数据暂不可用，使用模型目录或已保存的容量。"));
        })
        .finally(() => {
          if (active) setLoading(false);
        });
    }, 400);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
    // Only route changes or an explicit refresh may refetch; editing caps never does.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    profile.connectionId,
    profile.endpoint,
    profile.model,
    profile.adapter,
    refresh,
  ]);
  const source = !policy.automatic
    ? t("手动覆盖")
    : profile.adapter === "codex"
      ? t("Codex 运行时报告")
      : policy.limits?.source === "service"
        ? t("服务元数据")
        : policy.limits?.source === "catalog"
          ? t("官方模型目录")
          : t("默认估值（未查到官方容量）");
  return (
    <>
      <div>
        <label>
          {t("上下文窗口（token）")}
          <input
            type="number"
            min={8192}
            max={2000000}
            step={1}
            disabled={policy.automatic}
            value={policy.window ?? ""}
            placeholder={t("运行时获取")}
            onChange={(e) =>
              update({
                contextWindow: Number(e.target.value),
                contextWindowMode: "manual",
              })
            }
          />
        </label>
        <small>{t("来源：{v0}", { v0: source })}</small>
        <label className="model-context-auto">
          <input
            type="checkbox"
            checked={policy.automatic}
            onChange={(e) =>
              update({
                contextWindowMode: e.target.checked ? "auto" : "manual",
                contextWindow: e.target.checked
                  ? null
                  : (policy.window ?? 262144),
              })
            }
          />{" "}
          {t("自动获取上下文窗口")}
        </label>
        {profile.adapter !== "codex" && (
          <button
            type="button"
            disabled={loading || !profile.connectionId || !profile.model}
            onClick={() => setRefresh((n) => n + 1)}
          >
            {loading ? t("正在获取…") : t("刷新模型容量")}
          </button>
        )}
      </div>
      <label>
        {t("输出上限与预留（token）")}
        <input
          type="number"
          min={1}
          max={2000000}
          step={1}
          value={profile.maxOutputTokens ?? ""}
          disabled={
            profile.adapter === "codex" && profile.maxOutputTokens == null
          }
          placeholder={String(policy.output ?? t("服务默认"))}
          onChange={(e) =>
            update({
              maxOutputTokens: e.target.value ? Number(e.target.value) : null,
            })
          }
        />
        <small>
          {t(
            profile.adapter === "codex"
              ? "Codex 的输出上限由本机服务管理"
              : "预留与实际请求的输出上限一致，留空使用模型默认值。",
          )}
        </small>
      </label>
      <label>
        {t("压缩余量（token）")}
        <input
          type="number"
          min={0}
          max={2000000}
          step={1}
          value={profile.compactionHeadroom ?? ""}
          placeholder="65536"
          onChange={(e) =>
            update({
              compactionHeadroom: e.target.value
                ? Number(e.target.value)
                : null,
            })
          }
        />
        <small>{t("预留给压缩摘要及请求增长，默认 65,536。")}</small>
      </label>
      <p className="model-hint model-field-wide" role="status">
        {policy.threshold == null
          ? t("压缩阈值将在服务报告窗口后计算。")
          : t("预计压缩阈值：{v0} token", {
              v0: policy.threshold.toLocaleString(),
            })}{" "}
        {notice}
      </p>
    </>
  );
}
